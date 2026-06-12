//! Top-level error type for the `vibe-index` utility.
//!
//! A deliberately coarse surface — the CLI subcommands and the HTTP
//! server map their own richer failures down to these few variants at
//! the process boundary, where all the operator needs is a clear
//! message and a non-zero exit.

specmark::scope!("spec://vibevm/modules/vibe-index/PROP-005#root");

use std::path::PathBuf;

use specmark::spec;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
#[spec(implements = "spec://vibevm/modules/vibe-index/PROP-005#root")]
pub enum Error {
    /// User-supplied input failed validation.
    #[error(
        "invalid input: {0} \
         (violates spec://vibevm/modules/vibe-index/PROP-005#cli; \
          fix: correct the argument — `vibe-index <subcommand> --help` shows the shape)"
    )]
    InvalidInput(String),

    /// Filesystem I/O error attached to a path for diagnostics.
    #[error(
        "filesystem error at `{path}`: {message} \
         (violates spec://vibevm/modules/vibe-index/PROP-005#persistence; \
          fix: check the data directory exists and is writable)"
    )]
    Io { path: PathBuf, message: String },

    /// On-disk index files do not satisfy the schema invariants.
    #[error(
        "malformed index: {0} \
         (violates spec://vibevm/modules/vibe-index/PROP-005#persistence; \
          fix: re-run `vibe-index reindex` to rebuild the on-disk files)"
    )]
    Malformed(String),
}
