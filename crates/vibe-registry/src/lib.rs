//! Registry access: resolve, fetch, cache.
//!
//! M0 shipped a local-directory registry laid out per
//! `VIBEVM-SPEC.md` §8.2:
//!
//! ```text
//! <registry>/<kind>/<name>/v<major>.<minor>.<patch>/vibe.toml
//! ```
//!
//! M1 adds git support via the same on-disk layout cloned under
//! `~/.vibe/registries/<hash>/clone/`. All git I/O goes through
//! [`git_backend::GitBackend`] — see
//! [`spec/modules/vibe-registry/PROP-001-git-backend.md`][prop].
//!
//! Spec: `VIBEVM-SPEC.md` §8.
//!
//! [prop]: ../../../spec/modules/vibe-registry/PROP-001-git-backend.md

#![forbid(unsafe_code)]
specmark::scope!("spec://vibevm/modules/vibe-registry/PROP-001#root");

use std::fs;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use specmark::spec;
use thiserror::Error;
use vibe_core::manifest::Manifest;
use vibe_core::{Group, PackageRef};
use walkdir::WalkDir;

pub mod git_backend;
pub mod git_package_registry;
pub mod git_registry;
pub mod index_client;
mod local_registry;
pub mod multi_registry_resolver;
mod registry_cache;
pub mod search;
pub mod vendor;

pub use git_backend::{GitBackend, GitError, ShellGit};
pub use git_package_registry::GitPackageRegistry;
pub use git_registry::{GitRegistry, RegistryMeta, default_cache_root};
pub use index_client::{
    BindingSite, IndexClient, IndexError, PurlLookupHit, PurlLookupResults, SearchHit,
    SearchResults, index_url_for,
};
pub use local_registry::LocalRegistry;
pub use multi_registry_resolver::{
    DEFAULT_OVERRIDE_REF, MultiRegistryResolver, MultiResolution, RefreshReport, RefreshedEntry,
    RefreshedVia, RegistryWalkAttempt, ResolvedPathDep, SkippedEntry, WalkAttemptStatus,
};

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

/// Uniform surface over all registry backends — [`LocalRegistry`] and
/// [`GitRegistry`] both implement this trait. `vibe-install` and
/// `vibe-cli` consume registries exclusively through the trait so the
/// concrete backend can be chosen at CLI-argument-parse time.
///
/// The canonical implementation shape — answer the three questions
/// from whatever backing store you have (here: a registry that
/// carries no packages at all), then consume it as `&dyn Registry`:
///
/// ```
/// use std::path::Path;
/// use vibe_core::{Group, PackageRef};
/// use vibe_registry::{CachedPackage, Registry, RegistryError, ResolvedPackage};
///
/// struct EmptyRegistry;
///
/// impl Registry for EmptyRegistry {
///     fn list_versions(
///         &self,
///         group: &Group,
///         name: &str,
///     ) -> Result<Vec<semver::Version>, RegistryError> {
///         Err(RegistryError::UnknownPackage {
///             group: group.clone(),
///             name: name.to_owned(),
///         })
///     }
///     fn resolve(&self, pkgref: &PackageRef) -> Result<ResolvedPackage, RegistryError> {
///         let group = pkgref
///             .group
///             .clone()
///             .ok_or_else(|| RegistryError::UnqualifiedPkgref(pkgref.to_string()))?;
///         Err(RegistryError::UnknownPackage {
///             group,
///             name: pkgref.name.to_string(),
///         })
///     }
///     fn fetch(
///         &self,
///         resolved: &ResolvedPackage,
///         _cache_root: &Path,
///     ) -> Result<CachedPackage, RegistryError> {
///         Err(RegistryError::UnknownPackage {
///             group: resolved.group.clone(),
///             name: resolved.name.clone(),
///         })
///     }
/// }
///
/// let reg: &dyn Registry = &EmptyRegistry;
/// let err = reg
///     .resolve(&PackageRef::parse("org.vibevm/wal").unwrap())
///     .unwrap_err();
/// assert!(matches!(err, RegistryError::UnknownPackage { .. }));
/// ```
pub trait Registry {
    fn list_versions(
        &self,
        group: &Group,
        name: &str,
    ) -> Result<Vec<semver::Version>, RegistryError>;

    fn resolve(&self, pkgref: &PackageRef) -> Result<ResolvedPackage, RegistryError>;

    fn fetch(
        &self,
        resolved: &ResolvedPackage,
        cache_root: &Path,
    ) -> Result<CachedPackage, RegistryError>;
}

/// A package pinned to a concrete version, located in the registry on disk.
///
/// Built by [`Registry::resolve`]; constructible directly wherever the
/// pinned `(group, name, version)` identity and its source directory
/// are already known:
///
/// ```
/// use std::path::PathBuf;
/// use vibe_core::Group;
/// use vibe_registry::ResolvedPackage;
///
/// let resolved = ResolvedPackage {
///     group: Group::parse("org.vibevm").unwrap(),
///     name: "wal".to_string(),
///     version: semver::Version::parse("0.2.0").unwrap(),
///     source_dir: PathBuf::from("registry/org.vibevm/wal/v0.2.0"),
/// };
/// assert_eq!(resolved.version.to_string(), "0.2.0");
/// ```
#[derive(Debug, Clone)]
pub struct ResolvedPackage {
    /// Reverse-FQDN group — the identity qualifier (PROP-008 §2.2). With
    /// `name` it forms the `(group, name)` identity the registry resolves
    /// by; `kind` is metadata, read off the manifest once fetched.
    pub group: Group,
    pub name: String,
    pub version: semver::Version,
    /// Absolute path to the package's source directory inside the registry.
    pub source_dir: PathBuf,
}

/// A resolved package copied into the per-project cache.
///
/// Produced by [`Registry::fetch`]; the manifest comes off the cached
/// copy and always carries a `[package]` table (guarded at every
/// construction site, which is what makes [`CachedPackage::package_meta`]
/// sound):
///
/// ```
/// use std::path::PathBuf;
/// use vibe_core::Group;
/// use vibe_core::manifest::Manifest;
/// use vibe_registry::{CachedPackage, ResolvedPackage};
///
/// let manifest = Manifest::parse_str(
///     "[package]\ngroup = \"org.vibevm\"\nname = \"wal\"\nkind = \"flow\"\nversion = \"0.2.0\"\n",
/// )
/// .unwrap();
/// let cached = CachedPackage {
///     resolved: ResolvedPackage {
///         group: Group::parse("org.vibevm").unwrap(),
///         name: "wal".to_string(),
///         version: semver::Version::parse("0.2.0").unwrap(),
///         source_dir: PathBuf::from("registry/org.vibevm/wal/v0.2.0"),
///     },
///     cache_dir: PathBuf::from(".vibe/cache/org.vibevm/wal/v0.2.0"),
///     manifest,
///     content_hash: "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
///         .to_string(),
///     source_uri: "file:///registry/org.vibevm/wal/v0.2.0".to_string(),
///     registry_name: Some("main".to_string()),
///     source_ref: Some("v0.2.0".to_string()),
///     resolved_commit: None,
///     overridden: false,
///     is_git_source: false,
///     is_path_source: false,
///     via_redirect: None,
/// };
/// assert_eq!(cached.package_meta().version.to_string(), "0.2.0");
/// ```
#[derive(Debug, Clone)]
pub struct CachedPackage {
    pub resolved: ResolvedPackage,
    /// Absolute path to the cache directory (contents are a verbatim copy of
    /// `resolved.source_dir`).
    pub cache_dir: PathBuf,
    /// Parsed manifest from the cached copy.
    pub manifest: Manifest,
    /// `sha256:<hex>` content hash over every file in the package, using
    /// relative paths for stability. The **identity** half of the
    /// `(group, name, version, content_hash)` tuple per PROP-002 §2.1.
    pub content_hash: String,
    /// Source URI recorded in the lockfile under the `source_url` field.
    /// Informational — package identity does not depend on this string.
    pub source_uri: String,
    /// Name of the `[[registry]]` entry in `vibe.toml` that served this
    /// package. `None` for `LocalRegistry` (`--registry <path>`), the
    /// legacy monorepo `GitRegistry`, and packages resolved through
    /// `[[override]]`.
    pub registry_name: Option<String>,
    /// Git ref the content was fetched at — typically `v<version>` for
    /// per-package registries; the override's ref for `[[override]]`-resolved
    /// packages. `None` for non-git sources (file://, M0 local-directory).
    pub source_ref: Option<String>,
    /// Commit hash the ref resolved to at fetch time — `git rev-parse HEAD`
    /// of the clone at the requested tag, read via [`GitBackend::head_commit`].
    /// Populated by the per-package registry fetch path and recorded in the
    /// lockfile so a re-clone reconstructs identical content, including every
    /// submodule's gitlink (PROP-021 §2.4), and an `in-place` slot's identity
    /// is its commit (PROP-022 §2.5). `None` for non-git registries (file://,
    /// M0 local-directory) and the legacy monorepo path — no upstream commit
    /// to pin — and for any backend whose `head_commit` returns `None`.
    pub resolved_commit: Option<String>,
    /// `true` iff this package was resolved through a `[[override]]`
    /// entry rather than the registry layer.
    pub overridden: bool,
    /// `true` iff this package was resolved through a `[requires.packages]`
    /// table-form git-source declaration (PROP-002 §2.4.1) rather than
    /// the registry walk or `[[override]]`. Mutually exclusive with
    /// `overridden`. Lockfile maps this to `source_kind = "git"`.
    pub is_git_source: bool,

    /// `true` iff this package was resolved through a `[requires.packages]`
    /// table-form path-source declaration (PROP-007 §2.5) — a package in
    /// a local directory, typically a sibling workspace member — rather
    /// than the registry walk, `[[override]]`, or git-source. Mutually
    /// exclusive with `overridden` and `is_git_source`. Lockfile maps
    /// this to `source_kind = "path"`, and `source_uri` then carries the
    /// member's path relative to the workspace root — never a URL, never
    /// an absolute path.
    pub is_path_source: bool,

    /// When this package was resolved via a registry stub that
    /// redirected to an external URL (PROP-002 §2.4.2), the **stub**
    /// URL is recorded here while `source_uri` carries the **target**
    /// URL. `None` for direct registry / git-source / override
    /// resolutions. Lockfile mirrors this verbatim into
    /// `LockedPackage.via_redirect`.
    pub via_redirect: Option<String>,
}

impl CachedPackage {
    /// A `CachedPackage` always holds a publishable `[package]` manifest.
    ///
    /// Every construction site of `CachedPackage` reads a manifest off a
    /// fetched registry package and guards `manifest.package.is_some()`
    /// before building the struct, so this accessor's `.expect()` is
    /// sound — a fetched registry package always carries a `[package]`
    /// table.
    #[spec(
        deviates = "spec://vibevm/discipline/ENGINE-CONFORM-v0.1#rules",
        reason = "no-unwrap-in-domain: every CachedPackage construction site guards \
                  manifest.package.is_some() before building the struct, so the \
                  accessor's expect is a checked invariant; returning Result would \
                  force every reader of an already-validated package through error \
                  plumbing for a state the constructors exclude"
    )]
    pub fn package_meta(&self) -> &vibe_core::manifest::PackageMeta {
        self.manifest
            .package
            .as_ref()
            .expect("a fetched registry package always carries a [package] table")
    }
}

/// What [`GitPackageRegistry::materialise_in_place`](crate::GitPackageRegistry)
/// produced: the in-place slot is already populated on disk (a fresh clone or
/// an incremental `git fetch`), so these are the records the install layer
/// needs to write the lockfile entry and compose boot (PROP-022 §2.4/§2.5).
#[derive(Debug, Clone)]
pub struct InPlaceMaterialised {
    /// Canonical per-package source URL — the lockfile's `source_url`.
    pub source_uri: String,
    /// The version tag the slot was placed at (`v<version>`).
    pub source_ref: String,
    /// The commit the tag resolved to — the in-place identity (§2.5).
    pub resolved_commit: Option<String>,
    /// `sha256:<hex>` over the resolved commit — the lockfile `content_hash`
    /// for an in-place slot (a cheap, stable commit-derived hash, never a tree
    /// walk; identity is the commit). `sha256:` of the empty string when the
    /// backend reported no commit.
    pub content_hash: String,
    /// The slot's manifest, read back after placement.
    pub manifest: vibe_core::manifest::Manifest,
}

pub(crate) fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), RegistryError> {
    fs::create_dir_all(dst).map_err(|source| RegistryError::Io {
        path: dst.to_path_buf(),
        source,
    })?;
    for entry in WalkDir::new(src).into_iter().filter_map(|e| e.ok()) {
        let rel = entry.path().strip_prefix(src).unwrap_or(entry.path());
        let target = dst.join(rel);
        if entry.file_type().is_dir() {
            fs::create_dir_all(&target).map_err(|source| RegistryError::Io {
                path: target.clone(),
                source,
            })?;
        } else if entry.file_type().is_file() {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).map_err(|source| RegistryError::Io {
                    path: parent.to_path_buf(),
                    source,
                })?;
            }
            fs::copy(entry.path(), &target).map_err(|source| RegistryError::Io {
                path: target.clone(),
                source,
            })?;
        }
    }
    Ok(())
}

/// sha256 of concatenated (rel_path_bytes || 0x00 || file_bytes || 0x00) for
/// every file in the package, traversed in sorted order for determinism.
///
/// This is the **identity** half of the `(group, name, version,
/// content_hash)` tuple (PROP-002 §2.1). Reads every file under
/// `pkg_dir`:
///
/// ```no_run
/// use std::path::Path;
/// use vibe_registry::compute_content_hash;
///
/// let hash = compute_content_hash(Path::new("path/to/package")).unwrap();
/// assert!(hash.starts_with("sha256:"));
/// ```
pub fn compute_content_hash(pkg_dir: &Path) -> Result<String, RegistryError> {
    let mut files: Vec<PathBuf> = WalkDir::new(pkg_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(|e| e.path().to_path_buf())
        .collect();
    files.sort();

    let mut hasher = Sha256::new();
    for path in &files {
        let rel = path.strip_prefix(pkg_dir).unwrap_or(path);
        let rel_normalized = rel.to_string_lossy().replace('\\', "/");
        hasher.update(rel_normalized.as_bytes());
        hasher.update([0]);
        let bytes = fs::read(path).map_err(|source| RegistryError::Io {
            path: path.clone(),
            source,
        })?;
        hasher.update(&bytes);
        hasher.update([0]);
    }
    let digest = hasher.finalize();
    let hex = digest.iter().fold(String::new(), |mut s, b| {
        use std::fmt::Write;
        let _ = write!(&mut s, "{b:02x}");
        s
    });
    Ok(format!("sha256:{hex}"))
}
