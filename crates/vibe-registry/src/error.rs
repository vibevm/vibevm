//! The registry error layer (Class F): every variant cites its
//! violated REQ and a fix surface (PROP-002 §2.3's failure
//! discriminator).

specmark::scope!("spec://vibevm/modules/vibe-registry/PROP-002#failure-discriminator");

use std::path::PathBuf;

use specmark::spec;
use thiserror::Error;
use vibe_core::Group;

use crate::git_backend::GitError;

/// Failure surface of registry resolution, discriminated so the
/// multi-registry walk can route on it — `UnknownPackage` falls through
/// to the next registry, anything else halts (PROP-002 §2.3.1):
///
/// ```
/// use vibe_core::Group;
/// use vibe_registry::RegistryError;
///
/// let err = RegistryError::UnknownPackage {
///     group: Group::parse("org.vibevm").unwrap(),
///     name: "nope".to_string(),
/// };
/// assert_eq!(
///     err.to_string(),
///     "package `org.vibevm/nope` is not in the registry \
///      (violates spec://vibevm/modules/vibe-registry/PROP-002#failure-discriminator; \
///       fix: check the spelling or add a [[registry]] that carries it)",
/// );
/// ```
#[derive(Debug, Error)]
#[spec(implements = "spec://vibevm/modules/vibe-registry/PROP-002#failure-discriminator")]
pub enum RegistryError {
    #[error(
        "registry root `{0}` does not exist or is not a directory \
         (violates spec://vibevm/modules/vibe-registry/PROP-002#registry-model; \
          fix: check the [[registry]] url or pass --registry <dir>)"
    )]
    MissingRoot(PathBuf),

    #[error(
        "package `{group}/{name}` is not in the registry \
         (violates spec://vibevm/modules/vibe-registry/PROP-002#failure-discriminator; \
          fix: check the spelling or add a [[registry]] that carries it)"
    )]
    UnknownPackage { group: Group, name: String },

    #[error(
        "no version of `{group}/{name}` matches `{req}` \
         (violates spec://vibevm/modules/vibe-registry/PROP-002#failure-discriminator; \
          fix: relax the version requirement or run `vibe registry sync`)"
    )]
    NoMatchingVersion {
        group: Group,
        name: String,
        req: String,
    },

    /// A pkgref reached registry resolution without a `group`. A registry
    /// resolves by `(group, name)` identity (PROP-008 §2.2); a bare short
    /// name must be qualified at the CLI boundary first.
    #[error(
        "package reference `{0}` is not group-qualified — registry resolution needs \
         `<group>/<name>` (violates spec://vibevm/modules/vibe-registry/PROP-002#registry-model; \
         fix: qualify the reference as `<group>/<name>`)"
    )]
    UnqualifiedPkgref(String),

    #[error(
        "registry entry at `{path}` has an invalid directory name `{name}` — expected \
         `v<semver>` (violates spec://vibevm/modules/vibe-registry/PROP-002#layout; \
         fix: rename the version directory to `v<major>.<minor>.<patch>`)"
    )]
    BadVersionDir { path: PathBuf, name: String },

    #[error(transparent)]
    Core(#[from] vibe_core::Error),

    #[error(
        "git operation failed \
         (violates spec://vibevm/modules/vibe-registry/PROP-001#backend-trait; \
          fix: act on the wrapped git error): {0}"
    )]
    Git(#[from] GitError),

    #[error(
        "could not determine the user home directory; set HOME (or USERPROFILE on Windows), or \
         pass an explicit cache root \
         (violates spec://vibevm/modules/vibe-registry/PROP-002#cache; \
          fix: set HOME / USERPROFILE or VIBE_REGISTRY_CACHE)"
    )]
    NoHomeDir,

    #[error(
        "registry meta file at `{path}` is malformed \
         (violates spec://vibevm/modules/vibe-registry/PROP-002#failure-discriminator; \
          fix: correct or regenerate the file at that path): {reason}"
    )]
    MalformedMeta { path: PathBuf, reason: String },

    /// Registry is declared `auth = "token-env"` (PROP-002 §2.2.1) but
    /// the resolved env-var is empty / unset. Surfaces before any git
    /// invocation so the operator gets an actionable hint pointing at
    /// the env-var to set, instead of a generic 401 from the host.
    #[error(
        "registry `{registry}` declares `auth = \"token-env\"` but env-var `{env_var}` is empty or unset; \
         set it to a personal access token with read access to the registry org \
         (violates spec://vibevm/modules/vibe-registry/PROP-002#registry-auth; \
          fix: export {env_var})"
    )]
    MissingToken { registry: String, env_var: String },

    /// Aggregated walk-failure: every configured registry was tried,
    /// none had a satisfying answer, and at least one walked-past
    /// 401 / 403 (auth=none) needs surfacing so the operator sees
    /// per-registry status. `summary` is the pre-formatted
    /// multi-line block that `Display` renders verbatim;
    /// `attempts` carries the same information in structured form
    /// so `vibe-cli`'s install-error JSON envelope can ship a
    /// machine-readable per-registry array without the consumer
    /// having to parse prose. Returned only when at least one
    /// registry was walked; the no-registries-at-all path still
    /// returns the simpler `UnknownPackage` variant for back-compat
    /// with downstream consumers that match on it.
    #[error(
        "package `{group}/{name}` not found in any configured registry \
         (violates spec://vibevm/modules/vibe-registry/PROP-002#failure-discriminator; \
          fix: check the package name and `vibe registry list`).\nTried:\n{summary}"
    )]
    PackageNotFoundEverywhere {
        group: Group,
        name: String,
        summary: String,
        attempts: Vec<crate::multi_registry_resolver::RegistryWalkAttempt>,
    },

    #[error(
        "I/O error on `{path}` \
         (violates spec://vibevm/modules/vibe-registry/PROP-002#failure-discriminator; \
          fix: check the path's existence and permissions)"
    )]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// An `in-place` materialization (PROP-022 §2.4) was requested against a
    /// registry with no git backend — the local-directory registry
    /// (`--registry <path>`, the M0 shape). In-place needs a real git source
    /// to clone and incrementally update; there is nothing to clone from a
    /// directory tree.
    #[error(
        "package `{group}/{name}` declares in-place materialization but resolves through a \
         local-directory registry with no git backend \
         (violates spec://vibevm/modules/vibe-workspace/PROP-022#in-place; \
          fix: serve it from a git `[[registry]]`, or drop `materialization = \"in-place\"`)"
    )]
    InPlaceUnsupported { group: Group, name: String },
}
