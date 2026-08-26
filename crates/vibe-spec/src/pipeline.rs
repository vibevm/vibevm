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
//! 5. **absorb** — named `absorb` projects every normal contribution to its
//!    exact live occurrence order;
//! 6. **link** — named `link` resolves surviving cross-node short references;
//! 7. **emit** — the legacy tail concatenates surviving
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

use crate::address::{SpecAddress, SpecAddressError};
use crate::compiler::ir::{ClosureIr, StaticCompileMode};
use crate::embed::{EmbedError, SectionSource};
use crate::gate::DuplicateId;
use crate::qualify::RenameEntry;
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
/// → embed → qualify → absorb → link schedule, then the legacy assemble/emit tail.
/// Both public modes traverse the same pass list; mode is whole-artifact input
/// state, never a privileged wrapper branch.
fn compile_static_inner(
    seed: &SpecAddress,
    source: &impl SectionSource,
    mode: StaticCompileMode,
) -> Result<(String, Vec<(String, RenameEntry)>), CompileError> {
    let closure = crate::compiler::builtin::compile_linked_closure(seed, source, mode)?;
    compile_static_continuation(closure)
}

/// Bridge from the named link pass into the still-legacy assemble/emit tail.
fn compile_static_continuation(
    closure: ClosureIr,
) -> Result<(String, Vec<(String, RenameEntry)>), CompileError> {
    assert!(
        closure.pending_sources.is_none(),
        "legacy continuation requires the named merge pass"
    );
    assert!(
        closure.pending_embeds.is_none(),
        "legacy continuation requires the named embed pass"
    );
    let out = crate::compiler::link::linked_text(&closure)
        .unwrap_or_else(|error| panic!("legacy continuation received invalid link state: {error}"));

    let renames: Vec<(String, RenameEntry)> = closure
        .renames
        .into_iter()
        .map(|entry| (entry.origin, entry.rename))
        .collect();
    Ok((out, renames))
}

#[cfg(test)]
mod absorb_characterization_tests;
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
mod link_characterization_tests;
#[cfg(test)]
mod merge_characterization_tests;
#[cfg(test)]
mod qualify_characterization_tests;
#[cfg(test)]
mod tests;
