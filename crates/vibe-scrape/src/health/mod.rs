//! Platform-neutral PROP-056 health preparation, evidence and judgment.
//!
//! Process creation is deliberately absent from this module. `run_phase`
//! accepts only a crate-sealed backend whose advertised enforcement
//! capabilities cover the prepared check; an unsupported platform therefore
//! refuses before executing anything.

mod backend;
mod baseline;
mod discovery;
mod local;
mod model;
mod output;
pub mod platform;
mod prepare;
mod preset;
mod projected;
mod protocol;
mod system;
pub mod tree;
mod wire;

pub use backend::{HealthBackend, UnsupportedBackend, capability_blockers};
pub use baseline::judge;
pub use local::LocalProcessBackend;
pub use model::*;
pub use output::{StreamAccumulator, drain_concurrently};
pub use prepare::{HealthResolver, add_blockers, prepare};
pub use projected::validate_projected_final;
pub use protocol::{DEFAULT_RESULT_CAP, parse_health_result};
pub use run::run_phase;
pub use system::SystemHealthResolver;
pub use wire::{baseline as wire_baseline, limits as wire_limits, to_wire as to_wire_checks};

mod run;

#[cfg(test)]
mod tests;
