//! What an install really did — the one honest, slot-level progress record.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#install-workflow-in-detail");

use specmark::spec;

/// What an install really did to the dependency slots, measured AT the
/// mutation boundary rather than copied out of a successful return value.
///
/// The engine observes slots — directories — so every member here is a slot
/// path or a regenerated node path. Nothing in this type is a file count: the
/// install layer has never walked a slot's contents, and a report that claimed
/// otherwise would be inventing a census.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/VIBEVM-SPEC#install-workflow-in-detail")]
pub struct InstallProgress {
    /// Whether the apply ran to completion. False on a partial record — one
    /// captured because the invocation stopped part-way.
    pub complete: bool,
    /// Whether the PROP-011 §2.2 fresh fast path short-circuited the run.
    pub fresh: bool,
    pub materialised: Vec<String>,
    pub skipped: Vec<String>,
    pub pruned: Vec<String>,
    pub nodes_regenerated: Vec<String>,
}

impl InstallProgress {
    /// The complete record of a finished apply.
    #[must_use]
    pub fn complete(outcome: &vibe_workspace::install::InstallOutcome) -> Self {
        Self {
            complete: true,
            fresh: false,
            materialised: outcome.materialised.clone(),
            skipped: outcome.skipped.clone(),
            pruned: outcome.pruned.clone(),
            nodes_regenerated: outcome.nodes_regenerated.clone(),
        }
    }

    /// The fresh fast path: the lock was unchanged, so no slot moved.
    #[must_use]
    pub fn fresh(nodes_regenerated: Vec<String>) -> Self {
        Self {
            complete: true,
            fresh: true,
            nodes_regenerated,
            ..Self::default()
        }
    }
}
