//! Persisted lifecycle freshness state and deterministic fingerprints.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#PHASE-STATE-HOME");

mod fingerprint;
mod identity;
mod store;
mod validate;

pub use fingerprint::{
    FingerprintError, fingerprint_execution, fingerprint_execution_with,
    fingerprint_handler_execution, fingerprint_handler_execution_with,
    preparation_error_fingerprint, preparation_error_fingerprint_for_identity,
};
pub use identity::{RunIdentity, SupersededTrace, select_run_identity};
pub use store::{LifecycleStateError, LifecycleStateStore};

/// The durable-write fault seam. Tests arm it to prove the ordering of a
/// two-step cancellation; it does not exist in a release build.
#[cfg(test)]
pub(crate) use store::inject;

#[cfg(test)]
mod tests;
