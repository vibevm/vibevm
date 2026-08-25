//! Phase 3 — the `#source` fold machinery (PROP-035 §7.3, §8 phase 3), split
//! out of the parent [`pipeline`](super) module along the responsibility seam so
//! neither file breaches the 600-line budget (`conform.toml` `max_file_lines`).
//!
//! The parent composes the migrated parse schedule with the legacy close
//! continuation (topo/source-merge/embed → emit), owns the error layer, the
//! per-node qualification, and the short-link rewriting; this submodule holds
//! the recursive fold closure itself and the pre-fold source-section collision
//! gate (B-056) — the two pieces that walk the `#source` edges and need each
//! source's tree separate from the contract's.

use std::collections::HashMap;
use std::collections::HashSet;

use crate::address::SpecAddress;
use crate::doctree::{DocTree, NodeId, NodeKind};
use crate::embed::SectionSource;
use crate::gate::first_duplicate;
use crate::merge::fold_sources;
use crate::use_graph::{UseGraphError, source_addresses, source_fold_order};

use super::CompileError;

/// Phase 3 — fold a node's whole `#source` closure into one body (PROP-035
/// §7.3, §8 phase 3), **recursively**: a source that itself declares `#source`
/// folds BEFORE it merges into its parent, every node folds once, and a
/// `#source` cycle is judged by the guard [`source_fold_order`] (§9). This is
/// where the recursion law lands: the old fold inlined a declaring source's RAW
/// text, skipping its own fold, so a chain `a #source b #source c` never reached
/// `c`.
///
/// The guard returns the fold order — deepest sources first, `seed` last — so a
/// single pass accumulating each node's folded text lets a parent fold its
/// children's *folded* text. РТ-2: lifted out of [`super::compile_static_inner`]
/// because the phase body grows past readable with the recursion + the
/// per-level gate, and `compile_static_inner` is already long.
///
/// **Fast path.** When the seed declares no `#source`, the order is `[seed]`
/// alone and its text is returned byte-for-byte as authored — no parse, no
/// re-emit — so a no-`#source` lane is unchanged (B-056-L3B acceptance 5).
///
/// **Legal contract cycle (РТ-1).** The guard admits a `#source` cycle whose
/// every node is a contract (the forward-declaration case, §9). In fold order
/// that means a node's member can be its own ANCESTOR — not yet folded when the
/// node is reached. Such a member contributes *nothing*: it is exactly a C++
/// forward declaration, where the cycle closes on a declaration without a body.
/// Substituting the ancestor's raw text instead would double-count (the ancestor
/// later folds this same child in), trip the duplicate-anchor gate, and turn a
/// legal cycle into a build error — so an unfolded member is skipped, not
/// inlined. РТ-3: each member's folded text is parsed into a `DocTree` held in a
/// local `Vec` for the `fold_sources` call only (the slice borrows it); no source
/// text is cloned beyond the one `section_text` fetch per node.
///
/// **Inclusion guard (text dedup).** The owner ruling is that contracts fold in
/// recursively without the *graph* growing — dedup is part of the law, not just
/// an optimisation. The guard walks the `#source` edges once, and the fold
/// honours that at the TEXT level: a node's body enters the compiled document
/// exactly once. A member already inlined somewhere in this closure contributes
/// nothing on a second path, so a diamond `a #source b,c; b,c #source d` yields
/// `d` once — taken by whichever of `b`/`c` the deterministic fold order reaches
/// first (here `b`), then `c` skips it. Without this, a shared source carrying a
/// fact would inline twice and the post-merge gate would sink an ordinary plugin
/// composition (two plugins on a common base) on a surviving duplicate anchor.
/// A member can thus be skipped for one of TWO distinct reasons — see the inline
/// note where the members are gathered; they must not be conflated.
pub(super) fn fold_source_closure(
    seed_text: &str,
    addr: &SpecAddress,
    source: &impl SectionSource,
) -> Result<String, CompileError> {
    // РТ-4: the guard walks the `#source` edges BEFORE any fold text is loaded,
    // so an unreachable source surfaces here as `UseGraphError::Unresolved`
    // (naming that source) — measured: this path fires first, not the per-node
    // `section_text` load below. Normalise it to the pipeline's
    // `CompileError::Unresolved` so the public "cannot load" contract stays
    // stable (a `#source` that won't load is a load failure, not a graph-
    // ordering one) and the seed-level addr attribution is preserved; a true
    // cycle stays a graph error and propagates as `CompileError::UseGraph`.
    let order = source_fold_order(addr, source).map_err(|e| match e {
        UseGraphError::Unresolved { addr, reason } => CompileError::Unresolved { addr, reason },
        other => CompileError::UseGraph(other),
    })?;

    let seed_key = addr.without_pin();

    // Fast path: no `#source` edge reaches back, so the seed is the only node in
    // the fold order — return its text untouched (byte-identical, no parse).
    if order.len() == 1 {
        return Ok(seed_text.to_string());
    }

    // Recursive fold: deepest sources first, seed last. `folded` maps a node key
    // to its folded body, so a parent collects its children's folded text;
    // `included` records whose body is already inlined somewhere in this closure
    // — the inclusion guard that holds every node's text to exactly one copy in
    // the document (a diamond yields the shared source once, not once per path).
    let mut folded: HashMap<String, String> = HashMap::new();
    let mut included: HashSet<String> = HashSet::new();
    for key in &order {
        let node_addr = SpecAddress::parse(key)?;
        let text = if key == &seed_key {
            // Reuse the seed fetch the caller already paid; every other node is
            // loaded once here. (The guard loaded each to walk it, but did not
            // retain the texts.)
            seed_text.to_string()
        } else {
            source
                .section_text(&node_addr)
                .map_err(|reason| CompileError::Unresolved {
                    addr: key.clone(),
                    reason,
                })?
        };

        // This node's own `#source` members, gathered through `source_addresses`
        // — declaration order, each directive EXPANDED (a glob → its sorted
        // members, a point address → itself) — the merge order `fold_sources`
        // honours, and the SAME graph the guard walked (one edge law, one
        // place). A member can be SKIPPED for one of two DISTINCT reasons (do
        // not conflate them):
        //   (1) it is absent from `folded` — an ancestor still on the DFS stack,
        //       i.e. a legal contract cycle's forward declaration (РТ-1): it folds
        //       later and lives at its own level, so it brings no body here;
        //   (2) it is already `included` — its body was inlined on an earlier path
        //       of the deterministic fold order (the inclusion guard). Re-inlining
        //       would duplicate a node whose facts would then collide and sink
        //       the build, so the second path brings nothing. The content is NOT
        //       lost: it lives where the member was first inlined (the first
        //       parent the fold order reached through it), and that parent's body
        //       reaches the seed by the same recursive inclusion.
        let member_trees: Vec<DocTree> = source_addresses(&text, source)
            .map_err(|e| match e {
                UseGraphError::Unresolved { addr, reason } => {
                    CompileError::Unresolved { addr, reason }
                }
                other => CompileError::UseGraph(other),
            })?
            .iter()
            .filter_map(|m| {
                let mk = m.without_pin();
                if included.contains(&mk) {
                    return None; // (2) inclusion guard: body already inlined
                }
                folded.get(&mk).map(|t| {
                    included.insert(mk); // first inline of this member's body
                    DocTree::parse(t)
                }) // a `None` here is (1): the forward-declaration ancestor
            })
            .collect();
        let contract_tree = DocTree::parse(&text);
        let member_refs: Vec<&DocTree> = member_trees.iter().collect();
        let merged = fold_sources(&contract_tree, &member_refs);

        // Re-gate id uniqueness at EVERY level (not just the seed) over the
        // folded view, naming THIS node as the collision site (B-056-L3B
        // acceptance 6): the duplicate arose here, not at whichever node later
        // folds this one in.
        if let Some(dup) = first_duplicate(&DocTree::parse(&merged)) {
            return Err(CompileError::DuplicateId {
                addr: key.clone(),
                dup,
            });
        }

        // B-056: the fact gate above deliberately skips a pure heading-vs-heading
        // repeat (the `:add` artifact), so a section anchor two sources each
        // DEFINE — but the contract does not — slips through when no fact
        // collides to flag it. Catch it here, pre-fold provenance still in hand
        // (the per-member trees parsed above outlive the fold, which borrows
        // them): two source-only definitions of one name is the one-definition
        // rule. A FALLBACK to the gate above — placed after it so a colliding
        // fact still names its more specific id (preserving the gate's existing
        // behaviour); this fires solely for the heading-only hole.
        if let Some(anchor) = first_source_section_collision(&contract_tree, &member_refs) {
            return Err(CompileError::DuplicateSourceSection {
                addr: key.clone(),
                anchor,
            });
        }
        folded.insert(key.clone(), merged);
    }

    // The seed is last in fold order; its folded body is the node's resolved
    // text, and the pipeline continues as before (strip / embed / emit).
    Ok(folded
        .remove(&seed_key)
        .expect("seed is last in fold order, so it is in the accumulator"))
}

/// The first top-level section anchor that **two or more** `members` each
/// declare but the `contract` does **not** (B-056) — a one-definition collision
/// the post-merge [`first_duplicate`] gate cannot see. That gate deliberately
/// skips a pure heading-vs-heading repeat (the accepted `:add` artifact), so a
/// source-only anchor two sources both define slips through whenever no fact
/// collides to flag it. Caught pre-fold — each source's tree still separate —
/// and only as a **fallback** after [`first_duplicate`]: when a fact does
/// collide, the gate already fails the build on the more specific id, so this
/// fires solely for the heading-only hole.
///
/// A section whose anchor the contract ALSO declares is an `:add` sum (one
/// definition, many contributions) — legal, never a collision here. Only a
/// source-only anchor (the contract never declared it) is a fresh definition,
/// and two sources each defining it is two definitions of one name.
///
/// "Top-level section" is measured exactly as [`crate::merge::fold_sources`]
/// measures it — a child of the root that is a heading with an anchor (РТ-B),
/// read through the shared [`heading_anchor`] helper so the fold and this gate
/// can never disagree on what counts as a section. Returns the colliding anchor
/// in source-only emission order (members in slice order, each member's
/// headings in document order) — the name a person reads first. `None` when
/// every source-only anchor is declared by at most one source.
fn first_source_section_collision(contract: &DocTree, members: &[&DocTree]) -> Option<String> {
    // The contract's own top-level anchors: a section matching one is an :add
    // sum (the contract declared it), not a fresh definition.
    let contract_anchors: HashSet<&str> = contract
        .children(contract.root())
        .iter()
        .filter_map(|&c| heading_anchor(contract, c))
        .collect();

    // Distinct-member count per source-only top-level anchor. A single member
    // declaring the same anchor twice is a per-file duplicate (that source's
    // own `duplicate_anchors`), not a two-source collision — it counts once.
    let mut declared_by: HashMap<&str, usize> = HashMap::new();
    for m in members {
        let mut seen_here: HashSet<&str> = HashSet::new();
        for &c in m.children(m.root()) {
            if let Some(a) = heading_anchor(m, c)
                && !contract_anchors.contains(a)
            {
                seen_here.insert(a);
            }
        }
        for a in seen_here {
            *declared_by.entry(a).or_insert(0) += 1;
        }
    }

    // First collision in source-only emission order — the name a person reads
    // first matches the order the fold would emit these sections.
    for m in members {
        for &c in m.children(m.root()) {
            if let Some(a) = heading_anchor(m, c)
                && declared_by.get(a).is_some_and(|&n| n >= 2)
            {
                return Some(a.to_string());
            }
        }
    }
    None
}

/// A root-child's heading anchor, if it is a heading section that declared one
/// — the single notion of "top-level section" the merge and this gate share
/// (РТ-B: no second interpretation of what a top-level section is in the crate).
fn heading_anchor(tree: &DocTree, node: NodeId) -> Option<&str> {
    let n = tree.node(node);
    (n.kind == NodeKind::Heading)
        .then_some(n.id.as_deref())
        .flatten()
}
