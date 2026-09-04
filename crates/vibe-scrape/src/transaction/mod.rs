//! Recoverable transaction core for PROP-056 export and in-place scrape.
//!
//! Its public traits form the narrow integration seam between the finalized
//! prepared scrape, health backend, generated report mapper, durable store and
//! safefs directory primitives.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-056#IMPL-E");

mod engine;
mod gap;
mod integration;
mod model;
mod report;
mod safefs;
mod sha256;
mod store;
mod traits;
mod validate;
mod verifier;

pub use engine::Engine;
pub use gap::SafefsCapabilityGap;
pub use integration::{prepared_transaction, project_identity_token};
pub use model::*;
pub use report::{report_to_wire, report_to_wire_plan};
pub use safefs::SafefsTransactionFilesystem;
pub use sha256::project_key;
pub use store::SystemTransactionStore;
pub use traits::*;
pub use verifier::{PreparedHealthVerifier, RecoveryHealthVerifier};

#[cfg(test)]
mod tests;
