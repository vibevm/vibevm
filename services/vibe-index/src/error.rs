//! Top-level error type for the `vibe-index` utility.
//!
//! The variants are intentionally coarse for slice 1 — slices that
//! land actual subcommand bodies extend the surface as their own error
//! shapes settle. Until then, `NotYetImplemented` is returned by every
//! stubbed dispatcher entry.

use std::path::PathBuf;

use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    /// The requested subcommand has a clap entry but no implementation
    /// yet. Carries the subcommand name so the operator can correlate
    /// against the slice plan in PROP-005.
    #[error(
        "`{0}` is not yet implemented in this slice — see spec://vibevm/modules/vibe-index/PROP-005#phases"
    )]
    NotYetImplemented(&'static str),

    /// User-supplied input failed validation.
    #[error("invalid input: {0}")]
    InvalidInput(String),

    /// Filesystem I/O error attached to a path for diagnostics.
    #[error("filesystem error at `{path}`: {message}")]
    Io { path: PathBuf, message: String },

    /// On-disk index files do not satisfy the schema invariants.
    #[error("malformed index: {0}")]
    Malformed(String),
}
