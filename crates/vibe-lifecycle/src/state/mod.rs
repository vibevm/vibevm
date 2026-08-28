//! Persisted lifecycle freshness state and deterministic fingerprints.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#PHASE-STATE-HOME");

mod error;
mod fingerprint;
mod identity;
mod io;
mod recovery;
mod store;
mod tasks;
mod validate;

pub use error::{LifecycleStateError, PostPublicationRecovery};
pub use fingerprint::{
    FingerprintError, fingerprint_execution, fingerprint_execution_with,
    fingerprint_handler_execution, fingerprint_handler_execution_with,
    preparation_error_fingerprint, preparation_error_fingerprint_for_identity,
};
/// The one-walk prepared-input surface A4b's runner consumes through `state`.
/// Until that atom lands, only the tests read these names, so a non-test
/// build would otherwise flag the re-export unused.
#[allow(unused_imports)]
pub(crate) use fingerprint::{
    PreparedFingerprint, PreparedInputManifest, prepare_handler_execution_with,
};
pub use identity::{RunIdentity, SupersededTrace, select_run_identity};
pub use store::LifecycleStateStore;
pub use tasks::{LifecycleTasksError, pending_hosted_tasks};

/// The durable-write fault seams. Tests arm them to prove the ordering of a
/// transaction; they do not exist in a release build.
#[cfg(test)]
pub(crate) use store::inject;

#[cfg(test)]
mod tests;
