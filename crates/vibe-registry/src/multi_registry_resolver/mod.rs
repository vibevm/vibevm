//! Multi-registry resolver — PROP-002.
//!
//! Sits on top of one or more [`GitPerPackageRegistry`] instances and dispatches
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
//!    whose [`GitPerPackageRegistry::resolve`] succeeds wins. If a registry
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

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-model");

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use vibe_core::manifest::{
    GitPackageDep, Lockfile, Manifest, MirrorSection, OverrideSection, RedirectFile, RefPolicy,
    RegistrySection, parse_redirect_bytes,
};
use vibe_core::{Group, PackageRef, VersionSpec};

use crate::git_backend::{GitBackend, GitError, ShellGit};
use crate::git_package_registry::GitPerPackageRegistry;
use crate::registry_cache::{DEFAULT_FRESHNESS_SECS, default_cache_root, strip_git_plus_prefix};
use crate::{
    CachedPackage, InPlaceMaterialised, LocalRegistry, RegistryError, ResolvedPackage,
    compute_content_hash, store,
};

mod attempt;
mod dispatch;
mod offline;
mod override_resolve;
mod redirect_follow;
mod refresh;
mod resolution;
mod source;
mod sources;
mod walk;

pub use attempt::{RegistryWalkAttempt, WalkAttemptStatus};
pub use refresh::{RefreshReport, RefreshedEntry, RefreshedVia, SkippedEntry};
pub use resolution::{MultiResolution, ResolvedPathDep};
pub use source::{LocalRegistrySource, RegistrySource};
pub(crate) use source::{is_local_directory_url, local_path_from_url};

/// Default ref for `[[override]]` entries that omit `ref`. Most adopters
/// will pin a tag or branch explicitly; `main` is the practical default
/// for "just take HEAD on the canonical line".
pub const DEFAULT_OVERRIDE_REF: &str = "main";

/// Resolver coordinating an ordered set of [`GitPerPackageRegistry`]
/// instances plus the cross-cutting `[[mirror]]` and `[[override]]`
/// layers from `vibe.toml`.
pub struct MultiRegistryResolver {
    registries: Vec<Arc<GitPerPackageRegistry>>,
    /// The ordered walk list — git and local sources interleaved in the
    /// declared `[[registry]]` order. The four core operations
    /// (list / resolve / fetch-dep-manifest / fetch) iterate this; the
    /// git-only operations (index short-name, refresh, vendor clone-dir)
    /// stay on `registries`, the denormalised git subset, since a local
    /// directory has no index / git-refresh / per-package clone.
    sources: Vec<RegistrySource>,
    mirrors: Vec<MirrorSection>,
    overrides: HashMap<String, OverrideSection>,
    /// Git-source declarations from `[requires.packages]` table-form
    /// (PROP-002 §2.4.1), keyed by `<group>/<name>` qualified-name
    /// (PROP-008). Resolution order (resolve()): override > path-source
    /// > git-source > registry-walk.
    git_packages: HashMap<String, GitPackageDep>,
    /// Path-source declarations from `[requires.packages]` table-form
    /// (PROP-007 §2.5), keyed by `<group>/<name>` qualified-name
    /// (PROP-008). Sits one notch above git-source in the resolution
    /// order — a pkgref present here wins over a same-pkgref git-source
    /// declaration.
    path_packages: HashMap<String, ResolvedPathDep>,
    backend: Arc<dyn GitBackend>,
    cache_root: PathBuf,
    /// Strict-auth posture — when `true`, a 401 / 403 against a
    /// public (`auth = "none"`) registry is treated as a halt
    /// instead of a walk-to-next, even though the §2.3.1 default
    /// for that combination is fall-through. Useful in CI / cron
    /// where the operator wants to gate "private install must
    /// come from the private registry; if the private registry is
    /// down or its 401 leaks through to a fallback, fail loudly
    /// rather than silently picking up a public substitute."
    /// Toggled by `MultiRegistryResolver::with_strict_auth`.
    strict_auth: bool,
    /// Offline posture (PROP-010 §2.6, `RESOLVER-OFFLINE-MODE`): when
    /// `true`, resolution runs against the machine store — version
    /// candidates and dependency manifests are read from the store
    /// (`as of its last refresh`), plus the local `file://` sources,
    /// which never touch the network — and NO `git fetch` / `ls-remote`
    /// / archive call is made. A store miss is a hard error naming the
    /// package and the recovery recipes (`OFFLINE-HARD-ERROR`).
    /// Toggled by [`MultiRegistryResolver::with_offline`].
    offline: bool,
    /// The machine store root this resolver reads (PROP-010 §2.7) —
    /// handed in as a builder parameter (not resolved in place) so
    /// resolver tests isolate the store by parameter, the same way the
    /// whole store layer does; production callers pass
    /// [`store::store_root`].
    store_root: Option<PathBuf>,
    /// The project's lockfile entries, keyed by `<group>/<name>` — the
    /// provenance channel for store-backed resolutions: a store hit is
    /// authoritative for AVAILABILITY, but `source_uri` still comes
    /// from the existing lock record (the availability case is a
    /// re-resolve of an earlier install; minting a new `store://`
    /// wire value would be an owner act). Toggled by
    /// [`MultiRegistryResolver::with_locked_packages`].
    locked: HashMap<String, vibe_core::manifest::LockedPackage>,
}

impl MultiRegistryResolver {
    /// Direct constructor — every input handed in already-built. Used by
    /// tests and callers that want to substitute a specific backend. The git
    /// subset (`registries`) is derived from `sources` so the two never
    /// disagree.
    pub fn new(
        sources: Vec<RegistrySource>,
        mirrors: Vec<MirrorSection>,
        overrides: Vec<OverrideSection>,
        backend: Arc<dyn GitBackend>,
        cache_root: PathBuf,
    ) -> Self {
        let registries = sources
            .iter()
            .filter_map(|s| match s {
                RegistrySource::Git(g) => Some(Arc::clone(g)),
                RegistrySource::Local(_) => None,
            })
            .collect();
        let overrides = overrides
            .into_iter()
            .map(|o| (o.pkgref.clone(), o))
            .collect();
        MultiRegistryResolver {
            registries,
            sources,
            mirrors,
            overrides,
            git_packages: HashMap::new(),
            path_packages: HashMap::new(),
            backend,
            cache_root,
            strict_auth: false,
            offline: false,
            store_root: None,
            locked: HashMap::new(),
        }
    }

    /// Plumb in the git-source declarations from `vibe.toml`'s
    /// `[requires.packages]` table-form (PROP-002 §2.4.1). Builder-style
    /// so existing call-sites of `from_manifest` / `open` / `new` that
    /// don't yet thread git-source deps stay source-compatible.
    ///
    /// Keyed by `<group>/<name>` qualified-name (PROP-008) so a
    /// `pkgref.qualified_name()` lookup hits.
    pub fn with_git_packages(mut self, deps: Vec<GitPackageDep>) -> Self {
        self.git_packages = deps
            .into_iter()
            .map(|d| (format!("{}/{}", d.group, d.name), d))
            .collect();
        self
    }

    /// Read-only view of the registered git-source declarations.
    pub fn git_packages(&self) -> &HashMap<String, GitPackageDep> {
        &self.git_packages
    }

    /// Plumb in the path-source declarations from `vibe.toml`'s
    /// `[requires.packages]` table-form (PROP-007 §2.5). Builder-style,
    /// mirroring [`Self::with_git_packages`] — existing call-sites that
    /// don't thread path-source deps stay source-compatible. Each
    /// [`ResolvedPathDep`] arrives with `package_dir` / `workspace_rel`
    /// already computed by the workspace layer; the resolver does no
    /// filesystem path arithmetic itself.
    pub fn with_path_packages(mut self, deps: Vec<ResolvedPathDep>) -> Self {
        self.path_packages = deps
            .into_iter()
            .map(|d| (format!("{}/{}", d.group, d.name), d))
            .collect();
        self
    }

    /// Read-only view of the registered path-source declarations.
    pub fn path_packages(&self) -> &HashMap<String, ResolvedPathDep> {
        &self.path_packages
    }

    /// Toggle strict-auth posture (see field docs / PROP-002 §2.3.1
    /// strict-auth corollary). Builder-style consume-and-return.
    pub fn with_strict_auth(mut self, strict: bool) -> Self {
        self.strict_auth = strict;
        self
    }

    /// Toggle the offline posture (PROP-010 §2.6,
    /// `RESOLVER-OFFLINE-MODE`): resolution and fetch become
    /// satisfiable entirely from local sources — the machine store
    /// plus `file://` registries — and never run `git fetch` /
    /// `ls-remote` / archive. Anything not available locally is a
    /// hard error with an actionable message (PROP-010 §2.5,
    /// `OFFLINE-HARD-ERROR`). Builder-style consume-and-return.
    pub fn with_offline(mut self, offline: bool) -> Self {
        self.offline = offline;
        self
    }

    /// Whether the resolver is in the offline posture. The CLI
    /// surface reads this to confirm the toggle flowed through.
    pub fn offline(&self) -> bool {
        self.offline
    }

    /// Hand in the machine store root this resolver reads (PROP-010
    /// §2.7). A builder parameter, not an in-place resolve, so tests
    /// isolate the store by parameter — the same discipline the whole
    /// store layer uses; production callers pass `store::store_root()?`.
    pub fn with_store_root(mut self, root: PathBuf) -> Self {
        self.store_root = Some(root);
        self
    }

    /// Hand in the project's lockfile entries — the provenance channel
    /// for store-backed resolutions (PROP-010 §2.6): a store hit is
    /// authoritative for availability, but `source_uri` comes from the
    /// existing lock record; a package in no registry and in no lock
    /// entry is NOT rescued by the store. Builder-style.
    pub fn with_locked_packages(mut self, locked: Vec<vibe_core::manifest::LockedPackage>) -> Self {
        self.locked = locked
            .into_iter()
            .map(|p| (format!("{}/{}", p.group, p.name), p))
            .collect();
        self
    }

    /// Whether the resolver is in strict-auth mode. Tests + the
    /// CLI surface read this to confirm the toggle flowed through.
    pub fn strict_auth(&self) -> bool {
        self.strict_auth
    }

    /// Build a resolver from `vibe.toml`-shape sections plus a backend
    /// reused across all `GitPerPackageRegistry` instances. Production
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
        let mut sources: Vec<RegistrySource> = Vec::with_capacity(registries.len());
        for reg in registries {
            // PROP-002 §2.2.3 #enabled: a disabled registry is skipped
            // entirely — never built into the resolver, so no path (install /
            // outdated / search / sync) consults it. Flip `enabled` back to
            // re-activate; no re-add needed.
            if !reg.enabled {
                continue;
            }
            // A `[[registry]]` url with an explicit `file:` scheme (the
            // documented local-directory form, e.g. `file:///C:/repos/app`)
            // is served straight off the filesystem by `LocalRegistry` —
            // never git-cloned — so a plain on-disk directory works as a
            // registry (PROP-002 §2.2.2). A bare path or a `git+` transport
            // (`git+file://`, `git+https://`) stays on the git-clone backend:
            // those are local/remote *git* repos to clone, the historical
            // behaviour the multi-registry tests and local-git workflows
            // rely on. (`url_is_local` is a wider predicate used by the
            // separate `--offline` filter; the backend choice is narrower —
            // `file:` only.) `naming` / `auth` / `mirrors` / `index_client`
            // are git-only knobs and do not apply to a LocalRegistry.
            if is_local_directory_url(&reg.url) {
                let path = local_path_from_url(&reg.url)?;
                sources.push(RegistrySource::Local(LocalRegistrySource {
                    name: reg.name.clone(),
                    url: reg.url.clone(),
                    registry: LocalRegistry::new(path)?,
                }));
                continue;
            }
            // Compose the priority-sorted mirror chain for this registry
            // (named `of = "<reg.name>"` plus wildcard `of = "*"`). This
            // is exactly what `Self::mirrors_for` would compute, but
            // we're still building `self`.
            let mut chain: Vec<&MirrorSection> = mirrors
                .iter()
                .filter(|m| m.of == reg.name || m.of == "*")
                .collect();
            chain.sort_by_key(|m| m.priority);
            let mirror_urls: Vec<String> = chain.into_iter().map(|m| m.url.clone()).collect();

            // PROP-002 §2.2.1 — thread the registry's auth regime and
            // the explicit-or-derived token env-var name into the
            // registry instance, so it can pre-flight `MissingToken`
            // errors and inject the token into per-package URLs at
            // git invocation time.
            let token_env_name = if matches!(reg.auth, vibe_core::manifest::AuthKind::TokenEnv) {
                reg.resolve_token_env_name()
            } else {
                None
            };
            // PROP-002 §2.2.1 — a public (`auth = "none"`) registry must
            // never turn a 401/403 into an interactive credential prompt.
            // Hand it an anonymous-posture backend that forces the git
            // credential-silencing layer on regardless of TTY, so a
            // missing/private repo classifies as "no answer here" and the
            // walk continues (`GitBackend::anonymized_for_public`).
            // Authenticated regimes keep the shared backend, whose 401 is a
            // real, actionable failure the operator must see.
            let entry_backend = if matches!(reg.auth, vibe_core::manifest::AuthKind::None) {
                backend
                    .anonymized_for_public()
                    .unwrap_or_else(|| Arc::clone(&backend))
            } else {
                Arc::clone(&backend)
            };
            let mut entry = GitPerPackageRegistry::open_with_auth(
                &reg.name,
                &reg.url,
                &reg.r#ref,
                reg.naming,
                mirror_urls,
                &cache_root,
                entry_backend,
                freshness_secs,
                reg.auth,
                token_env_name.as_deref(),
            )?;
            // PROP-005 §2.10 slice 10 — when an upstream index is
            // configured for this registry via env vars, attach the
            // probed client. Probe is best-effort; absent or
            // unreachable index leaves the registry on the existing
            // git ls-remote path with no warning.
            if let Some(url) = crate::index_client::index_url_for(&reg.name) {
                // A2-INDEXAUTH — authenticate to this registry's index
                // with the registry's own credentials (bearer from
                // `auth`/`token_env`, the same source the git side
                // reads). `for_registry` is the scheme gate: a token is
                // attached only over `https://`.
                let auth = crate::index_client::IndexAuth::for_registry(reg, &url);
                match crate::index_client::IndexClient::probe(&url, auth) {
                    crate::index_client::ProbeOutcome::Found(client) => {
                        entry = entry.with_index_client(client);
                    }
                    // The index is there but refused us (401/403) —
                    // surface it, unlike the silent Absent fall-through,
                    // so a private index is not mistaken for a missing
                    // one.
                    crate::index_client::ProbeOutcome::Refused { reason } => {
                        tracing::warn!(
                            target: "vibe_registry::index_client",
                            "index for registry `{}` at `{url}` not used: {reason}",
                            reg.name
                        );
                    }
                    crate::index_client::ProbeOutcome::Absent => {}
                }
            }
            sources.push(RegistrySource::Git(Arc::new(entry)));
        }
        Ok(Self::new(
            sources,
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

    pub fn registries(&self) -> &[Arc<GitPerPackageRegistry>] {
        &self.registries
    }

    /// The ordered walk list — git and local-directory sources in declared
    /// `[[registry]]` order. The four core operations iterate this; the
    /// git-only operations use [`Self::registries`] (the git subset).
    pub fn sources(&self) -> &[RegistrySource] {
        &self.sources
    }

    /// Index-backed short-name candidate enumeration (PROP-008 §2.6).
    /// For each configured registry that exposes an index, fetch the
    /// `by-name/<name>.json` candidate set and union every `group`
    /// that publishes a package of this bare `name`. Registries
    /// without an index contribute nothing — a remote git host cannot
    /// be enumerated cheaply (PROP-005 §1), which is precisely why
    /// short-name resolution needs the index layer.
    ///
    /// A per-registry index error is logged and skipped, never
    /// propagated: one unreachable index must not block resolution
    /// against the others. The returned groups are de-duplicated and
    /// sorted; `len() > 1` is a short-name collision (PROP-008 §2.7),
    /// `len() == 0` means no index carried the name.
    pub fn resolve_name_candidates(&self, name: &str) -> Vec<Group> {
        let mut groups: Vec<Group> = Vec::new();
        for reg in &self.registries {
            let Some(client) = reg.index_client() else {
                continue;
            };
            match client.name_candidates(name) {
                Ok(found) => {
                    for g in found {
                        if !groups.contains(&g) {
                            groups.push(g);
                        }
                    }
                }
                Err(e) => {
                    tracing::debug!(
                        target: "vibe_registry::multi_registry_resolver",
                        package = %name,
                        error = %e,
                        "index short-name lookup failed; skipping this registry"
                    );
                }
            }
        }
        groups.sort();
        groups
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

/// Shared fixtures for this module's submodule tests — the canned
/// [`GitBackend`] fake plus section / resolver builders.
#[cfg(test)]
#[path = "test_support.rs"]
pub(crate) mod test_support;

#[cfg(test)]
#[path = "offline_tests.rs"]
mod offline_tests;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
