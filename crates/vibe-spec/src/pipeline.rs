//! The static compilation pipeline (PROP-035 §8) — the primitives, composed.
//!
//! `compile_static` runs the phases in the fixed order the spec pins:
//!
//! 1. **parse / close** — the scheduler loads the finite addressed worklist;
//!    named `parse` lowers each source, then named `close` owns explicit-`#use`
//!    graph cycles and dependency-before-dependent order (§7.2, §8 phase 2);
//! 2. **source-merge** — named `merge` folds every declared `#source` into
//!    `contract`, in declaration order (§7.3);
//! 3. **embed-expand** — named `embed` splices every surviving `#embed` to a
//!    fixed point (§7.1);
//! 4. **qualify** — named `qualify` lowers aliases in both modes, plans
//!    READ-ONCE absorption, and qualifies live node labels in qualified mode;
//! 5. **emit** — the legacy tail absorbs, links, and concatenates surviving
//!    nodes in topological order, each wrapped in
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
use std::fmt::Write as _;

use crate::address::{SpecAddress, SpecAddressError};
use crate::compiler::ir::{
    ClosureContribution, ClosureIr, ContributionAbsorption, DocumentAddress, QualificationState,
    StaticCompileMode,
};
use crate::embed::{EmbedError, SectionSource};
use crate::gate::DuplicateId;
use crate::qualify::{RenameEntry, read_anchor_id};
use crate::use_graph::UseGraphError;

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
    /// Two or more sources each declare a top-level section the contract does
    /// **not**, under one anchor — two definitions of one name (B-056). A
    /// section matching a contract anchor is an `:add` sum (one definition,
    /// many contributions); a section the contract never declared is a fresh
    /// definition, and a name has one. The post-merge [`DuplicateId`] gate
    /// cannot see this: by the time it runs, provenance is folded away and the
    /// repeat reads as the accepted `:add` artifact. Caught pre-fold, where each
    /// source's tree is still separate — and only as a fallback after
    /// [`DuplicateId`], so a colliding fact still names the more specific id.
    #[error(
        "merged {addr}: section `{anchor}` is defined by more than one source \
             but not by the contract — a source section matching a contract anchor \
             is an :add sum, but one the contract never declared is a definition, \
             and a name has one definition"
    )]
    DuplicateSourceSection { addr: String, anchor: String },
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
    let (out, _) = compile_static_inner(seed, source, StaticCompileMode::Plain)?;
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
    compile_static_inner(seed, source, StaticCompileMode::QualifyPerNode)
}

/// The shared phase loop (PROP-035 §8): declared parse → gather → close → merge
/// → embed → qualify schedule, then the legacy absorb/link/assemble/emit tail.
/// Both public modes traverse the same pass list; mode is whole-artifact input
/// state, never a privileged wrapper branch.
fn compile_static_inner(
    seed: &SpecAddress,
    source: &impl SectionSource,
    mode: StaticCompileMode,
) -> Result<(String, Vec<(String, RenameEntry)>), CompileError> {
    let closure = crate::compiler::builtin::compile_qualified_closure(seed, source, mode)?;
    compile_static_continuation(closure)
}

/// Bridge from the named qualify pass into the still-legacy artifact tail.
///
/// Absorb consumes the occurrence-aligned plan produced from pre-rewrite text;
/// it never recomputes against qualified bodies. Link remains the sole
/// cross-node short-name owner. Assembly/emission only render surviving nodes.
fn compile_static_continuation(
    mut closure: ClosureIr,
) -> Result<(String, Vec<(String, RenameEntry)>), CompileError> {
    assert!(
        closure.pending_sources.is_none(),
        "legacy continuation requires the named merge pass"
    );
    assert!(
        closure.pending_embeds.is_none(),
        "legacy continuation requires the named embed pass"
    );
    let mode = match closure.qualification {
        QualificationState::Applied(mode) => mode,
        QualificationState::Pending(_) => {
            panic!("legacy continuation requires the named qualify pass")
        }
    };
    let absorption = closure
        .absorption
        .take()
        .expect("legacy continuation requires qualify's absorption plan");
    crate::compiler::qualify::validate_absorption(&absorption, &closure).unwrap_or_else(|error| {
        panic!("legacy absorb received invalid qualification state: {error}")
    });
    let (emission_order, occurrences) = match (
        closure.contributions.as_slice(),
        absorption.contributions.as_slice(),
    ) {
        (
            [ClosureContribution::Normal { emission_order, .. }],
            [ContributionAbsorption::Normal { occurrences, .. }],
        ) => (emission_order.clone(), occurrences.clone()),
        _ => unreachable!("one-seed compatibility close returns one normal contribution"),
    };
    assert_eq!(
        emission_order.len(),
        occurrences.len(),
        "legacy absorb requires an identity-bound occurrence sequence"
    );

    let mut out = String::new();
    for (node_id, occurrence) in emission_order.into_iter().zip(occurrences) {
        assert_eq!(
            node_id, occurrence.node,
            "legacy absorb requires identity-bound occurrence alignment"
        );
        if occurrence.absorbed {
            continue;
        }
        let node = &closure.nodes[node_id.0];
        let DocumentAddress::Spec(address) = &node.address else {
            unreachable!("one-seed compatibility close contains only spec addresses")
        };
        let key = address.without_pin();
        let emitted = node.tree.text(node.tree.root());

        writeln!(out, "{}", crate::markers::open(&key)).unwrap(); // phase 5
        out.push_str(&emitted);
        if !emitted.ends_with('\n') {
            out.push('\n');
        }
        writeln!(out, "{}", crate::markers::close(&key)).unwrap();
    }

    let renames: Vec<(String, RenameEntry)> = closure
        .renames
        .into_iter()
        .map(|entry| (entry.origin, entry.rename))
        .collect();
    if matches!(mode, StaticCompileMode::QualifyPerNode) {
        // Second pass — resolve the cross-node short references the per-node
        // qualify could not see (B-006 rider).
        out = resolve_cross_node_short_links(&out, &renames)?;
    }
    Ok((out, renames))
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
mod characterization_tests;
#[cfg(test)]
mod collision_tests;
#[cfg(test)]
mod embed_characterization_tests;
#[cfg(test)]
mod fold_tests;
#[cfg(test)]
mod inheritance_parity_tests;
#[cfg(test)]
mod merge_characterization_tests;
#[cfg(test)]
mod qualify_characterization_tests;
#[cfg(test)]
mod tests;
