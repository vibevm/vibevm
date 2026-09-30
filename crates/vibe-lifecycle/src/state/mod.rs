//! Persisted lifecycle freshness state and deterministic fingerprints.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#PHASE-STATE-HOME");

mod error;
mod fingerprint;
mod identity;
mod install_stage;
mod io;
mod outcome_marker;
mod outcome_store;
mod recovery;
mod run_journal;
mod source_stage;
mod store;
mod submission;
mod tasks;
mod validate;

pub use error::{LifecycleStateError, PostPublicationRecovery};
pub use fingerprint::{
    FingerprintError, fingerprint_execution, fingerprint_execution_with,
    fingerprint_handler_execution, fingerprint_handler_execution_with,
    preparation_error_fingerprint, preparation_error_fingerprint_for_identity,
};
/// The one-walk prepared-input surface the runner consumes through `state`:
/// the execution fingerprint, its declaration sibling and the input
/// measurement, all from ONE pre-dispatch observation. The two type names
/// cross this path only in tests, so a non-test build would otherwise flag
/// the re-export unused.
#[allow(unused_imports)]
pub(crate) use fingerprint::{
    FramedHash, InputRefusal, ManifestOutcome, PreparedFingerprint, PreparedInputManifest,
    prepare_handler_execution_with,
};
pub use identity::{
    RunIdentity, SupersededTrace, select_run_identity, select_run_identity_for_reservation,
};
pub use install_stage::{InstallEnteredStage, InstallStageError, publish_install_entered_stage};
#[allow(
    unused_imports,
    reason = "C.2 pure validator; owner persistence follows"
)]
pub use outcome_marker::{
    LifecycleOutcomeMarkerError, LifecycleReportPayload, create_lifecycle_outcome_marker,
    validate_lifecycle_outcome_marker,
};
#[allow(unused_imports, reason = "C.2 owner adapter follows retention seam")]
pub use outcome_store::{
    LifecycleOutcomeInspection, LifecycleOutcomeRecoveryReason, LifecycleOutcomeStoreError,
    inspect_retained_lifecycle_outcome, retain_lifecycle_outcome,
};
#[allow(unused_imports, reason = "C.2 preparation; domain adapter follows")]
pub use run_journal::{
    LifecycleRunJournalError, LifecycleRunJournalObservation, inspect_reserved_lifecycle_run,
};
pub(crate) use source_stage::ScriptSourceAttempt;
pub use source_stage::{ScriptSourceStage, ScriptSourceStageError, publish_script_source_stage};
pub use store::LifecycleStateStore;
pub use submission::{
    JournalAdmission, ReservationDecision, SubmissionError, SubmissionNamespace,
    SubmissionReservations, SubmissionScope,
};
pub use tasks::{LifecycleTasksError, pending_hosted_tasks};

/// The durable-write fault seams. Tests arm them to prove the ordering of a
/// transaction; they do not exist in a release build.
#[cfg(test)]
pub(crate) use store::inject;

#[cfg(test)]
mod tests;
