//! Fallible demand initialization of the existing registry catalog.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#LOCKED-SOURCE");
use super::*;
use std::sync::{Mutex, OnceLock};
/// Data required by the existing catalog owner, obtained only on new-name demand.
pub struct RegistryDeclarations {
    pub registries: Vec<RegistrySection>,
    pub mirrors: Vec<MirrorSection>,
    pub overrides: Vec<OverrideSection>,
    pub admission: Option<Arc<crate::HttpAdmission>>,
}
/// Demand source of the registry declarations, consulted only on new-name demand.
///
/// ```
/// use std::sync::Arc;
/// use vibe_registry::{MultiRegistryResolver, RegistryError, ShellGit};
/// use vibe_registry::multi_registry_resolver::{RegistryCatalogProvider, RegistryDeclarations};
///
/// struct NoRegistries;
/// impl RegistryCatalogProvider for NoRegistries {
///     fn declarations(&self) -> Result<RegistryDeclarations, RegistryError> {
///         Ok(RegistryDeclarations {
///             registries: Vec::new(),
///             mirrors: Vec::new(),
///             overrides: Vec::new(),
///             admission: None,
///         })
///     }
/// }
///
/// // Retained, not read: construction loads no catalog, policy or tool.
/// let _resolver = MultiRegistryResolver::from_deferred_catalog(
///     Arc::new(NoRegistries),
///     Arc::new(ShellGit::new()),
///     std::env::temp_dir().join("vibe-catalog-doctest"),
///     3600,
/// );
/// ```
pub trait RegistryCatalogProvider: Send + Sync {
    fn declarations(&self) -> Result<RegistryDeclarations, RegistryError>;
}
struct Definitions {
    registries: Vec<RegistrySection>,
    mirrors: Vec<MirrorSection>,
    overrides: HashMap<String, OverrideSection>,
    admission: Option<Arc<crate::HttpAdmission>>,
}
pub(super) struct Catalog {
    declared: Vec<RegistrySection>,
    provider: Option<Arc<dyn RegistryCatalogProvider>>,
    definitions: OnceLock<Definitions>,
    definition_gate: Mutex<()>,
    freshness: u64,
    admission: Option<Arc<crate::HttpAdmission>>,
    opened: OnceLock<Opened>,
    initialization: Mutex<()>,
    selected: Mutex<HashMap<String, Arc<RegistrySource>>>,
}
struct Opened {
    sources: Vec<RegistrySource>,
    registries: Vec<Arc<GitPerPackageRegistry>>,
}
impl Opened {
    fn new(sources: Vec<RegistrySource>) -> Self {
        let registries = sources
            .iter()
            .filter_map(|source| match source {
                RegistrySource::Git(registry) => Some(registry.clone()),
                RegistrySource::Local(_) => None,
            })
            .collect();
        Self {
            sources,
            registries,
        }
    }
}
impl Catalog {
    pub(super) fn opened(sources: Vec<RegistrySource>) -> Self {
        Self {
            declared: Vec::new(),
            provider: None,
            definitions: OnceLock::new(),
            definition_gate: Mutex::new(()),
            freshness: 0,
            admission: None,
            opened: OnceLock::from(Opened::new(sources)),
            initialization: Mutex::new(()),
            selected: Mutex::new(HashMap::new()),
        }
    }
    pub(super) fn declared(
        declared: Vec<RegistrySection>,
        freshness: u64,
        admission: Option<Arc<crate::HttpAdmission>>,
    ) -> Self {
        Self {
            declared,
            provider: None,
            definitions: OnceLock::new(),
            definition_gate: Mutex::new(()),
            freshness,
            admission,
            opened: OnceLock::new(),
            initialization: Mutex::new(()),
            selected: Mutex::new(HashMap::new()),
        }
    }
    pub(super) fn deferred(provider: Arc<dyn RegistryCatalogProvider>, freshness: u64) -> Self {
        let mut value = Self::declared(Vec::new(), freshness, None);
        value.provider = Some(provider);
        value
    }
    fn definitions(&self, resolver: &MultiRegistryResolver) -> Result<&Definitions, RegistryError> {
        if self.definitions.get().is_none() {
            let _guard = self
                .definition_gate
                .lock()
                .map_err(|_| RegistryError::MalformedMeta {
                    path: "registry.toml".into(),
                    reason: "catalog declaration lock poisoned".into(),
                })?;
            if self.definitions.get().is_none() {
                let value = match &self.provider {
                    Some(provider) => provider.declarations()?,
                    None => RegistryDeclarations {
                        registries: self.declared.clone(),
                        mirrors: resolver.mirrors.clone(),
                        overrides: resolver.overrides.values().cloned().collect(),
                        admission: self.admission.clone(),
                    },
                };
                let _ = self.definitions.set(Definitions {
                    registries: value.registries,
                    mirrors: value.mirrors,
                    overrides: value
                        .overrides
                        .into_iter()
                        .map(|row| (row.pkgref.clone(), row))
                        .collect(),
                    admission: value.admission,
                });
            }
        }
        self.definitions
            .get()
            .ok_or_else(|| RegistryError::MalformedMeta {
                path: "registry.toml".into(),
                reason: "catalog declaration initialization produced no data".into(),
            })
    }
    pub(super) fn declared_registry_rows(
        &self,
        resolver: &MultiRegistryResolver,
    ) -> Result<Vec<RegistrySection>, RegistryError> {
        Ok(self.definitions(resolver)?.registries.clone())
    }
    pub(super) fn mirrors<'a>(
        &'a self,
        resolver: &MultiRegistryResolver,
    ) -> Result<&'a [MirrorSection], RegistryError> {
        Ok(&self.definitions(resolver)?.mirrors)
    }
    pub(super) fn overrides<'a>(
        &'a self,
        resolver: &MultiRegistryResolver,
    ) -> Result<&'a HashMap<String, OverrideSection>, RegistryError> {
        Ok(&self.definitions(resolver)?.overrides)
    }
    /// Capture one row without opening unrelated alternatives. Successful row
    /// owners are reused by inventory, token sealing and later acquisition.
    pub(super) fn selected(
        &self,
        resolver: &MultiRegistryResolver,
        name: &str,
    ) -> Result<Arc<RegistrySource>, RegistryError> {
        let invalid = |reason: &str| RegistryError::MalformedMeta {
            path: "registry.toml".into(),
            reason: reason.into(),
        };
        let mut retained = self
            .selected
            .lock()
            .map_err(|_| invalid("catalog row initialization lock poisoned"))?;
        if let Some(source) = retained.get(name) {
            return Ok(source.clone());
        }
        // Direct constructor callers already supplied opened immutable owners.
        if let Some(opened) = self.opened.get() {
            let mut matches = opened.sources.iter().filter(|source| source.name() == name);
            if let Some(source) = matches.next() {
                if matches.next().is_some() {
                    return Err(invalid("ambiguous duplicate registry name"));
                }
                let source = Arc::new(source.clone());
                retained.insert(name.into(), source.clone());
                return Ok(source);
            }
        }
        let definitions = self.definitions(resolver)?;
        let mut matches = definitions.registries.iter().filter(|row| row.name == name);
        let row = matches
            .next()
            .ok_or_else(|| invalid("selected registry name is not declared"))?;
        if matches.next().is_some() {
            return Err(invalid("ambiguous duplicate registry name"));
        }
        if !row.enabled {
            return Err(invalid("selected registry is disabled"));
        }
        let source = Arc::new(construct_source(
            row,
            &definitions.registries,
            &definitions.mirrors,
            &resolver.cache_root,
            &resolver.backend,
            self.freshness,
            &definitions.admission,
        )?);
        if let RegistrySource::Git(registry) = source.as_ref()
            && let Some(transfers) = resolver.progress.transfers()
        {
            registry.attach_index_transfers(transfers.clone());
        }
        retained.insert(name.into(), source.clone());
        Ok(source)
    }
    pub(super) fn attach_selected_transfers(
        &self,
        transfers: &vibe_core::progress::transfer::Transfers,
    ) {
        if let Ok(retained) = self.selected.lock() {
            for source in retained.values() {
                if let RegistrySource::Git(registry) = source.as_ref() {
                    registry.attach_index_transfers(transfers.clone());
                }
            }
        }
    }
    pub(super) fn initialized_registries(&self) -> &[Arc<GitPerPackageRegistry>] {
        self.opened
            .get()
            .map_or(&[], |opened| opened.registries.as_slice())
    }
    fn demand(&self, resolver: &MultiRegistryResolver) -> Result<&Opened, RegistryError> {
        let definitions = self.definitions(resolver)?;
        if self.opened.get().is_none() {
            let _guard = self
                .initialization
                .lock()
                .map_err(|_| RegistryError::MalformedMeta {
                    path: PathBuf::from("registry.toml"),
                    reason: "catalog initialization lock poisoned".into(),
                })?;
            if self.opened.get().is_none() {
                let sources = definitions
                    .registries
                    .iter()
                    .filter(|row| row.enabled)
                    .map(|row| {
                        self.selected(resolver, &row.name)
                            .map(|source| source.as_ref().clone())
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                let opened = Opened::new(sources);
                if let Some(transfers) = resolver.progress.transfers() {
                    for registry in &opened.registries {
                        registry.attach_index_transfers(transfers.clone());
                    }
                }
                let _ = self.opened.set(opened);
            }
        }
        self.opened
            .get()
            .ok_or_else(|| RegistryError::MalformedMeta {
                path: PathBuf::from("registry.toml"),
                reason: "catalog initialization produced no source set".into(),
            })
    }
    pub(super) fn sources(
        &self,
        resolver: &MultiRegistryResolver,
    ) -> Result<&[RegistrySource], RegistryError> {
        Ok(&self.demand(resolver)?.sources)
    }
    pub(super) fn registries(
        &self,
        resolver: &MultiRegistryResolver,
    ) -> Result<&[Arc<GitPerPackageRegistry>], RegistryError> {
        Ok(&self.demand(resolver)?.registries)
    }
}
fn construct_source(
    reg: &RegistrySection,
    registries: &[RegistrySection],
    mirrors: &[MirrorSection],
    cache_root: &Path,
    backend: &Arc<dyn GitBackend>,
    freshness_secs: u64,
    admission: &Option<Arc<crate::HttpAdmission>>,
) -> Result<RegistrySource, RegistryError> {
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
        return Ok(RegistrySource::Local(LocalRegistrySource {
            name: reg.name.clone(),
            url: reg.url.clone(),
            registry: LocalRegistry::new(path)?,
        }));
    }
    // PROP-066 `#locator`: the clone chain is the Locator's
    // candidate list for this registry, not a second redirection
    // mechanism beside it. A `[[mirror]]` user sees no change —
    // the mirror tables are read as rows of the local layer, in
    // the same priority order — and a `prefix`, `registry` or
    // `package` row now reaches the clone the same way.
    let mirror_routes = super::clone_chain::clone_chain_scoped(reg, mirrors, registries)?;
    let mirror_urls = mirror_routes
        .iter()
        .map(|route| route.url.clone())
        .collect();

    // PROP-002 §2.2.1 — thread the registry's auth regime and
    // the explicit-or-derived token env-var name into the
    // registry instance, so it can pre-flight `MissingToken`
    // errors and inject the token into per-package URLs at
    // git invocation time.
    let auth = crate::resolve_registry_auth(crate::RegistryAuthInput::SelectedRegistry(reg))?;
    let mut entry = GitPerPackageRegistry::open_with_deferred_auth(
        &reg.name,
        &reg.url,
        &reg.r#ref,
        reg.naming,
        mirror_urls,
        cache_root,
        backend.clone(),
        freshness_secs,
        auth.kind,
        auth.token_env_name.as_deref(),
    )?
    .with_mirror_publication(mirror_routes);
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
            let auth = reg.clone();
            entry = entry.with_index_discovery(
                url,
                auth,
                admission.clone(),
                backend.native_materialisation_for(&reg.url)?,
            );
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
    Ok(RegistrySource::Git(Arc::new(entry)))
}
