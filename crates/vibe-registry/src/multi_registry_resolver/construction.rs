//! Registry construction binds transports before any index probe.
specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-model");

use super::*;

impl MultiRegistryResolver {
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
        Self::from_manifest_bound(
            registries,
            mirrors,
            overrides,
            cache_root,
            backend,
            freshness_secs,
            None,
        )
    }

    /// Construct the HTTP-only capability before opening or probing any registry.
    pub fn from_manifest_with_http_admission(
        registries: &[RegistrySection],
        mirrors: &[MirrorSection],
        overrides: &[OverrideSection],
        cache_root: PathBuf,
        freshness_secs: u64,
        admission: Arc<crate::HttpAdmission>,
    ) -> Result<Self, RegistryError> {
        let backend = Arc::new(ShellGit::new().with_http_admission(Arc::clone(&admission)));
        Self::from_manifest_bound(
            registries,
            mirrors,
            overrides,
            cache_root,
            backend,
            freshness_secs,
            Some(admission),
        )
    }

    /// Construct the separate direct HTTPS preparation profile before probes.
    pub fn from_manifest_with_native_admission(
        registries: &[RegistrySection],
        mirrors: &[MirrorSection],
        overrides: &[OverrideSection],
        cache_root: PathBuf,
        freshness_secs: u64,
        profile: Arc<crate::git_backend::shell::native_admission::NativeGitAdmission>,
    ) -> Result<Self, RegistryError> {
        for registry in registries.iter().filter(|registry| registry.enabled) {
            profile.preflight_registry_auth(registry.auth)?;
        }
        let admission = profile.http_admission();
        let backend = Arc::new(ShellGit::new().with_native_admission(profile));
        Self::from_manifest_bound(
            registries,
            mirrors,
            overrides,
            cache_root,
            backend,
            freshness_secs,
            Some(admission),
        )
    }

    fn from_manifest_bound(
        registries: &[RegistrySection],
        mirrors: &[MirrorSection],
        overrides: &[OverrideSection],
        cache_root: PathBuf,
        backend: Arc<dyn GitBackend>,
        freshness_secs: u64,
        admission: Option<Arc<crate::HttpAdmission>>,
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
            // PROP-066 `#locator`: the clone chain is the Locator's
            // candidate list for this registry, not a second redirection
            // mechanism beside it. A `[[mirror]]` user sees no change —
            // the mirror tables are read as rows of the local layer, in
            // the same priority order — and a `prefix`, `registry` or
            // `package` row now reaches the clone the same way.
            let mirror_urls = clone_chain(reg, mirrors, registries)?;

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
            // PROP-005 §2.2 INDEX-URL-CONFIG — the index-location
            // ladder: env `VIBEVM_INDEX_URL_<REGISTRY>` (operator,
            // per-run) > the `[[registry]].index_url` key (project
            // property) > the default `<registry-url>/index`. `none`
            // on either explicit step switches the index off. Probe is
            // best-effort; absent or unreachable index leaves the
            // registry on the existing git ls-remote path with no
            // warning.
            match crate::index_client::resolve_index_url(reg) {
                crate::index_client::IndexUrlResolution::Url { base: url, .. } => {
                    // A2-INDEXAUTH — authenticate to this registry's index
                    // with the registry's own credentials (bearer from
                    // `auth`/`token_env`, the same source the git side
                    // reads). `for_registry` is the scheme gate: a token is
                    // attached only over `https://`.
                    let auth = crate::index_client::IndexAuth::for_registry(reg, &url);
                    let outcome = match &admission {
                        Some(binding) => match crate::IndexClient::probe_with_admission(
                            &url,
                            auth,
                            Arc::clone(binding),
                        ) {
                            Ok(outcome) => outcome,
                            Err(crate::IndexError::ContactRefused(
                                crate::ContactRefusal::ExcludedHost { .. },
                            )) if backend.uses_native_materialisation() => {
                                tracing::debug!(registry=%reg.name,"index destination excluded; registry Git candidates remain independently admitted");
                                crate::index_client::ProbeOutcome::Absent
                            }
                            Err(error) => return Err(error.into()),
                        },
                        None => crate::IndexClient::probe(&url, auth),
                    };
                    match outcome {
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
                crate::index_client::IndexUrlResolution::Disabled => {
                    // `none` — an explicit step switched the index off
                    // for this registry; nothing to probe.
                    tracing::debug!(
                        target: "vibe_registry::index_client",
                        "index for registry `{}` disabled (index_url = \"none\")",
                        reg.name
                    );
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
}
