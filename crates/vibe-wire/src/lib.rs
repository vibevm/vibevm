//! `vibe-wire` — generated Rust types for vibevm wire contracts.
//!
//! Every type under [`generated`] is **machine-generated** from a JTD
//! schema in [`schemas/`](../../../schemas/) at the repo root. Source of truth lives
//! there; this crate carries the codegen output verbatim. `cargo
//! xtask codegen` regenerates; `cargo xtask check-codegen` asserts no
//! drift (CI runs the latter). Per PROP-000 §16, JTD + codegen is the
//! standing pattern for wire contracts in this project.
//!
//! See [`tools/jtd-codegen/README.md`](../../../tools/jtd-codegen/README.md) for the
//! generator install procedure and pinned version.
//!
//! Migration of existing hand-written `Serialize` structs to
//! JTD-derived types lands incrementally — `vibe init --json` was the
//! first consumer.

#![forbid(unsafe_code)]
// jtd-codegen 0.4.1 emits structs with `pub camelCase` field names
// (e.g. `removedCount`) when the JTD schema property is `removed_count`.
// We have a `#[serde(rename = "removed_count")]` on the field so the
// wire format is correct regardless, but Rust's lint catches the camel
// case as non-snake. Allow it crate-wide rather than fighting the
// generator on a v0.4.1-only quirk.
#![allow(non_snake_case)]

/// Generated wire types. Populated by `cargo xtask codegen` from
/// `*.jtd.json` schemas under `schemas/` at the repo root. Each
/// submodule corresponds to one schema; the top-level
/// `generated/mod.rs` is itself synthesised by the xtask and lists the
/// submodules in alphabetical order.
pub mod generated;
