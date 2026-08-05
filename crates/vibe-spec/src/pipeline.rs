//! The static compilation pipeline (PROP-035 §8) — the primitives, composed.
//!
//! `compile_static` runs the phases in the fixed order the spec pins:
//!
//! 1. **parse / topo** — build the `#use` graph from the seed and order it so
//!    every dependency precedes its dependents (§7.2, §8 phase 2);
//! 2. **source-merge** — fold every declared `#source` into `contract`, in
//!    declaration order (§7.3);
//! 3. **embed-expand** — splice every `#embed` to a fixed point (§7.1);
//! 4. **emit** — concatenate the nodes in topological order, each wrapped in
//!    open/close markers (§11), so the output is reversible.
//!
//! A `#use` line is *resolved by the ordering* — its target is emitted, once,
//! above — so the line itself is stripped from a node's body on emit; it would
//! otherwise be a dangling directive in the compiled `STATIC.md`. `@spec`
//! in-place references are left in prose (their target is likewise already
//! above). No `#embed` survives (§7.1).
//!
//! This is the algorithmic, LLM-free static compiler (§2) — the reference
//! semantics the structural loader is later checked against.

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt::Write as _;

use crate::address::{Authority, SpecAddress, SpecAddressError};
use crate::directives::{DirectiveKind, Directives};
use crate::doctree::DocTree;
use crate::embed::{EmbedError, SectionSource, expand_embeds};
use crate::gate::{DuplicateId, first_duplicate};
use crate::merge::fold_sources;
use crate::qualify::{RenameEntry, qualify_contribution, read_anchor_id};
use crate::use_graph::{UseGraphError, source_fold_order, topo_order_from};

/// Why static compilation failed.
#[derive(Debug, thiserror::Error)]
pub enum CompileError {
    #[error(transparent)]
    UseGraph(#[from] UseGraphError),
    #[error(transparent)]
    Embed(#[from] EmbedError),
    #[error("internal: re-parsing topo key `{0}` failed")]
    Address(#[from] SpecAddressError),
    #[error("cannot load {addr}: {reason}")]
    Unresolved { addr: String, reason: String },
    /// The `#source` merge produced a document whose anchor namespace is no
    /// longer unique — a collision the per-fact override did not cancel
    /// (PROP-035 §7.3, clause 3). Fails the build; never a warning.
    #[error("merged {addr}: {dup}")]
    DuplicateId { addr: String, dup: DuplicateId },
    /// A short reference `(#x)` in the compiled closure names a label two or
    /// more nodes define (B-006 rider / PROP-035 §8 phase 5). Per-node
    /// qualification already resolved every within-node reference, so this
    /// fires only on a *cross-node* short link the compiler cannot attribute
    /// without guessing. Fails the build citing the candidate qualified heirs
    /// (B-011: fail with candidates, never a silent pick); the author must cite
    /// one explicitly.
    #[error("ambiguous short link `{label}`: defined by {}", .candidates.join(", "))]
    AmbiguousShortLink {
        label: String,
        candidates: Vec<String>,
    },
}

/// Compile the closure reachable from `seed` into a single static document —
/// the **unqualified** reference semantics (PROP-035 §2) the structural loader
/// is later checked against. See [`compile_static_qualified`] for the per-node
/// origin-qualified compile a `normal` static lane ships.
pub fn compile_static(
    seed: &SpecAddress,
    source: &impl SectionSource,
) -> Result<String, CompileError> {
    let (out, _) = compile_static_inner(seed, source, CompileMode::Plain)?;
    Ok(out)
}

/// Compile the closure reachable from `seed` and qualify **every node under its
/// own authoring origin** (PROP-035 §8 phase 5, B-006 rider).
///
/// Unlike [`compile_static`], each emitted node is passed through
/// [`qualify_contribution`] under the origin derived from its topo key
/// (`<group>/<name>`, or the host token) — so a node a `normal` package splices
/// in from *another* package via `#use` is qualified under THAT package's
/// origin, never the entry's. Returns the compiled lane alongside the per-node
/// rename map (`(origin, rename)`, in emit order) for the tombstone.
///
/// A second pass then resolves the cross-node short references the per-node
/// qualify leaves behind: a `(#x)` in node A whose target lives in node B is
/// rewritten to B's qualified heir; a label two or more nodes define is a build
/// error ([`CompileError::AmbiguousShortLink`]) citing the candidates; a label
/// no node defines is left for the loader's two-scope lookup.
pub fn compile_static_qualified(
    seed: &SpecAddress,
    source: &impl SectionSource,
) -> Result<(String, Vec<(String, RenameEntry)>), CompileError> {
    compile_static_inner(seed, source, CompileMode::QualifyPerNode)
}

/// Whether [`compile_static_inner`] qualifies each node under its own origin.
#[derive(Clone, Copy)]
enum CompileMode {
    /// Reference semantics — labels emitted as authored (the structural
    /// loader's oracle).
    Plain,
    /// Per-node origin qualification (PROP-035 §8 phase 5, B-006 rider).
    QualifyPerNode,
}

/// The shared phase loop (PROP-035 §8): parse/topo → source-merge → embed →
/// emit. In [`CompileMode::QualifyPerNode`] each node is qualified under its
/// own origin before emission and a second pass resolves cross-node short
/// references; in [`CompileMode::Plain`] the body is emitted as-authored and
/// the rename map is empty. One loop, parameterised by mode — never two copies
/// of the phase body (B-006 rider).
fn compile_static_inner(
    seed: &SpecAddress,
    source: &impl SectionSource,
    mode: CompileMode,
) -> Result<(String, Vec<(String, RenameEntry)>), CompileError> {
    let order = topo_order_from(seed, source)?; // phase 2
    let qualify = matches!(mode, CompileMode::QualifyPerNode);

    let mut out = String::new();
    let mut renames: Vec<(String, RenameEntry)> = Vec::new();
    for key in &order {
        let addr = SpecAddress::parse(key)?;
        let text = source
            .section_text(&addr)
            .map_err(|reason| CompileError::Unresolved {
                addr: key.clone(),
                reason,
            })?;

        // phase 3 — fold the node's whole `#source` closure RECURSIVELY (§7.3,
        // §8 phase 3): a source that itself declares `#source` folds before it
        // merges into its parent, every node folds once, and a cycle is judged
        // by `source_fold_order`. See [`fold_source_closure`] for the recursion,
        // the legal-cycle forward-declaration rule, and the per-level gate.
        let folded = fold_source_closure(&text, &addr, source)?;
        // phase 4 — embed over the use/source-resolved body.
        let body = strip_directive_lines(&folded, &[DirectiveKind::Use, DirectiveKind::Source]);
        let expanded = expand_embeds(&body, source)?;

        // B-011 §7.4 (PROP-035 §8 phase 5): rewrite every `@!<Alias>` to the
        // full `@spec://<target>` it denotes. The alias table is parsed from the
        // pre-strip `folded` text, so the `#use … as <Alias>` bindings survive
        // even though the declaration lines themselves are stripped above (they
        // leave the body together with every other `#use` line). The compiled
        // lane is then self-describing without the alias table, and resolvable
        // after any future cleaning — the alias binds to the address, never to
        // compiled text.
        let aliases = Directives::parse(&folded).aliases;
        let emitted = rewrite_at_bang(&expanded, &aliases);

        // B-006 rider (PROP-035 §8 phase 5): in qualified mode each node's
        // emitted body is qualified under ITS OWN authoring origin — derived
        // from the topo key the same way `normal_seed` derives a package
        // coordinate — never the entry's, so a node spliced in from another
        // package keeps its true provenance. Per-node, so a node referencing
        // its own label is resolved within the node; a cross-node short link is
        // left for the second pass below.
        let emitted = if qualify {
            let origin = node_origin(&addr);
            let (qualified, node_renames) = qualify_contribution(&emitted, &origin);
            renames.extend(node_renames.into_iter().map(|r| (origin.clone(), r)));
            qualified
        } else {
            emitted
        };

        writeln!(out, "{}", crate::markers::open(key)).unwrap(); // phase 5
        out.push_str(&emitted);
        if !emitted.ends_with('\n') {
            out.push('\n');
        }
        writeln!(out, "{}", crate::markers::close(key)).unwrap();
    }

    if qualify {
        // Second pass — resolve the cross-node short references the per-node
        // qualify could not see (B-006 rider).
        out = resolve_cross_node_short_links(&out, &renames)?;
    }
    Ok((out, renames))
}

/// The authoring origin of a closure node — `<group>/<name>` for a package, the
/// host token for the host project (PROP-035 §6). Derived from the node's topo
/// key by the same authority half `normal_seed` builds a coordinate from, so a
/// node compiled from another package's `#use` target is qualified under THAT
/// package's origin, not the entry's (B-006 rider).
fn node_origin(addr: &SpecAddress) -> String {
    match &addr.authority {
        Authority::Host(h) => h.clone(),
        Authority::Package { group, name, .. } => format!("{group}/{name}"),
    }
}

/// Every `#source` address a document declares, in declaration order (§7.3).
/// `Directives::parse` collects directives top-to-bottom by source line, so the
/// order here is the order the author wrote them — the merge order the fold
/// honours.
fn source_directives(text: &str) -> Vec<SpecAddress> {
    Directives::parse(text)
        .directives
        .into_iter()
        .filter(|d| d.kind == DirectiveKind::Source)
        .map(|d| d.address)
        .collect()
}

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
/// children's *folded* text. РТ-2: lifted out of [`compile_static_inner`] because
/// the phase body grows past readable with the recursion + the per-level gate,
/// and `compile_static_inner` is already long.
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
fn fold_source_closure(
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

        // This node's own `#source` members, in declaration order — the merge
        // order `fold_sources` honours. A member can be SKIPPED for one of two
        // DISTINCT reasons (do not conflate them):
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
        let member_trees: Vec<DocTree> = source_directives(&text)
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
        folded.insert(key.clone(), merged);
    }

    // The seed is last in fold order; its folded body is the node's resolved
    // text, and the pipeline continues as before (strip / embed / emit).
    Ok(folded
        .remove(&seed_key)
        .expect("seed is last in fold order, so it is in the accumulator"))
}

/// Remove directive lines of the given kinds. `#use` is resolved by the
/// ordering and `#source` by the fold, so both would be leftovers in the
/// compiled output.
fn strip_directive_lines(text: &str, kinds: &[DirectiveKind]) -> String {
    let directives = Directives::parse(text);
    let strip: HashSet<usize> = directives
        .directives
        .iter()
        .filter(|d| kinds.contains(&d.kind))
        .map(|d| d.line)
        .collect();

    let kept: Vec<&str> = text
        .lines()
        .enumerate()
        .filter(|(i, _)| !strip.contains(i))
        .map(|(_, line)| line)
        .collect();
    kept.join("\n")
}

/// Rewrite every `@!<Alias>` in `text` to the full `@spec://<target>` its alias
/// binds to (B-011 §7.4 / PROP-035 §8 phase 5). Fenced code blocks are left
/// untouched (the shared fence mask). An `@!X` whose `X` is not a declared alias
/// is left in place — it is already a `DirectiveError` the scan recorded, and
/// the rewrite must not silently drop prose. The fast path (no aliases) returns
/// the text unchanged, so a directive-free lane is byte-identical.
fn rewrite_at_bang(text: &str, aliases: &BTreeMap<String, SpecAddress>) -> String {
    if aliases.is_empty() {
        return text.to_string();
    }
    let lines: Vec<String> = text.split('\n').map(String::from).collect();
    let fenced = crate::doctree::fence_mask(&lines);
    let out_lines: Vec<String> = lines
        .iter()
        .enumerate()
        .map(|(i, line)| {
            if fenced[i] {
                line.clone()
            } else {
                rewrite_at_bang_line(line, aliases)
            }
        })
        .collect();
    out_lines.join("\n")
}

/// Rewrite `@!<Alias>` occurrences in a single non-fenced line, leaving
/// everything else byte-identical. The identifier boundary reuses
/// [`directives::identifier_run`] so this rewrite and the scanner can never
/// disagree on what counts as a name.
fn rewrite_at_bang_line(line: &str, aliases: &BTreeMap<String, SpecAddress>) -> String {
    let bytes = line.as_bytes();
    let mut out = String::with_capacity(line.len());
    let mut last = 0usize; // first not-yet-flushed byte (exclusive boundary)
    let mut i = 0usize;
    while i + 1 < bytes.len() {
        if bytes[i] == b'@' && bytes[i + 1] == b'!' {
            let id = crate::directives::identifier_run(&line[i + 2..]);
            if !id.is_empty()
                && let Some(target) = aliases.get(id)
            {
                out.push_str(&line[last..i]);
                out.push('@');
                out.push_str(&target.without_pin());
                let after = i + 2 + id.len();
                last = after;
                i = after;
                continue;
            }
        }
        i += 1;
    }
    out.push_str(&line[last..]);
    out
}

/// Second pass of per-node qualification (B-006 rider / PROP-035 §8 phase 5):
/// resolve the cross-node short references the per-node qualify left behind.
///
/// After every node's labels are qualified under its own origin, a `(#x)` in
/// node A whose target lives in node B is still bare — node A's qualify pass
/// could not see B's labels. This pass walks the assembled lane (outside fenced
/// code) and rewrites each remaining `(#x)` against the union of every node's
/// definitions: a label one node defines → that node's qualified heir; a label
/// ≥2 nodes define → a build error ([`CompileError::AmbiguousShortLink`]) citing
/// the candidates (B-011: fail with candidates, never a silent pick); a label no
/// node defines → left as written (resolving it is the loader's two-scope
/// lookup, not the compiler's).
fn resolve_cross_node_short_links(
    text: &str,
    renames: &[(String, RenameEntry)],
) -> Result<String, CompileError> {
    // The union map: short label → every (origin, qualified heir) that defines
    // it, across the whole closure. Built from the per-node rename maps.
    let mut defs: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
    for (origin, r) in renames {
        defs.entry(r.original.clone())
            .or_default()
            .push((origin.clone(), r.qualified.clone()));
    }

    // Split on '\n' (not `lines()`) so a trailing newline round-trips; reuse the
    // qualify cell's fence mask so this pass and the per-node pass agree on what
    // is code.
    let lines: Vec<String> = text.split('\n').map(String::from).collect();
    let fenced = crate::doctree::fence_mask(&lines);
    let mut out_lines: Vec<String> = Vec::with_capacity(lines.len());
    for (i, line) in lines.iter().enumerate() {
        if fenced[i] {
            out_lines.push(line.clone());
        } else {
            out_lines.push(rewrite_cross_node_links(line, &defs)?);
        }
    }
    Ok(out_lines.join("\n"))
}

/// Rewrite the remaining `(#x)` short references in one non-fenced line against
/// the union definition map (B-006 rider). Inline-code spans are skipped via the
/// same backtick toggle the qualify cell uses, and the anchor id is read with
/// the qualify cell's [`read_anchor_id`] scanner so the two passes never disagree
/// on what counts as a name. References already qualified by the per-node pass
/// (a `<slug>--<id>` form) are not keys in `defs` and so pass through untouched.
fn rewrite_cross_node_links(
    line: &str,
    defs: &BTreeMap<String, Vec<(String, String)>>,
) -> Result<String, CompileError> {
    let bytes = line.as_bytes();
    let mut out = String::with_capacity(line.len());
    let mut last = 0usize; // first not-yet-flushed byte (exclusive boundary)
    let mut i = 0usize;
    let mut in_code = false;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'`' {
            in_code = !in_code;
            i += 1;
            continue;
        }
        if !in_code
            && b == b'('
            && bytes.get(i + 1) == Some(&b'#')
            && let Some((id, after_id)) = read_anchor_id(bytes, i + 2)
            && bytes.get(after_id) == Some(&b')')
        {
            match defs.get(id) {
                Some(heirs) if heirs.len() == 1 => {
                    // A unique definer → rewrite to its qualified heir.
                    out.push_str(&line[last..i]);
                    out.push_str("(#");
                    out.push_str(&heirs[0].1);
                    out.push(')');
                    last = after_id + 1;
                    i = after_id + 1;
                    continue;
                }
                Some(heirs) => {
                    // ≥2 definers → ambiguous: fail citing the candidates.
                    let mut candidates: Vec<String> = heirs
                        .iter()
                        .map(|(origin, qualified)| format!("{qualified} ({origin})"))
                        .collect();
                    candidates.sort();
                    return Err(CompileError::AmbiguousShortLink {
                        label: id.to_string(),
                        candidates,
                    });
                }
                None => {} // no definer → leave the reference as written
            }
        }
        i += 1;
    }
    out.push_str(&line[last..]);
    Ok(out)
}

#[cfg(test)]
mod fold_tests;
#[cfg(test)]
mod tests;
