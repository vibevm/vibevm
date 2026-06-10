//! Top-level error type for the `vibe-index` utility.
//!
//! A deliberately coarse surface — the CLI subcommands and the HTTP
//! server map their own richer failures down to these few variants at
//! the process boundary, where all the operator needs is a clear
//! message and a non-zero exit.

specmark::scope!("spec://vibevm/modules/vibe-index/PROP-005#root");

use std::path::PathBuf;

use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
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
