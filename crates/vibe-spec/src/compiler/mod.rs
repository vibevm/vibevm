//! Typed compiler IR and pass machinery adopted one built-in at a time.
//!
//! `parse`, `close`, `merge`, `embed`, `qualify`, `absorb`, `link`, and `assemble`
//! form the production whole-artifact path through Lane. Public one-seed entry
//! points keep their linked compatibility tail until the named emit atom.

pub(crate) mod absorb;
pub(crate) mod assemble;
pub(crate) mod backend;
pub(crate) mod builtin;
pub(crate) mod close;
pub(crate) mod digest;
pub(crate) mod embed;
pub(crate) mod embed_snapshot;
pub(crate) mod emit;
pub(crate) mod ir;
pub(crate) mod link;
pub(crate) mod merge;
pub(crate) mod observer;
pub(crate) mod pass;
pub(crate) mod pipeline;
pub(crate) mod qualify;
pub(crate) mod source_snapshot;
pub(crate) mod trace;
pub(crate) mod transform;
pub(crate) mod verify;
pub(crate) mod wire;
pub(crate) mod worklist;

#[cfg(test)]
mod artifact_tests;
