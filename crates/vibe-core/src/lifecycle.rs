//! Pure lifecycle vocabulary shared by manifests and the lifecycle engine.
//!
//! This module owns the phase and extension-point types because they sit below
//! both consumers in the dependency graph: manifest schemas can store them
//! without depending on the engine, while `vibe-lifecycle` builds request chains
//! over the same values. Execution, state, and handler dispatch remain outside
//! this core vocabulary.

mod phase;
mod point;

pub use phase::{DEFAULT_PHASES, Phase, PhaseParseError};
pub use point::{
    CompilePoint, CompilePointParseError, ExtensionPoint, ExtensionPointParseError, PhasePoint,
    PhasePointParseError, SlotPoint, SlotPointParseError,
};
