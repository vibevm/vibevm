//! Typed compiler IR and pass machinery adopted one built-in at a time.
//!
//! `parse` and `close` are the production prefix of the declared schedule. The
//! remaining artifact phases stay on the legacy continuation until their own
//! R3.2 steps; public one-seed entry points remain compatibility wrappers.

pub(crate) mod builtin;
pub(crate) mod close;
pub(crate) mod ir;
pub(crate) mod pass;
pub(crate) mod pipeline;
