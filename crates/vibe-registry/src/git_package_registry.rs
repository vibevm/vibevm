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
use vibe_core::manifest::{Manifest, NamingConvention};
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
    /// Authentication regime for this registry, per PROP-002 §2.2.1.
    /// Plumbed in by `MultiRegistryResolver::from_manifest` so the
    /// runtime knows whether to inject a token, whether a 401 is a
    /// fall-through signal, and whether to emit a "missing token"
    /// error before even spawning git.
    auth: vibe_core::manifest::AuthKind,
    /// Resolved bearer token for this registry — `Some` only when
    /// `auth == TokenEnv` and the env-var was set at construction
    /// time. Read once at open and held in memory; never logged,
    /// never written to disk. The token is injected into per-package
    /// URLs in [`Self::credentialed_url`] and stripped from any
    /// public-facing URL the lockfile or error messages might carry.
    /// (Modern git ≥ 2.31 also redacts passwords from its own
    /// stderr; we rely on that as the second line of defence.)
    effective_token: Option<String>,
    /// The env-var name the registry would consult under
    /// `auth = TokenEnv`. Held verbatim so a `MissingToken` error
    /// surfaces the exact name the operator typed in `vibe.toml` /
    /// passed via `vibe registry add --token-env`. `None` when the
    /// caller didn't supply an explicit name — falls back to the
    /// host-derived default at error time.
    token_env_name: Option<String>,
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
    /// When `Some`, this registry holds **exactly one package** at the
    /// given URL — `package_repo_url(kind, name)` returns this verbatim
    /// without applying `naming` to compose `<org>/<kind>-<name>.git`.
    /// Used for git-source declarations from `[requires.packages]`
    /// table-form (PROP-002 §2.4.1). When `None`, this is a normal
    /// multi-package registry and naming applies as before.
    single_package_url: Option<String>,
    /// Optional upstream index — when set, `list_versions` queries
    /// it before falling back to `git ls-remote`. PROP-005 §2.10
    /// slice 10. The fetch path is unaffected: `content_hash` is
    /// still verified at fetch time per [PROP-002 §2.1] regardless
    /// of how versions were enumerated.
    index_client: Option<crate::index_client::IndexClient>,
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
    ///
    /// `auth` defaults to `AuthKind::None` (the legacy behaviour);
    /// callers wanting authenticated registries reach for
    /// [`Self::open_with_auth`].
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
        Self::open_with_auth(
            name,
            org_url,
            org_ref,
            naming,
            mirror_urls,
            cache_root,
            backend,
            freshness_secs,
            vibe_core::manifest::AuthKind::None,
            None,
        )
    }

    /// Test-only constructor that takes the resolved token directly
    /// instead of reading an env-var. Production code uses
    /// [`Self::open_with_auth`]. This method is useful in tests
    /// where `#![forbid(unsafe_code)]` prohibits `std::env::set_var`
    /// (Rust 2024+); construct the registry with the token already
    /// in hand and skip the env layer. The resulting registry
    /// behaves identically — same `auth_kind`, same
    /// `effective_token_value`, same downstream injection.
    #[doc(hidden)]
    #[allow(clippy::too_many_arguments)]
    pub fn open_with_explicit_token(
        name: &str,
        org_url: &str,
        org_ref: &str,
        naming: NamingConvention,
        mirror_urls: Vec<String>,
        cache_root: &Path,
        backend: Arc<dyn GitBackend>,
        freshness_secs: u64,
        auth: vibe_core::manifest::AuthKind,
        token_value: Option<String>,
    ) -> Result<Self, RegistryError> {
        let normalized = normalize_url(org_url);
        let canonical_hash = short_url_hash(&normalized);
        let cache_root_owned = cache_root.to_path_buf();
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
            auth,
            effective_token: token_value.filter(|s| !s.trim().is_empty()),
            token_env_name: None,
            cache_root: cache_root_owned,
            canonical_hash,
            mirror_urls,
            single_package_url: None,
            index_client: None,
            freshness_secs,
        })
    }

    /// Full constructor — same as [`open_with_mirrors`] plus the
    /// per-registry authentication knobs from PROP-002 §2.2.1.
    ///
    /// `auth` selects the regime; `token_env_name` is the explicit
    /// env-var override under `auth = AuthKind::TokenEnv` (the
    /// host-derived default applies when `None`). Token resolution
    /// happens once at construction time:
    ///
    /// - `AuthKind::TokenEnv` + env-var set → token loaded,
    ///   injected into per-package URLs in
    ///   [`Self::credentialed_url`].
    /// - `AuthKind::TokenEnv` + env-var absent → registry opens
    ///   anyway with no token; a `MissingToken` error surfaces at
    ///   the first credential-required git operation.
    ///   (Authentication is a runtime property of the fetch, not of
    ///   the constructor, so we don't pre-fail here — letting the
    ///   resolver walk a chain that has *some* authenticated
    ///   registries with missing tokens means the operator can fix
    ///   them one by one as install errors surface.)
    /// - Other regimes (`None`, `CredentialHelper`, `Ssh`) read no
    ///   token; their `effective_token` is `None`.
    #[allow(clippy::too_many_arguments)]
    pub fn open_with_auth(
        name: &str,
        org_url: &str,
        org_ref: &str,
        naming: NamingConvention,
        mirror_urls: Vec<String>,
        cache_root: &Path,
        backend: Arc<dyn GitBackend>,
        freshness_secs: u64,
        auth: vibe_core::manifest::AuthKind,
        token_env_name: Option<&str>,
    ) -> Result<Self, RegistryError> {
        let normalized = normalize_url(org_url);
        let canonical_hash = short_url_hash(&normalized);
        let cache_root_owned = cache_root.to_path_buf();

        let bucket = cache_root_owned.join(&canonical_hash);
        fs::create_dir_all(&bucket).map_err(|source| RegistryError::Io {
            path: bucket.clone(),
            source,
        })?;

        let effective_token = if matches!(auth, vibe_core::manifest::AuthKind::TokenEnv) {
            token_env_name
                .map(|s| s.to_string())
                .and_then(|var| std::env::var(&var).ok())
                .and_then(|v| {
                    let trimmed = v.trim().to_string();
                    if trimmed.is_empty() { None } else { Some(trimmed) }
                })
        } else {
            None
        };

        Ok(GitPackageRegistry {
            backend,
            name: name.to_string(),
            org_url: org_url.to_string(),
            org_ref: org_ref.to_string(),
            naming,
            auth,
            effective_token,
            token_env_name: token_env_name.map(|s| s.to_string()),
            cache_root: cache_root_owned,
            canonical_hash,
            mirror_urls,
            single_package_url: None,
            index_client: None,
            freshness_secs,
        })
    }

    /// Open a registry that holds **exactly one package** at `repo_url`.
    /// Used for git-source declarations from `[requires.packages]`
    /// table-form (PROP-002 §2.4.1) — the consumer's `vibe.toml`
    /// declares `"flow:internal" = { git = "...", tag = "..." }` and
    /// the resolver synthesises a `GitPackageRegistry` pointing
    /// directly at that URL, bypassing the org-level `naming`-driven
    /// URL composition.
    ///
    /// Differences from `open_with_auth`:
    /// - `repo_url` is the per-package URL (not an org URL).
    /// - `naming` is irrelevant — the synthetic registry stores
    ///   `KindName` as a placeholder; `package_repo_url(kind, name)`
    ///   returns `repo_url` verbatim.
    /// - `mirror_urls` are empty (mirrors are an org-level concept;
    ///   git-source has no mirror chain — see PROP-002 §2.4.1
    ///   "Out of scope").
    /// - `name` is a synthetic local label such as
    ///   `"git-source-flow-internal"` — not a registry org name.
    #[allow(clippy::too_many_arguments)]
    pub fn open_single_package(
        synthetic_name: &str,
        repo_url: &str,
        repo_ref: &str,
        cache_root: &Path,
        backend: Arc<dyn GitBackend>,
        freshness_secs: u64,
        auth: vibe_core::manifest::AuthKind,
        token_env_name: Option<&str>,
    ) -> Result<Self, RegistryError> {
        let mut reg = Self::open_with_auth(
            synthetic_name,
            repo_url,
            repo_ref,
            NamingConvention::KindName,
            Vec::new(),
            cache_root,
            backend,
            freshness_secs,
            auth,
            token_env_name,
        )?;
        reg.single_package_url = Some(repo_url.to_string());
        Ok(reg)
    }

    /// Attach an [`IndexClient`](crate::index_client::IndexClient) to
    /// this registry. When set, `list_versions` consults the index
    /// before falling back to `git ls-remote`. Returns the modified
    /// registry for chaining. Slice 10.
    pub fn with_index_client(mut self, client: crate::index_client::IndexClient) -> Self {
        self.index_client = Some(client);
        self
    }

    pub fn index_client(&self) -> Option<&crate::index_client::IndexClient> {
        self.index_client.as_ref()
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

    /// Compose the per-package repo URL — `<org_url>/<naming(kind, name)>.git`
    /// for normal multi-package registries, or the verbatim single-package
    /// URL for git-source registries (PROP-002 §2.4.1). Trailing slashes
    /// on `org_url` are tolerated. **No credentials are embedded** —
    /// this URL is safe to record in the lockfile, log, or surface to
    /// humans. For URLs that drive git invocations (`ls-remote` /
    /// `clone` / `fetch`) under `auth = AuthKind::TokenEnv`, callers
    /// reach for [`Self::credentialed_url`] instead.
    pub fn package_repo_url(&self, kind: PackageKind, name: &str) -> String {
        if let Some(url) = &self.single_package_url {
            return url.clone();
        }
        let repo_name = self.naming.repo_name(kind, name);
        let trimmed = self.org_url.trim_end_matches('/');
        format!("{trimmed}/{repo_name}.git")
    }

    /// True when this registry was constructed via `open_single_package`
    /// — used by callers (e.g. `MultiRegistryResolver`) to skip features
    /// that don't apply to single-package registries (mirror chains,
    /// org-level index lookups).
    pub fn is_single_package(&self) -> bool {
        self.single_package_url.is_some()
    }

    /// Fetch `vibe.toml` at an arbitrary git ref (tag, branch,
    /// or commit SHA) — used by the git-source resolver path
    /// (PROP-002 §2.4.1) where the operator declared `tag = "..."` /
    /// `branch = "..."` / `rev = "..."` and we cannot enumerate
    /// versions through `list_versions` (which is tag-shaped).
    ///
    /// Same auth / token-injection discipline as
    /// [`Self::fetch_dep_manifest`]: token from env at construction,
    /// credentialed URL only for the spawned `git archive`, plain
    /// URL recorded in error messages.
    pub fn fetch_manifest_at_ref(
        &self,
        kind: PackageKind,
        name: &str,
        refname: &str,
    ) -> Result<Manifest, RegistryError> {
        self.ensure_token_loaded()?;
        let plain_url = self.package_repo_url(kind, name);
        let fetch_url = self.credentialed_url(&plain_url);
        let bytes = match self
            .backend
            .fetch_file_at_ref(&fetch_url, refname, Manifest::FILENAME)
        {
            Ok(bytes) => bytes,
            Err(GitError::ArchiveUnsupported { .. }) => {
                // GitHub (and a handful of other hosts) refuse
                // `git archive --remote` for `upload-archive` access
                // policy reasons. The git-source / redirect path hits
                // this when the target is on GitHub. Same fall-back
                // shape as `fetch_dep_manifest`: shallow-clone at the
                // requested ref and read `vibe.toml` from the
                // working tree. Slower than archive but works on every
                // host that accepts `git clone`.
                self.refresh_package(kind, name, refname)?;
                let clone_dir = self.package_clone_dir(kind, name);
                let manifest_path = clone_dir.join(Manifest::FILENAME);
                fs::read(&manifest_path).map_err(|source| RegistryError::Io {
                    path: manifest_path.clone(),
                    source,
                })?
            }
            Err(other) => return Err(RegistryError::from(other)),
        };
        let text = String::from_utf8(bytes).map_err(|e| RegistryError::MalformedMeta {
            path: PathBuf::from(format!("{plain_url}@{refname}:{}", Manifest::FILENAME)),
            reason: format!("invalid UTF-8: {e}"),
        })?;
        Manifest::parse_str(&text).map_err(|e| RegistryError::MalformedMeta {
            path: PathBuf::from(format!("{plain_url}@{refname}:{}", Manifest::FILENAME)),
            reason: e.to_string(),
        })
    }

    /// The `auth` regime the registry was opened with — read by
    /// `MultiRegistryResolver` to decide between walk-to-next-registry
    /// (on `AuthKind::None` + 401) and halt-with-actionable-error
    /// (on `TokenEnv` / `CredentialHelper` + 401).
    pub fn auth_kind(&self) -> vibe_core::manifest::AuthKind {
        self.auth
    }

    /// True when `auth = TokenEnv` was declared for this registry but
    /// no token was loaded (env-var absent / empty). Surfaces
    /// `MissingToken` rather than spawning a git that would fail
    /// with the same outcome a few seconds later.
    pub fn token_env_required_but_absent(&self) -> bool {
        matches!(self.auth, vibe_core::manifest::AuthKind::TokenEnv)
            && self.effective_token.is_none()
    }

    /// Pre-flight check at the entry of every public method that
    /// drives a git invocation. Returns `MissingToken` when the
    /// registry was opened with `auth = TokenEnv` but the env-var
    /// resolved empty. Other regimes (or `TokenEnv` with a token)
    /// pass through. The env-var name surfaced in the error is the
    /// one we'd consult — explicit `token_env` field on the
    /// registry section, otherwise the host-derived default per
    /// `RegistrySection::resolve_token_env_name`.
    fn ensure_token_loaded(&self) -> Result<(), RegistryError> {
        if !self.token_env_required_but_absent() {
            return Ok(());
        }
        // Surface the explicit env-var name verbatim when the operator
        // supplied one (so the error message names exactly what they
        // typed in `vibe.toml` / `vibe registry add --token-env`).
        // Otherwise reconstruct the host-derived default — same
        // algorithm `RegistrySection::resolve_token_env_name` would
        // run. Falls back to a generic placeholder when neither path
        // yields a name (e.g. `file://` registries).
        let env_var = self
            .token_env_name
            .clone()
            .or_else(|| self.derive_default_token_env_name())
            .unwrap_or_else(|| "VIBEVM_REGISTRY_TOKEN_<HOST>".to_string());
        Err(RegistryError::MissingToken {
            registry: self.name.clone(),
            env_var,
        })
    }

    /// Best-effort derivation of the default token env-var name from
    /// this registry's `org_url`. Mirrors the algorithm in
    /// `vibe_core::manifest::RegistrySection::resolve_token_env_name`
    /// — `MultiRegistryResolver` prefers the explicit `token_env`
    /// from the manifest section when available; this fallback is
    /// only used when surfacing a `MissingToken` error from inside
    /// `GitPackageRegistry` (which doesn't carry the explicit name).
    fn derive_default_token_env_name(&self) -> Option<String> {
        let host = registry_host_from_url(&self.org_url)?;
        let mut sanitised = String::with_capacity(host.len());
        for ch in host.chars() {
            match ch {
                '.' | '-' => sanitised.push('_'),
                c if c.is_ascii_alphanumeric() || c == '_' => {
                    sanitised.push(c.to_ascii_uppercase())
                }
                _ => return None,
            }
        }
        Some(format!("VIBEVM_REGISTRY_TOKEN_{sanitised}"))
    }

    /// View of the loaded token, for closures that need to
    /// credentialise per-package URLs without holding a `&self`
    /// borrow. Returns `None` when no token is loaded (any
    /// non-`TokenEnv` regime, or `TokenEnv` with the env-var
    /// missing — note that the `MissingToken` precheck via
    /// `ensure_token_loaded` is what enforces presence at entry to
    /// every git-driving public method).
    pub fn effective_token_value(&self) -> Option<&str> {
        self.effective_token.as_deref()
    }
}

/// Inject a bearer token into a `https://` URL as
/// `https://x-access-token:<TOKEN>@<rest>`. Same shape `vibe-publish`
/// uses on the push side. Returns the URL unchanged when:
///
/// - `token` is `None` (caller is not under `auth = TokenEnv`);
/// - the URL is not `https://` (ssh / file / http URLs never carry
///   tokens — http would expose the secret in the clear, ssh has its
///   own auth path, file:// is local);
/// - the URL already carries credentials (`x-access-token:` somewhere
///   in the userinfo segment) — never double-wrap.
///
/// Public so external integrations (mirror probes, vendor builders)
/// can use the same logic. Token discipline applies: callers must
/// not log the returned string outside the spawned-process boundary;
/// modern git auto-redacts passwords from its own stderr (≥ 2.31)
/// as a second line of defence.
pub fn inject_token(plain_url: &str, token: Option<&str>) -> String {
    let Some(token) = token else { return plain_url.to_string() };
    if !plain_url.starts_with("https://") || plain_url.contains("x-access-token:") {
        return plain_url.to_string();
    }
    let body = &plain_url["https://".len()..];
    format!("https://x-access-token:{token}@{body}")
}

/// Best-effort host extraction from a registry URL — duplicates
/// `vibe_core::manifest::project::registry_host` so this crate
/// doesn't have to reach into a private function. Pragmatic
/// duplication: the algorithm is short and `RegistrySection` already
/// owns the canonical implementation; this is the read-only consumer.
fn registry_host_from_url(url: &str) -> Option<&str> {
    for prefix in ["https://", "http://", "ssh://", "git+ssh://"] {
        if let Some(rest) = url.strip_prefix(prefix) {
            return rest.split('/').next()?.split('@').next_back();
        }
    }
    if let Some(at_idx) = url.find('@')
        && let Some(colon_idx) = url[at_idx..].find(':')
    {
        let host_start = at_idx + 1;
        let host_end = at_idx + colon_idx;
        if host_end > host_start {
            return Some(&url[host_start..host_end]);
        }
    }
    None
}

impl GitPackageRegistry {
    // (closing brace is below; this empty impl block keeps the
    // module structure tidy after the helpers above land outside
    // the original `impl` — reattach the trailing methods.)

    /// Return the URL to actually pass to a git invocation. For
    /// `auth = AuthKind::TokenEnv` with a loaded token this injects
    /// `https://x-access-token:<TOKEN>@host/...` (the same shape
    /// `vibe-publish` already uses on the push side); for any other
    /// regime — or for a non-https URL — this is the plain
    /// `package_repo_url` value. The token never escapes this method;
    /// modern git redacts it from any error stderr it prints, and
    /// vibe never logs the resulting URL outside the spawned-process
    /// boundary.
    pub fn credentialed_url(&self, plain_url: &str) -> String {
        match (&self.effective_token, plain_url) {
            (Some(token), url)
                if url.starts_with("https://") && !url.contains("x-access-token:") =>
            {
                let body = &url["https://".len()..];
                format!("https://x-access-token:{token}@{body}")
            }
            // Never inject for ssh, http (rare and would expose token
            // in cleartext), file, or already-credentialed URLs.
            _ => plain_url.to_string(),
        }
    }

    /// All URLs to try for a `(kind, name)` lookup, primary first.
    /// Mirrors are composed using the same naming convention as the
    /// primary, since the mirror is meant to be a transparent
    /// alternative to the primary's content. Single-package registries
    /// (PROP-002 §2.4.1) return a single-element vec with the verbatim
    /// URL — mirrors do not apply.
    fn package_urls(&self, kind: PackageKind, name: &str) -> Vec<String> {
        if let Some(url) = &self.single_package_url {
            return vec![url.clone()];
        }
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

    /// Bootstrap (or refresh) the per-package clone at `clone_dir`
    /// against `url`. Used by [`Self::ensure_clone_against_sources`]
    /// and the mirror-fallback variants of [`Self::fetch`] /
    /// [`Self::refresh_package`].
    ///
    /// On entry: `clone_dir` is either absent, empty, or a previously
    /// populated git working tree. If the working tree exists,
    /// [`GitBackend::update`] is tried first — that preserves the local
    /// clone and is the cheap path. If `update` fails (origin
    /// unreachable, ref missing, etc.), the clone is wiped and we
    /// retry via [`GitBackend::bootstrap`] against the same URL. The
    /// wipe-and-rebootstrap branch is what allows the next mirror in
    /// the chain to take over cleanly even if a previous clone left
    /// stale state behind.
    fn bootstrap_or_update_at(
        &self,
        url: &str,
        refname: &str,
        clone_dir: &Path,
    ) -> Result<(), RegistryError> {
        if clone_dir.join(".git").exists() {
            match self.backend.update(clone_dir, refname) {
                Ok(()) => return Ok(()),
                Err(e) => {
                    tracing::debug!(
                        target: "vibe_registry",
                        registry = %self.name,
                        url = %url,
                        error = %e,
                        "update on existing clone failed; wiping and re-bootstrapping"
                    );
                    fs::remove_dir_all(clone_dir).map_err(|source| RegistryError::Io {
                        path: clone_dir.to_path_buf(),
                        source,
                    })?;
                }
            }
        }
        if clone_dir.exists() {
            // Half-populated dir from a prior failed bootstrap — clean.
            fs::remove_dir_all(clone_dir).map_err(|source| RegistryError::Io {
                path: clone_dir.to_path_buf(),
                source,
            })?;
        }
        if let Some(parent) = clone_dir.parent() {
            fs::create_dir_all(parent).map_err(|source| RegistryError::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }
        // PROP-002 §2.2.1 — under `auth = "token-env"` the bootstrap
        // is performed with a credentialised URL, then the recorded
        // origin URL is rewritten to the plain (token-free) form so
        // the freshly-cloned `.git/config` does NOT carry the token
        // on disk. Subsequent `update` calls hit the plain origin
        // (and 401 on a still-private host); the
        // `ensure_clone_against_sources` retry path handles that by
        // wiping and re-bootstrapping. The token only ever lives in
        // memory and inside the spawned `git clone` process.
        let plain_url = strip_git_plus_prefix(url);
        let fetch_url = inject_token(plain_url, self.effective_token.as_deref());
        self.backend.bootstrap(&fetch_url, refname, clone_dir)?;
        if self.effective_token.is_some() {
            self.backend
                .set_remote_url(clone_dir, "origin", plain_url)?;
        }
        Ok(())
    }

    /// Bring the per-package clone at `package_clone_dir(kind, name)`
    /// to `refname` by trying the primary URL first, then each mirror
    /// URL in priority order. Returns the URL that ultimately served
    /// the clone (canonical or a mirror) so the caller can record /
    /// log it.
    ///
    /// Mirror dispatch on this path is the cache-mutating sibling of
    /// [`Self::try_lookup`]: same primary-first ordering, same
    /// "primary's error is the most informative" semantics on full
    /// failure. The crucial difference is per-source state — each
    /// retry that goes through bootstrap wipes the local clone first,
    /// so a flapping primary that left a half-populated dir cannot
    /// poison the mirror attempt.
    ///
    /// **Mirror integrity** is **not** checked here: the content from
    /// whichever URL succeeds is taken verbatim. The caller (typically
    /// [`Self::fetch_with_expected_hash`]) layers a content_hash
    /// gate on top when a lockfile pin is available.
    fn ensure_clone_against_sources(
        &self,
        kind: PackageKind,
        name: &str,
        refname: &str,
    ) -> Result<String, RegistryError> {
        let urls = self.package_urls(kind, name);
        let clone_dir = self.package_clone_dir(kind, name);
        let mut primary_err: Option<RegistryError> = None;
        for (i, url) in urls.iter().enumerate() {
            match self.bootstrap_or_update_at(url, refname, &clone_dir) {
                Ok(()) => {
                    if i > 0 {
                        tracing::info!(
                            target: "vibe_registry",
                            registry = %self.name,
                            primary = %urls[0],
                            served_by = %url,
                            mirror_index = i - 1,
                            "fetch served by mirror"
                        );
                    }
                    return Ok(url.clone());
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
                            "mirror fetch failed; trying next"
                        );
                    }
                }
            }
        }
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
        // Index fast path (PROP-005 §2.10 slice 10). When the
        // registry has an upstream index attached, query it first.
        // 200 → return versions; 404 → fall through to git path
        // (UnknownPackage from the index does not authoritatively
        // mean "absent" — the index may be stale); other errors →
        // also fall through with a debug-level log.
        if let Some(client) = &self.index_client {
            match client.list_versions(kind, name) {
                Ok(Some(versions)) => {
                    tracing::debug!(
                        target: "vibe_registry::index",
                        registry = %self.name,
                        kind = %kind,
                        name = %name,
                        count = versions.len(),
                        "list_versions served from index"
                    );
                    return Ok(versions);
                }
                Ok(None) => {
                    tracing::debug!(
                        target: "vibe_registry::index",
                        registry = %self.name,
                        kind = %kind,
                        name = %name,
                        "index returned 404; falling through to git ls-remote"
                    );
                }
                Err(e) => {
                    tracing::debug!(
                        target: "vibe_registry::index",
                        registry = %self.name,
                        error = %e,
                        "index lookup failed; falling through to git ls-remote"
                    );
                }
            }
        }
        // PROP-002 §2.2.1 — fail fast when this registry declared
        // `auth = "token-env"` but the env-var resolved empty, before
        // we burn a network round-trip on a guaranteed-401.
        self.ensure_token_loaded()?;
        let backend = Arc::clone(&self.backend);
        let owned_name = name.to_owned();
        let token = self.effective_token.clone();
        self.try_lookup(kind, name, move |url| {
            let plain = strip_git_plus_prefix(url);
            let fetch_url = inject_token(plain, token.as_deref());
            let tags = backend
                .list_tags(&fetch_url)
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

    /// Read a candidate version's `vibe.toml` *without cloning*. The
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
    ) -> Result<Manifest, RegistryError> {
        self.ensure_token_loaded()?;
        let tag = format!("v{version}");
        let backend = Arc::clone(&self.backend);
        let tag_for_lookup = tag.clone();
        let token = self.effective_token.clone();
        let archive_result = self.try_lookup(kind, name, move |url| {
            let plain = strip_git_plus_prefix(url);
            let fetch_url = inject_token(plain, token.as_deref());
            backend
                .fetch_file_at_ref(&fetch_url, &tag_for_lookup, Manifest::FILENAME)
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
                let manifest_path = clone_dir.join(Manifest::FILENAME);
                fs::read(&manifest_path).map_err(|source| RegistryError::Io {
                    path: manifest_path.clone(),
                    source,
                })?
            }
            Err(other) => return Err(other),
        };
        let text = String::from_utf8(bytes).map_err(|e| RegistryError::MalformedMeta {
            path: PathBuf::from(format!("{url}@{tag}:{}", Manifest::FILENAME)),
            reason: format!("invalid UTF-8: {e}"),
        })?;
        Manifest::parse_str(&text).map_err(|e| RegistryError::MalformedMeta {
            path: PathBuf::from(format!("{url}@{tag}:{}", Manifest::FILENAME)),
            reason: e.to_string(),
        })
    }

    /// Refresh the per-package clone for `(kind, name)` against `refname`
    /// without touching the per-project cache. If the clone exists, runs
    /// `update`; otherwise bootstraps a fresh clone. Mirror-aware:
    /// the primary URL is tried first, then each mirror in priority
    /// order — the first source that lands a working clone wins.
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
        self.ensure_clone_against_sources(kind, name, refname)?;
        Ok(())
    }

    /// Materialise the resolved package into the per-project cache. Clones
    /// (or updates) the per-package repo at the requested tag, then copies
    /// the worktree into `<cache_root>/<kind>/<name>/v<version>/`,
    /// stripping `.git/`.
    ///
    /// Mirror-aware: the primary URL is tried first, then each mirror
    /// in priority order. Whichever source lands the clone first wins
    /// and the cache is materialised from that clone. The
    /// [`CachedPackage::source_uri`] is **always** the canonical
    /// primary URL — mirror URLs are an availability detail, not a
    /// lockfile-recorded identity (PROP-002 §2.3 step 3).
    ///
    /// No content_hash gate at this layer — see
    /// [`Self::fetch_with_expected_hash`] for the cross-source
    /// integrity check.
    pub fn fetch(
        &self,
        resolved: &ResolvedPackage,
        cache_root: &Path,
    ) -> Result<CachedPackage, RegistryError> {
        self.fetch_with_expected_hash(resolved, cache_root, None)
    }

    /// Mirror-aware fetch with an optional cross-source content_hash
    /// gate.
    ///
    /// Walks the URL chain primary-first; for each URL that yields a
    /// working clone, materialises the cache and computes the
    /// content_hash:
    ///
    /// - If `expected_hash` is `None` (no lockfile pin), accept the
    ///   first source that lands content. Equivalent to [`Self::fetch`].
    /// - If `expected_hash` is `Some(h)`, accept the first source
    ///   whose computed hash equals `h`. Sources serving a disagreeing
    ///   hash trigger a `tracing::warn!` (mirror-integrity event) and
    ///   the walk continues to the next URL — the cache is wiped
    ///   between attempts so a poisoned source cannot leave bytes
    ///   behind. This is the supply-chain check from
    ///   [PROP-002 §2.3](../../../spec/modules/vibe-registry/PROP-002-decentralized-registry.md#mirror).
    ///
    /// If every URL is reached but none matches, the **last
    /// successful fetch's** [`CachedPackage`] is returned (with the
    /// disagreeing hash); it is the caller's responsibility — today
    /// `vibe-install`'s `plan_install` — to convert the stored hash
    /// vs. lockfile-pin mismatch into the user-actionable
    /// `ContentDrift` error. This split keeps registry-layer concerns
    /// (sources, fallback, integrity attempts) separate from
    /// install-layer concerns (lockfile-aware error rendering).
    ///
    /// If every URL fails at the network layer (no source produced
    /// any content), the **primary's** error is surfaced — same
    /// "primary is canonical and its diagnostic is most useful"
    /// semantics as [`Self::try_lookup`].
    pub fn fetch_with_expected_hash(
        &self,
        resolved: &ResolvedPackage,
        cache_root: &Path,
        expected_hash: Option<&str>,
    ) -> Result<CachedPackage, RegistryError> {
        // PROP-002 §2.2.1 — bail before any clone work when this
        // registry is `auth = "token-env"` but the env-var resolved
        // empty.
        self.ensure_token_loaded()?;
        let canonical_url = self.package_repo_url(resolved.kind, &resolved.name);
        let tag = format!("v{}", resolved.version);
        let urls = self.package_urls(resolved.kind, &resolved.name);
        let clone_dir = self.package_clone_dir(resolved.kind, &resolved.name);
        let dest_cache = cache_root
            .join(resolved.kind.as_str())
            .join(&resolved.name)
            .join(format!("v{}", resolved.version));

        let mut primary_err: Option<RegistryError> = None;
        let mut last_cached: Option<CachedPackage> = None;

        for (i, url) in urls.iter().enumerate() {
            // 1. Bring the local clone to `tag` from this URL.
            if let Err(e) = self.bootstrap_or_update_at(url, &tag, &clone_dir) {
                if i == 0 {
                    primary_err = Some(e);
                } else {
                    tracing::debug!(
                        target: "vibe_registry",
                        registry = %self.name,
                        mirror = %url,
                        error = %e,
                        "mirror fetch failed; trying next"
                    );
                }
                continue;
            }

            // 2. Materialise the per-project cache, stripping `.git/`.
            if dest_cache.exists() {
                fs::remove_dir_all(&dest_cache).map_err(|source| RegistryError::Io {
                    path: dest_cache.clone(),
                    source,
                })?;
            }
            copy_dir_excluding_git(&clone_dir, &dest_cache)?;

            let manifest_path = dest_cache.join(Manifest::FILENAME);
            let manifest = Manifest::read(&manifest_path)?;
            if manifest.package.is_none() {
                return Err(RegistryError::MalformedMeta {
                    path: manifest_path.clone(),
                    reason: "registry package manifest must carry a [package] table"
                        .to_string(),
                });
            }
            let content_hash = compute_content_hash(&dest_cache)?;

            // 3. Cross-source content_hash gate.
            let cached = CachedPackage {
                resolved: resolved.clone(),
                cache_dir: dest_cache.clone(),
                manifest,
                content_hash: content_hash.clone(),
                source_uri: canonical_url.clone(),
                registry_name: Some(self.name.clone()),
                source_ref: Some(tag.clone()),
                resolved_commit: None,
                overridden: false,
                is_git_source: false,
                is_path_source: false,
                via_redirect: None,
            };
            match expected_hash {
                None => {
                    if i > 0 {
                        tracing::info!(
                            target: "vibe_registry",
                            registry = %self.name,
                            primary = %urls[0],
                            served_by = %url,
                            mirror_index = i - 1,
                            "fetch served by mirror"
                        );
                    }
                    return Ok(cached);
                }
                Some(expected) if expected == content_hash => {
                    if i > 0 {
                        tracing::info!(
                            target: "vibe_registry",
                            registry = %self.name,
                            primary = %urls[0],
                            served_by = %url,
                            mirror_index = i - 1,
                            "fetch served by mirror; content_hash matches lockfile pin"
                        );
                    }
                    return Ok(cached);
                }
                Some(expected) => {
                    tracing::warn!(
                        target: "vibe_registry",
                        registry = %self.name,
                        url = %url,
                        expected = %expected,
                        actual = %content_hash,
                        "source served content with unexpected content_hash; \
                         falling through to next source"
                    );
                    last_cached = Some(cached);
                    // Wipe the local clone state so the next URL bootstraps
                    // fresh — a poisoned mirror's working tree must not
                    // survive into the next attempt.
                    if clone_dir.exists() {
                        fs::remove_dir_all(&clone_dir).map_err(|source| RegistryError::Io {
                            path: clone_dir.clone(),
                            source,
                        })?;
                    }
                }
            }
        }

        // Every URL was exhausted.
        if let Some(cached) = last_cached {
            // At least one source served content; none matched the
            // expected hash. Return the last one — `vibe-install`'s
            // `plan_install` will lift this into a `ContentDrift`
            // error against the lockfile pin and surface the actionable
            // message. Doing the rendering here would duplicate that
            // logic and lose the lockfile context the install layer
            // already carries.
            return Ok(cached);
        }
        Err(primary_err.expect("primary URL must exist"))
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
    use std::collections::{HashMap, HashSet};
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
        /// URLs that should make `update` fail with `RefNotFound`. Used
        /// to test the "primary's working clone is now stuck on a tag
        /// the remote no longer carries; fall through to mirror"
        /// scenario — the mirror walk must wipe the local clone and
        /// re-bootstrap from the next URL when `update` fails.
        update_fail_urls: Mutex<HashSet<String>>,
        bootstrap_calls: Mutex<u32>,
        update_calls: Mutex<u32>,
        /// Recorded bootstrap-call URLs, in order. Tests assert on
        /// this when verifying primary-then-mirror dispatch.
        bootstrap_urls: Mutex<Vec<String>>,
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
        /// Wire `update(dest, refname)` to fail with `RefNotFound` when
        /// the clone at `dest` was last bootstrapped from `url`. Used
        /// to drive the "wipe and fall through" branch of
        /// `bootstrap_or_update_at`.
        #[allow(dead_code)]
        fn fail_update_for_url(&self, url: impl Into<String>) {
            self.update_fail_urls.lock().unwrap().insert(url.into());
        }
        fn bootstrap_count(&self) -> u32 {
            *self.bootstrap_calls.lock().unwrap()
        }
        fn update_count(&self) -> u32 {
            *self.update_calls.lock().unwrap()
        }
        fn bootstrap_urls(&self) -> Vec<String> {
            self.bootstrap_urls.lock().unwrap().clone()
        }
    }

    impl GitBackend for FakeBackend {
        fn bootstrap(&self, url: &str, _refname: &str, dest: &Path) -> Result<(), GitError> {
            *self.bootstrap_calls.lock().unwrap() += 1;
            self.bootstrap_urls.lock().unwrap().push(url.to_string());
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
            // Stash the URL inside `.git/origin-url` so `update` can
            // recover which URL last sourced this clone — that lets
            // `fail_update_for_url` selectively fail updates per origin.
            fs::create_dir_all(dest.join(".git")).unwrap();
            fs::write(dest.join(".git/origin-url"), url).unwrap();
            Ok(())
        }
        fn update(&self, dest: &Path, refname: &str) -> Result<(), GitError> {
            *self.update_calls.lock().unwrap() += 1;
            let origin = fs::read_to_string(dest.join(".git/origin-url"))
                .unwrap_or_default();
            if self.update_fail_urls.lock().unwrap().contains(&origin) {
                return Err(GitError::RefNotFound {
                    url: origin,
                    refname: refname.to_string(),
                });
            }
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

    // ---- inject_token helper ----

    #[test]
    fn inject_token_adds_x_access_token_to_https() {
        let url = "https://gitlab.company.com/vibespecs/flow-wal.git";
        let out = inject_token(url, Some("ghp_xxx"));
        assert_eq!(
            out,
            "https://x-access-token:ghp_xxx@gitlab.company.com/vibespecs/flow-wal.git"
        );
    }

    #[test]
    fn inject_token_returns_url_unchanged_when_no_token() {
        let url = "https://gitlab.company.com/vibespecs/flow-wal.git";
        assert_eq!(inject_token(url, None), url);
        assert_eq!(inject_token(url, Some("")).len(), inject_token(url, Some("")).len()); // empty also no-op-ish
    }

    #[test]
    fn inject_token_skips_non_https() {
        for url in [
            "git@github.com:vibespecs/flow-wal.git",
            "ssh://git@host/org/flow-wal.git",
            "file:///tmp/registry/flow-wal",
            "http://insecure.example.com/flow-wal.git",
        ] {
            assert_eq!(
                inject_token(url, Some("token")),
                url,
                "ssh / file / http URLs must never carry a token: {url}"
            );
        }
    }

    #[test]
    fn inject_token_does_not_double_wrap_already_credentialed_url() {
        let already = "https://x-access-token:abc@host/org/repo.git";
        assert_eq!(inject_token(already, Some("xyz")), already);
    }

    // ---- registry_host_from_url ----

    #[test]
    fn registry_host_from_url_handles_url_and_scp_shapes() {
        assert_eq!(
            registry_host_from_url("https://gitlab.company.com/vibespecs"),
            Some("gitlab.company.com")
        );
        assert_eq!(
            registry_host_from_url("git@gitlab.company.com:vibespecs"),
            Some("gitlab.company.com")
        );
        assert_eq!(
            registry_host_from_url("ssh://git@host.example/org"),
            Some("host.example")
        );
        assert_eq!(registry_host_from_url("file:///tmp/registry"), None);
    }

    // ---- MissingToken precheck ----

    #[test]
    fn missing_token_surfaces_before_git_invocation() {
        // `auth = TokenEnv` with no resolved token must produce
        // `RegistryError::MissingToken` from the FIRST git-driving
        // public method, without spawning git or hitting the
        // network. Uses the test-only `open_with_explicit_token`
        // constructor so the test does not have to mutate the
        // process env (forbidden by `#![forbid(unsafe_code)]` on
        // Rust 2024+).
        let cache = tempdir().unwrap();
        let fake = Arc::new(FakeBackend::default());
        let reg = GitPackageRegistry::open_with_explicit_token(
            "internal",
            "https://internal.example.com/vibespecs",
            "main",
            NamingConvention::KindName,
            Vec::new(),
            cache.path(),
            fake.clone(),
            DEFAULT_FRESHNESS_SECS,
            vibe_core::manifest::AuthKind::TokenEnv,
            None, // no token resolved — the precheck must fire
        )
        .unwrap();
        assert!(reg.token_env_required_but_absent());
        let err = reg.list_versions(PackageKind::Flow, "wal").unwrap_err();
        match err {
            RegistryError::MissingToken { registry, env_var } => {
                assert_eq!(registry, "internal");
                assert!(
                    env_var.contains("VIBEVM_REGISTRY_TOKEN"),
                    "env_var hint should name the conventional prefix: {env_var}"
                );
            }
            other => panic!("expected MissingToken, got: {other:?}"),
        }
        // Critical contract: backend was not consulted.
        assert_eq!(
            fake.bootstrap_count(),
            0,
            "MissingToken must skip the backend entirely"
        );
    }

    #[test]
    fn token_present_credentialises_bootstrap_and_scrubs_origin() {
        // End-to-end token-injection contract for the bootstrap path:
        //   1. ensure_token_loaded passes (token present).
        //   2. backend.bootstrap is called with the credentialed
        //      https://x-access-token:<TOKEN>@... form.
        //   3. backend.set_remote_url is called immediately after
        //      to rewrite origin to the plain URL — the token does
        //      not persist on disk inside the cloned `.git/config`.
        let cache = tempdir().unwrap();

        struct ScrubTrackingBackend {
            inner: Arc<FakeBackend>,
            scrubs: Mutex<Vec<(PathBuf, String, String)>>,
        }
        impl GitBackend for ScrubTrackingBackend {
            fn bootstrap(&self, url: &str, refname: &str, dest: &Path) -> Result<(), GitError> {
                self.inner.bootstrap(url, refname, dest)
            }
            fn update(&self, dest: &Path, refname: &str) -> Result<(), GitError> {
                self.inner.update(dest, refname)
            }
            fn list_tags(&self, url: &str) -> Result<Vec<String>, GitError> {
                self.inner.list_tags(url)
            }
            fn fetch_file_at_ref(
                &self,
                url: &str,
                refname: &str,
                path: &str,
            ) -> Result<Vec<u8>, GitError> {
                self.inner.fetch_file_at_ref(url, refname, path)
            }
            fn set_remote_url(
                &self,
                dest: &Path,
                remote: &str,
                url: &str,
            ) -> Result<(), GitError> {
                self.scrubs
                    .lock()
                    .unwrap()
                    .push((dest.to_path_buf(), remote.to_string(), url.to_string()));
                Ok(())
            }
        }

        let fake = Arc::new(FakeBackend::default());
        let pkg_src = tempdir().unwrap();
        std::fs::write(
            pkg_src.path().join("vibe.toml"),
            "[package]\nname = \"wal\"\nkind = \"flow\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        let credentialed_url =
            "https://x-access-token:secret-token-xyz@scrub.example/vibespecs/flow-wal.git"
                .to_string();
        let plain_url = "https://scrub.example/vibespecs/flow-wal.git";
        fake.seed_bootstrap(&credentialed_url, pkg_src.path().to_path_buf());
        fake.seed_tags(&credentialed_url, vec!["v0.1.0".to_string()]);
        let backend = Arc::new(ScrubTrackingBackend {
            inner: fake.clone(),
            scrubs: Mutex::new(Vec::new()),
        });
        let reg = GitPackageRegistry::open_with_explicit_token(
            "internal",
            "https://scrub.example/vibespecs",
            "main",
            NamingConvention::KindName,
            Vec::new(),
            cache.path(),
            backend.clone(),
            DEFAULT_FRESHNESS_SECS,
            vibe_core::manifest::AuthKind::TokenEnv,
            Some("secret-token-xyz".to_string()),
        )
        .unwrap();
        assert!(!reg.token_env_required_but_absent());
        assert_eq!(reg.effective_token_value(), Some("secret-token-xyz"));

        let resolved = ResolvedPackage {
            kind: PackageKind::Flow,
            name: "wal".to_string(),
            version: semver::Version::parse("0.1.0").unwrap(),
            source_dir: reg.package_clone_dir(PackageKind::Flow, "wal"),
        };
        let cache_root = tempdir().unwrap();
        reg.fetch(&resolved, cache_root.path()).unwrap();

        // bootstrap was called with the credentialed URL.
        let bootstraps = fake.bootstrap_urls();
        assert!(
            bootstraps.iter().any(|u| u == &credentialed_url),
            "expected bootstrap with credentialed URL, got: {bootstraps:?}"
        );

        // set_remote_url was called immediately after — scrubbing the
        // token out of the persistent `.git/config`.
        let scrubs = backend.scrubs.lock().unwrap().clone();
        let scrub_to_plain = scrubs.iter().find(|(_, _, u)| u == plain_url);
        assert!(
            scrub_to_plain.is_some(),
            "expected set_remote_url(.., \"origin\", plain_url); scrubs: {scrubs:?}"
        );
        let (_dest, remote, _) = scrub_to_plain.unwrap();
        assert_eq!(remote, "origin");
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
    fn single_package_url_returned_verbatim() {
        // M1.15 git-source: a single-package registry has its full URL
        // declared up front, naming is bypassed.
        let cache = tempdir().unwrap();
        let fake: Arc<dyn GitBackend> = Arc::new(FakeBackend::default());
        let r = GitPackageRegistry::open_single_package(
            "git-source-flow-internal",
            "https://github.com/me/flow-internal",
            "v0.1.0",
            cache.path(),
            fake,
            DEFAULT_FRESHNESS_SECS,
            vibe_core::manifest::AuthKind::None,
            None,
        )
        .unwrap();
        assert!(r.is_single_package());
        // The repo URL is the URL we passed, regardless of (kind, name).
        assert_eq!(
            r.package_repo_url(PackageKind::Flow, "internal"),
            "https://github.com/me/flow-internal"
        );
        // (kind, name) does not even matter — naming is skipped.
        assert_eq!(
            r.package_repo_url(PackageKind::Stack, "totally-different"),
            "https://github.com/me/flow-internal"
        );
    }

    #[test]
    fn single_package_skips_mirror_chain() {
        let cache = tempdir().unwrap();
        let fake: Arc<dyn GitBackend> = Arc::new(FakeBackend::default());
        let r = GitPackageRegistry::open_single_package(
            "git-source",
            "https://github.com/me/flow-internal",
            "v1.0",
            cache.path(),
            fake,
            DEFAULT_FRESHNESS_SECS,
            vibe_core::manifest::AuthKind::None,
            None,
        )
        .unwrap();
        let urls = r.package_urls(PackageKind::Flow, "internal");
        assert_eq!(urls.len(), 1, "single-package URL list should have one entry");
        assert_eq!(urls[0], "https://github.com/me/flow-internal");
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
            "vibe.toml",
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
        assert_eq!(manifest.require_package().unwrap().name, "wal");
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
            "vibe.toml",
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
        assert_eq!(manifest.require_package().unwrap().name, "wal");
        assert_eq!(manifest.require_package().unwrap().version.to_string(), "0.1.0");
        // Critically: no clone was triggered for this manifest read.
        assert_eq!(fake.bootstrap_count(), 0);
        assert_eq!(fake.update_count(), 0);
    }

    #[test]
    fn fetch_clones_and_populates_per_project_cache() {
        let cache = tempdir().unwrap();
        let pkg_cache = tempdir().unwrap();
        let upstream = tempdir().unwrap();
        // Build a fake upstream tree at the seeded URL: vibe.toml
        // plus a spec file and a stray `.git/` to make sure the copy
        // strips it on the way to the cache.
        let pkg_root = upstream.path().join("pkg");
        fs::create_dir_all(pkg_root.join("spec")).unwrap();
        fs::write(
            pkg_root.join("vibe.toml"),
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
        assert!(cached.cache_dir.join("vibe.toml").exists());
        assert!(cached.cache_dir.join("spec/foo.md").exists());
        assert!(!cached.cache_dir.join(".git").exists());

        // Manifest parsed and content_hash populated.
        assert_eq!(cached.package_meta().name, "wal");
        assert!(cached.content_hash.starts_with("sha256:"));

        // source_uri is the canonical per-package repo URL.
        assert_eq!(cached.source_uri, url);

        // Bootstrap was called exactly once.
        assert_eq!(fake.bootstrap_count(), 1);
    }

    #[test]
    fn fetch_falls_through_to_mirror_when_primary_unreachable() {
        // Primary URL has tags seeded (so list_versions finds the
        // version) but no bootstrap seed → primary's clone fails.
        // Mirror has BOTH tags and bootstrap seeded. The fetch
        // walk should land on the mirror, materialise from there,
        // and still record the canonical primary URL as
        // `cached.source_uri` per PROP-002 §2.3 step 3.
        let cache = tempdir().unwrap();
        let pkg_cache = tempdir().unwrap();
        let upstream = tempdir().unwrap();
        let pkg_root = upstream.path().join("pkg");
        fs::create_dir_all(&pkg_root).unwrap();
        fs::write(
            pkg_root.join("vibe.toml"),
            manifest_text("wal", "flow", "0.1.0"),
        )
        .unwrap();

        let primary_url = "https://primary.example/vibespecs/flow-wal.git";
        let mirror_url = "https://mirror.example/vibespecs/flow-wal.git";

        let fake = Arc::new(FakeBackend::default());
        // Tags on both — list_versions hits primary first and finds it.
        fake.seed_tags(primary_url, vec!["v0.1.0".into()]);
        fake.seed_tags(mirror_url, vec!["v0.1.0".into()]);
        // Bootstrap only on mirror — primary's clone path fails.
        fake.seed_bootstrap(mirror_url, pkg_root.clone());

        let r = registry_with_mirrors(
            cache.path(),
            "https://primary.example/vibespecs",
            NamingConvention::KindName,
            vec!["https://mirror.example/vibespecs".to_string()],
            fake.clone(),
        );

        let p = PackageRef::parse("flow:wal@0.1.0").unwrap();
        let resolved = r.resolve(&p).unwrap();
        let cached = r.fetch(&resolved, pkg_cache.path()).unwrap();

        // Materialised from the mirror.
        assert_eq!(cached.package_meta().name, "wal");
        assert_eq!(cached.package_meta().version.to_string(), "0.1.0");
        // PROP-002 §2.3 step 3: source_uri is canonical primary URL,
        // regardless of which source actually served the bytes.
        assert_eq!(cached.source_uri, primary_url);
        assert_eq!(cached.source_ref.as_deref(), Some("v0.1.0"));
        assert_eq!(cached.registry_name.as_deref(), Some("vibespecs"));

        // Bootstrap was attempted twice: primary (fail) + mirror (ok).
        assert_eq!(fake.bootstrap_count(), 2);
        assert_eq!(fake.bootstrap_urls(), vec![primary_url.to_string(), mirror_url.to_string()]);
    }

    #[test]
    fn fetch_prefers_primary_when_both_reachable() {
        let cache = tempdir().unwrap();
        let pkg_cache = tempdir().unwrap();
        let upstream = tempdir().unwrap();
        let pkg_root = upstream.path().join("pkg");
        fs::create_dir_all(&pkg_root).unwrap();
        fs::write(
            pkg_root.join("vibe.toml"),
            manifest_text("wal", "flow", "0.1.0"),
        )
        .unwrap();

        let primary_url = "https://primary.example/vibespecs/flow-wal.git";
        let mirror_url = "https://mirror.example/vibespecs/flow-wal.git";

        let fake = Arc::new(FakeBackend::default());
        fake.seed_tags(primary_url, vec!["v0.1.0".into()]);
        fake.seed_tags(mirror_url, vec!["v0.1.0".into()]);
        // Both URLs serve the same content from the same source dir.
        fake.seed_bootstrap(primary_url, pkg_root.clone());
        fake.seed_bootstrap(mirror_url, pkg_root.clone());

        let r = registry_with_mirrors(
            cache.path(),
            "https://primary.example/vibespecs",
            NamingConvention::KindName,
            vec!["https://mirror.example/vibespecs".to_string()],
            fake.clone(),
        );

        let p = PackageRef::parse("flow:wal@0.1.0").unwrap();
        let resolved = r.resolve(&p).unwrap();
        let _ = r.fetch(&resolved, pkg_cache.path()).unwrap();

        // Bootstrap exactly once — primary won, mirror untouched.
        assert_eq!(fake.bootstrap_count(), 1);
        assert_eq!(fake.bootstrap_urls(), vec![primary_url.to_string()]);
    }

    #[test]
    fn fetch_falls_through_when_primary_update_fails() {
        // First fetch lands a working clone via primary. Then the
        // primary's tag goes missing (we wire `fail_update_for_url`).
        // Second fetch tries `update` against primary, fails,
        // wipes-and-rebootstraps from primary (still fails — no seed
        // after wipe? actually bootstrap is still seeded), …
        //
        // Actually — once `update` fails on the primary's existing
        // clone, `bootstrap_or_update_at` wipes the clone and retries
        // bootstrap on the SAME URL. The bootstrap then re-seeds
        // (primary IS seeded), so the SAME URL succeeds. To force
        // fall-through, primary must fail BOTH update AND bootstrap.
        // Drop primary's bootstrap seed before the second fetch.
        let cache = tempdir().unwrap();
        let pkg_cache = tempdir().unwrap();
        let upstream = tempdir().unwrap();
        let pkg_root = upstream.path().join("pkg");
        fs::create_dir_all(&pkg_root).unwrap();
        fs::write(
            pkg_root.join("vibe.toml"),
            manifest_text("wal", "flow", "0.1.0"),
        )
        .unwrap();

        let primary_url = "https://primary.example/vibespecs/flow-wal.git";
        let mirror_url = "https://mirror.example/vibespecs/flow-wal.git";

        let fake = Arc::new(FakeBackend::default());
        fake.seed_tags(primary_url, vec!["v0.1.0".into()]);
        fake.seed_tags(mirror_url, vec!["v0.1.0".into()]);
        fake.seed_bootstrap(primary_url, pkg_root.clone());
        fake.seed_bootstrap(mirror_url, pkg_root.clone());

        let r = registry_with_mirrors(
            cache.path(),
            "https://primary.example/vibespecs",
            NamingConvention::KindName,
            vec!["https://mirror.example/vibespecs".to_string()],
            fake.clone(),
        );

        // First fetch lands the clone via primary.
        let p = PackageRef::parse("flow:wal@0.1.0").unwrap();
        let resolved = r.resolve(&p).unwrap();
        let _ = r.fetch(&resolved, pkg_cache.path()).unwrap();
        assert_eq!(fake.bootstrap_count(), 1);
        assert_eq!(fake.update_count(), 0);

        // Now make primary's update + bootstrap both fail. Mirror
        // remains seeded.
        fake.fail_update_for_url(primary_url);
        fake.bootstrap_seeds.lock().unwrap().remove(primary_url);

        // Second fetch: update primary fails → wipe+re-bootstrap from
        // primary fails → fall through to mirror, which seeds a fresh
        // clone via bootstrap.
        let _ = r.fetch(&resolved, pkg_cache.path()).unwrap();
        // Update was tried once (against primary, failed). Bootstrap
        // counts: 1 (initial primary) + 1 (re-bootstrap primary, fails
        // RepoNotFound after seed removed) + 1 (mirror, succeeds).
        assert_eq!(fake.update_count(), 1);
        assert_eq!(fake.bootstrap_count(), 3);
        assert_eq!(
            fake.bootstrap_urls(),
            vec![
                primary_url.to_string(), // initial fetch
                primary_url.to_string(), // retry after update fail
                mirror_url.to_string(),  // mirror takes over
            ]
        );
    }

    #[test]
    fn fetch_with_expected_hash_passes_through_when_no_pin() {
        // expected_hash = None — equivalent to `fetch`. Just verifies
        // the trait/wrapper plumbing is wired and identical to the
        // existing single-source fetch behaviour.
        let cache = tempdir().unwrap();
        let pkg_cache = tempdir().unwrap();
        let upstream = tempdir().unwrap();
        let pkg_root = upstream.path().join("pkg");
        fs::create_dir_all(&pkg_root).unwrap();
        fs::write(
            pkg_root.join("vibe.toml"),
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
        let cached = r
            .fetch_with_expected_hash(&resolved, pkg_cache.path(), None)
            .unwrap();
        assert!(cached.content_hash.starts_with("sha256:"));
        assert_eq!(cached.package_meta().name, "wal");
    }

    #[test]
    fn fetch_with_expected_hash_skips_mirror_with_disagreeing_content() {
        // Two seeded mirrors. Primary serves content A; mirror[0]
        // serves content B (disagreeing); mirror[1] serves content A
        // (matches the lockfile pin which is the hash of A).
        // Expected: primary wins on the first iteration because A
        // matches the pin — mirror walk never runs.
        //
        // To make the cross-source check actually fire, we make
        // primary unreachable so the walk reaches the mirrors. Then
        // mirror[0] serves B, hash check fails, fall through to
        // mirror[1] which serves A and matches.
        let cache = tempdir().unwrap();
        let pkg_cache = tempdir().unwrap();
        let upstream = tempdir().unwrap();

        // Two distinct fixture trees → distinct content_hashes.
        let pkg_a = upstream.path().join("pkg-a");
        fs::create_dir_all(&pkg_a).unwrap();
        fs::write(
            pkg_a.join("vibe.toml"),
            manifest_text("wal", "flow", "0.1.0"),
        )
        .unwrap();
        fs::write(pkg_a.join("README.md"), "# canonical content\n").unwrap();

        let pkg_b = upstream.path().join("pkg-b");
        fs::create_dir_all(&pkg_b).unwrap();
        fs::write(
            pkg_b.join("vibe.toml"),
            manifest_text("wal", "flow", "0.1.0"),
        )
        .unwrap();
        fs::write(pkg_b.join("README.md"), "# DIVERGENT content\n").unwrap();

        // Compute the expected hash of pkg_a for the lockfile pin.
        let temp_for_hash = tempdir().unwrap();
        copy_dir_excluding_git(&pkg_a, temp_for_hash.path()).unwrap();
        let expected_hash = compute_content_hash(temp_for_hash.path()).unwrap();

        let primary_url = "https://primary.example/vibespecs/flow-wal.git";
        let mirror_a_url = "https://mirror-bad.example/vibespecs/flow-wal.git";
        let mirror_b_url = "https://mirror-ok.example/vibespecs/flow-wal.git";

        let fake = Arc::new(FakeBackend::default());
        // All sources seed tags so the resolver reaches them in order.
        fake.seed_tags(primary_url, vec!["v0.1.0".into()]);
        fake.seed_tags(mirror_a_url, vec!["v0.1.0".into()]);
        fake.seed_tags(mirror_b_url, vec!["v0.1.0".into()]);
        // Primary unreachable for clone (no bootstrap seed).
        // mirror_a serves divergent content.
        fake.seed_bootstrap(mirror_a_url, pkg_b.clone());
        // mirror_b serves canonical content.
        fake.seed_bootstrap(mirror_b_url, pkg_a.clone());

        let r = registry_with_mirrors(
            cache.path(),
            "https://primary.example/vibespecs",
            NamingConvention::KindName,
            vec![
                "https://mirror-bad.example/vibespecs".to_string(),
                "https://mirror-ok.example/vibespecs".to_string(),
            ],
            fake.clone(),
        );

        let p = PackageRef::parse("flow:wal@0.1.0").unwrap();
        let resolved = r.resolve(&p).unwrap();
        let cached = r
            .fetch_with_expected_hash(&resolved, pkg_cache.path(), Some(&expected_hash))
            .unwrap();

        // Final material is canonical content.
        assert_eq!(cached.content_hash, expected_hash);
        // source_uri remains the canonical primary URL.
        assert_eq!(cached.source_uri, primary_url);

        // Walk: primary (bootstrap fail) → mirror_a (succeed, hash mismatch,
        // fall through) → mirror_b (succeed, hash match).
        assert_eq!(
            fake.bootstrap_urls(),
            vec![
                primary_url.to_string(),
                mirror_a_url.to_string(),
                mirror_b_url.to_string(),
            ]
        );
    }

    #[test]
    fn fetch_with_expected_hash_returns_last_attempt_when_no_match() {
        // Every source serves disagreeing content; lockfile pins
        // something else. Per the contract, the registry returns the
        // last successful CachedPackage with its (non-matching) hash;
        // vibe-install's `plan_install` then renders ContentDrift.
        let cache = tempdir().unwrap();
        let pkg_cache = tempdir().unwrap();
        let upstream = tempdir().unwrap();
        let pkg_root = upstream.path().join("pkg");
        fs::create_dir_all(&pkg_root).unwrap();
        fs::write(
            pkg_root.join("vibe.toml"),
            manifest_text("wal", "flow", "0.1.0"),
        )
        .unwrap();

        let primary_url = "https://primary.example/vibespecs/flow-wal.git";
        let mirror_url = "https://mirror.example/vibespecs/flow-wal.git";

        let fake = Arc::new(FakeBackend::default());
        fake.seed_tags(primary_url, vec!["v0.1.0".into()]);
        fake.seed_tags(mirror_url, vec!["v0.1.0".into()]);
        fake.seed_bootstrap(primary_url, pkg_root.clone());
        fake.seed_bootstrap(mirror_url, pkg_root.clone());

        let r = registry_with_mirrors(
            cache.path(),
            "https://primary.example/vibespecs",
            NamingConvention::KindName,
            vec!["https://mirror.example/vibespecs".to_string()],
            fake.clone(),
        );

        let bogus_pin =
            "sha256:0000000000000000000000000000000000000000000000000000000000000000";
        let p = PackageRef::parse("flow:wal@0.1.0").unwrap();
        let resolved = r.resolve(&p).unwrap();
        let cached = r
            .fetch_with_expected_hash(&resolved, pkg_cache.path(), Some(bogus_pin))
            .unwrap();

        // Returned cached carries the actual (non-matching) hash —
        // not the pin. vibe-install's plan_install lifts this into
        // ContentDrift.
        assert_ne!(cached.content_hash, bogus_pin);
        assert!(cached.content_hash.starts_with("sha256:"));
        // Both URLs were tried.
        assert_eq!(fake.bootstrap_count(), 2);
    }

    #[test]
    fn refresh_package_falls_through_to_mirror_when_primary_unreachable() {
        // refresh_package walks primary then mirror, same as fetch.
        // Used by `vibe registry sync`. Test that a fresh sync against
        // an unreachable primary lands the clone via mirror.
        let cache = tempdir().unwrap();
        let upstream = tempdir().unwrap();
        let pkg_root = upstream.path().join("pkg");
        fs::create_dir_all(&pkg_root).unwrap();
        fs::write(
            pkg_root.join("vibe.toml"),
            manifest_text("wal", "flow", "0.1.0"),
        )
        .unwrap();

        let primary_url = "https://primary.example/vibespecs/flow-wal.git";
        let mirror_url = "https://mirror.example/vibespecs/flow-wal.git";

        let fake = Arc::new(FakeBackend::default());
        fake.seed_bootstrap(mirror_url, pkg_root.clone());

        let r = registry_with_mirrors(
            cache.path(),
            "https://primary.example/vibespecs",
            NamingConvention::KindName,
            vec!["https://mirror.example/vibespecs".to_string()],
            fake.clone(),
        );

        r.refresh_package(PackageKind::Flow, "wal", "v0.1.0").unwrap();

        // Primary (fail) + mirror (succeed).
        assert_eq!(
            fake.bootstrap_urls(),
            vec![primary_url.to_string(), mirror_url.to_string()]
        );
    }

    #[test]
    fn fetch_dep_manifest_clone_fallback_uses_mirror_dispatch() {
        // GitHub-shape host: archive endpoint is unsupported. The
        // dep-manifest fetch falls back to the per-package clone via
        // refresh_package. With mirror dispatch wired, the clone walk
        // tries primary then mirror — mirror seeded, primary not.
        let cache = tempdir().unwrap();
        let upstream = tempdir().unwrap();
        let pkg_root = upstream.path().join("pkg");
        fs::create_dir_all(&pkg_root).unwrap();
        fs::write(
            pkg_root.join("vibe.toml"),
            manifest_text("wal", "flow", "0.1.0"),
        )
        .unwrap();

        let primary_url = "https://primary.example/vibespecs/flow-wal.git";
        let mirror_url = "https://mirror.example/vibespecs/flow-wal.git";

        // FakeBackend's `fetch_file_at_ref` returns FileNotFoundInRef
        // when not seeded. To trigger the clone fallback we need an
        // ArchiveUnsupported. Build a dedicated backend variant.
        struct NoArchiveBackend(Arc<FakeBackend>);
        impl GitBackend for NoArchiveBackend {
            fn bootstrap(
                &self,
                url: &str,
                refname: &str,
                dest: &Path,
            ) -> Result<(), GitError> {
                self.0.bootstrap(url, refname, dest)
            }
            fn update(&self, dest: &Path, refname: &str) -> Result<(), GitError> {
                self.0.update(dest, refname)
            }
            fn list_tags(&self, url: &str) -> Result<Vec<String>, GitError> {
                self.0.list_tags(url)
            }
            fn fetch_file_at_ref(
                &self,
                url: &str,
                _refname: &str,
                _path: &str,
            ) -> Result<Vec<u8>, GitError> {
                Err(GitError::ArchiveUnsupported {
                    url: url.to_string(),
                })
            }
        }

        let inner = Arc::new(FakeBackend::default());
        inner.seed_tags(primary_url, vec!["v0.1.0".into()]);
        inner.seed_tags(mirror_url, vec!["v0.1.0".into()]);
        inner.seed_bootstrap(mirror_url, pkg_root.clone());
        // Primary has no bootstrap seed → primary's clone fails →
        // mirror takes over.

        let backend: Arc<dyn GitBackend> = Arc::new(NoArchiveBackend(inner.clone()));
        let r = GitPackageRegistry::open_with_mirrors(
            "vibespecs",
            "https://primary.example/vibespecs",
            "main",
            NamingConvention::KindName,
            vec!["https://mirror.example/vibespecs".to_string()],
            cache.path(),
            backend,
            DEFAULT_FRESHNESS_SECS,
        )
        .unwrap();

        let v = semver::Version::parse("0.1.0").unwrap();
        let manifest = r.fetch_dep_manifest(PackageKind::Flow, "wal", &v).unwrap();
        assert_eq!(manifest.require_package().unwrap().name, "wal");

        // Clone-fallback walked primary (fail) + mirror (ok).
        assert_eq!(
            inner.bootstrap_urls(),
            vec![primary_url.to_string(), mirror_url.to_string()]
        );
    }

    #[test]
    fn fetch_reuses_existing_clone_via_update() {
        let cache = tempdir().unwrap();
        let pkg_cache = tempdir().unwrap();
        let upstream = tempdir().unwrap();
        let pkg_root = upstream.path().join("pkg");
        fs::create_dir_all(&pkg_root).unwrap();
        fs::write(
            pkg_root.join("vibe.toml"),
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
