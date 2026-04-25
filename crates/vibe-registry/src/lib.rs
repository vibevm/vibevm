//! Registry access: resolve, fetch, cache.
//!
//! M0 shipped a local-directory registry laid out per
//! `VIBEVM-SPEC.md` §8.2:
//!
//! ```text
//! <registry>/<kind>/<name>/v<major>.<minor>.<patch>/vibe-package.toml
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

use std::fs;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use thiserror::Error;
use vibe_core::manifest::PackageManifest;
use vibe_core::{PackageKind, PackageRef, VersionSpec};
use walkdir::WalkDir;

pub mod git_backend;
pub mod git_package_registry;
pub mod git_registry;
pub mod multi_registry_resolver;

pub use git_backend::{GitBackend, GitError, ShellGit};
pub use git_package_registry::GitPackageRegistry;
pub use git_registry::{GitRegistry, RegistryMeta, default_cache_root};
pub use multi_registry_resolver::{
    DEFAULT_OVERRIDE_REF, MultiRegistryResolver, MultiResolution,
};

#[derive(Debug, Error)]
pub enum RegistryError {
    #[error("registry root `{0}` does not exist or is not a directory")]
    MissingRoot(PathBuf),

    #[error("package `{kind}:{name}` is not in the registry")]
    UnknownPackage {
        kind: PackageKind,
        name: String,
    },

    #[error("no version of `{kind}:{name}` matches `{req}`")]
    NoMatchingVersion {
        kind: PackageKind,
        name: String,
        req: String,
    },

    #[error("registry entry at `{path}` has an invalid directory name `{name}` — expected `v<semver>`")]
    BadVersionDir { path: PathBuf, name: String },

    #[error(transparent)]
    Core(#[from] vibe_core::Error),

    #[error("git operation failed: {0}")]
    Git(#[from] GitError),

    #[error(
        "could not determine the user home directory; set HOME (or USERPROFILE on Windows), or pass an explicit cache root"
    )]
    NoHomeDir,

    #[error("registry meta file at `{path}` is malformed: {reason}")]
    MalformedMeta { path: PathBuf, reason: String },

    #[error("I/O error on `{path}`")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

/// Uniform surface over all registry backends — [`LocalRegistry`] and
/// [`GitRegistry`] both implement this trait. `vibe-install` and
/// `vibe-cli` consume registries exclusively through the trait so the
/// concrete backend can be chosen at CLI-argument-parse time.
pub trait Registry {
    fn list_versions(
        &self,
        kind: PackageKind,
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
#[derive(Debug, Clone)]
pub struct ResolvedPackage {
    pub kind: PackageKind,
    pub name: String,
    pub version: semver::Version,
    /// Absolute path to the package's source directory inside the registry.
    pub source_dir: PathBuf,
}

/// A resolved package copied into the per-project cache.
#[derive(Debug, Clone)]
pub struct CachedPackage {
    pub resolved: ResolvedPackage,
    /// Absolute path to the cache directory (contents are a verbatim copy of
    /// `resolved.source_dir`).
    pub cache_dir: PathBuf,
    /// Parsed manifest from the cached copy.
    pub manifest: PackageManifest,
    /// `sha256:<hex>` content hash over every file in the package, using
    /// relative paths for stability. The **identity** half of the
    /// `(kind, name, version, content_hash)` tuple per PROP-002 §2.1.
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
    /// Commit hash the ref resolved to at fetch time. Reserved for the
    /// resolver-aware install pipeline; populated by [`GitPackageRegistry`]
    /// in a follow-up commit. Always `None` here.
    pub resolved_commit: Option<String>,
    /// `true` iff this package was resolved through a `[[override]]`
    /// entry rather than the registry layer.
    pub overridden: bool,
}

pub struct LocalRegistry {
    root: PathBuf,
}

impl LocalRegistry {
    pub fn new(root: impl Into<PathBuf>) -> Result<Self, RegistryError> {
        let root = root.into();
        if !root.is_dir() {
            return Err(RegistryError::MissingRoot(root));
        }
        Ok(LocalRegistry { root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// List every version available for `<kind>:<name>`, sorted ascending.
    pub fn list_versions(
        &self,
        kind: PackageKind,
        name: &str,
    ) -> Result<Vec<semver::Version>, RegistryError> {
        let dir = self.root.join(kind.as_str()).join(name);
        if !dir.is_dir() {
            return Err(RegistryError::UnknownPackage {
                kind,
                name: name.to_owned(),
            });
        }
        let mut versions = Vec::new();
        let entries = fs::read_dir(&dir).map_err(|source| RegistryError::Io {
            path: dir.clone(),
            source,
        })?;
        for entry in entries {
            let entry = entry.map_err(|source| RegistryError::Io {
                path: dir.clone(),
                source,
            })?;
            if !entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                continue;
            }
            let n = entry.file_name();
            let s = n.to_string_lossy();
            let Some(ver_str) = s.strip_prefix('v') else {
                continue;
            };
            match semver::Version::parse(ver_str) {
                Ok(v) => versions.push(v),
                Err(_) => {
                    return Err(RegistryError::BadVersionDir {
                        path: entry.path(),
                        name: s.into_owned(),
                    });
                }
            }
        }
        versions.sort();
        Ok(versions)
    }

    /// Pick the highest version that satisfies `req`.
    pub fn resolve(&self, pkgref: &PackageRef) -> Result<ResolvedPackage, RegistryError> {
        let versions = self.list_versions(pkgref.kind, &pkgref.name)?;
        let picked = match &pkgref.version {
            VersionSpec::Latest => {
                // Latest stable = highest version with no pre-release segment.
                versions.iter().rev().find(|v| v.pre.is_empty()).cloned()
            }
            VersionSpec::Req(req) => versions
                .iter()
                .rev()
                .find(|v| req.matches(v) && v.pre.is_empty())
                .or_else(|| versions.iter().rev().find(|v| req.matches(v)))
                .cloned(),
        };
        let Some(version) = picked else {
            return Err(RegistryError::NoMatchingVersion {
                kind: pkgref.kind,
                name: pkgref.name.clone(),
                req: match &pkgref.version {
                    VersionSpec::Latest => "latest".to_string(),
                    VersionSpec::Req(r) => r.to_string(),
                },
            });
        };
        let source_dir = self
            .root
            .join(pkgref.kind.as_str())
            .join(&pkgref.name)
            .join(format!("v{version}"));
        Ok(ResolvedPackage {
            kind: pkgref.kind,
            name: pkgref.name.clone(),
            version,
            source_dir,
        })
    }

    /// Copy a resolved package into `<cache_root>/<kind>/<name>/<version>/`
    /// and return a `CachedPackage` with manifest and content hash populated.
    pub fn fetch(
        &self,
        resolved: &ResolvedPackage,
        cache_root: &Path,
    ) -> Result<CachedPackage, RegistryError> {
        let cache_dir = cache_root
            .join(resolved.kind.as_str())
            .join(&resolved.name)
            .join(format!("v{}", resolved.version));

        if cache_dir.exists() {
            fs::remove_dir_all(&cache_dir).map_err(|source| RegistryError::Io {
                path: cache_dir.clone(),
                source,
            })?;
        }
        copy_dir_recursive(&resolved.source_dir, &cache_dir)?;

        let manifest_path = cache_dir.join(PackageManifest::FILENAME);
        let manifest = PackageManifest::read(&manifest_path)?;
        let content_hash = compute_content_hash(&cache_dir)?;

        // Build a source URI. On local-registry M0 we just encode the absolute
        // source path. We intentionally do NOT include the cache path.
        let source_uri = source_uri_for_local(&resolved.source_dir);

        Ok(CachedPackage {
            resolved: resolved.clone(),
            cache_dir,
            manifest,
            content_hash,
            source_uri,
            // `LocalRegistry` (`--registry <path>`) is the M0-shape and
            // does not participate in the per-package / multi-registry
            // model; it leaves the lockfile-v2 provenance fields blank.
            registry_name: None,
            source_ref: None,
            resolved_commit: None,
            overridden: false,
        })
    }
}

impl Registry for LocalRegistry {
    fn list_versions(
        &self,
        kind: PackageKind,
        name: &str,
    ) -> Result<Vec<semver::Version>, RegistryError> {
        LocalRegistry::list_versions(self, kind, name)
    }
    fn resolve(&self, pkgref: &PackageRef) -> Result<ResolvedPackage, RegistryError> {
        LocalRegistry::resolve(self, pkgref)
    }
    fn fetch(
        &self,
        resolved: &ResolvedPackage,
        cache_root: &Path,
    ) -> Result<CachedPackage, RegistryError> {
        LocalRegistry::fetch(self, resolved, cache_root)
    }
}

fn source_uri_for_local(path: &Path) -> String {
    let mut s = path.to_string_lossy().replace('\\', "/");
    // Ensure a leading `/` on Windows paths like `C:/Users/…` so the URI has
    // the `file:///C:/Users/...` shape.
    if !s.starts_with('/') {
        s.insert(0, '/');
    }
    format!("file://{s}")
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn make_fixture_registry() -> (tempfile::TempDir, PathBuf) {
        let dir = tempdir().unwrap();
        let root = dir.path().to_path_buf();

        // flow/wal/v0.1.0
        let v1 = root.join("flow/wal/v0.1.0");
        fs::create_dir_all(&v1).unwrap();
        fs::write(
            v1.join("vibe-package.toml"),
            r#"[package]
name = "wal"
kind = "flow"
version = "0.1.0"
description = "WAL v0.1.0"
"#,
        )
        .unwrap();
        fs::write(v1.join("README.md"), "# wal 0.1.0\n").unwrap();

        // flow/wal/v0.2.0
        let v2 = root.join("flow/wal/v0.2.0");
        fs::create_dir_all(&v2).unwrap();
        fs::write(
            v2.join("vibe-package.toml"),
            r#"[package]
name = "wal"
kind = "flow"
version = "0.2.0"
description = "WAL v0.2.0"
"#,
        )
        .unwrap();
        fs::write(v2.join("README.md"), "# wal 0.2.0\n").unwrap();

        (dir, root)
    }

    #[test]
    fn lists_versions_sorted() {
        let (_guard, root) = make_fixture_registry();
        let reg = LocalRegistry::new(root).unwrap();
        let versions = reg.list_versions(PackageKind::Flow, "wal").unwrap();
        assert_eq!(
            versions.iter().map(|v| v.to_string()).collect::<Vec<_>>(),
            vec!["0.1.0", "0.2.0"]
        );
    }

    #[test]
    fn resolves_latest() {
        let (_guard, root) = make_fixture_registry();
        let reg = LocalRegistry::new(root).unwrap();
        let pkgref = PackageRef::parse("flow:wal").unwrap();
        let r = reg.resolve(&pkgref).unwrap();
        assert_eq!(r.version.to_string(), "0.2.0");
    }

    #[test]
    fn resolves_exact_version() {
        let (_guard, root) = make_fixture_registry();
        let reg = LocalRegistry::new(root).unwrap();
        let pkgref = PackageRef::parse("flow:wal@0.1.0").unwrap();
        let r = reg.resolve(&pkgref).unwrap();
        assert_eq!(r.version.to_string(), "0.1.0");
    }

    #[test]
    fn resolves_range_to_highest_match() {
        let (_guard, root) = make_fixture_registry();
        let reg = LocalRegistry::new(root).unwrap();
        let pkgref = PackageRef::parse("flow:wal@^0.1").unwrap();
        let r = reg.resolve(&pkgref).unwrap();
        // ^0.1 → >=0.1.0, <0.2.0 — so only 0.1.0 qualifies.
        assert_eq!(r.version.to_string(), "0.1.0");
    }

    #[test]
    fn unknown_package_errors_clearly() {
        let (_guard, root) = make_fixture_registry();
        let reg = LocalRegistry::new(root).unwrap();
        let pkgref = PackageRef::parse("flow:nope").unwrap();
        let err = reg.resolve(&pkgref).unwrap_err();
        assert!(matches!(err, RegistryError::UnknownPackage { .. }));
    }

    #[test]
    fn no_matching_version_errors() {
        let (_guard, root) = make_fixture_registry();
        let reg = LocalRegistry::new(root).unwrap();
        let pkgref = PackageRef::parse("flow:wal@^9.0").unwrap();
        let err = reg.resolve(&pkgref).unwrap_err();
        assert!(matches!(err, RegistryError::NoMatchingVersion { .. }));
    }

    #[test]
    fn fetch_populates_cache() {
        let (_guard, root) = make_fixture_registry();
        let reg = LocalRegistry::new(root).unwrap();
        let cache_dir = tempdir().unwrap();
        let pkgref = PackageRef::parse("flow:wal@0.2.0").unwrap();
        let resolved = reg.resolve(&pkgref).unwrap();
        let cached = reg.fetch(&resolved, cache_dir.path()).unwrap();
        assert!(cached.cache_dir.join("vibe-package.toml").exists());
        assert!(cached.cache_dir.join("README.md").exists());
        assert_eq!(cached.manifest.package.version.to_string(), "0.2.0");
        assert!(cached.content_hash.starts_with("sha256:"));
        assert!(cached.source_uri.starts_with("file://"));
    }

    #[test]
    fn content_hash_is_stable() {
        let (_guard, root) = make_fixture_registry();
        let reg = LocalRegistry::new(root).unwrap();
        let cache_a = tempdir().unwrap();
        let cache_b = tempdir().unwrap();
        let pkgref = PackageRef::parse("flow:wal@0.2.0").unwrap();
        let resolved = reg.resolve(&pkgref).unwrap();
        let a = reg.fetch(&resolved, cache_a.path()).unwrap();
        let b = reg.fetch(&resolved, cache_b.path()).unwrap();
        assert_eq!(a.content_hash, b.content_hash);
    }
}
