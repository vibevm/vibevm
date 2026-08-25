//! Deterministic lifecycle vocabulary and request chaining for vibe.
//!
//! The default lifecycle is a fixed nine-phase ritual. Requesting a phase
//! includes every phase before it; the independent clean lifecycle can be
//! prepended to that chain.
//!
//! ```
//! use vibe_lifecycle::{LifecycleRequest, LifecycleStep, Phase};
//!
//! assert_eq!(
//!     LifecycleRequest::Clean {
//!         then: Some(Phase::Build),
//!     }
//!     .steps(),
//!     vec![
//!         LifecycleStep::Clean,
//!         LifecycleStep::Default(Phase::Validate),
//!         LifecycleStep::Default(Phase::Install),
//!         LifecycleStep::Default(Phase::Generate),
//!         LifecycleStep::Default(Phase::Build),
//!     ],
//! );
//! ```

#![forbid(unsafe_code)]

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM");

mod chain;
mod phase;

pub use chain::{LifecycleRequest, LifecycleStep, inclusive_chain};
pub use phase::{DEFAULT_PHASES, Phase, PhaseParseError};
