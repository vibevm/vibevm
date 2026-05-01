//! Per-package git registry — PROP-002.
//!
//! `GitPackageRegistry` resolves a [`PackageRef`] against an organization-root
//! URL by:
//!
//! 1. Composing the per-package repo URL via the registry's [`NamingConvention`]
//!    (`flow:wal` + `KindName` → `<org>/flow-wal.git`).
//! 2. Listing tags on that repo via the cheap [`GitBackend::list_tags`]
//!    primitive — no clone.
//! 3. Filtering tags to `v<semver>` and picking the highest match for the
//!    requested [`VersionSpec`].
//! 4. For dep-graph walks: reading the candidate version's manifest via
//!    [`GitBackend::fetch_file_at_ref`] — still no clone.
//! 5. Only when the resolver commits to installing a specific version:
//!    [`GitBackend::bootstrap`] (or [`GitBackend::update`] if the clone
//!    already exists), copy the worktree into the per-project package
//!    cache (excluding `.git/`), parse manifest, compute `content_hash`.
//!
//! The cache layout follows PROP-002 §2.6:
//!
//! ```text
//! <cache_root>/<canonical_url_hash>/packages/<kind>-<name>/clone/
//! ```
//!
//! `<canonical_url_hash>` is keyed off the **canonical organization URL** of
//! the registry (not the mirror URL), so a transparent mirror does not
//! invalidate the cache. The internal cache subpath uses `<kind>-<name>`
//! always, decoupled from the registry's URL-shape `naming` — the cache is
//! organized by our identity, the URLs are just one routing decision.
//!
//! Spec: [PROP-002 §2.5 / §2.6 / §2.12](../../../spec/modules/vibe-registry/PROP-002-decentralized-registry.md).

use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use sha2::{Digest, Sha256};
use vibe_core::manifest::{NamingConvention, PackageManifest};
use vibe_core::{PackageKind, PackageRef, VersionSpec};

use crate::git_backend::{GitBackend, GitError, ShellGit};
use crate::git_registry::{
    DEFAULT_FRESHNESS_SECS, default_cache_root, normalize_url, strip_git_plus_prefix,
};
use crate::{
    CachedPackage, Registry, RegistryError, ResolvedPackage, compute_content_hash,
};

/// Per-package git registry — one organization URL, many package repos under it.
pub struct GitPackageRegistry {
    backend: Arc<dyn GitBackend>,
    name: String,
    org_url: String,
    org_ref: String,
    naming: NamingConvention,
    cache_root: PathBuf,
    canonical_hash: String,
    /// Org-level mirror URLs in priority order (lower index = tried
    /// first). Mirrors share the registry's [`NamingConvention`], so
    /// each mirror URL is treated as an alternate org root from which
    /// per-package URLs are composed identically. Empty in M0/M1.1 and
    /// when `vibe.toml` carries no `[[mirror]]` for this registry.
    /// Phase B v0 wires this only for the read-only lookup paths
    /// (`list_versions`, `fetch_dep_manifest` archive path) — the
    /// fetch/clone path stays primary-only until cross-source
    /// `content_hash` verification lands.
    mirror_urls: Vec<String>,
    /// Implicit-update freshness TTL — reserved for the next commit, where
    /// per-package `meta.toml` files track `last_synced_at`. Stored now so
    /// callers parameterising it do not need to thread it through later.
    #[allow(dead_code)]
    freshness_secs: u64,
}

impl GitPackageRegistry {
    /// Open a registry against the default cache root and a fresh
    /// [`ShellGit`] backend.
    pub fn open(
        name: &str,
        org_url: &str,
        org_ref: &str,
        naming: NamingConvention,
    ) -> Result<Self, RegistryError> {
        let cache_root = default_cache_root()?;
        Self::open_with(
            name,
            org_url,
            org_ref,
            naming,
            &cache_root,
            Arc::new(ShellGit::new()),
            DEFAULT_FRESHNESS_SECS,
        )
    }

    /// Lower-level constructor for tests and callers that want to plug in a
    /// custom backend or cache root.
    pub fn open_with(
        name: &str,
        org_url: &str,
        org_ref: &str,
        naming: NamingConvention,
        cache_root: &Path,
        backend: Arc<dyn GitBackend>,
        freshness_secs: u64,
    ) -> Result<Self, RegistryError> {
        Self::open_with_mirrors(
            name,
            org_url,
            org_ref,
            naming,
            Vec::new(),
            cache_root,
            backend,
            freshness_secs,
        )
    }

    /// Like [`open_with`](Self::open_with), but accepts an org-level
    /// mirror chain in priority order. Used by the multi-registry
    /// resolver to thread `[[mirror]]` from `vibe.toml` into the
    /// registry instance. Empty `mirror_urls` is the same as
    /// [`open_with`].
    #[allow(clippy::too_many_arguments)]
    pub fn open_with_mirrors(
        name: &str,
        org_url: &str,
        org_ref: &str,
        naming: NamingConvention,
        mirror_urls: Vec<String>,
        cache_root: &Path,
        backend: Arc<dyn GitBackend>,
        freshness_secs: u64,
    ) -> Result<Self, RegistryError> {
        let normalized = normalize_url(org_url);
        let canonical_hash = short_url_hash(&normalized);
        let cache_root_owned = cache_root.to_path_buf();

        // Ensure the registry-bucket directory exists. Nothing else gets
        // written here in this commit — the `meta.toml` for the bucket
        // lands together with the freshness machinery.
        let bucket = cache_root_owned.join(&canonical_hash);
        fs::create_dir_all(&bucket).map_err(|source| RegistryError::Io {
            path: bucket.clone(),
            source,
        })?;

        Ok(GitPackageRegistry {
            backend,
            name: name.to_string(),
            org_url: org_url.to_string(),
            org_ref: org_ref.to_string(),
            naming,
            cache_root: cache_root_owned,
            canonical_hash,
            mirror_urls,
            freshness_secs,
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn org_url(&self) -> &str {
        &self.org_url
    }

    pub fn org_ref(&self) -> &str {
        &self.org_ref
    }

    pub fn naming(&self) -> NamingConvention {
        self.naming
    }

    /// Root of this registry's cache bucket — `<cache_root>/<hash>/`.
    pub fn cache_dir(&self) -> PathBuf {
        self.cache_root.join(&self.canonical_hash)
    }

    /// Compose the per-package repo URL — `<org_url>/<naming(kind, name)>.git`.
    /// Trailing slashes on `org_url` are tolerated.
    pub fn package_repo_url(&self, kind: PackageKind, name: &str) -> String {
        let repo_name = self.naming.repo_name(kind, name);
        let trimmed = self.org_url.trim_end_matches('/');
        format!("{trimmed}/{repo_name}.git")
    }

    /// All URLs to try for a `(kind, name)` lookup, primary first.
    /// Mirrors are composed using the same naming convention as the
    /// primary, since the mirror is meant to be a transparent
    /// alternative to the primary's content.
    fn package_urls(&self, kind: PackageKind, name: &str) -> Vec<String> {
        let repo_name = self.naming.repo_name(kind, name);
        let mut urls = Vec::with_capacity(1 + self.mirror_urls.len());
        urls.push(format!(
            "{}/{}.git",
            self.org_url.trim_end_matches('/'),
            repo_name
        ));
        for mirror in &self.mirror_urls {
            urls.push(format!("{}/{}.git", mirror.trim_end_matches('/'), repo_name));
        }
        urls
    }

    /// Run a read-only lookup `f` against the primary URL first, then
    /// each mirror URL in priority order. Returns the first `Ok`
    /// produced by any URL. If every URL fails, the **primary's**
    /// error is returned (not the last mirror's) — the primary is the
    /// canonical source and its diagnostic is the most useful one for
    /// the operator. Mirror errors are recorded in `tracing::debug!`
    /// for ops to correlate.
    ///
    /// `f` MUST be a pure read against the host (no cache writes, no
    /// per-package clone state) — the fetch / refresh paths use
    /// dedicated logic with content-hash verification across mirrors.
    fn try_lookup<T, F>(
        &self,
        kind: PackageKind,
        name: &str,
        f: F,
    ) -> Result<T, RegistryError>
    where
        F: Fn(&str) -> Result<T, RegistryError>,
    {
        let urls = self.package_urls(kind, name);
        let mut primary_err: Option<RegistryError> = None;
        for (i, url) in urls.iter().enumerate() {
            match f(url) {
                Ok(v) => {
                    if i > 0 {
                        tracing::info!(
                            target: "vibe_registry",
                            registry = %self.name,
                            primary = %urls[0],
                            served_by = %url,
                            mirror_index = i - 1,
                            "lookup served by mirror"
                        );
                    }
                    return Ok(v);
                }
                Err(e) => {
                    if i == 0 {
                        primary_err = Some(e);
                    } else {
                        tracing::debug!(
                            target: "vibe_registry",
                            registry = %self.name,
                            mirror = %url,
                            error = %e,
                            "mirror lookup failed; trying next"
                        );
                    }
                }
            }
        }
        // Safety: urls always has at least one entry (primary), so the
        // first iteration sets primary_err on Err. If primary returned
        // Ok we'd have returned already; if it returned Err and no
        // mirror saved us, primary_err is the right diagnostic.
        Err(primary_err.expect("primary URL must exist"))
    }

    /// Where this package's clone lives on disk —
    /// `<cache_dir>/packages/<kind>-<name>/clone/`. Note the internal
    /// subdirectory is always `<kind>-<name>`, regardless of registry naming
    /// (which may have produced a different *URL*-side name).
    pub fn package_clone_dir(&self, kind: PackageKind, name: &str) -> PathBuf {
        let internal = format!("{}-{}", kind.as_str(), name);
        self.cache_dir().join("packages").join(internal).join("clone")
    }

    /// Enumerate available versions for `<kind>:<name>` *without cloning*.
    /// Tags that don't match `v<semver>` are silently dropped.
    ///
    /// Mirror-aware: tries the primary URL first, then each mirror in
    /// priority order. The first URL that yields a tag list wins. If
    /// every URL says `RepoNotFound`, the result is `UnknownPackage`
    /// (treated identically to the primary-only path).
    pub fn list_versions(
        &self,
        kind: PackageKind,
        name: &str,
    ) -> Result<Vec<semver::Version>, RegistryError> {
        let backend = Arc::clone(&self.backend);
        let owned_name = name.to_owned();
        self.try_lookup(kind, name, move |url| {
            let tags = backend
                .list_tags(strip_git_plus_prefix(url))
                .map_err(|e| match e {
                    GitError::RepoNotFound { .. } => RegistryError::UnknownPackage {
                        kind,
                        name: owned_name.clone(),
                    },
                    other => RegistryError::Git(other),
                })?;
            let mut versions: Vec<semver::Version> = tags
                .iter()
                .filter_map(|t| {
                    let stripped = t.strip_prefix('v')?;
                    semver::Version::parse(stripped).ok()
                })
                .collect();
            versions.sort();
            Ok(versions)
        })
    }

    /// Pick the best tag matching `pkgref.version` from the upstream tag list.
    /// Returns a [`ResolvedPackage`] whose `source_dir` points at the
    /// (not-yet-populated) clone directory under the cache bucket.
    pub fn resolve(&self, pkgref: &PackageRef) -> Result<ResolvedPackage, RegistryError> {
        let versions = self.list_versions(pkgref.kind, &pkgref.name)?;
        let picked = match &pkgref.version {
            VersionSpec::Latest => versions.iter().rev().find(|v| v.pre.is_empty()).cloned(),
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
        Ok(ResolvedPackage {
            kind: pkgref.kind,
            name: pkgref.name.clone(),
            version,
            source_dir: self.package_clone_dir(pkgref.kind, &pkgref.name),
        })
    }

    /// Read a candidate version's `vibe-package.toml` *without cloning*. The
    /// depsolver calls this during the resolve walk to read declared
    /// `[requires]` of a candidate before committing to install. A walk
    /// over N candidates of one package costs N `git archive` round-trips,
    /// not N clones.
    ///
    /// Mirror-aware on the archive path: the primary URL is tried
    /// first, then each mirror in priority order. The clone-fallback
    /// path (used when *every* URL says `ArchiveUnsupported`) clones
    /// only against the primary URL — the clone state is shared and
    /// cross-source verification has not yet landed (Phase B v0).
    pub fn fetch_dep_manifest(
        &self,
        kind: PackageKind,
        name: &str,
        version: &semver::Version,
    ) -> Result<PackageManifest, RegistryError> {
        let tag = format!("v{version}");
        let backend = Arc::clone(&self.backend);
        let tag_for_lookup = tag.clone();
        let archive_result = self.try_lookup(kind, name, move |url| {
            backend
                .fetch_file_at_ref(
                    strip_git_plus_prefix(url),
                    &tag_for_lookup,
                    PackageManifest::FILENAME,
                )
                .map_err(RegistryError::from)
        });
        let url = self.package_repo_url(kind, name);
        let bytes = match archive_result {
            Ok(bytes) => bytes,
            Err(RegistryError::Git(GitError::ArchiveUnsupported { .. })) => {
                // GitHub (and a few other hosts) disable
                // `upload-archive` server-side, so `git archive --remote`
                // can't pull a single file without cloning. Fall back to
                // a per-package shallow clone at the requested tag and
                // read the manifest from the working tree. Slower than
                // the archive path but works on every git host that
                // accepts `git clone`. The clone lands in the same
                // per-package cache directory the install path would
                // use anyway, so this is also pre-warming the cache for
                // the imminent install.
                //
                // Phase B v0: the clone fallback talks only to the
                // primary URL. Mirror dispatch for the clone path
                // requires the cross-source `content_hash` check to
                // come along with it, so it lands together with that.
                self.refresh_package(kind, name, &tag)?;
                let clone_dir = self.package_clone_dir(kind, name);
                let manifest_path = clone_dir.join(PackageManifest::FILENAME);
                fs::read(&manifest_path).map_err(|source| RegistryError::Io {
                    path: manifest_path.clone(),
                    source,
                })?
            }
            Err(other) => return Err(other),
        };
        let text = String::from_utf8(bytes).map_err(|e| RegistryError::MalformedMeta {
            path: PathBuf::from(format!("{url}@{tag}:{}", PackageManifest::FILENAME)),
            reason: format!("invalid UTF-8: {e}"),
        })?;
        let mut manifest: PackageManifest =
            toml::from_str(&text).map_err(|e| RegistryError::MalformedMeta {
                path: PathBuf::from(format!("{url}@{tag}:{}", PackageManifest::FILENAME)),
                reason: e.to_string(),
            })?;
        // Apply the same legacy-deps migration the on-disk reader does, so
        // resolver consumers always see modern-form `[requires]` / `[conflicts]`.
        manifest.normalize_legacy_deps();
        Ok(manifest)
    }

    /// Refresh the per-package clone for `(kind, name)` against `refname`
    /// without touching the per-project cache. If the clone exists, runs
    /// `update`; otherwise bootstraps a fresh clone.
    ///
    /// Used by `vibe registry sync` to walk lockfile entries and pull
    /// upstream changes for everything currently installed, without
    /// re-applying writes (that's `vibe update`'s job, not sync's).
    pub fn refresh_package(
        &self,
        kind: PackageKind,
        name: &str,
        refname: &str,
    ) -> Result<(), RegistryError> {
        let url = self.package_repo_url(kind, name);
        let clone_dir = self.package_clone_dir(kind, name);
        if clone_dir.join(".git").exists() {
            self.backend.update(&clone_dir, refname)?;
        } else {
            if clone_dir.exists() {
                fs::remove_dir_all(&clone_dir).map_err(|source| RegistryError::Io {
                    path: clone_dir.clone(),
                    source,
                })?;
            }
            if let Some(parent) = clone_dir.parent() {
                fs::create_dir_all(parent).map_err(|source| RegistryError::Io {
                    path: parent.to_path_buf(),
                    source,
                })?;
            }
            self.backend
                .bootstrap(strip_git_plus_prefix(&url), refname, &clone_dir)?;
        }
        Ok(())
    }

    /// Materialise the resolved package into the per-project cache. Clones
    /// (or updates) the per-package repo at the requested tag, then copies
    /// the worktree into `<cache_root>/<kind>/<name>/v<version>/`,
    /// stripping `.git/`.
    pub fn fetch(
        &self,
        resolved: &ResolvedPackage,
        cache_root: &Path,
    ) -> Result<CachedPackage, RegistryError> {
        let url = self.package_repo_url(resolved.kind, &resolved.name);
        let tag = format!("v{}", resolved.version);
        let clone_dir = self.package_clone_dir(resolved.kind, &resolved.name);

        if clone_dir.join(".git").exists() {
            // Clone exists from a previous fetch — refresh it. `update`
            // does `fetch --prune` + hard-reset to `origin/<ref>`. We pass
            // the *tag* as `<ref>`, which `git fetch` resolves through
            // refs/tags/*; the subsequent `git reset --hard origin/<tag>`
            // works because git looks up the ref via the same machinery.
            self.backend.update(&clone_dir, &tag)?;
        } else {
            if clone_dir.exists() {
                // Half-populated dir from a prior failed bootstrap — clean.
                fs::remove_dir_all(&clone_dir).map_err(|source| RegistryError::Io {
                    path: clone_dir.clone(),
                    source,
                })?;
            }
            if let Some(parent) = clone_dir.parent() {
                fs::create_dir_all(parent).map_err(|source| RegistryError::Io {
                    path: parent.to_path_buf(),
                    source,
                })?;
            }
            tracing::info!(
                target: "vibe_registry",
                url = %url,
                tag = %tag,
                dest = %clone_dir.display(),
                "cloning per-package repo"
            );
            self.backend
                .bootstrap(strip_git_plus_prefix(&url), &tag, &clone_dir)?;
        }

        // Materialise into the per-project package cache, stripping `.git/`.
        let dest_cache = cache_root
            .join(resolved.kind.as_str())
            .join(&resolved.name)
            .join(format!("v{}", resolved.version));
        if dest_cache.exists() {
            fs::remove_dir_all(&dest_cache).map_err(|source| RegistryError::Io {
                path: dest_cache.clone(),
                source,
            })?;
        }
        copy_dir_excluding_git(&clone_dir, &dest_cache)?;

        let manifest_path = dest_cache.join(PackageManifest::FILENAME);
        let manifest = PackageManifest::read(&manifest_path)?;
        let content_hash = compute_content_hash(&dest_cache)?;

        Ok(CachedPackage {
            resolved: resolved.clone(),
            cache_dir: dest_cache,
            manifest,
            content_hash,
            // Per-package canonical URL — what lockfile v2's `source_url`
            // stores. `resolved_commit` will be populated when the
            // resolver wires `git rev-parse` through; today it remains
            // `None` and the lockfile's `resolved_commit` stays blank
            // for entries fetched via this path.
            source_uri: url,
            registry_name: Some(self.name.clone()),
            source_ref: Some(tag),
            resolved_commit: None,
            overridden: false,
        })
    }
}

impl Registry for GitPackageRegistry {
    fn list_versions(
        &self,
        kind: PackageKind,
        name: &str,
    ) -> Result<Vec<semver::Version>, RegistryError> {
        GitPackageRegistry::list_versions(self, kind, name)
    }
    fn resolve(&self, pkgref: &PackageRef) -> Result<ResolvedPackage, RegistryError> {
        GitPackageRegistry::resolve(self, pkgref)
    }
    fn fetch(
        &self,
        resolved: &ResolvedPackage,
        cache_root: &Path,
    ) -> Result<CachedPackage, RegistryError> {
        GitPackageRegistry::fetch(self, resolved, cache_root)
    }
}

/// Lowercase hex of the first 8 bytes (16 chars) of `sha256(s)`. Matches the
/// hashing rule pinned in PROP-001 §2.4 / PROP-002 §2.6 — same identity
/// shape as the monorepo `GitRegistry` uses for its registry-level cache
/// directories.
fn short_url_hash(s: &str) -> String {
    let mut h = Sha256::new();
    h.update(s.as_bytes());
    let digest = h.finalize();
    digest.iter().take(8).fold(String::new(), |mut acc, b| {
        let _ = write!(&mut acc, "{b:02x}");
        acc
    })
}

/// Recursively copy `src` into `dst`, excluding any `.git` directory at any
/// depth. Used to materialise a clone into the package cache without
/// dragging the git index along — the cache holds payload only, identity
/// rides on `content_hash`. `pub(crate)` because the multi-registry
/// resolver shares the same materialisation path for `[[override]]` clones.
pub(crate) fn copy_dir_excluding_git(src: &Path, dst: &Path) -> Result<(), RegistryError> {
    fs::create_dir_all(dst).map_err(|source| RegistryError::Io {
        path: dst.to_path_buf(),
        source,
    })?;
    for entry in walkdir::WalkDir::new(src)
        .into_iter()
        .filter_entry(|e| e.file_name() != std::ffi::OsStr::new(".git"))
        .filter_map(|e| e.ok())
    {
        let rel = entry.path().strip_prefix(src).unwrap_or(entry.path());
        if rel.as_os_str().is_empty() {
            continue;
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;
    use tempfile::tempdir;

    /// Test-only `GitBackend` that serves a pre-seeded set of tags and
    /// archive-fetched files per `(url, ref, path)`, and on `bootstrap`
    /// copies a fixture directory into the destination clone.
    #[derive(Default)]
    struct FakeBackend {
        tags: Mutex<HashMap<String, Vec<String>>>,
        files: Mutex<HashMap<(String, String, String), Vec<u8>>>,
        bootstrap_seeds: Mutex<HashMap<String, PathBuf>>,
        bootstrap_calls: Mutex<u32>,
        update_calls: Mutex<u32>,
    }

    impl FakeBackend {
        fn seed_tags(&self, url: impl Into<String>, tags: Vec<String>) {
            self.tags.lock().unwrap().insert(url.into(), tags);
        }
        fn seed_file(
            &self,
            url: impl Into<String>,
            refname: impl Into<String>,
            path: impl Into<String>,
            bytes: Vec<u8>,
        ) {
            self.files
                .lock()
                .unwrap()
                .insert((url.into(), refname.into(), path.into()), bytes);
        }
        fn seed_bootstrap(&self, url: impl Into<String>, source_dir: PathBuf) {
            self.bootstrap_seeds
                .lock()
                .unwrap()
                .insert(url.into(), source_dir);
        }
        fn bootstrap_count(&self) -> u32 {
            *self.bootstrap_calls.lock().unwrap()
        }
        fn update_count(&self) -> u32 {
            *self.update_calls.lock().unwrap()
        }
    }

    impl GitBackend for FakeBackend {
        fn bootstrap(&self, url: &str, _refname: &str, dest: &Path) -> Result<(), GitError> {
            *self.bootstrap_calls.lock().unwrap() += 1;
            let seed = self
                .bootstrap_seeds
                .lock()
                .unwrap()
                .get(url)
                .cloned()
                .ok_or_else(|| GitError::RepoNotFound {
                    url: url.to_string(),
                })?;
            fs::create_dir_all(dest).unwrap();
            for entry in walkdir::WalkDir::new(&seed)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                let rel = entry.path().strip_prefix(&seed).unwrap();
                if rel.as_os_str().is_empty() {
                    continue;
                }
                let target = dest.join(rel);
                if entry.file_type().is_dir() {
                    fs::create_dir_all(&target).unwrap();
                } else if entry.file_type().is_file() {
                    fs::copy(entry.path(), &target).unwrap();
                }
            }
            // Mark dest as a real git repo for the `.git` presence check.
            fs::create_dir_all(dest.join(".git")).unwrap();
            Ok(())
        }
        fn update(&self, _dest: &Path, _refname: &str) -> Result<(), GitError> {
            *self.update_calls.lock().unwrap() += 1;
            Ok(())
        }
        fn list_tags(&self, url: &str) -> Result<Vec<String>, GitError> {
            self.tags
                .lock()
                .unwrap()
                .get(url)
                .cloned()
                .ok_or_else(|| GitError::RepoNotFound {
                    url: url.to_string(),
                })
        }
        fn fetch_file_at_ref(
            &self,
            url: &str,
            refname: &str,
            path: &str,
        ) -> Result<Vec<u8>, GitError> {
            let key = (url.to_string(), refname.to_string(), path.to_string());
            self.files.lock().unwrap().get(&key).cloned().ok_or_else(|| {
                GitError::FileNotFoundInRef {
                    url: url.to_string(),
                    refname: refname.to_string(),
                    path: path.to_string(),
                }
            })
        }
    }

    fn manifest_text(name: &str, kind: &str, version: &str) -> String {
        format!("[package]\nname = \"{name}\"\nkind = \"{kind}\"\nversion = \"{version}\"\n")
    }

    fn registry_with(
        cache: &Path,
        org_url: &str,
        naming: NamingConvention,
        backend: Arc<dyn GitBackend>,
    ) -> GitPackageRegistry {
        GitPackageRegistry::open_with(
            "vibespecs",
            org_url,
            "main",
            naming,
            cache,
            backend,
            DEFAULT_FRESHNESS_SECS,
        )
        .unwrap()
    }

    #[test]
    fn package_repo_url_default_naming() {
        let cache = tempdir().unwrap();
        let fake = Arc::new(FakeBackend::default());
        let r = registry_with(
            cache.path(),
            "git@gitverse.ru:vibespecs",
            NamingConvention::KindName,
            fake,
        );
        assert_eq!(
            r.package_repo_url(PackageKind::Flow, "wal"),
            "git@gitverse.ru:vibespecs/flow-wal.git"
        );
        assert_eq!(
            r.package_repo_url(PackageKind::Stack, "rust-cli"),
            "git@gitverse.ru:vibespecs/stack-rust-cli.git"
        );
    }

    #[test]
    fn package_repo_url_strips_trailing_slash() {
        let cache = tempdir().unwrap();
        let fake = Arc::new(FakeBackend::default());
        let r = registry_with(
            cache.path(),
            "https://gitverse.ru/vibespecs/",
            NamingConvention::KindName,
            fake,
        );
        assert_eq!(
            r.package_repo_url(PackageKind::Stack, "rust-cli"),
            "https://gitverse.ru/vibespecs/stack-rust-cli.git"
        );
    }

    #[test]
    fn package_repo_url_name_only_naming() {
        let cache = tempdir().unwrap();
        let fake = Arc::new(FakeBackend::default());
        let r = registry_with(
            cache.path(),
            "git@host:org",
            NamingConvention::Name,
            fake,
        );
        assert_eq!(
            r.package_repo_url(PackageKind::Flow, "wal"),
            "git@host:org/wal.git"
        );
    }

    #[test]
    fn package_repo_url_kind_slash_name_naming() {
        let cache = tempdir().unwrap();
        let fake = Arc::new(FakeBackend::default());
        let r = registry_with(
            cache.path(),
            "git@host:org",
            NamingConvention::KindSlashName,
            fake,
        );
        assert_eq!(
            r.package_repo_url(PackageKind::Feat, "welcome-page"),
            "git@host:org/feat/welcome-page.git"
        );
    }

    #[test]
    fn list_versions_filters_non_semver_and_sorts() {
        let cache = tempdir().unwrap();
        let fake = Arc::new(FakeBackend::default());
        let url = "git@host:org/flow-wal.git";
        fake.seed_tags(
            url,
            vec![
                "v0.2.0".into(),
                "v0.1.0".into(),
                "v0.10.0".into(),
                "release-foo".into(),
                "v1.0.0-rc.1".into(),
                "draft".into(),
                "1.2.3".into(), // missing `v` prefix — dropped
            ],
        );
        let r = registry_with(
            cache.path(),
            "git@host:org",
            NamingConvention::KindName,
            fake,
        );
        let versions = r.list_versions(PackageKind::Flow, "wal").unwrap();
        let strs: Vec<String> = versions.iter().map(|v| v.to_string()).collect();
        assert_eq!(strs, vec!["0.1.0", "0.2.0", "0.10.0", "1.0.0-rc.1"]);
    }

    #[test]
    fn list_versions_empty_when_repo_has_no_tags() {
        let cache = tempdir().unwrap();
        let fake = Arc::new(FakeBackend::default());
        fake.seed_tags("git@host:org/flow-wal.git", vec![]);
        let r = registry_with(
            cache.path(),
            "git@host:org",
            NamingConvention::KindName,
            fake,
        );
        let v = r.list_versions(PackageKind::Flow, "wal").unwrap();
        assert!(v.is_empty());
    }

    #[test]
    fn list_versions_repo_not_found_translates_to_unknown_package() {
        let cache = tempdir().unwrap();
        let fake = Arc::new(FakeBackend::default());
        // No seed for the URL → FakeBackend returns RepoNotFound.
        let r = registry_with(
            cache.path(),
            "git@host:org",
            NamingConvention::KindName,
            fake,
        );
        let err = r.list_versions(PackageKind::Flow, "ghost").unwrap_err();
        assert!(matches!(err, RegistryError::UnknownPackage { .. }));
    }

    fn registry_with_mirrors(
        cache: &Path,
        org_url: &str,
        naming: NamingConvention,
        mirror_urls: Vec<String>,
        backend: Arc<dyn GitBackend>,
    ) -> GitPackageRegistry {
        GitPackageRegistry::open_with_mirrors(
            "vibespecs",
            org_url,
            "main",
            naming,
            mirror_urls,
            cache,
            backend,
            DEFAULT_FRESHNESS_SECS,
        )
        .unwrap()
    }

    #[test]
    fn list_versions_falls_through_to_mirror_when_primary_unreachable() {
        // Primary's per-package URL is NOT seeded — primary returns
        // RepoNotFound. Mirror's URL IS seeded with tags. Mirror
        // dispatch should pick up the tag list from the mirror.
        let cache = tempdir().unwrap();
        let fake = Arc::new(FakeBackend::default());
        fake.seed_tags(
            "https://mirror.example/vibespecs/flow-wal.git",
            vec!["v0.1.0".into(), "v0.2.0".into()],
        );
        let r = registry_with_mirrors(
            cache.path(),
            "https://primary.example/vibespecs",
            NamingConvention::KindName,
            vec!["https://mirror.example/vibespecs".to_string()],
            fake,
        );
        let versions = r.list_versions(PackageKind::Flow, "wal").unwrap();
        assert_eq!(versions.len(), 2);
        assert_eq!(versions[0].to_string(), "0.1.0");
        assert_eq!(versions[1].to_string(), "0.2.0");
    }

    #[test]
    fn list_versions_prefers_primary_when_both_seeded() {
        // Primary has [v0.1.0] only; mirror has [v0.1.0, v0.2.0].
        // Primary wins because it answers first; the mirror is never
        // consulted. The user's lockfile thus reflects what the
        // canonical source publishes — mirrors don't get to introduce
        // versions the primary doesn't carry.
        let cache = tempdir().unwrap();
        let fake = Arc::new(FakeBackend::default());
        fake.seed_tags(
            "https://primary.example/vibespecs/flow-wal.git",
            vec!["v0.1.0".into()],
        );
        fake.seed_tags(
            "https://mirror.example/vibespecs/flow-wal.git",
            vec!["v0.1.0".into(), "v0.2.0".into()],
        );
        let r = registry_with_mirrors(
            cache.path(),
            "https://primary.example/vibespecs",
            NamingConvention::KindName,
            vec!["https://mirror.example/vibespecs".to_string()],
            fake,
        );
        let versions = r.list_versions(PackageKind::Flow, "wal").unwrap();
        assert_eq!(versions.len(), 1);
        assert_eq!(versions[0].to_string(), "0.1.0");
    }

    #[test]
    fn list_versions_returns_primary_error_when_all_urls_fail() {
        // Neither primary nor mirror is seeded. The result is
        // UnknownPackage from the *primary's* `RepoNotFound` — that's
        // the canonical "the package doesn't exist" diagnostic.
        let cache = tempdir().unwrap();
        let fake = Arc::new(FakeBackend::default());
        let r = registry_with_mirrors(
            cache.path(),
            "https://primary.example/vibespecs",
            NamingConvention::KindName,
            vec!["https://mirror.example/vibespecs".to_string()],
            fake,
        );
        let err = r.list_versions(PackageKind::Flow, "ghost").unwrap_err();
        assert!(matches!(err, RegistryError::UnknownPackage { .. }));
    }

    #[test]
    fn list_versions_walks_mirrors_in_priority_order() {
        // Three mirrors, only the third is seeded. The dispatcher
        // should iterate through mirrors[0], mirrors[1], mirrors[2]
        // and find the answer on mirrors[2]. Mirror order is the
        // caller's responsibility (MultiRegistryResolver does the
        // priority sort) — at this layer we only verify left-to-right
        // dispatch.
        let cache = tempdir().unwrap();
        let fake = Arc::new(FakeBackend::default());
        fake.seed_tags(
            "https://m3.example/vibespecs/flow-wal.git",
            vec!["v0.3.0".into()],
        );
        let r = registry_with_mirrors(
            cache.path(),
            "https://primary.example/vibespecs",
            NamingConvention::KindName,
            vec![
                "https://m1.example/vibespecs".to_string(),
                "https://m2.example/vibespecs".to_string(),
                "https://m3.example/vibespecs".to_string(),
            ],
            fake,
        );
        let versions = r.list_versions(PackageKind::Flow, "wal").unwrap();
        assert_eq!(versions.len(), 1);
        assert_eq!(versions[0].to_string(), "0.3.0");
    }

    #[test]
    fn resolve_picks_latest_stable() {
        let cache = tempdir().unwrap();
        let fake = Arc::new(FakeBackend::default());
        fake.seed_tags(
            "git@host:org/flow-wal.git",
            vec!["v0.1.0".into(), "v0.2.0".into(), "v1.0.0-rc.1".into()],
        );
        let r = registry_with(
            cache.path(),
            "git@host:org",
            NamingConvention::KindName,
            fake,
        );
        let p = PackageRef::parse("flow:wal").unwrap();
        let resolved = r.resolve(&p).unwrap();
        // 1.0.0-rc.1 is pre-release; latest stable wins.
        assert_eq!(resolved.version.to_string(), "0.2.0");
    }

    #[test]
    fn resolve_picks_exact_version() {
        let cache = tempdir().unwrap();
        let fake = Arc::new(FakeBackend::default());
        fake.seed_tags(
            "git@host:org/flow-wal.git",
            vec!["v0.1.0".into(), "v0.2.0".into(), "v0.3.0".into()],
        );
        let r = registry_with(
            cache.path(),
            "git@host:org",
            NamingConvention::KindName,
            fake,
        );
        let p = PackageRef::parse("flow:wal@0.2.0").unwrap();
        let resolved = r.resolve(&p).unwrap();
        assert_eq!(resolved.version.to_string(), "0.2.0");
    }

    #[test]
    fn resolve_picks_range() {
        let cache = tempdir().unwrap();
        let fake = Arc::new(FakeBackend::default());
        fake.seed_tags(
            "git@host:org/flow-wal.git",
            vec!["v0.1.0".into(), "v0.1.5".into(), "v0.2.0".into()],
        );
        let r = registry_with(
            cache.path(),
            "git@host:org",
            NamingConvention::KindName,
            fake,
        );
        let p = PackageRef::parse("flow:wal@^0.1").unwrap();
        let resolved = r.resolve(&p).unwrap();
        assert_eq!(resolved.version.to_string(), "0.1.5");
    }

    #[test]
    fn resolve_no_match_errors() {
        let cache = tempdir().unwrap();
        let fake = Arc::new(FakeBackend::default());
        fake.seed_tags("git@host:org/flow-wal.git", vec!["v0.1.0".into()]);
        let r = registry_with(
            cache.path(),
            "git@host:org",
            NamingConvention::KindName,
            fake,
        );
        let p = PackageRef::parse("flow:wal@^9.0").unwrap();
        let err = r.resolve(&p).unwrap_err();
        assert!(matches!(err, RegistryError::NoMatchingVersion { .. }));
    }

    #[test]
    fn fetch_dep_manifest_falls_through_to_mirror_on_archive_path() {
        // Primary's archive endpoint is empty (FakeBackend returns
        // FileNotFoundInRef). Mirror's archive endpoint has the
        // manifest. Dispatch should hit the mirror and return the
        // manifest without a clone.
        let cache = tempdir().unwrap();
        let fake = Arc::new(FakeBackend::default());
        let primary_url = "https://primary.example/vibespecs/flow-wal.git";
        let mirror_url = "https://mirror.example/vibespecs/flow-wal.git";
        // Tag list seeded only on the mirror — list_versions will land
        // on the mirror first too.
        fake.seed_tags(mirror_url, vec!["v0.1.0".into()]);
        fake.seed_file(
            mirror_url,
            "v0.1.0",
            "vibe-package.toml",
            manifest_text("wal", "flow", "0.1.0").into_bytes(),
        );
        let _ = primary_url; // documented for reading the test
        let r = registry_with_mirrors(
            cache.path(),
            "https://primary.example/vibespecs",
            NamingConvention::KindName,
            vec!["https://mirror.example/vibespecs".to_string()],
            fake.clone(),
        );
        let v = semver::Version::parse("0.1.0").unwrap();
        let manifest = r.fetch_dep_manifest(PackageKind::Flow, "wal", &v).unwrap();
        assert_eq!(manifest.package.name, "wal");
        // No clone — the mirror served the manifest via the archive
        // path, same as the primary-only test asserts.
        assert_eq!(fake.bootstrap_count(), 0);
        assert_eq!(fake.update_count(), 0);
    }

    #[test]
    fn fetch_dep_manifest_reads_via_archive_without_clone() {
        let cache = tempdir().unwrap();
        let fake = Arc::new(FakeBackend::default());
        let url = "git@host:org/flow-wal.git";
        fake.seed_tags(url, vec!["v0.1.0".into()]);
        fake.seed_file(
            url,
            "v0.1.0",
            "vibe-package.toml",
            manifest_text("wal", "flow", "0.1.0").into_bytes(),
        );
        let r = registry_with(
            cache.path(),
            "git@host:org",
            NamingConvention::KindName,
            fake.clone(),
        );
        let v = semver::Version::parse("0.1.0").unwrap();
        let manifest = r.fetch_dep_manifest(PackageKind::Flow, "wal", &v).unwrap();
        assert_eq!(manifest.package.name, "wal");
        assert_eq!(manifest.package.version.to_string(), "0.1.0");
        // Critically: no clone was triggered for this manifest read.
        assert_eq!(fake.bootstrap_count(), 0);
        assert_eq!(fake.update_count(), 0);
    }

    #[test]
    fn fetch_dep_manifest_normalises_legacy_deps() {
        let cache = tempdir().unwrap();
        let fake = Arc::new(FakeBackend::default());
        let url = "git@host:org/flow-wal.git";
        let legacy = r#"
[package]
name = "wal"
kind = "flow"
version = "0.1.0"

[dependencies]
required = ["flow:atomic-commits@^0.1"]
conflicts = ["flow:legacy-wal"]
"#;
        fake.seed_file(url, "v0.1.0", "vibe-package.toml", legacy.as_bytes().to_vec());
        let r = registry_with(
            cache.path(),
            "git@host:org",
            NamingConvention::KindName,
            fake,
        );
        let v = semver::Version::parse("0.1.0").unwrap();
        let m = r.fetch_dep_manifest(PackageKind::Flow, "wal", &v).unwrap();
        assert!(m.dependencies.is_empty(), "legacy section migrated away");
        assert_eq!(m.requires.packages.len(), 1);
        assert_eq!(m.conflicts.packages.len(), 1);
    }

    #[test]
    fn fetch_clones_and_populates_per_project_cache() {
        let cache = tempdir().unwrap();
        let pkg_cache = tempdir().unwrap();
        let upstream = tempdir().unwrap();
        // Build a fake upstream tree at the seeded URL: vibe-package.toml
        // plus a spec file and a stray `.git/` to make sure the copy
        // strips it on the way to the cache.
        let pkg_root = upstream.path().join("pkg");
        fs::create_dir_all(pkg_root.join("spec")).unwrap();
        fs::write(
            pkg_root.join("vibe-package.toml"),
            manifest_text("wal", "flow", "0.1.0"),
        )
        .unwrap();
        fs::write(pkg_root.join("spec/foo.md"), "content\n").unwrap();
        // Upstream tree has no .git/ — the FakeBackend creates one in
        // dest after copying; we want to verify our extractor strips it.

        let fake = Arc::new(FakeBackend::default());
        let url = "git@host:org/flow-wal.git";
        fake.seed_tags(url, vec!["v0.1.0".into()]);
        fake.seed_bootstrap(url, pkg_root.clone());

        let r = registry_with(
            cache.path(),
            "git@host:org",
            NamingConvention::KindName,
            fake.clone(),
        );

        let p = PackageRef::parse("flow:wal@0.1.0").unwrap();
        let resolved = r.resolve(&p).unwrap();
        let cached = r.fetch(&resolved, pkg_cache.path()).unwrap();

        // Cache populated, no .git/ dragged through.
        assert!(cached.cache_dir.join("vibe-package.toml").exists());
        assert!(cached.cache_dir.join("spec/foo.md").exists());
        assert!(!cached.cache_dir.join(".git").exists());

        // Manifest parsed and content_hash populated.
        assert_eq!(cached.manifest.package.name, "wal");
        assert!(cached.content_hash.starts_with("sha256:"));

        // source_uri is the canonical per-package repo URL.
        assert_eq!(cached.source_uri, url);

        // Bootstrap was called exactly once.
        assert_eq!(fake.bootstrap_count(), 1);
    }

    #[test]
    fn fetch_reuses_existing_clone_via_update() {
        let cache = tempdir().unwrap();
        let pkg_cache = tempdir().unwrap();
        let upstream = tempdir().unwrap();
        let pkg_root = upstream.path().join("pkg");
        fs::create_dir_all(&pkg_root).unwrap();
        fs::write(
            pkg_root.join("vibe-package.toml"),
            manifest_text("wal", "flow", "0.1.0"),
        )
        .unwrap();

        let fake = Arc::new(FakeBackend::default());
        let url = "git@host:org/flow-wal.git";
        fake.seed_tags(url, vec!["v0.1.0".into()]);
        fake.seed_bootstrap(url, pkg_root.clone());

        let r = registry_with(
            cache.path(),
            "git@host:org",
            NamingConvention::KindName,
            fake.clone(),
        );
        let p = PackageRef::parse("flow:wal@0.1.0").unwrap();
        let resolved = r.resolve(&p).unwrap();

        let _ = r.fetch(&resolved, pkg_cache.path()).unwrap();
        let _ = r.fetch(&resolved, pkg_cache.path()).unwrap();

        // First fetch: bootstrap; second: update (clone exists from first).
        assert_eq!(fake.bootstrap_count(), 1);
        assert_eq!(fake.update_count(), 1);
    }
}
