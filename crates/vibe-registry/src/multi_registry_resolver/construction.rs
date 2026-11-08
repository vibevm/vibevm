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
        let mut resolver = Self::new(
            Vec::new(),
            mirrors.to_vec(),
            overrides.to_vec(),
            backend,
            cache_root,
        );
        resolver.catalog =
            catalog::Catalog::declared(registries.to_vec(), freshness_secs, admission);
        Ok(resolver)
    }

    /// Retain a deferred declaration provider without loading policy/catalog/tools.
    pub fn from_deferred_catalog(
        provider: Arc<dyn RegistryCatalogProvider>,
        backend: Arc<dyn GitBackend>,
        cache_root: PathBuf,
        freshness_secs: u64,
    ) -> Self {
        let mut value = Self::new(Vec::new(), Vec::new(), Vec::new(), backend, cache_root);
        value.catalog = catalog::Catalog::deferred(provider, freshness_secs);
        value
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
