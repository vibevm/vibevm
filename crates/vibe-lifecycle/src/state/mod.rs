//! Persisted lifecycle freshness state and deterministic fingerprints.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#PHASE-STATE-HOME");

mod fingerprint;
mod store;

pub use fingerprint::{FingerprintError, fingerprint_execution, preparation_error_fingerprint};
pub use store::{LifecycleStateError, LifecycleStateStore};

#[cfg(test)]
mod tests;
