//! Typed compiler IR and pass machinery adopted one built-in at a time.
//!
//! `parse`, `close`, and `merge` are the production prefix of the declared
//! schedule. Remaining artifact phases stay on the legacy continuation until
//! their R3.2 steps; public one-seed entry points remain compatibility wrappers.

pub(crate) mod builtin;
pub(crate) mod close;
pub(crate) mod ir;
pub(crate) mod merge;
pub(crate) mod pass;
pub(crate) mod pipeline;
pub(crate) mod source_snapshot;
