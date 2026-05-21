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
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use vibe_core::manifest::{
    GitPackageDep, Lockfile, Manifest, MirrorSection, OverrideSection, RedirectFile, RefPolicy,
    RegistrySection, parse_redirect_bytes,
};
use vibe_core::{PackageRef, VersionSpec};

use crate::git_backend::{GitBackend, GitError, ShellGit};
use crate::git_package_registry::{GitPackageRegistry, copy_dir_excluding_git};
use crate::git_registry::{DEFAULT_FRESHNESS_SECS, default_cache_root, strip_git_plus_prefix};
use crate::{
    CachedPackage, RegistryError, ResolvedPackage, compute_content_hash,
};
use vibe_core::PackageKind;

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
    pub kind: PackageKind,
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
    mirrors: Vec<MirrorSection>,
    overrides: HashMap<String, OverrideSection>,
    /// Git-source declarations from `[requires.packages]` table-form
    /// (PROP-002 §2.4.1), keyed by `<kind>:<name>`. Resolution order
    /// (resolve()): override > path-source > git-source > registry-walk.
    git_packages: HashMap<String, GitPackageDep>,
    /// Path-source declarations from `[requires.packages]` table-form
    /// (PROP-007 §2.5), keyed by `<kind>:<name>`. Sits one notch above
    /// git-source in the resolution order — a pkgref present here wins
    /// over a same-pkgref git-source declaration.
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

/// One row in the aggregated "tried these registries" report
/// surfaced via [`RegistryError::PackageNotFoundEverywhere`].
/// Captured per-registry during the walk in
/// [`MultiRegistryResolver::resolve`]; carried through the
/// `DepProvider` error chain into `vibe-cli`'s install-error
/// JSON envelope so machine-readable consumers can branch on the
/// per-registry status without parsing prose.
#[derive(Debug, Clone, serde::Serialize)]
pub struct RegistryWalkAttempt {
    pub name: String,
    pub url: String,
    pub auth: vibe_core::manifest::AuthKind,
    pub status: WalkAttemptStatus,
}

#[derive(Debug, Clone, Copy, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum WalkAttemptStatus {
    /// Registry's `resolve` returned `UnknownPackage` — the
    /// registry was reachable, manifest parsed, just no version
    /// matching the pkgref.
    NotFound,
    /// Registry returned 401 / 403 (`AuthFailed`) but was declared
    /// `auth = "none"` and `strict_auth` was off, so the resolver
    /// reclassified the failure as "no public answer here" and
    /// walked past. The line below tells the operator the host
    /// would need credentials if they want to access this
    /// registry as authenticated.
    Public401,
}

impl WalkAttemptStatus {
    pub fn as_label(&self) -> &'static str {
        match self {
            WalkAttemptStatus::NotFound => "not found",
            WalkAttemptStatus::Public401 => "access denied (401, walked past — auth=none)",
        }
    }
}

fn format_walk_attempts(attempts: &[RegistryWalkAttempt]) -> String {
    // Compute the column width for the registry-name column so
    // the rendered table stays aligned regardless of label
    // length. URLs are too varied to align — they wrap to the
    // right of the arrow.
    let name_width = attempts
        .iter()
        .map(|a| a.name.len())
        .max()
        .unwrap_or(0);
    let url_width = attempts
        .iter()
        .map(|a| a.url.len())
        .max()
        .unwrap_or(0);
    let mut out = String::new();
    for a in attempts {
        // Indent each line with two spaces so the report nests
        // visually under the parent error's "Tried:" label.
        let _ = writeln!(
            out,
            "  - {:<name_width$}  ({:<url_width$})  → {} (auth={})",
            a.name,
            a.url,
            a.status.as_label(),
            a.auth.as_str(),
            name_width = name_width,
            url_width = url_width,
        );
    }
    // Hint at the bottom — the most common operator next step
    // when nothing was found anywhere.
    if attempts.iter().any(|a| matches!(a.status, WalkAttemptStatus::Public401)) {
        out.push_str(
            "\nHint: at least one registry returned 401 / 403 and was walked past as `auth=none`.\n\
             If that registry is actually private, set `auth = \"token-env\"` and provide the\n\
             token via `VIBEVM_REGISTRY_TOKEN_<HOST>`; see docs/registry-auth.md.",
        );
    }
    out
}

/// Probe the `(kind, name)` slot in `reg` at `tag` for a
/// `vibe-redirect.toml` marker. Returns `Some(parsed)` when the
/// marker exists; `None` when the file is absent (the common case
/// — non-stub package). Surfaces parse errors and other I/O errors
/// directly. Cheap: one extra `git archive` call per registry-walk
/// success.
fn try_fetch_redirect(
    backend: &Arc<dyn GitBackend>,
    reg: &GitPackageRegistry,
    resolved: &ResolvedPackage,
    tag: &str,
) -> Result<Option<RedirectFile>, RegistryError> {
    try_fetch_redirect_for_url(backend, reg, resolved.kind, &resolved.name, tag)
}

/// Lower-level form of `try_fetch_redirect`: take an already-built
/// `GitPackageRegistry` plus `(kind, name, ref)` and probe its repo
/// for a `vibe-redirect.toml`. Used both for the initial probe at
/// the stub layer and the hop-limit check at the target.
///
/// Two-path read shape — same idea as `fetch_dep_manifest`:
///
/// 1. `git archive --remote=<url> <ref> -- vibe-redirect.toml` is the
///    cheap, no-clone read. Works against `file://` and the handful
///    of hosts that expose `upload-archive`.
/// 2. When the host refuses `upload-archive` (GitHub, by design) the
///    archive call returns `ArchiveUnsupported`. Fall back to a
///    shallow clone of the repo at `refname` and read the file from
///    the working tree. The clone directory is the same the install
///    pipeline would use later, so this is also pre-warming.
///
/// Returns `Ok(None)` when neither path finds the marker — the common
/// "non-stub package" case where `vibe-redirect.toml` simply isn't
/// part of the package payload.
fn try_fetch_redirect_for_url(
    backend: &Arc<dyn GitBackend>,
    reg: &GitPackageRegistry,
    kind: vibe_core::PackageKind,
    name: &str,
    refname: &str,
) -> Result<Option<RedirectFile>, RegistryError> {
    let plain_url = reg.package_repo_url(kind, name);
    let fetch_url = reg.credentialed_url(&plain_url);
    let bytes = match backend.fetch_file_at_ref(
        strip_git_plus_prefix(&fetch_url),
        refname,
        RedirectFile::FILENAME,
    ) {
        Ok(b) => b,
        Err(crate::git_backend::GitError::FileNotFoundInRef { .. }) => return Ok(None),
        Err(crate::git_backend::GitError::ArchiveUnsupported { .. }) => {
            // Host refuses `upload-archive` — the GitHub case. Fall
            // back to a shallow clone and read the marker from the
            // clone's working tree. `refresh_package` reuses the
            // existing per-package clone if present; for a fresh
            // bucket it bootstraps. The token-discipline (M1.14)
            // applies — if the registry has `auth = "token-env"`,
            // the clone uses the credentialed URL and immediately
            // scrubs `.git/config` after bootstrap.
            reg.refresh_package(kind, name, refname)?;
            let marker_path = reg
                .package_clone_dir(kind, name)
                .join(RedirectFile::FILENAME);
            if !marker_path.exists() {
                return Ok(None);
            }
            std::fs::read(&marker_path).map_err(|source| RegistryError::Io {
                path: marker_path,
                source,
            })?
        }
        Err(other) => return Err(other.into()),
    };
    let r = parse_redirect_bytes(&bytes).map_err(|e| RegistryError::MalformedMeta {
        path: PathBuf::from(format!(
            "{plain_url}@{refname}:{}",
            RedirectFile::FILENAME
        )),
        reason: e.to_string(),
    })?;
    Ok(Some(r))
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
    pub fn with_git_packages(mut self, deps: Vec<GitPackageDep>) -> Self {
        self.git_packages = deps
            .into_iter()
            .map(|d| (format!("{}:{}", d.kind, d.name), d))
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
            .map(|d| (format!("{}:{}", d.kind, d.name), d))
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
            let mut entry = GitPackageRegistry::open_with_auth(
                &reg.name,
                &reg.url,
                &reg.r#ref,
                reg.naming,
                mirror_urls,
                &cache_root,
                Arc::clone(&backend),
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

        // Step 1.25: path-source short-circuit (PROP-007 §2.5).
        // `[requires.packages]` table-form may declare a dep as
        // `{ path = "..." }`; the package lives in a local directory
        // (typically a sibling workspace member). Path-source sits one
        // notch above git-source — a pkgref present in both sets
        // resolves via path-source. No registry walk, no git clone.
        if let Some(dep) = self.path_packages.get(&pkgref.qualified_name()) {
            return self.resolve_path_source(pkgref, dep);
        }

        // Step 1.5: git-source short-circuit (PROP-002 §2.4.1).
        // `[requires.packages]` table-form may declare a dep as
        // `{ git = "...", tag/branch/rev = "..." }`; the resolver
        // bypasses the `[[registry]]` walk for that pkgref entirely
        // and fetches directly from the declared URL.
        if let Some(dep) = self.git_packages.get(&pkgref.qualified_name()) {
            return self.resolve_git_source(pkgref, dep);
        }

        // Step 2: priority-ordered registry walk. PROP-002 §2.3.1
        // failure-mode discriminator:
        //
        // - `UnknownPackage` → fall through to next registry.
        // - `Git(AuthFailed)` on an `auth = "none"` registry →
        //   reclassify as `UnknownPackage` and fall through. (For
        //   public registries 401 / 403 means "no public answer
        //   here", e.g. GitVerse's policy on missing repos.)
        // - `Git(AuthFailed)` on an authenticated registry
        //   (`token-env`, `credential-helper`) → halt with the
        //   error — the operator declared this registry expects
        //   credentials, the credentials presented were rejected,
        //   the operator must see that.
        // - `MissingToken` on any registry → halt — the manifest
        //   declared `auth = "token-env"` but the env-var is unset;
        //   the operator must fix the env. (Walking past this would
        //   silently downgrade a private registry to "not present"
        //   which would mask configuration errors.)
        // - any other error → halt as before (network, malformed
        //   manifest, server error, ...).
        let mut attempts: Vec<RegistryWalkAttempt> = Vec::new();
        for reg in &self.registries {
            match reg.resolve(pkgref) {
                Ok(resolved) => {
                    let stub_tag = format!("v{}", resolved.version);
                    // Step 2a: redirect probe (PROP-002 §2.4.2). The
                    // registry served a tag; check whether the repo
                    // at that tag is a stub pointing elsewhere. The
                    // probe is one extra `git archive` call, only
                    // when the registry-walk leg succeeded; cheap.
                    if let Some(redirect) =
                        try_fetch_redirect(&self.backend, reg, &resolved, &stub_tag)?
                    {
                        return self.follow_redirect(
                            pkgref, &resolved, reg, &redirect, &stub_tag,
                        );
                    }
                    let url = reg.package_repo_url(resolved.kind, &resolved.name);
                    return Ok(MultiResolution {
                        resolved,
                        registry_name: Some(reg.name().to_string()),
                        source_url: url,
                        source_ref: Some(stub_tag),
                        overridden: false,
                        is_git_source: false,
                        is_path_source: false,
                        via_redirect: None,
                        redirect_target_auth: vibe_core::manifest::AuthKind::None,
                        redirect_target_token_env: None,
                    });
                }
                Err(RegistryError::UnknownPackage { .. }) => {
                    attempts.push(RegistryWalkAttempt {
                        name: reg.name().to_string(),
                        url: reg.org_url().to_string(),
                        auth: reg.auth_kind(),
                        status: WalkAttemptStatus::NotFound,
                    });
                    continue;
                }
                Err(RegistryError::Git(crate::git_backend::GitError::AuthFailed { .. }))
                    if matches!(
                        reg.auth_kind(),
                        vibe_core::manifest::AuthKind::None
                    ) && !self.strict_auth =>
                {
                    tracing::debug!(
                        target: "vibe_registry::resolve",
                        registry = %reg.name(),
                        "auth_failed on auth=none registry treated as unknown-package; walking"
                    );
                    attempts.push(RegistryWalkAttempt {
                        name: reg.name().to_string(),
                        url: reg.org_url().to_string(),
                        auth: reg.auth_kind(),
                        status: WalkAttemptStatus::Public401,
                    });
                    continue;
                }
                Err(other) => return Err(other),
            }
        }

        // No registry had a satisfying answer. Two shapes:
        //
        // - If we walked at least one registry, surface the
        //   aggregate per-registry status so the operator sees
        //   exactly what happened where (PackageNotFoundEverywhere).
        // - Otherwise (no `[[registry]]` configured) fall back to
        //   the simpler UnknownPackage for back-compat with
        //   downstream consumers that match on it specifically.
        if attempts.is_empty() {
            return Err(RegistryError::UnknownPackage {
                kind: pkgref.kind,
                name: pkgref.name.clone(),
            });
        }
        let summary = format_walk_attempts(&attempts);
        Err(RegistryError::PackageNotFoundEverywhere {
            kind: pkgref.kind,
            name: pkgref.name.clone(),
            summary,
            attempts,
        })
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
        let meta = manifest
            .require_package()
            .map_err(|e| RegistryError::MalformedMeta {
                path: PathBuf::from(format!("{}@{}:vibe.toml", ovr.source_url, refname)),
                reason: e.to_string(),
            })?;
        // Sanity: the override is supposed to point at *this* package. If
        // the manifest at the pinned ref names a different (kind, name),
        // installing it would silently misroute on disk. Refuse loudly.
        if meta.kind != pkgref.kind || meta.name != pkgref.name {
            return Err(RegistryError::MalformedMeta {
                path: PathBuf::from(format!("{}@{}:vibe.toml", ovr.source_url, refname)),
                reason: format!(
                    "override for `{}:{}` points at a manifest declaring `{}:{}` — refusing to install",
                    pkgref.kind, pkgref.name, meta.kind, meta.name
                ),
            });
        }
        let resolved = ResolvedPackage {
            kind: pkgref.kind,
            name: pkgref.name.clone(),
            version: meta.version.clone(),
            source_dir: self.override_clone_dir(pkgref.kind, &pkgref.name),
        };
        Ok(MultiResolution {
            resolved,
            registry_name: None,
            source_url: ovr.source_url.clone(),
            source_ref: Some(refname),
            overridden: true,
            is_git_source: false,
            is_path_source: false,
            via_redirect: None,
            redirect_target_auth: vibe_core::manifest::AuthKind::None,
            redirect_target_token_env: None,
        })
    }

    /// Follow a `vibe-redirect.toml` marker found in a registry stub
    /// repo (PROP-002 §2.4.2). The stub registry served a tag; we
    /// re-resolve against the redirect's `target_url` at the
    /// pass-through-tag (default — same tag as the stub) or the
    /// `pinned_ref` (when `ref_policy = "pinned"`). Hop limit = 1:
    /// if the target is itself a stub, raise
    /// `RedirectChainNotAllowed`.
    fn follow_redirect(
        &self,
        pkgref: &PackageRef,
        stub_resolved: &ResolvedPackage,
        stub_reg: &GitPackageRegistry,
        redirect: &RedirectFile,
        stub_tag: &str,
    ) -> Result<MultiResolution, RegistryError> {
        let target_url = redirect.redirect.target_url.clone();
        let target_ref = match redirect.redirect.ref_policy {
            RefPolicy::PassThroughTag => stub_tag.to_string(),
            RefPolicy::Pinned => redirect
                .redirect
                .pinned_ref
                .clone()
                .expect("RedirectSection parser guarantees pinned_ref when ref_policy=pinned"),
        };
        let synthetic_name = format!("redirect-target-{}-{}", pkgref.kind, pkgref.name);
        let target_reg = GitPackageRegistry::open_single_package(
            &synthetic_name,
            &target_url,
            &target_ref,
            &self.cache_root,
            Arc::clone(&self.backend),
            DEFAULT_FRESHNESS_SECS,
            redirect.redirect.auth,
            redirect.redirect.token_env.as_deref(),
        )?;
        // Hop limit = 1: target cannot itself be a stub. Probe
        // `vibe-redirect.toml` at the target ref BEFORE attempting to
        // read the target's `vibe.toml` — a stub-only repo
        // carries only the marker, so reading the manifest first
        // would return `FileNotFoundInRef` and the chain detection
        // would never fire. Marker-first preserves the policy contract
        // independent of what the target's content shape happens to
        // be.
        let target_redirect = try_fetch_redirect_for_url(
            &self.backend,
            &target_reg,
            stub_resolved.kind,
            &stub_resolved.name,
            &target_ref,
        )?;
        if target_redirect.is_some() {
            return Err(RegistryError::MalformedMeta {
                path: PathBuf::from(format!(
                    "{target_url}@{target_ref}:{}",
                    RedirectFile::FILENAME
                )),
                reason: format!(
                    "redirect chain not allowed: stub `{}` redirects to `{target_url}` which is itself a stub (hop limit = 1, PROP-002 §2.4.2)",
                    stub_reg.package_repo_url(stub_resolved.kind, &stub_resolved.name),
                ),
            });
        }
        let target_manifest =
            target_reg.fetch_manifest_at_ref(pkgref.kind, &pkgref.name, &target_ref)?;
        let target_meta =
            target_manifest
                .require_package()
                .map_err(|e| RegistryError::MalformedMeta {
                    path: PathBuf::from(format!(
                        "{target_url}@{target_ref}:{}",
                        Manifest::FILENAME
                    )),
                    reason: e.to_string(),
                })?;
        // Sanity: the target's `[package]` must declare the same
        // `(kind, name)` that the consumer asked for. Mismatch =
        // org owner pointed at the wrong target, refuse to install.
        if target_meta.kind != pkgref.kind || target_meta.name != pkgref.name {
            return Err(RegistryError::MalformedMeta {
                path: PathBuf::from(format!(
                    "{target_url}@{target_ref}:{}",
                    Manifest::FILENAME
                )),
                reason: format!(
                    "redirect target for `{}:{}` declares `{}:{}` — refusing to install",
                    pkgref.kind, pkgref.name, target_meta.kind, target_meta.name
                ),
            });
        }
        let stub_url = stub_reg.package_repo_url(stub_resolved.kind, &stub_resolved.name);
        let resolved = ResolvedPackage {
            kind: pkgref.kind,
            name: pkgref.name.clone(),
            version: target_meta.version.clone(),
            source_dir: self.redirect_clone_dir(pkgref.kind, &pkgref.name),
        };
        Ok(MultiResolution {
            resolved,
            // Registry name from the stub layer — that's the surface
            // the consumer's `vibe.toml` `[[registry]]` named.
            registry_name: Some(stub_reg.name().to_string()),
            source_url: target_url,
            source_ref: Some(target_ref),
            overridden: false,
            is_git_source: false,
            is_path_source: false,
            via_redirect: Some(stub_url),
            redirect_target_auth: redirect.redirect.auth,
            redirect_target_token_env: redirect.redirect.token_env.clone(),
        })
    }

    /// Where redirect-followed clones live —
    /// `<cache_root>/__redirects__/<kind>-<name>/clone/`. Distinct
    /// from registry-served, override, and git-source clones so
    /// re-resolutions across modes do not share state.
    fn redirect_clone_dir(&self, kind: vibe_core::PackageKind, name: &str) -> PathBuf {
        self.cache_root
            .join("__redirects__")
            .join(format!("{}-{}", kind.as_str(), name))
            .join("clone")
    }

    /// Read `vibe.toml` for a resolved `(kind, name, version)`,
    /// transparently following any registry-redirect stub (PROP-002
    /// §2.4.2) or git-source declaration (§2.4.1). The depsolver's
    /// [`DepProvider::fetch_manifest`] adapter uses this so a
    /// stub-served pkgref returns the **target's** manifest (the stub
    /// itself carries only `vibe-redirect.toml`) and a git-source
    /// pkgref returns the manifest at the declared `tag`/`branch`/`rev`.
    ///
    /// The implementation re-runs [`Self::resolve`] with the version
    /// constraint pinned to `=<version>` so it converges on the same
    /// `MultiResolution` the install pipeline already saw, then reads
    /// the manifest from whichever URL the resolution recorded —
    /// stub's target for redirects, declared URL for git-source,
    /// the registry's own URL otherwise. Walking registries directly
    /// (the pre-M1.16 shape) cannot serve a stub-only repo.
    pub fn fetch_manifest(
        &self,
        kind: PackageKind,
        name: &str,
        version: &semver::Version,
    ) -> Result<Manifest, RegistryError> {
        // Build a pinned pkgref so `resolve` converges on the exact
        // slot the install pipeline committed to (the depsolver pinned
        // the version via `resolve_version` first). For pass-through
        // redirects (and direct registry installs) the stub's tag list
        // contains `v<version>` and the pinned resolve hits it
        // immediately. For pinned-policy redirects the stub may have
        // unrelated tags (the pinned semantic — every consumer goes
        // to the target's pinned ref, so the stub tag is irrelevant);
        // we fall back to a constraint-free resolve and verify the
        // resolved version still matches.
        let pinned_pkgref = PackageRef::parse(&format!("{kind}:{name}@={version}")).map_err(
            |e| RegistryError::MalformedMeta {
                path: PathBuf::from("<synthetic-pkgref>"),
                reason: format!("constructing pinned pkgref: {e}"),
            },
        )?;
        let resolution = match self.resolve(&pinned_pkgref) {
            Ok(r) => r,
            Err(RegistryError::NoMatchingVersion { .. })
            | Err(RegistryError::PackageNotFoundEverywhere { .. })
            | Err(RegistryError::UnknownPackage { .. }) => {
                // The stub's tag list does not contain `=version` —
                // happens with pinned-policy redirects where the
                // stub-side tag and the target version are decoupled.
                // Re-resolve without a constraint and accept the
                // result as long as the version it produces matches
                // what the depsolver pinned.
                let fallback_pkgref =
                    PackageRef::parse(&format!("{kind}:{name}")).map_err(|e| {
                        RegistryError::MalformedMeta {
                            path: PathBuf::from("<synthetic-pkgref>"),
                            reason: format!("constructing latest pkgref: {e}"),
                        }
                    })?;
                let r = self.resolve(&fallback_pkgref)?;
                if &r.resolved.version != version {
                    return Err(RegistryError::NoMatchingVersion {
                        kind,
                        name: name.to_string(),
                        req: format!("={version}"),
                    });
                }
                r
            }
            Err(other) => return Err(other),
        };

        if resolution.is_path_source {
            // Path-source: the package lives in a local directory.
            // `path_packages` carries the resolver-side `package_dir`
            // (already canonicalised by the workspace layer); read
            // `vibe.toml` straight off disk so transitive dependencies
            // of a path-source package resolve.
            let dep = self
                .path_packages
                .get(&pinned_pkgref.qualified_name())
                .ok_or_else(|| RegistryError::UnknownPackage {
                    kind,
                    name: name.to_string(),
                })?;
            let manifest_path = dep.package_dir.join(Manifest::FILENAME);
            return Manifest::read(&manifest_path).map_err(RegistryError::from);
        }

        if resolution.via_redirect.is_some() {
            // Redirect-resolved: target_url is in source_url, target_ref
            // is in source_ref. Open a synthetic single-package
            // registry on the target and read the manifest at the
            // recorded ref. Auth carries the redirect's declared
            // policy so private targets keep working.
            let target_url = resolution.source_url.clone();
            let target_ref = resolution.source_ref.clone().unwrap_or_default();
            let synthetic_name = format!("redirect-target-{kind}-{name}");
            let target_reg = GitPackageRegistry::open_single_package(
                &synthetic_name,
                &target_url,
                &target_ref,
                &self.cache_root,
                Arc::clone(&self.backend),
                DEFAULT_FRESHNESS_SECS,
                resolution.redirect_target_auth,
                resolution.redirect_target_token_env.as_deref(),
            )?;
            return target_reg.fetch_manifest_at_ref(kind, name, &target_ref);
        }

        if resolution.is_git_source {
            // Git-source: source_url + source_ref carry the operator-
            // declared `tag`/`branch`/`rev`. Construct the same
            // synthetic registry the resolver used and re-read the
            // manifest at that ref.
            //
            // Note: `git_packages` lookup gives us the original
            // `auth` / `token_env`; the resolver did the lookup at
            // `resolve` time and stored the values there too.
            let dep = self
                .git_packages
                .get(&pinned_pkgref.qualified_name())
                .ok_or_else(|| RegistryError::UnknownPackage {
                    kind,
                    name: name.to_string(),
                })?;
            let source_ref = resolution
                .source_ref
                .clone()
                .unwrap_or_else(|| dep.ref_kind.as_str().to_string());
            let synthetic_name = format!("git-source-{kind}-{name}");
            let reg = GitPackageRegistry::open_single_package(
                &synthetic_name,
                &dep.url,
                &source_ref,
                &self.cache_root,
                Arc::clone(&self.backend),
                DEFAULT_FRESHNESS_SECS,
                dep.auth,
                dep.token_env.as_deref(),
            )?;
            return reg.fetch_manifest_at_ref(kind, name, &source_ref);
        }

        // Override or registry: walk in priority order, preferring the
        // registry the resolver picked. Override-served packages have
        // `registry_name = None`, so we just walk and the first match
        // wins (overrides are not consulted by `fetch_dep_manifest` —
        // those are handled by the install pipeline directly).
        if let Some(name_filter) = &resolution.registry_name
            && let Some(reg) = self
                .registries
                .iter()
                .find(|r| r.name() == name_filter.as_str())
        {
            return reg.fetch_dep_manifest(kind, name, version);
        }
        let mut last_err: Option<RegistryError> = None;
        for reg in &self.registries {
            match reg.fetch_dep_manifest(kind, name, version) {
                Ok(m) => return Ok(m),
                Err(err)
                    if matches!(
                        err,
                        RegistryError::Git(GitError::FileNotFoundInRef { .. })
                            | RegistryError::Git(GitError::ArchiveUnsupported { .. })
                            | RegistryError::Io { .. }
                            | RegistryError::MalformedMeta { .. }
                            | RegistryError::UnknownPackage { .. }
                            | RegistryError::NoMatchingVersion { .. }
                    ) =>
                {
                    last_err = Some(err);
                    continue;
                }
                Err(other) => return Err(other),
            }
        }
        Err(last_err.unwrap_or(RegistryError::UnknownPackage {
            kind,
            name: name.to_string(),
        }))
    }

    /// Resolve a `[requires.packages]` git-source declaration
    /// (PROP-002 §2.4.1). Synthesises a single-package
    /// `GitPackageRegistry` pointing at `dep.url`, fetches
    /// `vibe.toml` at the declared `tag`/`branch`/`rev`,
    /// verifies `(kind, name)` matches and the optional `version`
    /// constraint is satisfied, returns a `MultiResolution` with
    /// `is_git_source = true`.
    fn resolve_git_source(
        &self,
        pkgref: &PackageRef,
        dep: &GitPackageDep,
    ) -> Result<MultiResolution, RegistryError> {
        let synthetic_name = format!("git-source-{}-{}", dep.kind, dep.name);
        let refname = dep.ref_kind.as_str().to_string();
        let reg = GitPackageRegistry::open_single_package(
            &synthetic_name,
            &dep.url,
            &refname,
            &self.cache_root,
            Arc::clone(&self.backend),
            DEFAULT_FRESHNESS_SECS,
            dep.auth,
            dep.token_env.as_deref(),
        )?;
        let manifest = reg.fetch_manifest_at_ref(dep.kind, &dep.name, &refname)?;
        let meta = manifest
            .require_package()
            .map_err(|e| RegistryError::MalformedMeta {
                path: PathBuf::from(format!("{}@{}:{}", dep.url, refname, Manifest::FILENAME)),
                reason: e.to_string(),
            })?;
        // Sanity: the declaration says (kind, name) but the repo's
        // manifest declares some other identity. Refuse to install —
        // pulling code under a misnamed slot would silently misroute
        // on disk and confuse downstream commands.
        if meta.kind != pkgref.kind || meta.name != pkgref.name {
            return Err(RegistryError::MalformedMeta {
                path: PathBuf::from(format!("{}@{}:{}", dep.url, refname, Manifest::FILENAME)),
                reason: format!(
                    "git-source `{}:{}` points at a manifest declaring `{}:{}` — refusing to install",
                    pkgref.kind, pkgref.name, meta.kind, meta.name
                ),
            });
        }
        // Verify the optional version constraint, if the operator declared one.
        if let Some(spec) = &dep.version
            && !spec.matches(&meta.version)
        {
            return Err(RegistryError::MalformedMeta {
                path: PathBuf::from(format!("{}@{}:{}", dep.url, refname, Manifest::FILENAME)),
                reason: format!(
                    "git-source `{}:{}@{}` declares version `{}`, which does not satisfy the constraint `{}`",
                    pkgref.kind, pkgref.name, refname, meta.version, spec
                ),
            });
        }
        let resolved = ResolvedPackage {
            kind: pkgref.kind,
            name: pkgref.name.clone(),
            version: meta.version.clone(),
            source_dir: self.git_source_clone_dir(pkgref.kind, &pkgref.name),
        };
        Ok(MultiResolution {
            resolved,
            registry_name: None,
            source_url: dep.url.clone(),
            source_ref: Some(refname),
            overridden: false,
            is_git_source: true,
            is_path_source: false,
            via_redirect: None,
            redirect_target_auth: vibe_core::manifest::AuthKind::None,
            redirect_target_token_env: None,
        })
    }

    /// Resolve a `[requires.packages]` path-source declaration
    /// (PROP-007 §2.5). The package lives in a local directory
    /// (`dep.package_dir`, already canonicalised by the workspace
    /// layer); there is no registry walk and no git clone. Reads the
    /// package's `vibe.toml`, verifies `(kind, name)` matches and the
    /// optional `version` constraint is satisfied, returns a
    /// `MultiResolution` with `is_path_source = true` and the source
    /// recorded as the workspace-relative path (`dep.workspace_rel`).
    fn resolve_path_source(
        &self,
        pkgref: &PackageRef,
        dep: &ResolvedPathDep,
    ) -> Result<MultiResolution, RegistryError> {
        let manifest_path = dep.package_dir.join(Manifest::FILENAME);
        let manifest = Manifest::read(&manifest_path)?;
        let meta = manifest
            .require_package()
            .map_err(|e| RegistryError::MalformedMeta {
                path: manifest_path.clone(),
                reason: e.to_string(),
            })?;
        // Sanity: the declaration says (kind, name) but the package's
        // own manifest declares some other identity. Refuse to install —
        // pulling code under a misnamed slot would silently misroute
        // on disk and confuse downstream commands.
        if meta.kind != pkgref.kind || meta.name != pkgref.name {
            return Err(RegistryError::MalformedMeta {
                path: manifest_path.clone(),
                reason: format!(
                    "path-source `{}:{}` points at a manifest declaring `{}:{}` — refusing to install",
                    pkgref.kind, pkgref.name, meta.kind, meta.name
                ),
            });
        }
        // Verify the optional version constraint, if the path-dep
        // carried the dual-form `{ path, version }`. The resolved
        // version is the package's own `[package].version`.
        if let Some(spec) = &dep.version
            && !spec.matches(&meta.version)
        {
            return Err(RegistryError::MalformedMeta {
                path: manifest_path.clone(),
                reason: format!(
                    "path-source `{}:{}` at `{}` declares version `{}`, which does not satisfy the constraint `{}`",
                    pkgref.kind, pkgref.name, dep.workspace_rel, meta.version, spec
                ),
            });
        }
        let resolved = ResolvedPackage {
            kind: pkgref.kind,
            name: pkgref.name.clone(),
            version: meta.version.clone(),
            source_dir: dep.package_dir.clone(),
        };
        Ok(MultiResolution {
            resolved,
            registry_name: None,
            // `source_url` records the workspace-relative path, never an
            // absolute path and never a URL — PROP-007 §2.5.
            source_url: dep.workspace_rel.clone(),
            source_ref: None,
            overridden: false,
            is_git_source: false,
            is_path_source: true,
            via_redirect: None,
            redirect_target_auth: vibe_core::manifest::AuthKind::None,
            redirect_target_token_env: None,
        })
    }

    /// Where git-source clones live —
    /// `<cache_root>/__git_sources__/<kind>-<name>/clone/`. Distinct
    /// from registry-served clones and from override clones so a
    /// package that flips between resolution modes does not share
    /// state across modes.
    fn git_source_clone_dir(&self, kind: vibe_core::PackageKind, name: &str) -> PathBuf {
        self.cache_root
            .join("__git_sources__")
            .join(format!("{}-{}", kind, name))
            .join("clone")
    }

    fn read_override_manifest(
        &self,
        url: &str,
        refname: &str,
    ) -> Result<Manifest, RegistryError> {
        let bytes = self.backend.fetch_file_at_ref(
            strip_git_plus_prefix(url),
            refname,
            Manifest::FILENAME,
        )?;
        let text = String::from_utf8(bytes).map_err(|e| RegistryError::MalformedMeta {
            path: PathBuf::from(format!("{url}@{refname}:{}", Manifest::FILENAME)),
            reason: format!("invalid UTF-8: {e}"),
        })?;
        Manifest::parse_str(&text).map_err(|e| RegistryError::MalformedMeta {
            path: PathBuf::from(format!("{url}@{refname}:{}", Manifest::FILENAME)),
            reason: e.to_string(),
        })
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
    /// returned — its `content_hash` differs from `expected_hash`, so the
    /// caller can compare the two to detect drift against the lockfile pin.
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
        if resolution.is_path_source {
            return self.fetch_path_source(resolution, project_cache);
        }
        if resolution.is_git_source {
            return self.fetch_git_source(resolution, project_cache, expected_hash);
        }
        if resolution.via_redirect.is_some() {
            return self.fetch_via_redirect(resolution, project_cache, expected_hash);
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

        let manifest_path = dest.join(Manifest::FILENAME);
        let manifest = Manifest::read(&manifest_path)?;
        if manifest.package.is_none() {
            return Err(RegistryError::MalformedMeta {
                path: manifest_path.clone(),
                reason: "registry package manifest must carry a [package] table".to_string(),
            });
        }
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
            is_git_source: false,
            is_path_source: false,
            via_redirect: None,
        })
    }

    /// Fetch a redirect-resolved package — the target's content lives at
    /// `resolution.source_url`, fetched with auth from
    /// `resolution.redirect_target_auth` / `redirect_target_token_env`.
    /// The stub URL is preserved as `via_redirect` on the lockfile entry
    /// for diagnostic / auditing.
    fn fetch_via_redirect(
        &self,
        resolution: &MultiResolution,
        project_cache: &Path,
        _expected_hash: Option<&str>,
    ) -> Result<CachedPackage, RegistryError> {
        let kind = resolution.resolved.kind;
        let name = resolution.resolved.name.as_str();
        let target_url = resolution.source_url.clone();
        let refname = resolution
            .source_ref
            .clone()
            .ok_or_else(|| RegistryError::MalformedMeta {
                path: PathBuf::from(format!("{target_url}:{}", Manifest::FILENAME)),
                reason: "redirect resolution carries no source_ref — internal invariant violated"
                    .to_string(),
            })?;

        // Synthesise a single-package registry on the target URL with
        // the redirect's declared auth, so the M1.14 token-injection
        // + scrub-from-`.git/config` discipline applies here too.
        let synthetic_name = format!("redirect-target-{kind}-{name}");
        let target_reg = GitPackageRegistry::open_single_package(
            &synthetic_name,
            &target_url,
            &refname,
            &self.cache_root,
            Arc::clone(&self.backend),
            DEFAULT_FRESHNESS_SECS,
            resolution.redirect_target_auth,
            resolution.redirect_target_token_env.as_deref(),
        )?;
        let plain_url = target_reg.package_repo_url(kind, name);
        let credentialed = target_reg.credentialed_url(&plain_url);

        let clone_dir = self.redirect_clone_dir(kind, name);
        ensure_clone_at(self.backend.as_ref(), &credentialed, &refname, &clone_dir)?;
        if credentialed != plain_url {
            self.backend
                .set_remote_url(&clone_dir, "origin", &plain_url)
                .ok();
        }

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
        let manifest_path = dest.join(Manifest::FILENAME);
        let manifest = Manifest::read(&manifest_path)?;
        if manifest.package.is_none() {
            return Err(RegistryError::MalformedMeta {
                path: manifest_path.clone(),
                reason: "registry package manifest must carry a [package] table".to_string(),
            });
        }
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
            source_uri: plain_url,
            // Lockfile records the stub registry's name so the entry
            // is associated with the registry the consumer's
            // `vibe.toml` named.
            registry_name: resolution.registry_name.clone(),
            source_ref: Some(refname),
            resolved_commit: None,
            overridden: false,
            is_git_source: false,
            is_path_source: false,
            via_redirect: resolution.via_redirect.clone(),
        })
    }

    /// Fetch a git-source-resolved package into the per-project cache.
    /// Same shape as `fetch_override` but threads `dep.auth` /
    /// `dep.token_env` through so private targets get token injection
    /// and the M1.14 scrub-from-`.git/config` discipline applies.
    fn fetch_git_source(
        &self,
        resolution: &MultiResolution,
        project_cache: &Path,
        _expected_hash: Option<&str>,
    ) -> Result<CachedPackage, RegistryError> {
        let kind = resolution.resolved.kind;
        let name = resolution.resolved.name.as_str();
        let qualified = format!("{kind}:{name}");
        let dep = self.git_packages.get(&qualified).ok_or_else(|| {
            RegistryError::UnknownPackage {
                kind,
                name: name.to_string(),
            }
        })?;
        let refname = resolution
            .source_ref
            .clone()
            .unwrap_or_else(|| dep.ref_kind.as_str().to_string());

        // Synthesise a single-package registry just to leverage its
        // `package_repo_url` / `credentialed_url` plumbing for token
        // injection + scrub. The synthetic registry's clone path is
        // not used here — we clone into our own `__git_sources__`
        // sub-tree so the cache stays organised by resolution mode.
        let synthetic_name = format!("git-source-{kind}-{name}");
        let reg = GitPackageRegistry::open_single_package(
            &synthetic_name,
            &dep.url,
            &refname,
            &self.cache_root,
            Arc::clone(&self.backend),
            DEFAULT_FRESHNESS_SECS,
            dep.auth,
            dep.token_env.as_deref(),
        )?;
        let plain_url = reg.package_repo_url(kind, name);
        let credentialed = reg.credentialed_url(&plain_url);

        let clone_dir = self.git_source_clone_dir(kind, name);
        ensure_clone_at(self.backend.as_ref(), &credentialed, &refname, &clone_dir)?;
        // Token-discipline (M1.14): scrub any credentialed URL from
        // the freshly-bootstrapped `.git/config` so the token does not
        // persist on disk. Best-effort — if the backend has no
        // `set_remote_url` impl, the default is a no-op (the
        // credentialed URL was only ever in-memory anyway for
        // backends that don't write a `.git/config`).
        if credentialed != plain_url {
            self.backend
                .set_remote_url(&clone_dir, "origin", &plain_url)
                .ok();
        }

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
        let manifest_path = dest.join(Manifest::FILENAME);
        let manifest = Manifest::read(&manifest_path)?;
        if manifest.package.is_none() {
            return Err(RegistryError::MalformedMeta {
                path: manifest_path.clone(),
                reason: "registry package manifest must carry a [package] table".to_string(),
            });
        }
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
            source_uri: plain_url,
            registry_name: None,
            source_ref: Some(refname),
            resolved_commit: None,
            overridden: false,
            is_git_source: true,
            is_path_source: false,
            via_redirect: None,
        })
    }

    /// Fetch a path-source-resolved package into the per-project cache.
    /// Unlike git-source there is NO git clone — a path-source package
    /// is a local directory. `resolution.resolved.source_dir` carries
    /// the resolver-supplied absolute `package_dir`; we copy its content
    /// (excluding any `.git/`) straight into the per-project package
    /// cache and hash the copied tree. PROP-007 §2.5.
    fn fetch_path_source(
        &self,
        resolution: &MultiResolution,
        project_cache: &Path,
    ) -> Result<CachedPackage, RegistryError> {
        let kind = resolution.resolved.kind;
        let name = resolution.resolved.name.as_str();
        // The resolver stored the canonicalised package directory on
        // `resolved.source_dir`; `workspace_rel` is in `source_url`.
        let package_dir = resolution.resolved.source_dir.clone();
        let workspace_rel = resolution.source_url.clone();

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
        // Copy the local directory's content into the cache, excluding
        // any `.git/` — same exclusion the registry / override / git-
        // source paths apply. A path-source package directory is
        // ordinarily not a git checkout of its own, but a workspace
        // member can be, so the exclusion is load-bearing.
        copy_dir_excluding_git(&package_dir, &dest)?;
        let manifest_path = dest.join(Manifest::FILENAME);
        let manifest = Manifest::read(&manifest_path)?;
        if manifest.package.is_none() {
            return Err(RegistryError::MalformedMeta {
                path: manifest_path.clone(),
                reason: "registry package manifest must carry a [package] table".to_string(),
            });
        }
        let content_hash = compute_content_hash(&dest)?;

        Ok(CachedPackage {
            resolved: ResolvedPackage {
                kind,
                name: name.to_string(),
                version: resolution.resolved.version.clone(),
                source_dir: package_dir,
            },
            cache_dir: dest,
            manifest,
            content_hash,
            // `source_uri` records the workspace-relative path — the
            // lockfile `source_url` for a path entry. Never a URL,
            // never absolute.
            source_uri: workspace_rel,
            registry_name: None,
            source_ref: None,
            resolved_commit: None,
            overridden: false,
            is_git_source: false,
            is_path_source: true,
            via_redirect: None,
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
        /// URLs whose `list_tags` should fail with `AuthFailed` —
        /// simulates a host that returned 401 / 403. Used to drive
        /// the per-`auth` walk-vs-halt rules in §2.3.1.
        auth_failure_urls: Mutex<std::collections::HashSet<String>>,
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
        fn seed_auth_failure(&self, url: impl Into<String>) {
            self.auth_failure_urls.lock().unwrap().insert(url.into());
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
            if self.auth_failure_urls.lock().unwrap().contains(url) {
                return Err(GitError::AuthFailed {
                    url: url.to_string(),
                });
            }
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

    fn registry_section_token_env(name: &str, url: &str, env_var: &str) -> RegistrySection {
        RegistrySection {
            name: name.to_string(),
            url: url.to_string(),
            r#ref: "main".to_string(),
            naming: NamingConvention::KindName,
            auth: vibe_core::manifest::AuthKind::TokenEnv,
            token_env: Some(env_var.to_string()),
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
    fn resolve_follows_registry_redirect_pass_through_tag() {
        // M1.16 PROP-002 §2.4.2: a registry stub repo carries
        // vibe-redirect.toml at its root. The resolver detects the
        // marker, follows it, and returns a MultiResolution carrying
        // the target URL in source_url and the stub URL in via_redirect.
        let cache = tempdir().unwrap();
        let fake = Arc::new(FakeBackend::default());
        // Stub repo at the registry: tag v0.3.0 has a vibe-redirect.toml
        // pointing at the target URL. NO vibe.toml.
        let stub_url = "git@host:org-stub/flow-internal.git";
        fake.seed_tags(stub_url, vec!["v0.3.0".into()]);
        fake.seed_file(
            stub_url,
            "v0.3.0",
            "vibe-redirect.toml",
            br#"[redirect]
target_url = "git@host:external/flow-internal.git"
"#
            .to_vec(),
        );
        // Target repo at the external host: tag v0.3.0 has a real
        // vibe.toml.
        let target_url = "git@host:external/flow-internal.git";
        fake.seed_tags(target_url, vec!["v0.3.0".into()]);
        fake.seed_file(
            target_url,
            "v0.3.0",
            "vibe.toml",
            manifest_text("internal", "flow", "0.3.0").into_bytes(),
        );
        let r = build_resolver(
            cache.path(),
            vec![registry_section("stub-org", "git@host:org-stub")],
            vec![],
            vec![],
            fake,
        );
        let p = PackageRef::parse("flow:internal").unwrap();
        let m = r.resolve(&p).expect("redirect-follow must succeed");
        assert_eq!(
            m.via_redirect.as_deref(),
            Some(stub_url),
            "via_redirect must carry the stub URL"
        );
        assert_eq!(
            m.source_url, target_url,
            "source_url must carry the target URL"
        );
        assert_eq!(m.source_ref.as_deref(), Some("v0.3.0"));
        assert!(!m.is_git_source);
        assert!(!m.overridden);
        assert_eq!(m.registry_name.as_deref(), Some("stub-org"));
    }

    #[test]
    fn resolve_redirect_chain_rejected_at_hop_two() {
        // Two stubs in sequence: the first redirects to a URL that is
        // itself a stub. Per PROP-002 §2.4.2 hop limit = 1, this is
        // rejected with `RedirectChainNotAllowed` (surfaced as
        // MalformedMeta with chain-not-allowed reason).
        let cache = tempdir().unwrap();
        let fake = Arc::new(FakeBackend::default());
        let stub_a = "git@host:org-a/flow-foo.git";
        let stub_b = "git@host:org-b/flow-foo.git";
        fake.seed_tags(stub_a, vec!["v1.0.0".into()]);
        fake.seed_file(
            stub_a,
            "v1.0.0",
            "vibe-redirect.toml",
            format!(
                "[redirect]\ntarget_url = \"{stub_b}\"\n"
            )
            .into_bytes(),
        );
        // Hop limit probe: target URL has its own vibe-redirect.toml
        // at the same tag. Chain rejected.
        fake.seed_file(
            stub_b,
            "v1.0.0",
            "vibe-redirect.toml",
            br#"[redirect]
target_url = "git@host:org-c/flow-foo.git"
"#
            .to_vec(),
        );
        // The target's vibe.toml is also seeded (some hop-2
        // detectors only check the redirect file; ours also fetches
        // the manifest first, so seed it to avoid noise).
        fake.seed_file(
            stub_b,
            "v1.0.0",
            "vibe.toml",
            manifest_text("foo", "flow", "1.0.0").into_bytes(),
        );
        let r = build_resolver(
            cache.path(),
            vec![registry_section("a", "git@host:org-a")],
            vec![],
            vec![],
            fake,
        );
        let p = PackageRef::parse("flow:foo").unwrap();
        let err = r.resolve(&p).expect_err("redirect chain must reject");
        let msg = err.to_string();
        assert!(
            msg.contains("redirect chain not allowed") || msg.contains("hop limit"),
            "expected hop-limit rejection, got: {msg}"
        );
    }

    #[test]
    fn resolve_redirect_pinned_uses_pinned_ref() {
        // ref_policy = "pinned" + pinned_ref = "v1.0.0": stub at
        // any tag should resolve to target's v1.0.0, regardless of
        // the stub's own tag.
        let cache = tempdir().unwrap();
        let fake = Arc::new(FakeBackend::default());
        let stub_url = "git@host:org-stub/flow-pinned.git";
        let target_url = "git@host:external/flow-pinned.git";
        // Stub has v9.9.9 tag (irrelevant — pinned overrides).
        fake.seed_tags(stub_url, vec!["v9.9.9".into()]);
        fake.seed_file(
            stub_url,
            "v9.9.9",
            "vibe-redirect.toml",
            br#"[redirect]
target_url = "git@host:external/flow-pinned.git"
ref_policy = "pinned"
pinned_ref = "v1.0.0"
"#
            .to_vec(),
        );
        // Target has v1.0.0 (the pinned ref).
        fake.seed_file(
            target_url,
            "v1.0.0",
            "vibe.toml",
            manifest_text("pinned", "flow", "1.0.0").into_bytes(),
        );
        let r = build_resolver(
            cache.path(),
            vec![registry_section("stub-org", "git@host:org-stub")],
            vec![],
            vec![],
            fake,
        );
        let p = PackageRef::parse("flow:pinned").unwrap();
        let m = r.resolve(&p).expect("pinned redirect must succeed");
        assert_eq!(m.source_ref.as_deref(), Some("v1.0.0"));
        assert_eq!(m.resolved.version.to_string(), "1.0.0");
        assert_eq!(m.via_redirect.as_deref(), Some(stub_url));
    }

    #[test]
    fn resolve_redirect_target_kind_name_mismatch_rejected() {
        // Stub redirects to a target whose vibe.toml declares
        // a different (kind, name). Refuse — pulling code under the
        // wrong pkgref slot would silently misroute on disk.
        let cache = tempdir().unwrap();
        let fake = Arc::new(FakeBackend::default());
        let stub_url = "git@host:org-stub/flow-internal.git";
        let target_url = "git@host:external/some-other-pkg.git";
        fake.seed_tags(stub_url, vec!["v0.1.0".into()]);
        fake.seed_file(
            stub_url,
            "v0.1.0",
            "vibe-redirect.toml",
            format!(
                "[redirect]\ntarget_url = \"{target_url}\"\n"
            )
            .into_bytes(),
        );
        fake.seed_file(
            target_url,
            "v0.1.0",
            "vibe.toml",
            manifest_text("something-else", "feat", "0.1.0").into_bytes(),
        );
        let r = build_resolver(
            cache.path(),
            vec![registry_section("stub-org", "git@host:org-stub")],
            vec![],
            vec![],
            fake,
        );
        let p = PackageRef::parse("flow:internal").unwrap();
        let err = r.resolve(&p).expect_err("identity mismatch must reject");
        assert!(
            err.to_string().contains("refusing to install"),
            "got: {err}"
        );
    }

    #[test]
    fn resolve_dispatches_to_git_source_short_circuiting_registries() {
        // M1.15: a `[requires.packages]` git-source declaration bypasses
        // the registry walk for that pkgref. The resolver synthesises a
        // single-package registry pointing at `dep.url`, fetches the
        // manifest at the declared ref, returns
        // `MultiResolution { is_git_source: true, ... }`.
        let cache = tempdir().unwrap();
        let fake = Arc::new(FakeBackend::default());
        // Registry has nothing — would fail without git-source dispatch.
        // git-source URL has the manifest at v0.3.0 tag.
        let url = "git@host:owner/flow-internal.git";
        fake.seed_file(
            url,
            "v0.3.0",
            "vibe.toml",
            manifest_text("internal", "flow", "0.3.0").into_bytes(),
        );

        let dep = vibe_core::manifest::GitPackageDep {
            kind: vibe_core::PackageKind::Flow,
            name: "internal".to_string(),
            url: url.to_string(),
            ref_kind: vibe_core::manifest::GitRefKind::Tag("v0.3.0".to_string()),
            version: None,
            auth: vibe_core::manifest::AuthKind::None,
            token_env: None,
        };
        let r = build_resolver(cache.path(), vec![], vec![], vec![], fake)
            .with_git_packages(vec![dep]);

        let p = PackageRef::parse("flow:internal").unwrap();
        let m = r.resolve(&p).expect("git-source resolution must succeed");
        assert!(m.is_git_source);
        assert!(!m.overridden);
        assert_eq!(m.registry_name, None);
        assert_eq!(m.source_url, url);
        assert_eq!(m.source_ref.as_deref(), Some("v0.3.0"));
        assert_eq!(m.resolved.version.to_string(), "0.3.0");
    }

    #[test]
    fn resolve_git_source_rejects_kind_name_mismatch() {
        // The repo's `vibe.toml` says feat:something-else, but
        // the consumer's `[requires.packages]` declared flow:internal
        // pointing at this URL. Refuse — pulling code under the wrong
        // pkgref slot would silently misroute on disk.
        let cache = tempdir().unwrap();
        let fake = Arc::new(FakeBackend::default());
        let url = "git@host:owner/wrong-pkg.git";
        fake.seed_file(
            url,
            "v0.1.0",
            "vibe.toml",
            manifest_text("something-else", "feat", "0.1.0").into_bytes(),
        );
        let dep = vibe_core::manifest::GitPackageDep {
            kind: vibe_core::PackageKind::Flow,
            name: "internal".to_string(),
            url: url.to_string(),
            ref_kind: vibe_core::manifest::GitRefKind::Tag("v0.1.0".to_string()),
            version: None,
            auth: vibe_core::manifest::AuthKind::None,
            token_env: None,
        };
        let r = build_resolver(cache.path(), vec![], vec![], vec![], fake)
            .with_git_packages(vec![dep]);

        let p = PackageRef::parse("flow:internal").unwrap();
        let err = r.resolve(&p).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("refusing to install"),
            "expected identity-mismatch refusal, got: {msg}"
        );
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
    fn resolve_aggregates_walk_attempts_when_no_registry_has_it() {
        let cache = tempdir().unwrap();
        let fake = Arc::new(FakeBackend::default());
        // No seed for any URL — both registries return UnknownPackage
        // for `flow:ghost`. The resolver collects both into the
        // aggregate `PackageNotFoundEverywhere` report so the
        // operator sees per-registry status.

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
        match err {
            RegistryError::PackageNotFoundEverywhere {
                kind,
                name,
                summary,
                attempts,
            } => {
                assert_eq!(attempts.len(), 2, "expected 2 walk attempts: {attempts:?}");
                assert_eq!(kind, vibe_core::PackageKind::Flow);
                assert_eq!(name, "ghost");
                assert!(
                    summary.contains("a") && summary.contains("b"),
                    "summary must list both walked registries: {summary}"
                );
                assert!(
                    summary.contains("not found"),
                    "expected `not found` status label: {summary}"
                );
            }
            other => panic!(
                "expected PackageNotFoundEverywhere with attempts, got: {other:?}"
            ),
        }
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

    /// PROP-002 §2.3.1 strict-auth corollary: when
    /// `with_strict_auth(true)` is set, a 401 on an `auth = "none"`
    /// public registry halts instead of walking past. Useful for
    /// CI / cron where the operator wants to gate "must come from
    /// the private registry; if its 401 leaks to a public fallback,
    /// fail loudly". Default behaviour (without strict_auth) is
    /// covered by `resolve_walks_past_auth_failed_when_registry_is_public`
    /// below.
    #[test]
    fn resolve_strict_auth_halts_on_public_401_instead_of_walking() {
        let cache = tempdir().unwrap();
        let fake = Arc::new(FakeBackend::default());
        // Primary public registry returns AuthFailed; secondary has
        // the package. With strict_auth on, the resolver must NOT
        // walk to the secondary.
        fake.seed_auth_failure("git@host:org-a/flow-wal.git");
        fake.seed_tags("git@host:org-b/flow-wal.git", vec!["v0.5.0".into()]);

        let r = build_resolver(
            cache.path(),
            vec![
                registry_section("public-a", "git@host:org-a"),
                registry_section("public-b", "git@host:org-b"),
            ],
            vec![],
            vec![],
            fake,
        )
        .with_strict_auth(true);
        assert!(r.strict_auth());

        let p = PackageRef::parse("flow:wal").unwrap();
        let err = r.resolve(&p).unwrap_err();
        match err {
            RegistryError::Git(GitError::AuthFailed { url }) => {
                assert!(
                    url.contains("org-a"),
                    "halt error must surface the failing registry's URL: {url}"
                );
            }
            other => panic!(
                "strict-auth: expected halt with AuthFailed on first registry, got: {other:?}"
            ),
        }
    }

    /// PROP-002 §2.3.1: 401 / 403 on an `auth = "none"` registry is
    /// reclassified as "no public answer here", and the resolver
    /// walks to the next registry. Closes the original opencode
    /// regression where GitVerse's 401 (its policy on missing
    /// public repos) halted resolution before GitHub got a chance.
    #[test]
    fn resolve_walks_past_auth_failed_when_registry_is_public() {
        let cache = tempdir().unwrap();
        let fake = Arc::new(FakeBackend::default());
        // First registry: returns AuthFailed (think GitVerse-style 401
        // for a missing public repo). Second registry: serves the
        // package.
        fake.seed_auth_failure("git@host:org-a/flow-wal.git");
        fake.seed_tags("git@host:org-b/flow-wal.git", vec!["v0.5.0".into()]);

        let r = build_resolver(
            cache.path(),
            vec![
                registry_section("public-a", "git@host:org-a"),
                registry_section("public-b", "git@host:org-b"),
            ],
            vec![],
            vec![],
            fake,
        );

        let p = PackageRef::parse("flow:wal").unwrap();
        let m = r
            .resolve(&p)
            .expect("public-a's AuthFailed must walk to public-b, not halt");
        assert_eq!(m.registry_name.as_deref(), Some("public-b"));
        assert_eq!(m.resolved.version.to_string(), "0.5.0");
    }

    /// PROP-002 §2.3.1: 401 / 403 on an authenticated registry
    /// (`auth = "token-env"` in this test) is a real `AuthFailed`
    /// halt — the operator declared this registry expects creds and
    /// the creds presented were rejected (or absent / expired).
    /// Walking past would mask the configuration error.
    ///
    /// We use `open_with_explicit_token` indirectly through the
    /// resolver's `from_manifest` path by pre-loading the env-var.
    /// Skipping the env layer in this test would require a
    /// resolver-level test-only constructor; instead we set the
    /// env via a helper that doesn't need `unsafe` (read-only,
    /// because the value is already there from the caller).
    ///
    /// In this test we don't actually need a token *value* — the
    /// walk-vs-halt decision is gated on `auth_kind`, not on
    /// whether the token resolved. We mark the registry
    /// `auth = "token-env"` with no env-var set; the resolver's
    /// `MissingToken` precheck does NOT fire because the
    /// `MissingToken` path only triggers when a git invocation is
    /// attempted, and AuthFailed is already on the wire from
    /// `list_tags`. So this test exercises the AuthFailed-on-
    /// authenticated-registry branch directly.
    #[test]
    fn resolve_halts_on_auth_failed_against_authenticated_registry() {
        let cache = tempdir().unwrap();
        let fake = Arc::new(FakeBackend::default());
        // The authenticated registry returns AuthFailed.
        fake.seed_auth_failure("https://internal.example.com/vibespecs/flow-wal.git");
        // A second registry has the package — but the resolver must
        // NOT walk to it (the operator declared the first registry
        // as authenticated; AuthFailed is information they need).
        fake.seed_tags(
            "git@host:org-public/flow-wal.git",
            vec!["v0.5.0".into()],
        );

        // Stash the token in an env-var so `from_manifest` can find
        // one. We can't `set_var` from this test (`forbid(unsafe_code)`),
        // so we use a name that's already in the test process env or
        // leverage a side door. Simplest: declare `auth = token-env`
        // with NO `token_env` field — `resolve_token_env_name` will
        // derive a name from the host that almost certainly isn't
        // set, so the registry opens with `effective_token = None`.
        // The MissingToken precheck would normally fire, but our
        // FakeBackend's `list_tags` returns AuthFailed first, before
        // any token-aware code path runs. (The AuthFailed comes from
        // the seeded backend, simulating a real 401 from the host;
        // we bypass the precheck by virtue of how the fake works.)
        //
        // Actually simpler still: just set `auth = "credential-helper"`,
        // which never triggers MissingToken (the precheck only fires
        // for `TokenEnv`). The walk-vs-halt rule applies the same:
        // any `auth != None` halts on AuthFailed.
        let auth_section = RegistrySection {
            name: "internal".to_string(),
            url: "https://internal.example.com/vibespecs".to_string(),
            r#ref: "main".to_string(),
            naming: NamingConvention::KindName,
            auth: vibe_core::manifest::AuthKind::CredentialHelper,
            token_env: None,
        };
        let r = build_resolver(
            cache.path(),
            vec![
                auth_section,
                registry_section("public-fallback", "git@host:org-public"),
            ],
            vec![],
            vec![],
            fake,
        );

        let p = PackageRef::parse("flow:wal").unwrap();
        let err = r.resolve(&p).unwrap_err();
        match err {
            RegistryError::Git(GitError::AuthFailed { url }) => {
                assert!(
                    url.contains("internal.example.com"),
                    "halt error must surface the authenticated registry's URL, got: {url}"
                );
            }
            other => panic!(
                "expected halt with AuthFailed against authenticated registry, got: {other:?}"
            ),
        }
    }

    /// PROP-002 §2.2.1 + §2.3.1 corollary: when a registry is
    /// declared `auth = "token-env"` but the env-var is absent, the
    /// resolver must surface `MissingToken` immediately on that
    /// registry — it must NOT silently walk past, because doing so
    /// would mask the operator's configuration error (the
    /// authenticated registry was supposed to answer; a missing
    /// token is a setup mistake, not a "package not here" signal).
    #[test]
    fn resolve_halts_on_missing_token_for_authenticated_registry() {
        let cache = tempdir().unwrap();
        let fake = Arc::new(FakeBackend::default());
        // Public fallback also has the package — must NOT be walked
        // past the missing-token registry.
        fake.seed_tags(
            "git@host:org-public/flow-wal.git",
            vec!["v0.5.0".into()],
        );

        // `auth = token-env` with an env-var that resolves to nothing
        // (deliberately exotic name unlikely to be set anywhere).
        let env_name = "VIBEVM_REGISTRY_TOKEN_DEFINITELY_NOT_SET_ABCXYZ";
        let r = build_resolver(
            cache.path(),
            vec![
                registry_section_token_env("internal", "https://internal.example/vibespecs", env_name),
                registry_section("public-fallback", "git@host:org-public"),
            ],
            vec![],
            vec![],
            fake,
        );

        let p = PackageRef::parse("flow:wal").unwrap();
        let err = r.resolve(&p).unwrap_err();
        match err {
            RegistryError::MissingToken { registry, env_var } => {
                assert_eq!(registry, "internal");
                assert_eq!(env_var, env_name);
            }
            other => panic!(
                "expected MissingToken halt, got: {other:?}; resolver must NOT walk past missing-token registries"
            ),
        }
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
            "vibe.toml",
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
            "vibe.toml",
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
            "vibe.toml",
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
            pkg_root.join("vibe.toml"),
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
        assert_eq!(cached.package_meta().version.to_string(), "0.5.0");
        assert!(cached.cache_dir.join("vibe.toml").exists());
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
            pkg_root.join("vibe.toml"),
            manifest_text("wal", "flow", "0.9.0"),
        )
        .unwrap();

        let fake = Arc::new(FakeBackend::default());
        // For override: backend serves manifest via `fetch_file_at_ref`
        // (resolve), then clones via `bootstrap` (fetch).
        fake.seed_file(
            "git@my-fork:vibevm/wal-fork.git",
            "my-fix",
            "vibe.toml",
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
        assert_eq!(cached.package_meta().version.to_string(), "0.9.0");
        // Override clone lives under cache_root/__overrides__/flow-wal/clone/
        let overrides_root = cache.path().join("__overrides__").join("flow-wal").join("clone");
        assert!(overrides_root.join(".git").exists());
        // Materialised cache holds payload only.
        assert!(cached.cache_dir.join("vibe.toml").exists());
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

    // ----- path-source (PROP-007 §2.5) ------------------------------

    /// Lay down a path-source package directory under `parent`:
    /// `<parent>/<dirname>/vibe.toml` carrying a `[package]` table.
    /// Returns the package directory.
    fn seed_path_package(
        parent: &Path,
        dirname: &str,
        name: &str,
        kind: &str,
        version: &str,
    ) -> PathBuf {
        let dir = parent.join(dirname);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("vibe.toml"), manifest_text(name, kind, version)).unwrap();
        dir
    }

    #[test]
    fn resolve_dispatches_to_path_source_short_circuiting_registries() {
        // PROP-007 §2.5: a `[requires.packages]` path-source declaration
        // bypasses the registry walk for that pkgref. The resolver reads
        // the package's `vibe.toml` straight off the local directory and
        // returns `MultiResolution { is_path_source: true, ... }`.
        let cache = tempdir().unwrap();
        let ws = tempdir().unwrap();
        let fake = Arc::new(FakeBackend::default());
        // Registry has nothing — would fail without path-source dispatch.
        let pkg_dir = seed_path_package(ws.path(), "flow-internal", "internal", "flow", "0.3.0");

        let dep = ResolvedPathDep {
            kind: vibe_core::PackageKind::Flow,
            name: "internal".to_string(),
            version: None,
            package_dir: pkg_dir.clone(),
            workspace_rel: "flow-internal".to_string(),
        };
        let r = build_resolver(cache.path(), vec![], vec![], vec![], fake)
            .with_path_packages(vec![dep]);

        let p = PackageRef::parse("flow:internal").unwrap();
        let m = r.resolve(&p).expect("path-source resolution must succeed");
        assert!(m.is_path_source);
        assert!(!m.is_git_source);
        assert!(!m.overridden);
        assert_eq!(m.registry_name, None);
        // source_url carries the workspace-relative path, never an
        // absolute path and never a URL.
        assert_eq!(m.source_url, "flow-internal");
        assert_eq!(m.source_ref, None);
        assert_eq!(m.resolved.version.to_string(), "0.3.0");
    }

    #[test]
    fn resolve_path_source_rejects_kind_name_mismatch() {
        // The package's `vibe.toml` says feat:something-else, but the
        // consumer's `[requires.packages]` declared flow:internal
        // pointing at this directory. Refuse — installing code under a
        // misnamed slot would silently misroute on disk.
        let cache = tempdir().unwrap();
        let ws = tempdir().unwrap();
        let fake = Arc::new(FakeBackend::default());
        let pkg_dir =
            seed_path_package(ws.path(), "wrong-pkg", "something-else", "feat", "0.1.0");

        let dep = ResolvedPathDep {
            kind: vibe_core::PackageKind::Flow,
            name: "internal".to_string(),
            version: None,
            package_dir: pkg_dir,
            workspace_rel: "wrong-pkg".to_string(),
        };
        let r = build_resolver(cache.path(), vec![], vec![], vec![], fake)
            .with_path_packages(vec![dep]);

        let p = PackageRef::parse("flow:internal").unwrap();
        let err = r.resolve(&p).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("refusing to install"),
            "expected identity-mismatch refusal, got: {msg}"
        );
    }

    #[test]
    fn resolve_path_source_rejects_version_constraint_mismatch() {
        // The path-dep carried a dual-form `{ path, version }` constraint
        // that the package's own `[package].version` does not satisfy.
        // Refuse — same shape as the git-source version check.
        let cache = tempdir().unwrap();
        let ws = tempdir().unwrap();
        let fake = Arc::new(FakeBackend::default());
        let pkg_dir = seed_path_package(ws.path(), "flow-wal", "wal", "flow", "0.1.0");

        let dep = ResolvedPathDep {
            kind: vibe_core::PackageKind::Flow,
            name: "wal".to_string(),
            // Package is 0.1.0; constraint demands ^0.3 — mismatch.
            version: Some(VersionSpec::parse("^0.3").unwrap()),
            package_dir: pkg_dir,
            workspace_rel: "flow-wal".to_string(),
        };
        let r = build_resolver(cache.path(), vec![], vec![], vec![], fake)
            .with_path_packages(vec![dep]);

        let p = PackageRef::parse("flow:wal").unwrap();
        let err = r.resolve(&p).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("does not satisfy the constraint"),
            "expected version-constraint refusal, got: {msg}"
        );
    }

    #[test]
    fn resolve_path_source_wins_over_same_pkgref_git_source() {
        // PROP-007 §2.5 priority: a pkgref declared as BOTH path-source
        // and git-source resolves via path-source — path-source sits one
        // notch above git-source in the resolution order.
        let cache = tempdir().unwrap();
        let ws = tempdir().unwrap();
        let fake = Arc::new(FakeBackend::default());
        // path-source package: version 0.5.0.
        let pkg_dir = seed_path_package(ws.path(), "flow-dual", "dual", "flow", "0.5.0");
        // git-source for the SAME pkgref: a different version on a URL.
        let git_url = "git@host:owner/flow-dual.git";
        fake.seed_file(
            git_url,
            "v9.9.9",
            "vibe.toml",
            manifest_text("dual", "flow", "9.9.9").into_bytes(),
        );

        let path_dep = ResolvedPathDep {
            kind: vibe_core::PackageKind::Flow,
            name: "dual".to_string(),
            version: None,
            package_dir: pkg_dir,
            workspace_rel: "flow-dual".to_string(),
        };
        let git_dep = vibe_core::manifest::GitPackageDep {
            kind: vibe_core::PackageKind::Flow,
            name: "dual".to_string(),
            url: git_url.to_string(),
            ref_kind: vibe_core::manifest::GitRefKind::Tag("v9.9.9".to_string()),
            version: None,
            auth: vibe_core::manifest::AuthKind::None,
            token_env: None,
        };
        let r = build_resolver(cache.path(), vec![], vec![], vec![], fake)
            .with_git_packages(vec![git_dep])
            .with_path_packages(vec![path_dep]);

        let p = PackageRef::parse("flow:dual").unwrap();
        let m = r.resolve(&p).expect("path-source must win and resolve");
        assert!(m.is_path_source, "path-source must win over git-source");
        assert!(!m.is_git_source);
        // The path-source version (0.5.0), not the git-source (9.9.9).
        assert_eq!(m.resolved.version.to_string(), "0.5.0");
        assert_eq!(m.source_url, "flow-dual");
    }

    #[test]
    fn fetch_path_source_copies_local_dir_and_computes_hash() {
        // PROP-007 §2.5: fetching a path-source package copies the local
        // directory's content into the per-project package cache,
        // excludes any `.git/`, and computes a content_hash over the
        // copied tree. No git clone happens.
        let cache = tempdir().unwrap();
        let pkg_cache = tempdir().unwrap();
        let ws = tempdir().unwrap();
        let fake = Arc::new(FakeBackend::default());

        // Path-source package with a regular file AND a `.git/` subtree
        // that must NOT make it into the cache.
        let pkg_dir = seed_path_package(ws.path(), "flow-local", "local", "flow", "0.2.0");
        fs::write(pkg_dir.join("README.md"), "# local package\n").unwrap();
        let git_dir = pkg_dir.join(".git");
        fs::create_dir_all(&git_dir).unwrap();
        fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n").unwrap();

        let dep = ResolvedPathDep {
            kind: vibe_core::PackageKind::Flow,
            name: "local".to_string(),
            version: None,
            package_dir: pkg_dir,
            workspace_rel: "flow-local".to_string(),
        };
        let r = build_resolver(cache.path(), vec![], vec![], vec![], fake.clone())
            .with_path_packages(vec![dep]);

        let p = PackageRef::parse("flow:local").unwrap();
        let resolution = r.resolve(&p).unwrap();
        let cached = r.fetch(&resolution, pkg_cache.path()).unwrap();

        assert!(cached.is_path_source);
        assert!(!cached.is_git_source);
        assert!(!cached.overridden);
        assert_eq!(cached.registry_name, None);
        assert_eq!(cached.source_ref, None);
        // source_uri is the workspace-relative path, recorded verbatim
        // as the lockfile `source_url` for a path entry.
        assert_eq!(cached.source_uri, "flow-local");
        assert_eq!(cached.package_meta().version.to_string(), "0.2.0");
        // Cache is populated with the package payload.
        assert!(cached.cache_dir.join("vibe.toml").exists());
        assert!(cached.cache_dir.join("README.md").exists());
        // `.git/` was excluded.
        assert!(!cached.cache_dir.join(".git").exists());
        // content_hash computed over the copied tree.
        assert!(cached.content_hash.starts_with("sha256:"));
        // No git clone — `bootstrap` was never invoked.
        assert_eq!(fake.bootstrap_count(), 0);
    }
}
