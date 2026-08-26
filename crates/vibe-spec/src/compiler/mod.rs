//! Typed compiler IR and pass machinery adopted one built-in at a time.
//!
//! `parse`, `close`, `merge`, `embed`, and `qualify` are the production prefix
//! of the declared schedule. Remaining artifact phases stay on the legacy
//! continuation until their R3.2 steps; public one-seed entry points remain
//! compatibility wrappers.

pub(crate) mod builtin;
pub(crate) mod close;
pub(crate) mod embed;
pub(crate) mod embed_snapshot;
pub(crate) mod ir;
pub(crate) mod merge;
pub(crate) mod pass;
pub(crate) mod pipeline;
pub(crate) mod qualify;
pub(crate) mod source_snapshot;
pub(crate) mod worklist;
