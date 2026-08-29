//! The T8 match-time selector admission gate (R4-TRANSFORM-PLAN-ABI §5–§6.3):
//! the one cell that judges one document's [`DocumentSubject`] against one
//! plan entry's compiled selector, and the only production cell in this
//! subtree that may name the extension kernel's selector surface at all.
//!
//! Five properties of this cell are load-bearing and easy to lose.
//!
//! **This cell exists because the wrapper cell must NOT acquire a kernel
//! selector surface.** `schedule_fence_tests` bans the identifier
//! `vibe_extension_registry` and the method `.matches()` inside `schedule.rs`
//! — the unscoped-subject trap, made mechanical — and that ban stays exactly
//! as it was. Selector evaluation legitimately needs both, so it lives here,
//! under its own fence family that permits precisely those two surfaces and
//! nothing else. [`SelectorGate`] is what makes the split real rather than
//! spelled: the wrapper stores a gate, never a `CompiledSelector`, so the
//! kernel type name never has to reach the wrapper cell — not even through a
//! re-export, which would have satisfied the fence's letter while defeating
//! its purpose.
//!
//! **The verdict table, read off a TOTAL provider.** [`DocumentProvider`] has
//! an answer for every document, and the two absences are two different
//! claims, so the table has four rows rather than three:
//!
//! | subject provider | authored `packages` | verdict |
//! |---|---|---|
//! | a coordinate arm | yes | build the typed kernel identity, ask the kernel |
//! | `Unclaimed` | yes | no match — skip the behavior |
//! | `Undetermined` | yes | refuse, typed |
//! | any | no | evaluate `paths` only; the provider is irrelevant |
//!
//! `Unclaimed` matching nothing is also what the kernel answers for a subject
//! carrying no provider, and that is not a coincidence being inherited: it is
//! the verdict this cell CHOSE, because the address' authority is the package
//! that OWNS a reached document, which is not the question a `packages`
//! dimension asks. Choosing it is expressed by mapping `Unclaimed` onto an
//! absent kernel provider and letting the one glob authority answer, so no
//! second copy of the absent-value rule exists to drift. The same kernel
//! answer would be silently WRONG for `Undetermined` — there a declaring row
//! does exist and merely has no typed spelling yet — so that arm refuses
//! instead, keeping [`super::fault::TransformCapabilityGap::SelectorSubject`]
//! alive, narrowed to exactly the case that is still undecidable.
//!
//! **A refusal at match time does not weaken the construction transaction.**
//! T6b's law is that the whole plan resolves before anything is pushed, first
//! fault wins, nothing pushed on a partial walk — and that law is about
//! CONSTRUCTION. An `Undetermined` refusal cannot be a construction fault,
//! because no document, and therefore no subject, exists when a plan is
//! resolved: the subject is per-document evidence that is born during
//! discovery. So this refusal is in the same class as a behavior fault — a
//! typed, per-entry, run-time refusal the wrapper projects onto the entry's
//! identity — and the construction transaction is untouched. What remains
//! checkable at construction is only what does not need a subject: the
//! entry's registry resolution, its pass name, its pipeline insertion, and
//! the grammar law that lane/emitted carry no selector at all (enforced one
//! layer earlier, by `plan_validate::validate_selector_stage`).
//!
//! **The separator contract is closed here (`BACKLOG.md` `B-117`).** The
//! kernel compiles an authored `paths` glob with `require_literal_separator`,
//! so `\` is not a path separator to any pattern a `declared_path` can be
//! matched against: a backslashed path does not match the wrong rows, it
//! matches NOTHING, silently, and the symptom is a transform that quietly
//! never applies. Three boundaries already refuse such a path — the artifact
//! plan, the wire scalar gate and the inter-pass verifier — but a subject
//! REACHED live through `#use`/`#source`/`#embed` crosses none of them, and
//! `SpecAddress::parse` admits a backslash inside a path segment. This gate
//! is the first place such a subject meets a selector, so it refuses one
//! BEFORE matching, in its own vocabulary. The refusal is deliberately
//! unconditional in the authored dimensions: a malformed path is malformed
//! whether or not today's entry happens to author `paths`, exactly as the
//! three existing boundaries refuse it without asking what will read it. That
//! is the difference from `Undetermined`, which is a well-formed value whose
//! answer is merely unknown and therefore only refuses when something asks.
//!
//! **Nothing here parses a rendered spelling.** The kernel identity is built
//! component by component from the subject's already-validated coordinate
//! members, and the kernel renders it through its own one codec at match
//! time. The path is passed through byte for byte.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#CONTRIB-SELECTOR");

use vibe_core::{Group, PackageName};
use vibe_extension_registry::{
    CompiledSelector, DependencyProviderId, HostIdentity, SelectorProvider, SelectorSubject,
};

use crate::compiler::ir::{DocumentProvider, DocumentSubject};

use super::plan_validate::{BoundedPreview, bounded};

/// Why one document cannot be judged against one entry's selector.
///
/// Both arms are refusals, never verdicts: a document that merely fails to
/// match is [`SelectorVerdict::Skipped`], which is not an error. The wrapper
/// above adds entry identity, and it projects
/// [`SelectorAdmissionError::UndeterminedProvider`] onto
/// [`super::fault::TransformCapabilityGap::SelectorSubject`] — the gap owns
/// the user-facing wording and the pointer to the atom that closes it, while
/// this arm states the local fact. Nothing here knows about plan order, keys
/// or stages.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(crate) enum SelectorAdmissionError {
    #[error(
        "a declared path must be forward-slashed to be matched by an `applies_to.paths` glob compiled with a literal separator; this one {path} would match nothing at all"
    )]
    BackslashedDeclaredPath { path: BoundedPreview },
    #[error(
        "the declaring provider of this document is undetermined; an authored `applies_to.packages` dimension cannot be judged against it"
    )]
    UndeterminedProvider,
}

/// Whether one document is in scope for one entry's behavior.
///
/// [`SelectorVerdict::Skipped`] is a lawful outcome, not a fault: the entry
/// simply does not apply to this document, and the wrapper returns its input
/// untouched.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SelectorVerdict {
    /// The document satisfies every authored dimension; run the behavior.
    Matched,
    /// The document fails at least one authored dimension; skip it.
    Skipped,
}

/// One plan entry's compiled selector, retained for match-time admission.
///
/// A newtype rather than a re-export: it is what keeps the kernel selector
/// type inside this one named cell while the wrapper cell still stores a
/// selector per entry.
#[derive(Debug, Clone)]
pub(crate) struct SelectorGate(CompiledSelector);

impl SelectorGate {
    /// Retain one entry's compiled selector.
    ///
    /// Cloned off the seed exactly once, at schedule resolution, so matching
    /// never reaches back into the plan.
    pub(crate) fn new(selector: &CompiledSelector) -> Self {
        Self(selector.clone())
    }

    /// Judge one document against this entry's selector.
    ///
    /// Refusals come first and in a frozen order: the separator contract
    /// before anything is matched, then the undecidable provider, then the
    /// kernel's own verdict. The path check leads because a backslashed path
    /// is malformed rather than merely unknown — deciding a malformed value
    /// against a glob would answer a question that was never well posed.
    pub(crate) fn admit(
        &self,
        subject: &DocumentSubject,
    ) -> Result<SelectorVerdict, SelectorAdmissionError> {
        #[cfg(test)]
        ADMISSIONS.with(|count| count.set(count.get() + 1));
        let declared_path = subject.declared_path();
        if !DocumentSubject::path_is_forward_slashed(declared_path) {
            return Err(SelectorAdmissionError::BackslashedDeclaredPath {
                path: bounded(declared_path),
            });
        }
        let identity = self.provider_identity(subject.provider())?;
        #[cfg(test)]
        MATCH_EVALUATIONS.with(|count| count.set(count.get() + 1));
        let query = SelectorSubject::scoped(
            identity.as_ref().map(ProviderIdentity::borrowed),
            Some(declared_path),
        );
        if self.0.matches(query) {
            Ok(SelectorVerdict::Matched)
        } else {
            Ok(SelectorVerdict::Skipped)
        }
    }

    /// The typed kernel identity this subject's provider presents to a
    /// `packages` dimension, or the reason it cannot present one.
    ///
    /// Absence is returned for both `Unclaimed` and — when nothing asks — for
    /// `Undetermined`, but the two reach it for different reasons and only
    /// one of them can also refuse; fusing them is precisely the mistake the
    /// two arms exist to prevent.
    fn provider_identity(
        &self,
        provider: &DocumentProvider,
    ) -> Result<Option<ProviderIdentity>, SelectorAdmissionError> {
        let identity = match provider {
            DocumentProvider::Dependency { group, name } => {
                ProviderIdentity::Dependency(coordinate(group, name))
            }
            DocumentProvider::HostUngrouped { name } => {
                ProviderIdentity::Host(HostIdentity::ungrouped_project(name.clone()))
            }
            DocumentProvider::HostCoordinate { group, name } => {
                ProviderIdentity::Host(HostIdentity::coordinate(coordinate(group, name)))
            }
            DocumentProvider::HostVirtualWorkspace => {
                ProviderIdentity::Host(HostIdentity::virtual_workspace())
            }
            // Chosen, not inherited: no contribution row declared this
            // document, so there is no owner a `packages` dimension could
            // name, and the kernel's absent-value answer is the final one.
            DocumentProvider::Unclaimed => return Ok(None),
            // A row DID declare this document; its owner exists and merely
            // has no typed spelling yet, so an authored dimension gets a
            // refusal rather than a confident `false`.
            DocumentProvider::Undetermined if self.0.package_patterns().is_some() => {
                return Err(SelectorAdmissionError::UndeterminedProvider);
            }
            // Nothing asked, so the unknown answer is not needed and the
            // `paths` dimension alone decides.
            DocumentProvider::Undetermined => return Ok(None),
        };
        Ok(Some(identity))
    }
}

/// One owned kernel provider identity, alive for the length of one match.
///
/// The kernel's [`SelectorProvider`] borrows its identity, so the value it
/// borrows has to outlive the call; this is that value, built from validated
/// components and never from a parsed spelling.
enum ProviderIdentity {
    Dependency(DependencyProviderId),
    Host(HostIdentity),
}

impl ProviderIdentity {
    /// The borrowed kernel coordinate this identity presents.
    fn borrowed(&self) -> SelectorProvider<'_> {
        match self {
            Self::Dependency(id) => SelectorProvider::Dependency(id),
            Self::Host(identity) => SelectorProvider::Host(identity),
        }
    }
}

/// The kernel's versionless coordinate, rebuilt from validated components.
fn coordinate(group: &Group, name: &PackageName) -> DependencyProviderId {
    DependencyProviderId::new(group.clone(), name.clone())
}

// The gate's own instrumentation. A behavior's invocation counter proves
// which documents RAN; these two prove the gate was consulted per document
// and how often it reached the kernel at all — a distinction a single
// counter cannot make. Thread-local because the suite runs tests in
// parallel.
#[cfg(test)]
std::thread_local! {
    static ADMISSIONS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static MATCH_EVALUATIONS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
pub(crate) fn reset_selector_admission_counts() {
    ADMISSIONS.with(|count| count.set(0));
    MATCH_EVALUATIONS.with(|count| count.set(0));
}

/// The two gate counts `(admissions, match evaluations)`.
#[cfg(test)]
pub(crate) fn selector_admission_counts() -> (usize, usize) {
    (
        ADMISSIONS.with(std::cell::Cell::get),
        MATCH_EVALUATIONS.with(std::cell::Cell::get),
    )
}
