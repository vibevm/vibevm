//! Retained registry candidate identity; this is not transport authority.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-ROUTES");
use super::*;

/// Runtime-only selected registry candidate. Canonical lock provenance remains
/// separate. Private fields prevent retargeting a selection to another package.
#[derive(Clone)]
pub struct RegistryRoute {
    registry: String,
    group: Group,
    name: String,
    version: semver::Version,
    candidate: String,
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
        candidate: String,
    ) -> Self {
        Self {
            registry: registry.name.clone(),
            group: resolved.group.clone(),
            name: resolved.name.clone(),
            version: resolved.version.clone(),
            candidate,
            reference: None,
        }
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
        let mut route = RegistryRoute::selected(self, resolved, url.into());
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
        let mut target = Self::open_with_explicit_token(
            &self.name,
            url,
            reference,
            self.naming,
            vec![],
            &self.cache_root,
            Arc::clone(&self.backend),
            self.freshness_secs,
            self.auth,
            self.effective_token.clone(),
        )?;
        target.single_package_url = Some(url.into());
        target.token_env_name = self.token_env_name.clone();
        Ok(target)
    }
}
