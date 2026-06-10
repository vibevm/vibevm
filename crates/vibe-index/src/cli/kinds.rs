//! CLI-side re-export of the canonical `kinds` types.
//!
//! The actual definitions live in [`crate::types::kinds`] (so they
//! can be referenced from `types/entry.rs` without an awkward
//! cli-to-types import). CLI subcommand modules still reach for them
//! through `crate::cli::kinds::*` for path-stability.

specmark::scope!("spec://vibevm/modules/vibe-index/PROP-005#root");

pub use crate::types::kinds::*;
