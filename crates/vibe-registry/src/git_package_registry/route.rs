//! Retained registry candidate identity; this is not transport authority.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-ROUTES");
use super::*;

/// Address and report origin travel together through the existing candidate chain.
#[derive(Clone)]
pub(crate) struct PublicationRoute {
    pub url: String,
    pub publication: vibe_core::RoutePublication,
}
impl PublicationRoute {
    pub(crate) fn local(url: String) -> Self {
        Self {
            url,
            publication: vibe_core::RoutePublication::Local,
        }
    }
}
struct AcquisitionOwner {
    registry: Arc<GitPerPackageRegistry>,
    manifest: Manifest,
    commit: Option<String>,
    serving: Option<String>,
    native: Option<std::sync::Mutex<Option<crate::git_package_registry::RetainedNativePackage>>>,
}
/// Runtime-only selected registry candidate. Canonical lock provenance remains
/// separate. Private fields prevent retargeting a selection to another package.
#[derive(Clone)]
pub struct RegistryRoute {
    registry: String,
    group: Group,
    name: String,
    version: semver::Version,
    candidate: String,
    canonical: String,
    publication: vibe_core::RoutePublication,
    acquisition: Option<Arc<AcquisitionOwner>>,
    reference: Option<String>,
}
impl std::fmt::Debug for RegistryRoute {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RegistryRoute")
            .field("registry", &self.registry)
            .field("group", &self.group)
            .field("name", &self.name)
            .field("version", &self.version)
            .finish_non_exhaustive()
    }
}
impl RegistryRoute {
    pub(super) fn selected(
        registry: &GitPerPackageRegistry,
        resolved: &ResolvedPackage,
        candidate: PublicationRoute,
    ) -> Result<Self, RegistryError> {
        Ok(Self {
            registry: registry.name.clone(),
            group: resolved.group.clone(),
            name: resolved.name.clone(),
            version: resolved.version.clone(),
            canonical: registry.package_repo_url(&resolved.group, &resolved.name)?,
            candidate: candidate.url,
            publication: candidate.publication,
            reference: Some(registry.package_reference(resolved)),
            acquisition: None,
        })
    }
    pub(crate) fn selected_address(&self) -> &str {
        &self.candidate
    }
    pub(crate) fn publication_scope(&self) -> vibe_core::RoutePublication {
        self.publication
    }
    /// Redacted observation for reports, not an address for acquisition.
    pub(crate) fn observed_endpoint(&self) -> Option<String> {
        self.acquisition
            .as_ref()?
            .serving
            .as_deref()
            .map(vibe_core::progress::transfer::safe_remote)
    }
    pub(crate) fn metadata_commit(&self) -> Option<&str> {
        self.acquisition.as_ref()?.commit.as_deref()
    }
    pub(crate) fn captured_manifest(&self) -> Option<&Manifest> {
        self.acquisition.as_ref().map(|owner| &owner.manifest)
    }
    fn with_acquisition(
        mut self,
        registry: GitPerPackageRegistry,
        manifest: Manifest,
        commit: Option<String>,
        serving: Option<String>,
        native: Option<crate::git_package_registry::RetainedNativePackage>,
    ) -> Self {
        self.publication = self.publication.observed(
            &self.candidate,
            serving.as_deref().unwrap_or(&self.candidate),
        );
        self.acquisition = Some(Arc::new(AcquisitionOwner {
            registry: Arc::new(registry),
            manifest,
            commit,
            serving,
            native: native.map(|value| std::sync::Mutex::new(Some(value))),
        }));
        self
    }
    pub(crate) fn acquire_captured(
        &self,
        resolution: &crate::MultiResolution,
        manifest: &Manifest,
        store: &Path,
        expected: Option<&str>,
        slot: Option<&Path>,
    ) -> Result<PackageAcquisition, RegistryError> {
        let resolved = &resolution.resolved;
        let canonical = resolution.source_url.as_str();
        let reference =
            resolution
                .source_ref
                .as_deref()
                .ok_or_else(|| RegistryError::MalformedMeta {
                    path: "retained-source".into(),
                    reason: "retained source has no ref".into(),
                })?;
        self.validate_reference(reference)?;
        if self.group != resolved.group
            || self.name != resolved.name
            || self.version != resolved.version
            || self.canonical != canonical
        {
            return Err(RegistryError::MalformedMeta {
                path: "retained-source".into(),
                reason: "retained registry identity changed".into(),
            });
        }
        let owner = self
            .acquisition
            .as_ref()
            .ok_or_else(|| RegistryError::MalformedMeta {
                path: "retained-source".into(),
                reason: "registry route has no captured metadata owner".into(),
            })?;
        if &owner.manifest != manifest {
            return Err(RegistryError::MalformedMeta {
                path: "retained-source".into(),
                reason: "retained manifest changed".into(),
            });
        }
        if let Some(native) = &owner.native {
            let source = native
                .lock()
                .map_err(|_| RegistryError::MalformedMeta {
                    path: "retained-source".into(),
                    reason: "retained source lock poisoned".into(),
                })?
                .take()
                .ok_or_else(|| RegistryError::MalformedMeta {
                    path: "retained-source".into(),
                    reason: "retained source already consumed".into(),
                })?;
            return owner
                .registry
                .consume_retained_native(source, resolved, store, expected, slot);
        }
        let reference = owner
            .commit
            .as_deref()
            .ok_or_else(|| RegistryError::MalformedMeta {
                path: "retained-source".into(),
                reason: "legacy metadata has no observed commit".into(),
            })?;
        let selected = owner
            .registry
            .configured_redirect(&resolved.group, &resolved.name, &self.candidate, reference)?
            .with_source_publication(self.publication);
        if manifest.require_package()?.materialization.is_in_place()
            && let Some(slot) = slot
        {
            return selected
                .prepare_locked_selection(resolved, slot, true)
                .map(PackageAcquisition::InPlace);
        }
        selected.fetch_locked_selection(resolved, store, expected, None)
    }
    pub(crate) fn validate_reference(&self, reference: &str) -> Result<(), RegistryError> {
        if self
            .reference
            .as_deref()
            .is_some_and(|selected| selected != reference)
        {
            return Err(RegistryError::MalformedMeta {
                path: PathBuf::from("vibe-redirect.toml"),
                reason: "redirect ref changed after route selection".into(),
            });
        }
        Ok(())
    }
    pub(crate) fn validate<'a>(
        &'a self,
        registry: &GitPerPackageRegistry,
        resolved: &ResolvedPackage,
    ) -> Result<&'a str, RegistryError> {
        let (primary, mirrors) = registry.package_urls(&resolved.group, &resolved.name)?;
        if self.registry != registry.name
            || self.group != resolved.group
            || self.name != resolved.name
            || self.version != resolved.version
            || !std::iter::once(&primary)
                .chain(mirrors.iter())
                .any(|url| url == &self.candidate)
        {
            return Err(RegistryError::MalformedMeta {
                path: resolved.source_dir.clone(),
                reason:
                    "registry candidate no longer belongs to the selected coordinate and registry"
                        .into(),
            });
        }
        Ok(&self.candidate)
    }
}
pub(crate) fn excluded_route(error: &RegistryError) -> bool {
    matches!(
        error,
        RegistryError::ContactRefused(crate::ContactRefusal::ExcludedHost { .. })
            | RegistryError::Git(GitError::ContactRefused(
                crate::ContactRefusal::ExcludedHost { .. }
            ))
    )
}
pub(super) fn terminal_lookup_error(error: &RegistryError) -> bool {
    matches!(
        error,
        RegistryError::ContactRefused(_)
            | RegistryError::MalformedMeta { .. }
            | RegistryError::Git(
                GitError::ContactRefused(_)
                    | GitError::SshHostKeyRejected
                    | GitError::SshAuthenticationRejected
                    | GitError::NativeUnavailable { .. }
                    | GitError::NativeContext { .. }
                    | GitError::PreparedValidation { .. }
                    | GitError::PreparedCheckout { .. }
                    | GitError::PreparedCheckoutUnsupported
            )
    )
}

impl GitPerPackageRegistry {
    pub(crate) fn checkout_directory(&self, group: &Group, name: &str) -> PathBuf {
        self.package_clone_dir(group, name)
    }
    pub(crate) fn retain_redirect_route(
        &self,
        resolved: &ResolvedPackage,
        url: &str,
        reference: &str,
    ) -> Result<RegistryRoute, RegistryError> {
        let mut route = RegistryRoute::selected(
            self,
            resolved,
            PublicationRoute {
                url: url.into(),
                publication: self.source_publication,
            },
        )?;
        route.validate(self, resolved)?;
        route.reference = Some(reference.into());
        Ok(route)
    }
    pub(crate) fn contains_candidate(
        &self,
        group: &Group,
        name: &str,
        url: &str,
    ) -> Result<bool, RegistryError> {
        let (primary, mirrors) = self.package_urls(group, name)?;
        Ok(std::iter::once(primary)
            .chain(mirrors)
            .any(|candidate| strip_git_plus_prefix(&candidate) == strip_git_plus_prefix(url)))
    }
    /// Select an explicit URL/ref using this registry's existing immutable
    /// credential binding and captured token, never a marker-named environment.
    pub(crate) fn configured_redirect(
        &self,
        group: &Group,
        name: &str,
        url: &str,
        reference: &str,
    ) -> Result<Self, RegistryError> {
        if !self.contains_candidate(group, name, url)? {
            return Err(RegistryError::MalformedMeta {
                path: PathBuf::from("vibe-redirect.toml"),
                reason: "redirect target is outside the configured registry candidate chain".into(),
            });
        }
        // The endpoint binding is per package, but the cache bucket stays the
        // configured registry's: `<cache>/<hash(registry url)>/packages/...`
        // is where `registry sync` and later lookups find the clone.
        let target = Self {
            backend: bind_registry_backend(Arc::clone(&self.backend), self.auth)?,
            name: self.name.clone(),
            org_url: url.into(),
            org_ref: reference.into(),
            naming: self.naming,
            auth: self.auth,
            effective_token: self.effective_token.clone(),
            deferred_token: self.deferred_token.clone(),
            token_env_name: self.token_env_name.clone(),
            cache_root: self.cache_root.clone(),
            canonical_hash: self.canonical_hash.clone(),
            mirror_urls: vec![],
            source_publication: vibe_core::RoutePublication::Local,
            single_package_url: Some(url.into()),
            locked_source: false,
            index_client: std::sync::OnceLock::new(),
            index_discovery: None,
            index_transfers: std::sync::OnceLock::new(),
            freshness_secs: self.freshness_secs,
        };
        Ok(target)
    }
}

impl GitPerPackageRegistry {
    pub(crate) fn fetch_dep_manifest_observed(
        &self,
        resolved: &ResolvedPackage,
        retained: Option<&RegistryRoute>,
    ) -> Result<(Manifest, RegistryRoute), RegistryError> {
        let reference = self.package_reference(resolved);
        if let Some(route) = retained {
            route.validate(self, resolved)?;
            route.validate_reference(&reference)?;
            if let Some(manifest) = route.captured_manifest() {
                return Ok((manifest.clone(), route.clone()));
            }
            return self.observe_candidate(resolved, &reference, route.clone());
        }
        let (primary, mirrors) = self.package_candidates(&resolved.group, &resolved.name)?;
        let mut failure = None;
        let mut excluded = None;
        for candidate in std::iter::once(primary).chain(mirrors) {
            let route = RegistryRoute::selected(self, resolved, candidate)?;
            match self.observe_candidate(resolved, &reference, route) {
                Ok(observed) => return Ok(observed),
                Err(error)
                    if excluded_route(&error)
                        && self.backend.native_materialisation_for(&self.org_url)? =>
                {
                    excluded.get_or_insert(error);
                }
                Err(
                    error @ RegistryError::Git(
                        GitError::RepoNotFound { .. }
                        | GitError::NetworkUnreachable { .. }
                        | GitError::RefNotFound { .. },
                    ),
                ) => {
                    failure.get_or_insert(error);
                }
                Err(error @ RegistryError::Git(GitError::AuthFailed { .. }))
                    if self.auth_kind() == vibe_core::manifest::AuthKind::None =>
                {
                    failure.get_or_insert(error);
                }
                Err(error) => return Err(error),
            }
        }
        Err(failure
            .or(excluded)
            .ok_or_else(|| RegistryError::UnknownPackage {
                group: resolved.group.clone(),
                name: resolved.name.clone(),
            })?)
    }
    fn observe_candidate(
        &self,
        resolved: &ResolvedPackage,
        reference: &str,
        route: RegistryRoute,
    ) -> Result<(Manifest, RegistryRoute), RegistryError> {
        let owner = self
            .configured_redirect(&resolved.group, &resolved.name, &route.candidate, reference)?
            .with_source_publication(route.publication);
        let (manifest, commit, serving, native) = if owner.native_source_for(&route.candidate)? {
            let source = owner.retain_native_package(
                &resolved.group,
                &resolved.name,
                &route.candidate,
                reference,
            )?;
            (
                source.manifest().clone(),
                Some(source.commit().to_owned()),
                source.serving_endpoint().map(str::to_owned),
                Some(source),
            )
        } else {
            // Metadata is read from the chosen endpoint, not an unrelated warm
            // clone. Captured commit constrains later acquisition at that URL.
            let clone = owner.package_clone_dir(&resolved.group, &resolved.name);
            owner.ensure_selected_clone(reference, &clone)?;
            (
                Manifest::read(clone.join(Manifest::FILENAME))?,
                owner.backend.head_commit(&clone).map_err(|source| {
                    GitError::PreparedValidation {
                        source: Box::new(source),
                    }
                })?,
                Some(route.candidate.clone()),
                None,
            )
        };
        let meta = manifest.require_package()?;
        if meta.group != resolved.group
            || meta.name != resolved.name
            || meta.version != resolved.version
        {
            return Err(RegistryError::MalformedMeta {
                path: "retained-source".into(),
                reason: "selected endpoint manifest differs from the resolved identity".into(),
            });
        }
        Ok((
            manifest.clone(),
            route.with_acquisition(owner, manifest, commit, serving, native),
        ))
    }
}
