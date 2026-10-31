//! Remap: replaceable artifact sources (`PROP-066`).
//!
//! A vibe installation has to work where the public hosts are unreachable,
//! filtered, or forbidden by policy. Every artifact vibe fetches already
//! carries a hash, so **where** the bytes are looked for is a replaceable
//! detail and **what** they are is not. A remap row changes only the
//! former; the digest check stays exactly where it was, so a remap can
//! make a fetch fail or succeed and can never change what is installed.
//!
//! The three pieces, lowest to highest:
//!
//! - [`row`] — one `[[remap]]` row and the per-file `[policy]` table.
//! - [`layers`] — where rows come from, including PROP-002's `[[mirror]]`
//!   and `[[override]]` tables read as rows.
//! - [`locator`] — the one place a declared source becomes an ordered list
//!   of places to try.
//!
//! Not here, and deliberately: remap *packages*, consent, the provider
//! map, `remap test`, export/import. The layer enum carries the package
//! variants and a row carries its provenance so those land as sources of
//! rows rather than as a second mechanism.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-066#purpose");

pub mod active;
pub mod artifact;
pub mod layers;
pub mod locator;
pub mod row;

pub use active::{active, install};
pub use artifact::{Artifact, ArtifactKind, Candidate, Explanation, Resolution, RowMatch, RowRef};
pub use layers::{
    Layer, LoadedSource, default_raw_file_rows, env_remap_files, rows_from_file, rows_from_mirrors,
    rows_from_overrides, rows_from_settings,
};
pub use locator::{Locator, RemapRefusal};
pub use row::{
    PolicySection, Provenance, RemapFileDecl, RemapKey, RemapParseError, RemapRow, RemapScope,
    RemapTarget, RowDecl, UnmatchedPolicy,
};

/// The error code a refused fetch carries (`PROP-066#row`): the fetch was
/// refused before any connection, by a `deny` row or by
/// `unmatched = "deny"`.
pub const REMAP_DENIED: &str = "remap_denied";

/// The `--remap <file>` flag's long name, and the environment variable
/// that carries the same list — named here so the composition root and
/// every diagnostic spell them the same way.
pub const REMAP_ENV: &str = "VIBE_REMAP";

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
