//! Git-backed registry.
//!
//! A [`GitRegistry`] is a thin layer on top of a [`LocalRegistry`]: it
//! owns a local clone of a remote git repository at
//! `~/.vibe/registries/<hash>/clone/` and delegates resolve / fetch to
//! a [`LocalRegistry`] pointed at that clone. The on-disk layout is
//! identical to a hand-written local registry, so all the version
//! discovery logic stays in one place.
//!
//! Decisions (cache layout, freshness TTL, source-URI shape,
//! normalization rules) are pinned in
//! [`spec/modules/vibe-registry/PROP-001-git-backend.md`][prop].
//!
//! [prop]: ../../../../spec/modules/vibe-registry/PROP-001-git-backend.md

specmark::scope!("spec://vibevm/modules/vibe-registry/PROP-001#registry-trait");

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use vibe_core::manifest::Manifest;
use vibe_core::timestamp;
use vibe_core::{Group, PackageRef};

use crate::git_backend::{GitBackend, ShellGit};
use crate::{
    CachedPackage, LocalRegistry, Registry, RegistryError, ResolvedPackage, compute_content_hash,
    copy_dir_recursive,
};

/// Default freshness TTL for an implicit pull: 1 hour.
pub const DEFAULT_FRESHNESS_SECS: u64 = 3600;

/// Structure persisted to `<cache_root>/<hash>/meta.toml`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegistryMeta {
    /// Normalized URL of the registry (for debugging).
    pub url: String,
    /// The ref name (usually `main`) checked out in the clone.
    pub r#ref: String,
    /// RFC-3339 UTC timestamp of the last successful clone or update.
    pub last_pulled_at: String,
    /// Full 64-character sha256 hex of the normalized url, for audit.
    pub url_hash: String,
}

/// Git-backed [`Registry`] implementation.
pub struct GitRegistry {
    backend: Arc<dyn GitBackend>,
    url: String,
    refname: String,
    cache_dir: PathBuf,
    clone_dir: PathBuf,
    local: LocalRegistry,
}

impl GitRegistry {
    /// Open (cloning if necessary) a git-backed registry at `url#ref`
    /// using the default [`ShellGit`] backend and the default
    /// cache root under `~/.vibe/registries/`.
    ///
    /// Implicit freshness policy: if the cache is younger than
    /// [`DEFAULT_FRESHNESS_SECS`], no network call is made. Otherwise
    /// the backend does a `git fetch` + `git reset --hard origin/<ref>`.
    pub fn open(url: &str, refname: &str) -> Result<Self, RegistryError> {
        let cache_root = default_cache_root()?;
        Self::open_with(
            url,
            refname,
            &cache_root,
            Arc::new(ShellGit::new()),
            DEFAULT_FRESHNESS_SECS,
        )
    }

    /// Lower-level constructor exposing backend, cache root and TTL.
    /// Used by tests; production callers prefer [`GitRegistry::open`].
    pub fn open_with(
        url: &str,
        refname: &str,
        cache_root: &Path,
        backend: Arc<dyn GitBackend>,
        freshness_secs: u64,
    ) -> Result<Self, RegistryError> {
        fs::create_dir_all(cache_root).map_err(|source| RegistryError::Io {
            path: cache_root.to_path_buf(),
            source,
        })?;

        let normalized = normalize_url(url);
        let full_hash = full_sha256_hex(&normalized);
        let short_hash = &full_hash[..16];
        let cache_dir = cache_root.join(short_hash);
        let clone_dir = cache_dir.join("clone");
        let meta_path = cache_dir.join("meta.toml");

        fs::create_dir_all(&cache_dir).map_err(|source| RegistryError::Io {
            path: cache_dir.clone(),
            source,
        })?;

        let existing_meta = read_meta_if_present(&meta_path)?;
        let needs_clone = !clone_dir.join(".git").exists();

        if needs_clone {
            // Clean up any half-populated clone dir from a prior failure.
            if clone_dir.exists() {
                fs::remove_dir_all(&clone_dir).map_err(|source| RegistryError::Io {
                    path: clone_dir.clone(),
                    source,
                })?;
            }
            tracing::info!(target: "vibe_registry", url = %normalized, dest = %clone_dir.display(), "cloning registry");
            backend.bootstrap(strip_git_plus_prefix(url), refname, &clone_dir)?;
            write_meta(&meta_path, &normalized, refname, &full_hash)?;
        } else if should_pull(existing_meta.as_ref(), freshness_secs) {
            tracing::info!(target: "vibe_registry", url = %normalized, "updating registry (cache stale)");
            backend.update(&clone_dir, refname)?;
            write_meta(&meta_path, &normalized, refname, &full_hash)?;
        } else {
            tracing::debug!(target: "vibe_registry", url = %normalized, "registry cache fresh");
        }

        let local = LocalRegistry::new(clone_dir.clone())?;
        Ok(GitRegistry {
            backend,
            url: url.to_string(),
            refname: refname.to_string(),
            cache_dir,
            clone_dir,
            local,
        })
    }

    /// Force a `git fetch` + reset regardless of freshness. Invoked by
    /// `vibe registry sync`.
    pub fn sync(&self) -> Result<(), RegistryError> {
        tracing::info!(target: "vibe_registry", url = %self.url, "forcing registry sync");
        self.backend.update(&self.clone_dir, &self.refname)?;
        let normalized = normalize_url(&self.url);
        let full_hash = full_sha256_hex(&normalized);
        write_meta(
            &self.cache_dir.join("meta.toml"),
            &normalized,
            &self.refname,
            &full_hash,
        )?;
        Ok(())
    }

    pub fn url(&self) -> &str {
        &self.url
    }

    pub fn refname(&self) -> &str {
        &self.refname
    }

    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }

    pub fn clone_dir(&self) -> &Path {
        &self.clone_dir
    }
}

impl Registry for GitRegistry {
    fn list_versions(
        &self,
        group: &Group,
        name: &str,
    ) -> Result<Vec<semver::Version>, RegistryError> {
        self.local.list_versions(group, name)
    }

    fn resolve(&self, pkgref: &PackageRef) -> Result<ResolvedPackage, RegistryError> {
        self.local.resolve(pkgref)
    }

    /// Fetch overrides [`LocalRegistry::fetch`] so the lockfile
    /// `source_uri` encodes the registry's git URL instead of the
    /// on-disk clone path.
    fn fetch(
        &self,
        resolved: &ResolvedPackage,
        cache_root: &Path,
    ) -> Result<CachedPackage, RegistryError> {
        let cache_dir = cache_root
            .join(resolved.group.as_str())
            .join(&resolved.name)
            .join(format!("v{}", resolved.version));

        if cache_dir.exists() {
            fs::remove_dir_all(&cache_dir).map_err(|source| RegistryError::Io {
                path: cache_dir.clone(),
                source,
            })?;
        }
        copy_dir_recursive(&resolved.source_dir, &cache_dir)?;

        let manifest_path = cache_dir.join(Manifest::FILENAME);
        let manifest = Manifest::read(&manifest_path)?;
        if manifest.package.is_none() {
            return Err(RegistryError::MalformedMeta {
                path: manifest_path.clone(),
                reason: "registry package manifest must carry a [package] table".to_string(),
            });
        }
        let content_hash = compute_content_hash(&cache_dir)?;

        let source_uri = source_uri_for_git(
            &self.url,
            &resolved.group,
            &resolved.name,
            &resolved.version.to_string(),
        );

        Ok(CachedPackage {
            resolved: resolved.clone(),
            cache_dir,
            manifest,
            content_hash,
            source_uri,
            // The legacy monorepo registry pre-dates the per-package /
            // multi-registry / override schema; lockfile-v2 provenance
            // fields stay blank for content fetched through this path.
            registry_name: None,
            source_ref: None,
            resolved_commit: None,
            overridden: false,
            is_git_source: false,
            is_path_source: false,
            via_redirect: None,
        })
    }
}

/// Return the default cache root for registry clones.
///
/// Honours `VIBE_REGISTRY_CACHE` if set (used by tests and by users
/// who want an explicit out-of-home location), otherwise returns
/// `~/.vibe/registries/` per `VIBEVM-SPEC.md` §8.3.
pub fn default_cache_root() -> Result<PathBuf, RegistryError> {
    if let Some(custom) = std::env::var_os("VIBE_REGISTRY_CACHE") {
        return Ok(PathBuf::from(custom));
    }
    let home = dirs::home_dir().ok_or(RegistryError::NoHomeDir)?;
    Ok(home.join(".vibe").join("registries"))
}

/// Strip a `git+` transport-wrapper prefix (`git+ssh://`, `git+https://`,
/// `git+file://`, `git+http://`) before handing the URL to git.
///
/// `git+` is a pip / Cargo convention that labels a URL as "this is a
/// git source" in a lockfile or manifest. Native git does not
/// understand the prefix itself, so we peel it off at the backend
/// boundary. Used by `GitRegistry`, `GitPackageRegistry`, and the
/// override path in `MultiRegistryResolver`.
pub(crate) fn strip_git_plus_prefix(url: &str) -> &str {
    url.strip_prefix("git+").unwrap_or(url)
}

/// Normalize a registry URL for hashing and comparison.
///
/// Lowercases the whole string and strips a trailing `.git` plus
/// trailing slashes. This is intentionally lossy but consistent: we
/// only need it to key the on-disk cache directory, not to reconstruct
/// the URL. The full hash is recorded in `meta.toml` for audit.
pub fn normalize_url(url: &str) -> String {
    let t = url.trim().trim_end_matches('/');
    let t = t.strip_suffix(".git").unwrap_or(t);
    t.to_lowercase()
}

fn full_sha256_hex(s: &str) -> String {
    let mut h = Sha256::new();
    h.update(s.as_bytes());
    let digest = h.finalize();
    digest.iter().fold(String::new(), |mut acc, b| {
        use std::fmt::Write;
        let _ = write!(&mut acc, "{b:02x}");
        acc
    })
}

fn read_meta_if_present(path: &Path) -> Result<Option<RegistryMeta>, RegistryError> {
    if !path.exists() {
        return Ok(None);
    }
    let raw = fs::read_to_string(path).map_err(|source| RegistryError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let meta: RegistryMeta = toml::from_str(&raw).map_err(|e| RegistryError::MalformedMeta {
        path: path.to_path_buf(),
        reason: e.to_string(),
    })?;
    Ok(Some(meta))
}

fn write_meta(
    path: &Path,
    normalized_url: &str,
    refname: &str,
    full_hash: &str,
) -> Result<(), RegistryError> {
    let meta = RegistryMeta {
        url: normalized_url.to_string(),
        r#ref: refname.to_string(),
        last_pulled_at: timestamp::now_utc(),
        url_hash: full_hash.to_string(),
    };
    let raw = toml::to_string_pretty(&meta).map_err(|e| RegistryError::MalformedMeta {
        path: path.to_path_buf(),
        reason: e.to_string(),
    })?;
    fs::write(path, raw).map_err(|source| RegistryError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(())
}

fn should_pull(existing: Option<&RegistryMeta>, ttl_secs: u64) -> bool {
    let Some(meta) = existing else {
        return true;
    };
    let Some(prev) = timestamp::parse_unix_utc(&meta.last_pulled_at) else {
        return true;
    };
    let Some(now) = current_epoch_secs() else {
        return true;
    };
    // `>=` (not `>`) so ttl_secs == 0 always pulls even when both
    // timestamps resolve to the same second.
    now.saturating_sub(prev) >= ttl_secs
}

fn current_epoch_secs() -> Option<u64> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .map(|d| d.as_secs())
}

/// `git+ssh://git@host/owner/repo.git#<group>/<name>/v<ver>` etc.
/// Preserves the original transport in the scheme prefix. The fragment
/// is keyed by `(group, name)` identity — the group-native shape after
/// PROP-008.
pub fn source_uri_for_git(url: &str, group: &Group, name: &str, version: &str) -> String {
    let transport = detect_transport(url);
    format!(
        "{transport}://{host_path}#{group}/{name}/v{version}",
        transport = transport,
        host_path = to_uri_body(url),
        group = group.as_str(),
        name = name,
        version = version,
    )
}

fn detect_transport(url: &str) -> &'static str {
    let t = url.trim_start();
    if let Some((scheme, _)) = t.split_once("://") {
        match scheme.to_lowercase().as_str() {
            "https" => "git+https",
            "http" => "git+http",
            "ssh" => "git+ssh",
            "file" => "git+file",
            "git+ssh" => "git+ssh",
            "git+https" => "git+https",
            "git+http" => "git+http",
            "git+file" => "git+file",
            _ => "git",
        }
    } else if t.starts_with("git@") || (t.contains(':') && !t.starts_with('/')) {
        // Scp-style shorthand `git@host:owner/repo.git`.
        "git+ssh"
    } else {
        "git"
    }
}

/// Strip the transport prefix from `url` and rewrite scp-style
/// shorthand to a proper URI body.
fn to_uri_body(url: &str) -> String {
    let t = url.trim();
    if let Some((_, rest)) = t.split_once("://") {
        return rest.trim_end_matches('/').to_string();
    }
    // Convert `git@host:owner/repo.git` → `git@host/owner/repo.git`.
    if let Some(colon) = t.find(':')
        && !t.starts_with('/')
    {
        let (left, right) = t.split_at(colon);
        let path = &right[1..]; // skip ':'
        return format!("{left}/{path}").trim_end_matches('/').to_string();
    }
    t.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git_backend::GitError;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::Mutex;
    use tempfile::tempdir;

    /// Test-only backend that records calls and expects the caller to
    /// pre-seed the clone directory with a file tree. Replaces a real
    /// git clone with a filesystem copy.
    #[derive(Debug, Default)]
    struct FakeGit {
        source: Mutex<Option<PathBuf>>,
        clone_calls: Mutex<u32>,
        update_calls: Mutex<u32>,
    }

    impl FakeGit {
        fn seed_source(&self, src: PathBuf) {
            *self.source.lock().unwrap() = Some(src);
        }
        fn clone_count(&self) -> u32 {
            *self.clone_calls.lock().unwrap()
        }
        fn update_count(&self) -> u32 {
            *self.update_calls.lock().unwrap()
        }
    }

    impl GitBackend for FakeGit {
        fn bootstrap(&self, _url: &str, _refname: &str, dest: &Path) -> Result<(), GitError> {
            *self.clone_calls.lock().unwrap() += 1;
            let src = self.source.lock().unwrap().clone().unwrap();
            fs::create_dir_all(dest).unwrap();
            copy_tree(&src, dest);
            // Mark as a real git repo for the `.git` presence check.
            fs::create_dir_all(dest.join(".git")).unwrap();
            Ok(())
        }
        fn update(&self, _dest: &Path, _refname: &str) -> Result<(), GitError> {
            *self.update_calls.lock().unwrap() += 1;
            Ok(())
        }
        fn list_tags(&self, _url: &str) -> Result<Vec<String>, GitError> {
            // Tests using FakeGit pre-seed a working tree directly; they
            // do not exercise the resolver's tag-listing path. Default to
            // empty so a stray call does not panic.
            Ok(Vec::new())
        }
        fn fetch_file_at_ref(
            &self,
            url: &str,
            refname: &str,
            path: &str,
        ) -> Result<Vec<u8>, GitError> {
            Err(GitError::FileNotFoundInRef {
                url: url.to_string(),
                refname: refname.to_string(),
                path: path.to_string(),
            })
        }
    }

    fn copy_tree(src: &Path, dst: &Path) {
        for entry in walkdir::WalkDir::new(src)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let rel = entry.path().strip_prefix(src).unwrap();
            let target = dst.join(rel);
            if entry.file_type().is_dir() {
                fs::create_dir_all(&target).unwrap();
            } else if entry.file_type().is_file() {
                fs::copy(entry.path(), target).unwrap();
            }
        }
    }

    fn seed_fixture_layout(root: &Path) {
        // Group-native on-disk layout (PROP-008): `<group>/<name>/v<ver>/`.
        let v = root.join("org.vibevm/wal/v0.1.0");
        fs::create_dir_all(&v).unwrap();
        fs::write(
            v.join("vibe.toml"),
            r#"[package]
group = "org.vibevm"
name = "wal"
kind = "flow"
version = "0.1.0"
description = "WAL v0.1.0"
"#,
        )
        .unwrap();
        fs::write(v.join("README.md"), "# wal 0.1.0\n").unwrap();
    }

    #[test]
    fn normalize_url_strips_git_and_lowercases() {
        assert_eq!(
            normalize_url("git@Gitverse.ru:Anarchic/VibeSpecs.git"),
            "git@gitverse.ru:anarchic/vibespecs"
        );
        assert_eq!(
            normalize_url("https://Gitverse.ru/anarchic/vibespecs.git/"),
            "https://gitverse.ru/anarchic/vibespecs"
        );
    }

    #[test]
    fn detect_transport_variants() {
        assert_eq!(detect_transport("git@host:o/r.git"), "git+ssh");
        assert_eq!(detect_transport("ssh://git@host/o/r.git"), "git+ssh");
        assert_eq!(detect_transport("https://host/o/r.git"), "git+https");
        assert_eq!(detect_transport("file:///a/b"), "git+file");
        assert_eq!(detect_transport("git+https://host/o/r"), "git+https");
    }

    #[test]
    fn source_uri_for_git_produces_fragment() {
        let group = Group::parse("org.vibevm").unwrap();
        let s = source_uri_for_git(
            "git@gitverse.ru:anarchic/vibespecs.git",
            &group,
            "wal",
            "0.1.0",
        );
        assert_eq!(
            s,
            "git+ssh://git@gitverse.ru/anarchic/vibespecs.git#org.vibevm/wal/v0.1.0"
        );
    }

    #[test]
    fn open_clones_on_first_use_and_skips_when_fresh() {
        let tmp = tempdir().unwrap();
        let cache_root = tmp.path().join("cache");
        let upstream = tmp.path().join("upstream");
        fs::create_dir_all(&upstream).unwrap();
        seed_fixture_layout(&upstream);

        let fake = Arc::new(FakeGit::default());
        fake.seed_source(upstream.clone());

        // First open → clone.
        let r1 = GitRegistry::open_with(
            "git@host:o/r.git",
            "main",
            &cache_root,
            fake.clone(),
            DEFAULT_FRESHNESS_SECS,
        )
        .unwrap();
        assert_eq!(fake.clone_count(), 1);
        assert_eq!(fake.update_count(), 0);
        assert!(
            r1.clone_dir()
                .join("org.vibevm/wal/v0.1.0/vibe.toml")
                .exists()
        );
        assert!(r1.cache_dir().join("meta.toml").exists());

        // Second open with fresh TTL → no update.
        let _r2 = GitRegistry::open_with(
            "git@host:o/r.git",
            "main",
            &cache_root,
            fake.clone(),
            DEFAULT_FRESHNESS_SECS,
        )
        .unwrap();
        assert_eq!(fake.clone_count(), 1);
        assert_eq!(fake.update_count(), 0);

        // Second open with zero TTL → update fires.
        let _r3 = GitRegistry::open_with("git@host:o/r.git", "main", &cache_root, fake.clone(), 0)
            .unwrap();
        assert_eq!(fake.clone_count(), 1);
        assert_eq!(fake.update_count(), 1);
    }

    #[test]
    fn sync_always_updates() {
        let tmp = tempdir().unwrap();
        let cache_root = tmp.path().join("cache");
        let upstream = tmp.path().join("upstream");
        fs::create_dir_all(&upstream).unwrap();
        seed_fixture_layout(&upstream);

        let fake = Arc::new(FakeGit::default());
        fake.seed_source(upstream.clone());

        let r = GitRegistry::open_with(
            "git@host:o/r.git",
            "main",
            &cache_root,
            fake.clone(),
            DEFAULT_FRESHNESS_SECS,
        )
        .unwrap();
        assert_eq!(fake.update_count(), 0);
        r.sync().unwrap();
        assert_eq!(fake.update_count(), 1);
        r.sync().unwrap();
        assert_eq!(fake.update_count(), 2);
    }

    #[test]
    fn resolve_and_fetch_produce_git_source_uri() {
        let tmp = tempdir().unwrap();
        let cache_root = tmp.path().join("cache");
        let upstream = tmp.path().join("upstream");
        fs::create_dir_all(&upstream).unwrap();
        seed_fixture_layout(&upstream);

        let fake = Arc::new(FakeGit::default());
        fake.seed_source(upstream.clone());

        let r = GitRegistry::open_with(
            "git@gitverse.ru:anarchic/vibespecs.git",
            "main",
            &cache_root,
            fake.clone(),
            DEFAULT_FRESHNESS_SECS,
        )
        .unwrap();
        let pkgref = PackageRef::parse("org.vibevm/wal@0.1.0").unwrap();
        let resolved = r.resolve(&pkgref).unwrap();
        assert_eq!(resolved.version.to_string(), "0.1.0");

        let pkg_cache = tmp.path().join("pkg_cache");
        fs::create_dir_all(&pkg_cache).unwrap();
        let cached = r.fetch(&resolved, &pkg_cache).unwrap();
        assert!(cached.source_uri.starts_with("git+ssh://"));
        assert!(cached.source_uri.ends_with("#org.vibevm/wal/v0.1.0"));
    }
}
