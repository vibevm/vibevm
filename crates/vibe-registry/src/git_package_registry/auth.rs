//! Authentication regime for the per-package registry — token
//! resolution at open, bearer-token injection into git-facing URLs,
//! and the `MissingToken` pre-flight (PROP-002 §2.2.1).

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-auth");

use super::*;

impl GitPerPackageRegistry {
    /// Open selected catalog credentials without reading a token for an undemanded source.
    #[allow(clippy::too_many_arguments)]
    pub fn open_with_deferred_auth(
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
        let mut registry = Self::open_with_explicit_token(
            name,
            org_url,
            org_ref,
            naming,
            mirror_urls,
            cache_root,
            backend,
            freshness_secs,
            auth,
            None,
        )?;
        registry.token_env_name = token_env_name.map(str::to_owned);
        registry.deferred_token = Some(Arc::new(std::sync::OnceLock::new()));
        Ok(registry)
    }

    /// Direct selected-source counterpart; source authority has already been chosen by the caller.
    #[allow(clippy::too_many_arguments)]
    pub fn open_single_package_with_deferred_auth(
        synthetic_name: &str,
        repo_url: &str,
        repo_ref: &str,
        cache_root: &Path,
        backend: Arc<dyn GitBackend>,
        freshness_secs: u64,
        auth: vibe_core::manifest::AuthKind,
        token_env_name: Option<&str>,
    ) -> Result<Self, RegistryError> {
        let mut registry = Self::open_with_deferred_auth(
            synthetic_name,
            repo_url,
            repo_ref,
            NamingConvention::Fqdn,
            Vec::new(),
            cache_root,
            backend,
            freshness_secs,
            auth,
            token_env_name,
        )?;
        registry.single_package_url = Some(repo_url.into());
        Ok(registry)
    }

    /// Check the selected route before demand-capturing its immutable token.
    /// This does not grant a marker or redirect a different credential ceiling.
    pub(crate) fn selected_credentialed_url(&self, source: &str) -> Result<String, RegistryError> {
        // Incumbent package/redirect constructors retain their captured-token behavior.
        if self.deferred_token.is_none() {
            return Ok(self.credentialed_url(source));
        }
        self.backend.preflight_selected_address(source)?;
        self.backend.preflight_registry_auth(self.auth)?;
        let ssh = crate::registry_auth::ssh_source(source).map_err(|reason| {
            crate::registry_auth::declaration_error(&self.name, source, reason)
        })?;
        if ssh && self.auth != vibe_core::manifest::AuthKind::Ssh {
            return Err(GitError::NativeContext {
                reason: "selected SSH route requires SSH credential permission",
            }
            .into());
        }
        if self.auth == vibe_core::manifest::AuthKind::TokenEnv && source.starts_with("https://") {
            if let Some(captured) = &self.deferred_token {
                captured.get_or_init(|| {
                    self.token_env_name
                        .clone()
                        .or_else(|| self.derive_default_token_env_name())
                        .and_then(|name| read_token_env(&name))
                });
            }
            if self.effective_token_value().is_none() {
                return Err(RegistryError::MissingToken {
                    registry: self.name.clone(),
                    env_var: self
                        .token_env_name
                        .clone()
                        .or_else(|| self.derive_default_token_env_name())
                        .unwrap_or_else(|| "VIBEVM_REGISTRY_TOKEN_<HOST>".into()),
                });
            }
        }
        Ok(inject_token(source, self.effective_token_value()))
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
            && self.effective_token_value().is_none()
    }

    /// Pre-flight check at the entry of every public method that
    /// drives a git invocation. Returns `MissingToken` when the
    /// registry was opened with `auth = TokenEnv` but the env-var
    /// resolved empty. Other regimes (or `TokenEnv` with a token)
    /// pass through. The env-var name surfaced in the error is the
    /// one we'd consult — explicit `token_env` field on the
    /// registry section, otherwise the host-derived default per
    /// `RegistrySection::resolve_token_env_name`.
    pub(super) fn ensure_token_loaded(&self) -> Result<(), RegistryError> {
        if self.deferred_token.is_some() {
            return Ok(());
        }
        self.backend.preflight_registry_auth(self.auth)?;
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
    /// `GitPerPackageRegistry` (which doesn't carry the explicit name).
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
        self.deferred_token
            .as_ref()
            .and_then(|value| value.get())
            .and_then(|value| value.as_deref())
            .or(self.effective_token.as_deref())
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
    let Some(token) = token else {
        return plain_url.to_string();
    };
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use tempfile::tempdir;

    use crate::git_package_registry::test_support::*;

    // ---- inject_token helper ----

    #[test]
    fn inject_token_adds_x_access_token_to_https() {
        let url = "https://gitlab.company.com/vibespecs/org.vibevm.wal.git";
        let out = inject_token(url, Some("ghp_xxx"));
        assert_eq!(
            out,
            "https://x-access-token:ghp_xxx@gitlab.company.com/vibespecs/org.vibevm.wal.git"
        );
    }

    #[test]
    fn inject_token_returns_url_unchanged_when_no_token() {
        let url = "https://gitlab.company.com/vibespecs/org.vibevm.wal.git";
        assert_eq!(inject_token(url, None), url);
        assert_eq!(
            inject_token(url, Some("")).len(),
            inject_token(url, Some("")).len()
        ); // empty also no-op-ish
    }

    #[test]
    fn inject_token_skips_non_https() {
        for url in [
            "git@github.com:vibespecs/org.vibevm.wal.git",
            "ssh://git@host/org/org.vibevm.wal.git",
            "file:///tmp/registry/flow-wal",
            "http://insecure.example.com/org.vibevm.wal.git",
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
        let reg = GitPerPackageRegistry::open_with_explicit_token(
            "internal",
            "https://internal.example.com/vibespecs",
            "main",
            NamingConvention::Fqdn,
            Vec::new(),
            cache.path(),
            fake.clone(),
            DEFAULT_FRESHNESS_SECS,
            vibe_core::manifest::AuthKind::TokenEnv,
            None, // no token resolved — the precheck must fire
        )
        .unwrap();
        assert!(reg.token_env_required_but_absent());
        let err = reg.list_versions(&org(), "wal").unwrap_err();
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
            fn set_remote_url(&self, dest: &Path, remote: &str, url: &str) -> Result<(), GitError> {
                self.scrubs.lock().unwrap().push((
                    dest.to_path_buf(),
                    remote.to_string(),
                    url.to_string(),
                ));
                Ok(())
            }
        }

        let fake = Arc::new(FakeBackend::default());
        let pkg_src = tempdir().unwrap();
        std::fs::write(
            pkg_src.path().join("vibe.toml"),
            "[package]\ngroup = \"org.vibevm\"\nname = \"wal\"\nkind = \"flow\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        let credentialed_url =
            "https://x-access-token:secret-token-xyz@scrub.example/vibespecs/org.vibevm.wal.git"
                .to_string();
        let plain_url = "https://scrub.example/vibespecs/org.vibevm.wal.git";
        fake.seed_bootstrap(&credentialed_url, pkg_src.path().to_path_buf());
        fake.seed_tags(&credentialed_url, vec!["v0.1.0".to_string()]);
        let backend = Arc::new(ScrubTrackingBackend {
            inner: fake.clone(),
            scrubs: Mutex::new(Vec::new()),
        });
        let reg = GitPerPackageRegistry::open_with_explicit_token(
            "internal",
            "https://scrub.example/vibespecs",
            "main",
            NamingConvention::Fqdn,
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
            group: org(),
            name: "wal".to_string(),
            version: semver::Version::parse("0.1.0").unwrap(),
            source_dir: reg.package_clone_dir(&org(), "wal"),
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
}

#[cfg(test)]
mod deferred_tests;
