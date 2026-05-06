//! `vibe-index` — standalone package index utility for vibevm.
//!
//! Spec: [`spec://vibevm/modules/vibe-index/PROP-005`][prop].
//! This crate ships a single binary `vibe-index` plus a thin library
//! surface so integration tests can call into the dispatcher without
//! shelling out to the binary.
//!
//! Slice 1 lands the CLI skeleton (clap-derived subcommand dispatch +
//! a stubbed entry point per subcommand returning
//! [`Error::NotYetImplemented`]). The subsequent slices fill in the
//! actual subcommand bodies. Help-text rendering for every subcommand
//! is exercised by `tests/help_smoke.rs` and is a regression invariant
//! across all later slices.
//!
//! [prop]: ../../../spec/modules/vibe-index/PROP-005-package-index.md

#![forbid(unsafe_code)]

pub mod cli;
pub mod content_hash;
pub mod error;
pub mod index;
pub mod scanner;
pub mod types;

pub use error::{Error, Result};
