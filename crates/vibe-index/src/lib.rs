//! `vibe-index` — standalone package index utility for vibevm.
//!
//! Spec: [`spec://vibevm/modules/vibe-index/PROP-005`][prop].
//! This crate ships a single binary `vibe-index` plus a thin library
//! surface so integration tests can call into the dispatcher without
//! shelling out to the binary.
//!
//! The utility builds and serves an opt-in per-org metadata catalog:
//! `init` / `reindex` / `add` / `remove` mutate a data directory of
//! index files; `get` / `list` / `search` / `capabilities` / `purls`
//! / `outdated` read it; `serve` exposes the same surface over HTTP.
//! Help-text rendering for every subcommand is exercised by
//! `tests/help_smoke.rs` and is a standing regression invariant.
//!
//! [prop]: ../../../spec/modules/vibe-index/PROP-005-package-index.md

#![forbid(unsafe_code)]

pub mod cli;
pub mod content_hash;
pub mod error;
pub mod index;
pub mod lockfile;
pub mod scanner;
pub mod server;
pub mod types;

pub use error::{Error, Result};
