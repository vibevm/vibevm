//! Top-level error type for the `vibe-index` utility.
//!
//! A deliberately coarse surface — the CLI subcommands and the HTTP
//! server map their own richer failures down to these few variants at
//! the process boundary, where all the operator needs is a clear
//! message and a non-zero exit.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#root");

use std::path::PathBuf;

use specmark::spec;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
#[spec(implements = "spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#root")]
pub enum Error {
    /// User-supplied input failed validation.
    #[error(
        "invalid input: {0} \
         (violates spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#cli; \
          fix: correct the argument — `vibe-index <subcommand> --help` shows the shape)"
    )]
    InvalidInput(String),

    /// Filesystem I/O error attached to a path for diagnostics.
    #[error(
        "filesystem error at `{path}`: {message} \
         (violates spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#persistence; \
          fix: check the data directory exists and is writable)"
    )]
    Io { path: PathBuf, message: String },

    /// On-disk index files do not satisfy the schema invariants.
    #[error(
        "malformed index: {0} \
         (violates spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#persistence; \
          fix: re-run `vibe-index reindex` to rebuild the on-disk files)"
    )]
    Malformed(String),

    /// The journal cannot be folded into a catalog: it establishes no
    /// registry identity, or it names a fact whose carrier this build
    /// has not built. Refusing is the point — the journal is the truth
    /// and the catalog its projection (PROP-044 §3), so inventing an
    /// identity or silently skipping a fact would make the catalog
    /// assert a state the journal does not describe.
    #[error(
        "unprojectable journal: {0} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-044#truth; \
          fix: run `vibe-index init` when the journal carries no identity; \
          update vibe-index when an event names a carrier this build lacks)"
    )]
    Unprojectable(String),

    /// A structurally bounded JSON-envelope count did not fit the
    /// schema's exact `uint32` domain. Never truncate it.
    #[error(
        "cannot encode wire field `{field}` value {value}: exceeds uint32 \
         (violates spec://org.vibevm.core/vibevm/common/PROP-044#machinery; \
          fix: reduce the result/page size or widen the field's schema and writer together)"
    )]
    WireCountOverflow { field: &'static str, value: usize },
}

impl From<crate::wire_count::CountOverflow> for Error {
    fn from(error: crate::wire_count::CountOverflow) -> Self {
        Self::WireCountOverflow {
            field: error.field,
            value: error.value,
        }
    }
}
