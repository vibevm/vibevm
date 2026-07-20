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

specmark::scope!("spec://vibevm/modules/vibe-registry/PROP-002#registry-model");

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use vibe_core::manifest::{
    GitPackageDep, Lockfile, Manifest, MirrorSection, OverrideSection, RedirectFile, RefPolicy,
    RegistrySection, parse_redirect_bytes,
};
use vibe_core::{Group, PackageKind, PackageRef, VersionSpec, url_is_local};

use crate::git_backend::{GitBackend, GitError, ShellGit};
use crate::git_package_registry::{GitPackageRegistry, copy_dir_excluding_git};
use crate::registry_cache::{DEFAULT_FRESHNESS_SECS, default_cache_root, strip_git_plus_prefix};
use crate::{
    CachedPackage, InPlaceMaterialised, LocalRegistry, RegistryError, ResolvedPackage,
    compute_content_hash,
};

mod attempt;
mod dispatch;
mod redirect_follow;
mod refresh;
mod source;
mod sources;
mod walk;

pub use attempt::{RegistryWalkAttempt, WalkAttemptStatus};
pub use refresh::{RefreshReport, RefreshedEntry, RefreshedVia, SkippedEntry};
pub(crate) use source::local_path_from_url;
pub use source::{LocalRegistrySource, RegistrySource};

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
    /// override-resolved and git-source entries.
    pub registry_name: Option<String>,
    /// What goes into lockfile `source_url`.
    pub source_url: String,
    /// What goes into lockfile `source_ref` — typically the version tag
    /// (`v0.3.0`) for registry resolutions, or the override's / git-source
    /// `tag`/`branch`/`rev` value.
    pub source_ref: Option<String>,
    pub overridden: bool,
    /// True when this package was resolved via a `[requires.packages]`
    /// git-source declaration (PROP-002 §2.4.1) rather than through
    /// the registry walk or `[[override]]`. Lockfile maps this to
    /// `source_kind = "git"`.
    pub is_git_source: bool,
    /// True when this package was resolved via a `[requires.packages]`
    /// path-source declaration (PROP-007 §2.5) — a package in a local
    /// directory, typically a sibling workspace member — rather than the
    /// registry walk, `[[override]]`, or git-source. Lockfile maps this
    /// to `source_kind = "path"`, and `source_url` then carries the
    /// member's path relative to the workspace root, not a URL.
    pub is_path_source: bool,
    /// When this package was resolved via a registry stub that
    /// redirected to an external URL (PROP-002 §2.4.2), the **stub**
    /// URL is recorded here while `source_url` carries the **target**
    /// URL. `None` for non-redirected resolutions.
    pub via_redirect: Option<String>,
    /// Auth regime declared in the redirect's `[redirect].auth`. Only
    /// meaningful when `via_redirect.is_some()`; for non-redirected
    /// resolutions the registry's own auth applies via `registry_name`
    /// → registry lookup. The fetch path uses this to synthesise a
    /// target-side `GitPackageRegistry` with the right auth without
    /// re-fetching the redirect marker.
    pub redirect_target_auth: vibe_core::manifest::AuthKind,
    /// Env-var name when `redirect_target_auth = TokenEnv`. `None`
    /// otherwise.
    pub redirect_target_token_env: Option<String>,
}

/// A `[requires.packages]` path-source declaration (PROP-007 §2.5) with
/// the on-disk location already computed by the caller. A path-source
/// dependency is a package living in a local directory — typically a
/// sibling workspace member — so there is no registry walk and no git
/// clone: the source is a directory the resolver reads and copies.
///
/// The resolver does **no** filesystem path arithmetic. The caller (the
/// workspace layer, a later milestone) resolves `PathPackageDep.path`
/// against the declaring manifest's directory, canonicalises it, and
/// hands the absolute `package_dir` plus the workspace-relative
/// `workspace_rel` in already-computed. The resolver just consumes them.
#[derive(Debug, Clone)]
pub struct ResolvedPathDep {
    /// Optional `kind` prefix carried by the pkgref key (PROP-008 §2.4).
    /// Metadata only — never used to resolve; `(group, name)` is identity.
    pub kind: Option<PackageKind>,
    /// Reverse-FQDN group — a manifest pkgref is always qualified.
    pub group: Group,
    pub name: String,
    /// Optional dual-form version constraint from `{ path, version }`.
    /// When present, the package's own `[package].version` must satisfy
    /// it; mismatch is a hard error — same shape as the git-source
    /// version check.
    pub version: Option<VersionSpec>,
    /// Absolute directory where the dependency package lives. The caller
    /// resolves `PathPackageDep.path` against the declaring manifest's
    /// directory and canonicalises it; the resolver just consumes it.
    pub package_dir: PathBuf,
    /// `package_dir` relative to the workspace absolute root,
    /// forward-slashed. Recorded verbatim as the lockfile `source_url`
    /// for this entry — a portable relative path, never a URL, never
    /// absolute.
    pub workspace_rel: String,
}

/// Resolver coordinating an ordered set of [`GitPackageRegistry`]
/// instances plus the cross-cutting `[[mirror]]` and `[[override]]`
/// layers from `vibe.toml`.
pub struct MultiRegistryResolver {
    registries: Vec<Arc<GitPackageRegistry>>,
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

    /// Whether the resolver is in strict-auth mode. Tests + the
    /// CLI surface read this to confirm the toggle flowed through.
    pub fn strict_auth(&self) -> bool {
        self.strict_auth
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
        let mut sources: Vec<RegistrySource> = Vec::with_capacity(registries.len());
        for reg in registries {
            // PROP-002 §2.2.3 #enabled: a disabled registry is skipped
            // entirely — never built into the resolver, so no path (install /
            // outdated / search / sync) consults it. Flip `enabled` back to
            // re-activate; no re-add needed.
            if !reg.enabled {
                continue;
            }
            // A local `[[registry]]` url (`file://` / bare path, per
            // `url_is_local`) — but NOT a `git+` transport (`git+file://` is a
            // local *git* repo to clone, not a plain directory) — is served
            // straight off the filesystem by `LocalRegistry`, never
            // git-cloned, so a plain on-disk directory works as a registry
            // (PROP-002 §2.2.2). `url_is_local` still classifies `git+file://`
            // as local for the `--offline` filter (no network); the `git+`
            // guard here keeps it on the git-clone backend. `naming` / `auth`
            // / `mirrors` / `index_client` are git-only knobs and do not apply
            // (LocalRegistry reads `<root>/<group>/<name>/v<version>/` directly).
            let is_git_transport = reg.url.trim().to_ascii_lowercase().starts_with("git+");
            if url_is_local(&reg.url) && !is_git_transport {
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
            let mut entry = GitPackageRegistry::open_with_auth(
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
            if let Some(url) = crate::index_client::index_url_for(&reg.name)
                && let Some(client) = crate::index_client::IndexClient::probe(&url)
            {
                entry = entry.with_index_client(client);
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

    pub fn registries(&self) -> &[Arc<GitPackageRegistry>] {
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
#[path = "tests.rs"]
mod tests;
