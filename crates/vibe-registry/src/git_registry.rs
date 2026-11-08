//! Git-backed registry.
//!
//! A [`GitMonorepoRegistry`] is a thin layer on top of a [`LocalRegistry`]: it
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
//! [prop]: ../../../../vibevm/vibespecs/modules/vibe-registry/PROP-001-git-backend.xml

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-001#registry-trait");

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use specmark::cell;
use vibe_core::manifest::Manifest;
use vibe_core::timestamp;
use vibe_core::{Group, PackageRef};

use crate::git_backend::{GitBackend, ShellGit};
pub use crate::registry_cache::{DEFAULT_FRESHNESS_SECS, default_cache_root, normalize_url};
use crate::{
    CachedPackage, LocalRegistry, Registry, RegistryError, ResolvedPackage, compute_content_hash,
    store,
};

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
#[cell(seam = "Registry", variant = "git-monorepo")]
pub struct GitMonorepoRegistry {
    backend: Arc<dyn GitBackend>,
    url: String,
    /// The candidate the current clone was made from — the canonical `url`
    /// unless a `[[mirror]]` / remap row served it (`PROP-066#provenance`).
    served_url: String,
    /// `[[registry]].name` this clone serves, recorded as the lockfile
    /// `registry` (PROP-002 `#registry-type`).
    name: Option<String>,
    refname: String,
    cache_dir: PathBuf,
    clone_dir: PathBuf,
    local: LocalRegistry,
    /// The commit of an operation-owned snapshot (controlled transport),
    /// which has no `.git` to ask.
    snapshot_commit: Option<String>,
    /// Keeps an operation-owned snapshot alive for the registry's lifetime.
    _snapshot: Option<tempfile::TempDir>,
}

impl GitMonorepoRegistry {
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
    /// Used by tests; production callers prefer [`GitMonorepoRegistry::open`].
    pub fn open_with(
        url: &str,
        refname: &str,
        cache_root: &Path,
        backend: Arc<dyn GitBackend>,
        freshness_secs: u64,
    ) -> Result<Self, RegistryError> {
        Self::open_routes(url, &[], refname, cache_root, backend, freshness_secs)
    }

    /// [`Self::open_with`] with alternative clone candidates (the remap
    /// Locator's `[[mirror]]` / row targets for this repository): a fresh
    /// clone tries `url` first, then each candidate in order. The cache is
    /// keyed by the canonical `url`, so a mirror-made clone is the same
    /// cache entry.
    pub fn open_routes(
        url: &str,
        candidates: &[String],
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
        let mut served_url = url.to_string();

        if needs_clone {
            served_url =
                clone_first_reachable(backend.as_ref(), url, candidates, refname, &clone_dir)?;
            write_meta(&meta_path, &normalized, refname, &full_hash)?;
        } else if should_pull(existing_meta.as_ref(), freshness_secs) {
            tracing::info!(target: "vibe_registry", url = %normalized, "updating registry (cache stale)");
            backend.update(&clone_dir, refname)?;
            write_meta(&meta_path, &normalized, refname, &full_hash)?;
        } else {
            tracing::debug!(target: "vibe_registry", url = %normalized, "registry cache fresh");
        }

        let local = LocalRegistry::new(clone_dir.clone())?;
        Ok(GitMonorepoRegistry {
            backend,
            url: url.to_string(),
            served_url,
            name: None,
            refname: refname.to_string(),
            cache_dir,
            clone_dir,
            local,
            snapshot_commit: None,
            _snapshot: None,
        })
    }

    /// Open the registry under a controlled transport, where only the
    /// backend's native preparation may contact a remote (PROP-073
    /// `#NATIVE-PREPARATION`): `refname` is captured into operation-owned
    /// objects and its verified tree copied into a snapshot directory that
    /// lives as long as this value. Candidates are tried in order, as for
    /// [`Self::open_routes`].
    pub fn open_native(
        url: &str,
        candidates: &[String],
        refname: &str,
        cache_root: &Path,
        backend: Arc<dyn GitBackend>,
    ) -> Result<Self, RegistryError> {
        fs::create_dir_all(cache_root).map_err(|source| RegistryError::Io {
            path: cache_root.to_path_buf(),
            source,
        })?;
        let cache = std::path::absolute(cache_root).map_err(|source| RegistryError::Io {
            path: cache_root.to_path_buf(),
            source,
        })?;
        let snapshot = tempfile::Builder::new()
            .prefix("monorepo-native-")
            .tempdir_in(&cache)
            .map_err(|source| RegistryError::Io {
                path: cache.clone(),
                source,
            })?;
        let mut first_error = None;
        for (attempt, candidate) in std::iter::once(url)
            .chain(candidates.iter().map(String::as_str))
            .enumerate()
        {
            let work = snapshot.path().join(attempt.to_string());
            match native_snapshot(backend.as_ref(), candidate, refname, &work) {
                Ok((tree, commit)) => {
                    let local = LocalRegistry::new(tree.clone())?;
                    return Ok(GitMonorepoRegistry {
                        backend,
                        url: url.to_string(),
                        served_url: candidate.to_string(),
                        name: None,
                        refname: refname.to_string(),
                        cache_dir: snapshot.path().to_path_buf(),
                        clone_dir: tree,
                        local,
                        snapshot_commit: Some(commit),
                        _snapshot: Some(snapshot),
                    });
                }
                Err(error) => {
                    tracing::warn!(target: "vibe_registry", url = %candidate, %error, "registry snapshot candidate failed");
                    first_error.get_or_insert(error);
                }
            }
        }
        Err(first_error.unwrap_or_else(|| RegistryError::MissingRoot(snapshot.path().into())))
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

    /// Record the `[[registry]].name` this clone serves.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn url(&self) -> &str {
        &self.url
    }

    /// The local-directory view of the clone.
    pub fn local(&self) -> &LocalRegistry {
        &self.local
    }

    /// The commit the clone (or snapshot) holds.
    pub fn head_commit(&self) -> Result<Option<String>, RegistryError> {
        if let Some(commit) = &self.snapshot_commit {
            return Ok(Some(commit.clone()));
        }
        Ok(self.backend.head_commit(&self.clone_dir)?)
    }

    /// The candidate url that served the clone.
    pub fn served_url(&self) -> &str {
        &self.served_url
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

impl Registry for GitMonorepoRegistry {
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
    /// on-disk clone path. The payload is inserted into the
    /// machine-global store write-once (PROP-010 §2.7), same shape as
    /// the local-directory path.
    fn fetch(
        &self,
        resolved: &ResolvedPackage,
        store_root: &Path,
    ) -> Result<CachedPackage, RegistryError> {
        let content_hash = compute_content_hash(&resolved.source_dir)?;
        let inserted = store::insert_current_at(
            store_root,
            &resolved.source_dir,
            &resolved.group,
            &resolved.name,
            &resolved.version,
            &content_hash,
        )?;
        let reused = matches!(&inserted, store::InsertOutcome::AlreadyPresent(_));
        let cache_dir = inserted.into_entry();

        let manifest_path = cache_dir.join(Manifest::FILENAME);
        let manifest = Manifest::read(&manifest_path)?;
        if manifest.package.is_none() {
            return Err(RegistryError::MalformedMeta {
                path: manifest_path.clone(),
                reason: "registry package manifest must carry a [package] table".to_string(),
            });
        }

        let source_uri = monorepo_source_url(
            &self.url,
            &resolved.group,
            &resolved.name,
            &resolved.version,
        );
        let resolved_commit = self.head_commit()?;

        Ok(CachedPackage {
            locked_selection: None,
            route_publication: vibe_core::RoutePublication::Local,
            recorded_source_kind: None,
            store_reused: Some(reused),
            acquisition: Some(vibe_wire::generated::shared::SourceAcquisition::GitRepository),
            resolved: resolved.clone(),
            cache_dir,
            manifest,
            content_hash,
            source_uri,
            // A mirror- or remap-served clone is recorded beside the
            // canonical repository (`PROP-066#provenance`).
            fetched_from: (self.served_url != self.url).then(|| self.served_url.clone()),
            // Provenance of a monorepo row (PROP-002 `#registry-type`): the
            // registry, the branch it was read on and the exact commit, so
            // replay reads the same subpath at the same commit.
            registry_name: self.name.clone(),
            source_ref: Some(self.refname.clone()),
            resolved_commit,
            overridden: false,
            is_git_source: false,
            is_path_source: false,
            is_embedded: false,
            is_local: false,
            via_redirect: None,
        })
    }
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

/// The lock `source_url` of a package read from a monorepo registry
/// (PROP-002 `#registry-type`): the registry url verbatim, then
/// `#<group>/<name>/v<version>` naming the package directory inside it. The
/// url is kept byte-for-byte (an scp-form `git@host:path` stays
/// home-relative) so replay clones exactly the declared repository.
///
/// ```
/// use vibe_core::Group;
/// use vibe_registry::git_registry::{monorepo_source_url, parse_monorepo_source_url};
///
/// let group = Group::parse("org.corp").unwrap();
/// let version = semver::Version::new(1, 2, 0);
/// let url = monorepo_source_url("git@git.corp:tools/vibe.git", &group, "demo", &version);
/// assert_eq!(url, "git@git.corp:tools/vibe.git#org.corp/demo/v1.2.0");
/// assert_eq!(
///     parse_monorepo_source_url(&url),
///     Some(("git@git.corp:tools/vibe.git", "org.corp/demo/v1.2.0")),
/// );
/// assert_eq!(parse_monorepo_source_url("https://github.com/o/r.git"), None);
/// ```
pub fn monorepo_source_url(
    url: &str,
    group: &Group,
    name: &str,
    version: &semver::Version,
) -> String {
    format!("{url}#{}", monorepo_package_path(group, name, version))
}

/// `<group>/<name>/v<version>` — a package's directory inside a monorepo or
/// local-directory registry.
pub fn monorepo_package_path(group: &Group, name: &str, version: &semver::Version) -> String {
    format!("{}/{name}/v{version}", group.as_str())
}

/// Split a [`monorepo_source_url`] into the repository url and the package
/// directory; `None` when the value carries no `#<path>` fragment.
pub fn parse_monorepo_source_url(source_url: &str) -> Option<(&str, &str)> {
    let (repository, path) = source_url.rsplit_once('#')?;
    (!repository.is_empty() && path.split('/').count() == 3).then_some((repository, path))
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

#[path = "git_registry/acquire.rs"]
mod acquire;
use acquire::{clone_first_reachable, native_snapshot};

#[cfg(test)]
#[path = "git_registry/tests.rs"]
mod tests;
