//! Typed compiler IR and pass machinery adopted one built-in at a time.
//!
//! `parse` is the first production pass on the declared schedule. The remaining
//! artifact phases stay on the legacy continuation until their own R3.2 steps;
//! the public one-seed entry points remain compatibility wrappers throughout.

pub(crate) mod builtin;
pub(crate) mod ir;
pub(crate) mod pass;
pub(crate) mod pipeline;
