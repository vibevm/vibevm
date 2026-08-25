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
//!
//! The vocabulary remains available from this crate for compatibility even
//! though its single owner is now `vibe_core::lifecycle`:
//!
//! ```
//! use vibe_lifecycle::{CompilePoint, ExtensionPoint, Phase, PhasePoint};
//!
//! let phase = PhasePoint::Default(Phase::Build);
//! let point = ExtensionPoint::Phase(phase);
//! let core_point: vibe_core::lifecycle::ExtensionPoint = point;
//! assert_eq!(core_point.to_string(), "phase:build");
//! assert_eq!("compile:pass".parse(), Ok(CompilePoint::Pass));
//! ```

#![forbid(unsafe_code)]

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM");

mod chain;

pub use chain::{LifecycleRequest, LifecycleStep, inclusive_chain};
pub use vibe_core::lifecycle::{
    CompilePoint, CompilePointParseError, DEFAULT_PHASES, ExtensionPoint, ExtensionPointParseError,
    Phase, PhaseParseError, PhasePoint, PhasePointParseError, SlotPoint, SlotPointParseError,
};
