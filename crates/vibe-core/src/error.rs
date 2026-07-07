//! Error types for `vibe-core`.
//!
//! Parsing, validation, and I/O errors surfaced from this crate. Concrete
//! operational errors (e.g. network, git) live in the crates that perform them.

specmark::scope!("spec://vibevm/VIBEVM-SPEC#package-identity");
specmark::scope!("spec://vibevm/VIBEVM-SPEC#manifest-schema");

use std::path::PathBuf;

use specmark::spec;
use thiserror::Error;

pub type Result<T, E = Error> = std::result::Result<T, E>;

/// The crate's error type — one `thiserror` enum for the parse, validate,
/// and I/O layer of `vibe-core`. Every variant's `Display` embeds the
/// `spec://` REQ it guards plus a fix hint, so a failing run is navigable
/// back to the requirement without source access.
///
/// ```
/// use vibe_core::Error;
///
/// let e = Error::BadPackageKind("xml".into());
/// let msg = e.to_string();
/// assert!(msg.contains("must be one of: flow, feat, stack, tool, mcp"));
/// assert!(msg.contains("spec://vibevm/VIBEVM-SPEC#four-installable-kinds"));
/// ```
#[derive(Debug, Error)]
#[spec(implements = "spec://vibevm/VIBEVM-SPEC#package-identity")]
pub enum Error {
    #[error(
        "invalid package reference `{input}`: {reason} \
         (violates spec://vibevm/modules/vibe-registry/PROP-008#pkgref; \
          fix: write the reference as `[kind:][group/]name[@version]`)"
    )]
    BadPackageRef { input: String, reason: String },

    #[error(
        "invalid package kind `{0}` — must be one of: flow, feat, stack, tool, mcp \
         (violates spec://vibevm/VIBEVM-SPEC#four-installable-kinds; \
          fix: use one of the installable kinds)"
    )]
    BadPackageKind(String),

    #[error(
        "invalid package name `{0}` — must be kebab-case (lowercase letters, digits, \
         and internal hyphens only) \
         (violates spec://vibevm/modules/vibe-registry/PROP-008#pkgref; \
          fix: rename to kebab-case)"
    )]
    BadPackageName(String),

    #[error(
        "invalid package group `{input}`: {reason} \
         (violates spec://vibevm/modules/vibe-registry/PROP-008#pkgref; \
          fix: use a reverse-FQDN group like `org.vibevm`)"
    )]
    BadGroup { input: String, reason: String },

    #[error(
        "invalid capability reference `{input}`: {reason} \
         (violates spec://vibevm/modules/vibe-registry/PROP-002#capability; \
          fix: write the capability as `interface:<name>[@version]`)"
    )]
    BadCapabilityRef { input: String, reason: String },

    #[error(
        "invalid content hash `{input}`: {reason} \
         (violates spec://vibevm/modules/vibe-registry/PROP-008#identity; \
          fix: use a `sha256:<hex>` digest as produced by the indexer)"
    )]
    BadContentHash { input: String, reason: String },

    #[error(
        "invalid version spec `{input}` \
         (violates spec://vibevm/modules/vibe-registry/PROP-008#pkgref; \
          fix: use a Cargo-style requirement such as `^1.2`, `~1.2.3`, or `=1.2.3`)"
    )]
    BadVersionSpec {
        input: String,
        #[source]
        source: semver::Error,
    },

    #[error(
        "invalid dependency declaration for `{input}`: {reason} \
         (violates spec://vibevm/modules/vibe-registry/PROP-002#git-source; \
          fix: correct the [requires] entry in vibe.toml)"
    )]
    BadDependencyDecl { input: String, reason: String },

    #[error(
        "invalid `when` condition `{input}`: {reason} \
         (violates spec://vibevm/VIBEVM-SPEC#manifest-schema; \
          fix: correct the `when` predicate on the dependency)"
    )]
    BadWhenCondition { input: String, reason: String },

    #[error(
        "invalid manifest: {reason} \
         (violates spec://vibevm/VIBEVM-SPEC#manifest-schema; \
          fix: correct vibe.toml against the schema)"
    )]
    InvalidManifest { reason: String },

    #[error(
        "unsupported vibe.lock schema version {found} — expected {expected} \
         (violates spec://vibevm/VIBEVM-SPEC#lockfile-schema; \
          fix: regenerate with `vibe install`)"
    )]
    UnsupportedLockfile { found: u32, expected: u32 },

    #[error(
        "failed to read file at {path} \
         (violates spec://vibevm/VIBEVM-SPEC#directory-layout; \
          fix: check the path exists and is readable)"
    )]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error(
        "failed to write file at {path} \
         (violates spec://vibevm/VIBEVM-SPEC#directory-layout; \
          fix: check the parent directory exists and is writable)"
    )]
    Write {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error(
        "failed to parse TOML at {path} \
         (violates spec://vibevm/VIBEVM-SPEC#manifest-schema; \
          fix: repair the TOML syntax at the reported location)"
    )]
    ParseToml {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },

    #[error(
        "failed to serialize TOML \
         (violates spec://vibevm/VIBEVM-SPEC#manifest-schema; \
          fix: act on the wrapped serializer error)"
    )]
    SerializeToml(#[from] toml::ser::Error),
}
