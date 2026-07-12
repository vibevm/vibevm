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

use std::path::{Path, PathBuf};

use specmark::spec;
use vibe_core::manifest::Manifest;
use vibe_core::{Group, PackageRef};

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
        deviates = "spec://core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#rules",
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
///
/// ```
/// use vibe_core::manifest::Manifest;
/// use vibe_registry::InPlaceMaterialised;
///
/// // The install layer reads these back off a freshly-placed in-place slot.
/// let manifest = Manifest::parse_str(
///     "[package]\ngroup = \"org.vibevm\"\nname = \"giant\"\nkind = \"feat\"\nversion = \"1.0.0\"\nmaterialization = \"in-place\"\n",
/// )
/// .unwrap();
/// let placed = InPlaceMaterialised {
///     source_uri: "https://example.test/giant.git".to_string(),
///     source_ref: "v1.0.0".to_string(),
///     resolved_commit: Some("9e3c1f0a".to_string()),
///     content_hash: "sha256:1d3a…".to_string(),
///     manifest,
/// };
/// // Identity is the resolved commit (§2.5); the tag rode along for the lockfile.
/// assert_eq!(placed.source_ref, "v1.0.0");
/// assert!(placed.manifest.package.is_some());
/// ```
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

mod error;
mod shippable;

pub use error::RegistryError;
pub use shippable::compute_content_hash;
pub(crate) use shippable::copy_dir_recursive;
