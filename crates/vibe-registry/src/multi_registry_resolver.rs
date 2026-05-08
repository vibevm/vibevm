//! Multi-registry resolver — PROP-002.
//!
//! Sits on top of one or more [`GitPackageRegistry`] instances and dispatches
//! resolution / fetch through the priority + override + (eventually) mirror
//! decision tree pinned in [PROP-002 §2.2 / §2.3 / §2.4](../../../spec/modules/vibe-registry/PROP-002-decentralized-registry.md).
//!
//! Decision order on `resolve(pkgref)`:
//!
//! 1. **`[[override]]` first.** If `vibe.toml` carries an override for this
//!    pkgref, the registry layer is bypassed entirely. The override's
//!    `source_url` / `ref` is fetched directly; the version is taken
//!    verbatim from the manifest at that ref. `overridden = true` ends up
//!    in the lockfile so `vibe list --overrides` and audit tooling can
//!    surface it.
//!
//! 2. **`[[registry]]` array, in priority order.** The first registry
//!    whose [`GitPackageRegistry::resolve`] succeeds wins. If a registry
//!    answers `UnknownPackage` (the package repo simply does not exist
//!    under that org URL), we fall through to the next. Other errors
//!    (network, auth, malformed manifest) bubble up immediately — those
//!    are not "package missing", they are operational failures the user
//!    should see.
//!
//! 3. **Mirror chain per registry** — schema-wired in this commit, runtime
//!    dispatch lands together with content-hash cross-source verification
//!    in M1.6 (Phase B). [`MultiRegistryResolver::mirrors_for`] exposes
//!    the priority-sorted list so downstream code is ready when fetch
//!    learns to consult it.
//!
//! `MultiResolution` and `MultiCached` enrich the registry-trait return
//! types with provenance (`registry_name`, `source_url`, `source_ref`,
//! `overridden`) — exactly what lockfile schema v2 needs to fill on each
//! install. Callers that only need the M0-shape `ResolvedPackage` /
//! `CachedPackage` continue to use them via the `.resolved` / `.cached`
//! field on the wrapper.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use vibe_core::PackageRef;
use vibe_core::manifest::{
    Lockfile, MirrorSection, OverrideSection, PackageManifest, RegistrySection,
};

use crate::git_backend::{GitBackend, ShellGit};
use crate::git_package_registry::{GitPackageRegistry, copy_dir_excluding_git};
use crate::git_registry::{DEFAULT_FRESHNESS_SECS, default_cache_root, strip_git_plus_prefix};
use crate::{
    CachedPackage, RegistryError, ResolvedPackage, compute_content_hash,
};

/// Default ref for `[[override]]` entries that omit `ref`. Most adopters
/// will pin a tag or branch explicitly; `main` is the practical default
/// for "just take HEAD on the canonical line".
pub const DEFAULT_OVERRIDE_REF: &str = "main";

/// A resolved package with provenance — which registry served it, the
/// URL / ref recorded in the lockfile, and whether the resolution
/// short-circuited via an override.
#[derive(Debug, Clone)]
pub struct MultiResolution {
    pub resolved: ResolvedPackage,
    /// Name of the `[[registry]]` that served this package. `None` for
    /// override-resolved entries.
    pub registry_name: Option<String>,
    /// What goes into lockfile `source_url`.
    pub source_url: String,
    /// What goes into lockfile `source_ref` — typically the version tag
    /// (`v0.3.0`) for registry resolutions, or the override's ref.
    pub source_ref: Option<String>,
    pub overridden: bool,
}

/// Resolver coordinating an ordered set of [`GitPackageRegistry`]
/// instances plus the cross-cutting `[[mirror]]` and `[[override]]`
/// layers from `vibe.toml`.
pub struct MultiRegistryResolver {
    registries: Vec<Arc<GitPackageRegistry>>,
    mirrors: Vec<MirrorSection>,
    overrides: HashMap<String, OverrideSection>,
    backend: Arc<dyn GitBackend>,
    cache_root: PathBuf,
}

impl MultiRegistryResolver {
    /// Direct constructor — every input handed in already-built. Used by
    /// tests and callers that want to substitute a specific backend.
    pub fn new(
        registries: Vec<Arc<GitPackageRegistry>>,
        mirrors: Vec<MirrorSection>,
        overrides: Vec<OverrideSection>,
        backend: Arc<dyn GitBackend>,
        cache_root: PathBuf,
    ) -> Self {
        let overrides = overrides
            .into_iter()
            .map(|o| (o.pkgref.clone(), o))
            .collect();
        MultiRegistryResolver {
            registries,
            mirrors,
            overrides,
            backend,
            cache_root,
        }
    }

    /// Build a resolver from `vibe.toml`-shape sections plus a backend
    /// reused across all `GitPackageRegistry` instances. Production
    /// callers pass `Arc::new(ShellGit::new())` as the backend; tests
    /// pass a fake.
    pub fn from_manifest(
        registries: &[RegistrySection],
        mirrors: &[MirrorSection],
        overrides: &[OverrideSection],
        cache_root: PathBuf,
        backend: Arc<dyn GitBackend>,
        freshness_secs: u64,
    ) -> Result<Self, RegistryError> {
        let mut built = Vec::with_capacity(registries.len());
        for reg in registries {
            // Compose the priority-sorted mirror chain for this registry
            // (named `of = "<reg.name>"` plus wildcard `of = "*"`). This
            // is exactly what `Self::mirrors_for` would compute, but
            // we're still building `self`.
            let mut chain: Vec<&MirrorSection> = mirrors
                .iter()
                .filter(|m| m.of == reg.name || m.of == "*")
                .collect();
            chain.sort_by_key(|m| m.priority);
            let mirror_urls: Vec<String> =
                chain.into_iter().map(|m| m.url.clone()).collect();

            let mut entry = GitPackageRegistry::open_with_mirrors(
                &reg.name,
                &reg.url,
                &reg.r#ref,
                reg.naming,
                mirror_urls,
                &cache_root,
                Arc::clone(&backend),
                freshness_secs,
            )?;
            // PROP-005 §2.10 slice 10 — when an upstream index is
            // configured for this registry via env vars, attach the
            // probed client. Probe is best-effort; absent or
            // unreachable index leaves the registry on the existing
            // git ls-remote path with no warning.
            if let Some(url) = crate::index_client::index_url_for(&reg.name)
                && let Some(client) = crate::index_client::IndexClient::probe(&url)
            {
                entry = entry.with_index_client(client);
            }
            built.push(Arc::new(entry));
        }
        Ok(Self::new(
            built,
            mirrors.to_vec(),
            overrides.to_vec(),
            backend,
            cache_root,
        ))
    }

    /// Default-flavoured constructor: `ShellGit` backend, default
    /// `~/.vibe/registries/` cache root, 1-hour freshness.
    pub fn open(
        registries: &[RegistrySection],
        mirrors: &[MirrorSection],
        overrides: &[OverrideSection],
    ) -> Result<Self, RegistryError> {
        let cache_root = default_cache_root()?;
        Self::from_manifest(
            registries,
            mirrors,
            overrides,
            cache_root,
            Arc::new(ShellGit::new()),
            DEFAULT_FRESHNESS_SECS,
        )
    }

    pub fn registries(&self) -> &[Arc<GitPackageRegistry>] {
        &self.registries
    }

    pub fn mirrors(&self) -> &[MirrorSection] {
        &self.mirrors
    }

    pub fn overrides(&self) -> &HashMap<String, OverrideSection> {
        &self.overrides
    }

    /// Mirrors targeting the named registry (plus any wildcard `of = "*"`
    /// entries), sorted by `priority` ascending.
    pub fn mirrors_for(&self, registry_name: &str) -> Vec<&MirrorSection> {
        let mut v: Vec<&MirrorSection> = self
            .mirrors
            .iter()
            .filter(|m| m.of == registry_name || m.of == "*")
            .collect();
        v.sort_by_key(|m| m.priority);
        v
    }

    /// Resolve a pkgref through the override-then-registries decision tree.
    pub fn resolve(&self, pkgref: &PackageRef) -> Result<MultiResolution, RegistryError> {
        // Step 1: override short-circuit.
        if let Some(ovr) = self.overrides.get(&pkgref.qualified_name()) {
            return self.resolve_override(pkgref, ovr);
        }

        // Step 2: priority-ordered registry walk.
        let mut last_unknown: Option<RegistryError> = None;
        for reg in &self.registries {
            match reg.resolve(pkgref) {
                Ok(resolved) => {
                    let url = reg.package_repo_url(resolved.kind, &resolved.name);
                    let source_ref = format!("v{}", resolved.version);
                    return Ok(MultiResolution {
                        resolved,
                        registry_name: Some(reg.name().to_string()),
                        source_url: url,
                        source_ref: Some(source_ref),
                        overridden: false,
                    });
                }
                Err(RegistryError::UnknownPackage { .. }) => {
                    last_unknown = Some(RegistryError::UnknownPackage {
                        kind: pkgref.kind,
                        name: pkgref.name.clone(),
                    });
                    continue;
                }
                Err(other) => return Err(other),
            }
        }

        Err(last_unknown.unwrap_or(RegistryError::UnknownPackage {
            kind: pkgref.kind,
            name: pkgref.name.clone(),
        }))
    }

    fn resolve_override(
        &self,
        pkgref: &PackageRef,
        ovr: &OverrideSection,
    ) -> Result<MultiResolution, RegistryError> {
        let refname = ovr
            .r#ref
            .clone()
            .unwrap_or_else(|| DEFAULT_OVERRIDE_REF.to_string());
        let manifest = self.read_override_manifest(&ovr.source_url, &refname)?;
        // Sanity: the override is supposed to point at *this* package. If
        // the manifest at the pinned ref names a different (kind, name),
        // installing it would silently misroute on disk. Refuse loudly.
        if manifest.package.kind != pkgref.kind || manifest.package.name != pkgref.name {
            return Err(RegistryError::MalformedMeta {
                path: PathBuf::from(format!("{}@{}:vibe-package.toml", ovr.source_url, refname)),
                reason: format!(
                    "override for `{}:{}` points at a manifest declaring `{}:{}` — refusing to install",
                    pkgref.kind, pkgref.name, manifest.package.kind, manifest.package.name
                ),
            });
        }
        let resolved = ResolvedPackage {
            kind: pkgref.kind,
            name: pkgref.name.clone(),
            version: manifest.package.version.clone(),
            source_dir: self.override_clone_dir(pkgref.kind, &pkgref.name),
        };
        Ok(MultiResolution {
            resolved,
            registry_name: None,
            source_url: ovr.source_url.clone(),
            source_ref: Some(refname),
            overridden: true,
        })
    }

    fn read_override_manifest(
        &self,
        url: &str,
        refname: &str,
    ) -> Result<PackageManifest, RegistryError> {
        let bytes = self.backend.fetch_file_at_ref(
            strip_git_plus_prefix(url),
            refname,
            PackageManifest::FILENAME,
        )?;
        let text = String::from_utf8(bytes).map_err(|e| RegistryError::MalformedMeta {
            path: PathBuf::from(format!("{url}@{refname}:{}", PackageManifest::FILENAME)),
            reason: format!("invalid UTF-8: {e}"),
        })?;
        let mut m: PackageManifest =
            toml::from_str(&text).map_err(|e| RegistryError::MalformedMeta {
                path: PathBuf::from(format!("{url}@{refname}:{}", PackageManifest::FILENAME)),
                reason: e.to_string(),
            })?;
        m.normalize_legacy_deps();
        Ok(m)
    }

    /// Materialise a previously-resolved package into the per-project cache.
    /// The returned [`CachedPackage`] carries lockfile-v2 provenance
    /// (`registry_name` / `source_ref` / `overridden`) populated by the
    /// `GitPackageRegistry` impl or by the override path.
    pub fn fetch(
        &self,
        resolution: &MultiResolution,
        project_cache: &Path,
    ) -> Result<CachedPackage, RegistryError> {
        self.fetch_with_expected_hash(resolution, project_cache, None)
    }

    /// Mirror-aware fetch with an optional cross-source content_hash gate.
    ///
    /// `expected_hash`, when supplied (typically the lockfile pin for
    /// this `(kind, name, version)`), is enforced source-by-source:
    /// each URL in the registry's primary-then-mirror chain is tried,
    /// and the first whose served content matches the pin wins. A
    /// disagreeing source is logged at `tracing::warn!` and skipped.
    /// If every source disagrees, the last one's [`CachedPackage`] is
    /// returned and the install layer's
    /// [`vibe_install::plan_install`] surfaces the
    /// `ContentDrift` error against the lockfile pin.
    ///
    /// Override-resolved entries skip mirror dispatch entirely —
    /// `[[override]]` is a surgical pin to one specific URL/ref by
    /// design, so the same URL is the only legitimate source.
    pub fn fetch_with_expected_hash(
        &self,
        resolution: &MultiResolution,
        project_cache: &Path,
        expected_hash: Option<&str>,
    ) -> Result<CachedPackage, RegistryError> {
        if resolution.overridden {
            return self.fetch_override(resolution, project_cache);
        }
        let registry_name =
            resolution
                .registry_name
                .as_deref()
                .ok_or_else(|| RegistryError::UnknownPackage {
                    kind: resolution.resolved.kind,
                    name: resolution.resolved.name.clone(),
                })?;
        let reg =
            self.registries
                .iter()
                .find(|r| r.name() == registry_name)
                .ok_or_else(|| RegistryError::UnknownPackage {
                    kind: resolution.resolved.kind,
                    name: resolution.resolved.name.clone(),
                })?;
        // `GitPackageRegistry::fetch_with_expected_hash` already populates
        // `registry_name` / `source_ref` / `overridden = false` correctly;
        // nothing to wrap.
        reg.fetch_with_expected_hash(&resolution.resolved, project_cache, expected_hash)
    }

    fn fetch_override(
        &self,
        resolution: &MultiResolution,
        project_cache: &Path,
    ) -> Result<CachedPackage, RegistryError> {
        let url = &resolution.source_url;
        let refname = resolution
            .source_ref
            .clone()
            .unwrap_or_else(|| DEFAULT_OVERRIDE_REF.to_string());
        let kind = resolution.resolved.kind;
        let name = resolution.resolved.name.as_str();

        let clone_dir = self.override_clone_dir(kind, name);
        ensure_clone_at(self.backend.as_ref(), url, &refname, &clone_dir)?;

        let dest = project_cache
            .join(kind.as_str())
            .join(name)
            .join(format!("v{}", resolution.resolved.version));
        if dest.exists() {
            std::fs::remove_dir_all(&dest).map_err(|source| RegistryError::Io {
                path: dest.clone(),
                source,
            })?;
        }
        copy_dir_excluding_git(&clone_dir, &dest)?;

        let manifest_path = dest.join(PackageManifest::FILENAME);
        let manifest = PackageManifest::read(&manifest_path)?;
        let content_hash = compute_content_hash(&dest)?;

        Ok(CachedPackage {
            resolved: ResolvedPackage {
                kind,
                name: name.to_string(),
                version: resolution.resolved.version.clone(),
                source_dir: clone_dir,
            },
            cache_dir: dest,
            manifest,
            content_hash,
            source_uri: url.clone(),
            registry_name: None,
            source_ref: Some(refname),
            resolved_commit: None,
            overridden: true,
        })
    }

    /// Where override clones live —
    /// `<cache_root>/__overrides__/<kind>-<name>/clone/`. Distinct
    /// directory tree from registry-served clones so a package that flips
    /// between override and registry origins on different days does not
    /// share state across modes.
    fn override_clone_dir(&self, kind: vibe_core::PackageKind, name: &str) -> PathBuf {
        self.cache_root
            .join("__overrides__")
            .join(format!("{}-{}", kind.as_str(), name))
            .join("clone")
    }

    /// Walk every entry in `lockfile` and refresh its on-disk clone
    /// (registry-served entries via the appropriate `[[registry]]`,
    /// override-resolved entries via the `__overrides__` subtree).
    /// Called by `vibe registry sync`.
    ///
    /// Entries with `registry: None` and `overridden: false` (legacy
    /// content fetched through pre-PROP-002 paths, or `LocalRegistry`
    /// installs) are reported as skipped — there is nothing per-package
    /// to refresh for them.
    ///
    /// Errors short-circuit: a partial refresh can still leave
    /// already-refreshed clones up-to-date, but we surface the first
    /// failure rather than silently swallowing.
    pub fn refresh_lockfile_clones(
        &self,
        lockfile: &Lockfile,
    ) -> Result<RefreshReport, RegistryError> {
        let mut report = RefreshReport::default();
        for entry in &lockfile.packages {
            if entry.overridden {
                self.refresh_override_entry(entry, &mut report)?;
            } else if let Some(registry_name) = entry.registry.as_deref() {
                self.refresh_registry_entry(entry, registry_name, &mut report)?;
            } else {
                report.skipped.push(SkippedEntry {
                    kind: entry.kind,
                    name: entry.name.clone(),
                    reason: "lockfile entry has neither `registry` nor `overridden = true` \
                             (likely installed via `--registry <path>` or a legacy v1 path)"
                        .to_string(),
                });
            }
        }
        Ok(report)
    }

    fn refresh_registry_entry(
        &self,
        entry: &vibe_core::manifest::LockedPackage,
        registry_name: &str,
        report: &mut RefreshReport,
    ) -> Result<(), RegistryError> {
        let Some(reg) = self.registries.iter().find(|r| r.name() == registry_name) else {
            report.skipped.push(SkippedEntry {
                kind: entry.kind,
                name: entry.name.clone(),
                reason: format!(
                    "lockfile names registry `{registry_name}` but no `[[registry]]` with that \
                     name exists in `vibe.toml` — drop the lockfile entry or restore the registry"
                ),
            });
            return Ok(());
        };
        // Use the recorded source_ref if present (typically `v<version>`);
        // fall back to the registry's own ref otherwise.
        let refname = entry
            .source_ref
            .clone()
            .unwrap_or_else(|| format!("v{}", entry.version));
        reg.refresh_package(entry.kind, &entry.name, &refname)?;
        report.refreshed.push(RefreshedEntry {
            kind: entry.kind,
            name: entry.name.clone(),
            via: RefreshedVia::Registry(registry_name.to_string()),
            refname,
        });
        Ok(())
    }

    fn refresh_override_entry(
        &self,
        entry: &vibe_core::manifest::LockedPackage,
        report: &mut RefreshReport,
    ) -> Result<(), RegistryError> {
        let url = entry.source_url.clone();
        let refname = entry
            .source_ref
            .clone()
            .unwrap_or_else(|| DEFAULT_OVERRIDE_REF.to_string());
        let clone_dir = self.override_clone_dir(entry.kind, &entry.name);
        ensure_clone_at(self.backend.as_ref(), &url, &refname, &clone_dir)?;
        report.refreshed.push(RefreshedEntry {
            kind: entry.kind,
            name: entry.name.clone(),
            via: RefreshedVia::Override,
            refname,
        });
        Ok(())
    }
}

/// Per-entry outcome of [`MultiRegistryResolver::refresh_lockfile_clones`].
#[derive(Debug, Clone, Default)]
pub struct RefreshReport {
    pub refreshed: Vec<RefreshedEntry>,
    pub skipped: Vec<SkippedEntry>,
}

#[derive(Debug, Clone)]
pub struct RefreshedEntry {
    pub kind: vibe_core::PackageKind,
    pub name: String,
    pub via: RefreshedVia,
    pub refname: String,
}

#[derive(Debug, Clone)]
pub enum RefreshedVia {
    Registry(String),
    Override,
}

#[derive(Debug, Clone)]
pub struct SkippedEntry {
    pub kind: vibe_core::PackageKind,
    pub name: String,
    pub reason: String,
}

fn ensure_clone_at(
    backend: &dyn GitBackend,
    url: &str,
    refname: &str,
    clone_dir: &Path,
) -> Result<(), RegistryError> {
    if clone_dir.join(".git").exists() {
        backend.update(clone_dir, refname)?;
        return Ok(());
    }
    if clone_dir.exists() {
        std::fs::remove_dir_all(clone_dir).map_err(|source| RegistryError::Io {
            path: clone_dir.to_path_buf(),
            source,
        })?;
    }
    if let Some(parent) = clone_dir.parent() {
        std::fs::create_dir_all(parent).map_err(|source| RegistryError::Io {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    backend.bootstrap(strip_git_plus_prefix(url), refname, clone_dir)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::fs;
    use std::sync::Mutex;
    use tempfile::tempdir;
    use vibe_core::manifest::NamingConvention;
    use vibe_core::PackageRef;

    use crate::git_backend::GitError;

    /// Test-only `GitBackend` shared across multi-registry tests. Same
    /// shape as the one in `git_package_registry::tests`; duplicated
    /// here rather than promoted to a shared `test_support` module to
    /// keep this commit narrow — that consolidation can land separately.
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

    fn registry_section(name: &str, url: &str) -> RegistrySection {
        RegistrySection {
            name: name.to_string(),
            url: url.to_string(),
            r#ref: "main".to_string(),
            naming: NamingConvention::KindName,
            auth: vibe_core::manifest::AuthKind::None,
            token_env: None,
        }
    }

    fn manifest_text(name: &str, kind: &str, version: &str) -> String {
        format!("[package]\nname = \"{name}\"\nkind = \"{kind}\"\nversion = \"{version}\"\n")
    }

    fn build_resolver(
        cache: &Path,
        registries: Vec<RegistrySection>,
        mirrors: Vec<MirrorSection>,
        overrides: Vec<OverrideSection>,
        backend: Arc<FakeBackend>,
    ) -> MultiRegistryResolver {
        MultiRegistryResolver::from_manifest(
            &registries,
            &mirrors,
            &overrides,
            cache.to_path_buf(),
            backend,
            DEFAULT_FRESHNESS_SECS,
        )
        .unwrap()
    }

    #[test]
    fn resolve_picks_first_registry_with_match() {
        let cache = tempdir().unwrap();
        let fake = Arc::new(FakeBackend::default());
        // Both registries have the package; first wins.
        fake.seed_tags(
            "git@host:org-a/flow-wal.git",
            vec!["v0.1.0".into(), "v0.2.0".into()],
        );
        fake.seed_tags("git@host:org-b/flow-wal.git", vec!["v0.5.0".into()]);

        let r = build_resolver(
            cache.path(),
            vec![
                registry_section("a", "git@host:org-a"),
                registry_section("b", "git@host:org-b"),
            ],
            vec![],
            vec![],
            fake,
        );

        let p = PackageRef::parse("flow:wal").unwrap();
        let m = r.resolve(&p).unwrap();
        assert_eq!(m.registry_name.as_deref(), Some("a"));
        assert_eq!(m.resolved.version.to_string(), "0.2.0");
        assert!(!m.overridden);
        assert_eq!(m.source_url, "git@host:org-a/flow-wal.git");
        assert_eq!(m.source_ref.as_deref(), Some("v0.2.0"));
    }

    #[test]
    fn resolve_falls_through_to_next_registry_on_unknown_package() {
        let cache = tempdir().unwrap();
        let fake = Arc::new(FakeBackend::default());
        // First registry: no seed for this URL → RepoNotFound → fall through.
        fake.seed_tags("git@host:org-b/flow-wal.git", vec!["v0.5.0".into()]);

        let r = build_resolver(
            cache.path(),
            vec![
                registry_section("a", "git@host:org-a"),
                registry_section("b", "git@host:org-b"),
            ],
            vec![],
            vec![],
            fake,
        );

        let p = PackageRef::parse("flow:wal").unwrap();
        let m = r.resolve(&p).unwrap();
        assert_eq!(m.registry_name.as_deref(), Some("b"));
        assert_eq!(m.resolved.version.to_string(), "0.5.0");
    }

    #[test]
    fn resolve_unknown_package_when_no_registry_has_it() {
        let cache = tempdir().unwrap();
        let fake = Arc::new(FakeBackend::default());
        // No seed for any URL.

        let r = build_resolver(
            cache.path(),
            vec![
                registry_section("a", "git@host:org-a"),
                registry_section("b", "git@host:org-b"),
            ],
            vec![],
            vec![],
            fake,
        );

        let p = PackageRef::parse("flow:ghost").unwrap();
        let err = r.resolve(&p).unwrap_err();
        assert!(matches!(err, RegistryError::UnknownPackage { .. }));
    }

    #[test]
    fn resolve_unknown_when_no_registries_and_no_override() {
        let cache = tempdir().unwrap();
        let fake = Arc::new(FakeBackend::default());
        let r = build_resolver(cache.path(), vec![], vec![], vec![], fake);
        let p = PackageRef::parse("flow:wal").unwrap();
        let err = r.resolve(&p).unwrap_err();
        assert!(matches!(err, RegistryError::UnknownPackage { .. }));
    }

    #[test]
    fn override_short_circuits_registry_resolution() {
        let cache = tempdir().unwrap();
        let fake = Arc::new(FakeBackend::default());
        // Registry has flow:wal at 0.2.0, but override pins to a fork.
        fake.seed_tags("git@host:org-a/flow-wal.git", vec!["v0.2.0".into()]);
        // Override URL: serve a manifest pinned at "my-fix" branch.
        fake.seed_file(
            "git@my-fork:vibevm/wal-fork.git",
            "my-fix",
            "vibe-package.toml",
            manifest_text("wal", "flow", "0.2.0").into_bytes(),
        );

        let ovr = OverrideSection {
            pkgref: "flow:wal".to_string(),
            source_url: "git@my-fork:vibevm/wal-fork.git".to_string(),
            r#ref: Some("my-fix".to_string()),
            reason: Some("waiting on upstream PR".to_string()),
        };

        let r = build_resolver(
            cache.path(),
            vec![registry_section("a", "git@host:org-a")],
            vec![],
            vec![ovr],
            fake,
        );

        let p = PackageRef::parse("flow:wal").unwrap();
        let m = r.resolve(&p).unwrap();
        assert!(m.overridden);
        assert!(m.registry_name.is_none());
        assert_eq!(m.source_url, "git@my-fork:vibevm/wal-fork.git");
        assert_eq!(m.source_ref.as_deref(), Some("my-fix"));
        assert_eq!(m.resolved.version.to_string(), "0.2.0");
    }

    #[test]
    fn override_uses_default_ref_when_unspecified() {
        let cache = tempdir().unwrap();
        let fake = Arc::new(FakeBackend::default());
        fake.seed_file(
            "git@my-fork:vibevm/wal-fork.git",
            DEFAULT_OVERRIDE_REF,
            "vibe-package.toml",
            manifest_text("wal", "flow", "1.0.0").into_bytes(),
        );

        let ovr = OverrideSection {
            pkgref: "flow:wal".to_string(),
            source_url: "git@my-fork:vibevm/wal-fork.git".to_string(),
            r#ref: None,
            reason: None,
        };

        let r = build_resolver(cache.path(), vec![], vec![], vec![ovr], fake);
        let p = PackageRef::parse("flow:wal").unwrap();
        let m = r.resolve(&p).unwrap();
        assert_eq!(m.source_ref.as_deref(), Some(DEFAULT_OVERRIDE_REF));
        assert_eq!(m.resolved.version.to_string(), "1.0.0");
    }

    #[test]
    fn override_refuses_when_manifest_identity_mismatches() {
        let cache = tempdir().unwrap();
        let fake = Arc::new(FakeBackend::default());
        // The manifest at the pinned ref claims to be `flow:atomic-commits`,
        // but the override is for `flow:wal`. Refuse loudly — silently
        // installing as `flow:wal` would corrupt the lockfile.
        fake.seed_file(
            "git@my-fork:vibevm/wal-fork.git",
            "main",
            "vibe-package.toml",
            manifest_text("atomic-commits", "flow", "0.1.0").into_bytes(),
        );

        let ovr = OverrideSection {
            pkgref: "flow:wal".to_string(),
            source_url: "git@my-fork:vibevm/wal-fork.git".to_string(),
            r#ref: None,
            reason: None,
        };
        let r = build_resolver(cache.path(), vec![], vec![], vec![ovr], fake);
        let p = PackageRef::parse("flow:wal").unwrap();
        let err = r.resolve(&p).unwrap_err();
        match err {
            RegistryError::MalformedMeta { reason, .. } => {
                assert!(reason.contains("refusing to install"));
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn fetch_dispatches_to_registry_that_resolved() {
        let cache = tempdir().unwrap();
        let pkg_cache = tempdir().unwrap();
        let upstream = tempdir().unwrap();

        // Build an upstream tree at the second registry's URL.
        let pkg_root = upstream.path().join("pkg");
        fs::create_dir_all(&pkg_root).unwrap();
        fs::write(
            pkg_root.join("vibe-package.toml"),
            manifest_text("wal", "flow", "0.5.0"),
        )
        .unwrap();

        let fake = Arc::new(FakeBackend::default());
        fake.seed_tags("git@host:org-b/flow-wal.git", vec!["v0.5.0".into()]);
        fake.seed_bootstrap("git@host:org-b/flow-wal.git", pkg_root.clone());

        let r = build_resolver(
            cache.path(),
            vec![
                registry_section("a", "git@host:org-a"), // empty (no seed)
                registry_section("b", "git@host:org-b"),
            ],
            vec![],
            vec![],
            fake.clone(),
        );

        let p = PackageRef::parse("flow:wal").unwrap();
        let resolution = r.resolve(&p).unwrap();
        let cached = r.fetch(&resolution, pkg_cache.path()).unwrap();

        assert_eq!(cached.registry_name.as_deref(), Some("b"));
        assert!(!cached.overridden);
        assert_eq!(cached.source_uri, "git@host:org-b/flow-wal.git");
        assert_eq!(cached.source_ref.as_deref(), Some("v0.5.0"));
        assert_eq!(cached.manifest.package.version.to_string(), "0.5.0");
        assert!(cached.cache_dir.join("vibe-package.toml").exists());
        assert!(!cached.cache_dir.join(".git").exists());
        // Bootstrap exactly once — only against registry "b".
        assert_eq!(fake.bootstrap_count(), 1);
    }

    #[test]
    fn fetch_override_clones_into_overrides_subtree_and_marks_overridden() {
        let cache = tempdir().unwrap();
        let pkg_cache = tempdir().unwrap();
        let upstream = tempdir().unwrap();

        let pkg_root = upstream.path().join("pkg");
        fs::create_dir_all(&pkg_root).unwrap();
        fs::write(
            pkg_root.join("vibe-package.toml"),
            manifest_text("wal", "flow", "0.9.0"),
        )
        .unwrap();

        let fake = Arc::new(FakeBackend::default());
        // For override: backend serves manifest via `fetch_file_at_ref`
        // (resolve), then clones via `bootstrap` (fetch).
        fake.seed_file(
            "git@my-fork:vibevm/wal-fork.git",
            "my-fix",
            "vibe-package.toml",
            manifest_text("wal", "flow", "0.9.0").into_bytes(),
        );
        fake.seed_bootstrap("git@my-fork:vibevm/wal-fork.git", pkg_root.clone());

        let ovr = OverrideSection {
            pkgref: "flow:wal".to_string(),
            source_url: "git@my-fork:vibevm/wal-fork.git".to_string(),
            r#ref: Some("my-fix".to_string()),
            reason: Some("PR pending".to_string()),
        };

        let r = build_resolver(cache.path(), vec![], vec![], vec![ovr], fake.clone());

        let p = PackageRef::parse("flow:wal").unwrap();
        let resolution = r.resolve(&p).unwrap();
        let cached = r.fetch(&resolution, pkg_cache.path()).unwrap();

        assert!(cached.overridden);
        assert!(cached.registry_name.is_none());
        assert_eq!(cached.source_uri, "git@my-fork:vibevm/wal-fork.git");
        assert_eq!(cached.source_ref.as_deref(), Some("my-fix"));
        assert_eq!(cached.manifest.package.version.to_string(), "0.9.0");
        // Override clone lives under cache_root/__overrides__/flow-wal/clone/
        let overrides_root = cache.path().join("__overrides__").join("flow-wal").join("clone");
        assert!(overrides_root.join(".git").exists());
        // Materialised cache holds payload only.
        assert!(cached.cache_dir.join("vibe-package.toml").exists());
        assert!(!cached.cache_dir.join(".git").exists());
    }

    #[test]
    fn mirrors_for_filters_and_sorts() {
        let cache = tempdir().unwrap();
        let fake = Arc::new(FakeBackend::default());

        let mirrors = vec![
            MirrorSection {
                of: "vibespecs".to_string(),
                url: "https://a".to_string(),
                priority: 2,
            },
            MirrorSection {
                of: "vibespecs".to_string(),
                url: "https://b".to_string(),
                priority: 1,
            },
            MirrorSection {
                of: "*".to_string(),
                url: "https://catchall".to_string(),
                priority: 99,
            },
            MirrorSection {
                of: "other".to_string(),
                url: "https://unrelated".to_string(),
                priority: 0,
            },
        ];
        let r = build_resolver(
            cache.path(),
            vec![registry_section("vibespecs", "git@host:org")],
            mirrors,
            vec![],
            fake,
        );

        let m = r.mirrors_for("vibespecs");
        assert_eq!(m.len(), 3);
        assert_eq!(m[0].url, "https://b");
        assert_eq!(m[1].url, "https://a");
        assert_eq!(m[2].url, "https://catchall");
    }
}
