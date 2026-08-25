//! Typed compiler IR and pass machinery introduced ahead of the phase move.
//!
//! R3.1 deliberately leaves the shipping one-seed compiler untouched. These
//! cells name the values and the pass seam that R3.2 will adopt phase by phase;
//! no default pipeline is registered here and no public compile entry point
//! reaches this module yet.

pub(crate) mod ir;
pub(crate) mod pass;
pub(crate) mod pipeline;
