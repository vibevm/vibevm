//! Read-only evidence for one externally reserved lifecycle run identity.
//!
//! The lifecycle state is a single current-run freshness record, not a
//! terminal result journal. Even all-`ok` execution rows cannot prove that
//! the requested operation completed. This reader never allocates a run,
//! repairs state, or dispatches a handler.

use crate::state::current::LifecycleState;

use super::{LifecycleStateError, LifecycleStateStore};
use crate::LifecycleLease;

/// The exact validated current state is retained as evidence, but no
/// terminal outcome is inferred from its execution rows.
#[derive(Debug)]
pub enum LifecycleRunJournalObservation {
    /// No lifecycle state file exists under the leased root.
    Absent,
    /// The current header names the reserved run; inspect the domain owner
    /// before any replay because terminal outcome is not recorded here.
    RecoveryRequired { state: LifecycleState },
    /// A state file exists, but it has no attributable run ID. It must not be
    /// mistaken for an absent journal that may be initialized from scratch.
    Uncertain { state: LifecycleState },
}

#[derive(Debug, thiserror::Error)]
pub enum LifecycleRunJournalError {
    #[error("reserved lifecycle run ID is blank, overlong or control-bearing")]
    InvalidRunId,
    #[error("lifecycle state belongs to run `{observed}`, not reserved run `{reserved}`")]
    ForeignCurrent { reserved: String, observed: String },
    #[error(transparent)]
    State(#[from] LifecycleStateError),
}

/// Read through the *already-held* workspace lease and the ordinary
/// validated state reader. The caller supplies a run ID reserved by its
/// owning admission path; this value alone does not grant an effect.
#[allow(dead_code, reason = "C.2 preparation; a domain adapter has not landed")]
pub fn inspect_reserved_lifecycle_run(
    lease: &LifecycleLease,
    reserved_run_id: &str,
) -> Result<LifecycleRunJournalObservation, LifecycleRunJournalError> {
    if reserved_run_id.is_empty()
        || reserved_run_id.len() > 256
        || reserved_run_id.trim() != reserved_run_id
        || reserved_run_id.chars().any(char::is_control)
    {
        return Err(LifecycleRunJournalError::InvalidRunId);
    }
    let Some(state) = LifecycleStateStore::peek_with_lease(lease)? else {
        return Ok(LifecycleRunJournalObservation::Absent);
    };
    match state.run.run_id.as_deref() {
        Some(observed) if observed == reserved_run_id => {
            Ok(LifecycleRunJournalObservation::RecoveryRequired { state })
        }
        Some(observed) => Err(LifecycleRunJournalError::ForeignCurrent {
            reserved: reserved_run_id.to_owned(),
            observed: observed.to_owned(),
        }),
        None => Ok(LifecycleRunJournalObservation::Uncertain { state }),
    }
}
