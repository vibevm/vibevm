//! Error types for `vibe-core`.
//!
//! Parsing, validation, and I/O errors surfaced from this crate. Concrete
//! operational errors (e.g. network, git) live in the crates that perform them.

use std::path::PathBuf;

use thiserror::Error;

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("invalid package reference `{input}`: {reason}")]
    BadPackageRef { input: String, reason: String },

    #[error("invalid package kind `{0}` — must be one of: flow, feat, stack, tool")]
    BadPackageKind(String),

    #[error(
        "invalid package name `{0}` — must be kebab-case (lowercase letters, digits, and internal hyphens only)"
    )]
    BadPackageName(String),

    #[error("invalid version spec `{input}`")]
    BadVersionSpec {
        input: String,
        #[source]
        source: semver::Error,
    },

    #[error("failed to read file at {path}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to write file at {path}")]
    Write {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to parse TOML at {path}")]
    ParseToml {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },

    #[error("failed to serialize TOML")]
    SerializeToml(#[from] toml::ser::Error),
}
