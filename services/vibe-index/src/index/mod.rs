//! In-memory index + on-disk persistence.
//!
//! The [`Index`] struct holds the canonical RAM copy. Persistence
//! reads and writes the on-disk files described in PROP-005 §2.4
//! atomically (tmp + rename + fsync). Slice 2 ships the read/write
//! pipeline for `repomd.json`, `primary.jsonl`, and
//! `by-name/<kind>/<name>.json`. `by-cap/` and `by-purl/` join in
//! later slices.

pub mod by_name;
pub mod memory;
pub mod persistence;
pub mod primary;
pub mod repomd;

pub use memory::Index;
pub use persistence::{atomic_write, compute_sha256_hex, sha256_of_bytes};
