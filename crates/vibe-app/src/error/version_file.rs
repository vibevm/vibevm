//! The refusals of `--write-version-file` (PROP-075 `#VERSION-FILE-WRITE`),
//! beside `error.rs` because they are the one error family of the
//! version-file plane and that file is at its budget.
//!
//! Every one of them lands **before** any byte is written, and each names
//! the thing a user has to change: the path, the slot and its package, or
//! both lines of a duplicated key.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-075#VERSION-FILE-WRITE");

use specmark::spec;
use thiserror::Error;

use super::AppError;

/// What stops a version-file write (PROP-075 `#VERSION-FILE-WRITE`).
#[derive(Debug, Error)]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSION-FILE-WRITE")]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSION-FILE-WRITE")]
pub enum VersionFileError {
    #[error(
        "`{path}` is a symbolic link (violates spec://org.vibevm.core/vibevm/common/PROP-075#VERSION-FILE-WRITE; \
         fix: write the file the link points at, or replace the link with a regular file)"
    )]
    Symlink { path: String },
    #[error(
        "slot `{slot}` declares no version file, so there is nothing to write: its package `{package}` has no \
         [[application.version_file]] row (violates spec://org.vibevm.core/vibevm/common/PROP-075#VERSION-FILE-WRITE; \
         fix: drop --write-version-file, or declare a version file in the package)"
    )]
    NoVersionFile { slot: String, package: String },
    #[error(
        "`{path}` is not a version file of slot `{slot}`, which declares {names} (violates \
         spec://org.vibevm.core/vibevm/common/PROP-075#VERSION-FILE-WRITE; \
         fix: name a path whose file name is one of those, or pass --write-version-file without a value)"
    )]
    UnknownName {
        path: String,
        slot: String,
        names: String,
    },
    #[error(
        "`{path}` carries the key `{key}` twice, on lines {first} and {second} (violates \
         spec://org.vibevm.core/vibevm/common/PROP-075#VERSION-FILE-WRITE; \
         fix: delete one of the two lines, so the file states one version for the key)"
    )]
    DuplicateKey {
        path: String,
        key: String,
        first: usize,
        second: usize,
    },
    #[error(
        "{action} `{path}` failed: {source} (violates spec://org.vibevm.core/vibevm/common/PROP-075#VERSION-FILE-WRITE; \
         fix: check that the directory exists and is writable)"
    )]
    Io {
        action: &'static str,
        path: String,
        #[source]
        source: std::io::Error,
    },
}

impl From<VersionFileError> for AppError {
    fn from(error: VersionFileError) -> Self {
        Self::VersionFile(Box::new(error))
    }
}
