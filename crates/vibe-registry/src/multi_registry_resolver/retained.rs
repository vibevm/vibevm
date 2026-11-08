//! Consume the exact selected owner; metadata is not permission to follow a live origin.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#LOCKED-SOURCE");
use super::*;
impl MultiRegistryResolver {
    pub(crate) fn fetch_retained_registry(
        &self,
        resolution: &MultiResolution,
        manifest: &Manifest,
        store: &Path,
        expected: Option<&str>,
        slot: Option<&Path>,
    ) -> Result<crate::PackageAcquisition, RegistryError> {
        let acquisition = if let Some(route) = &resolution.selected_route {
            route.acquire_captured(resolution, manifest, store, expected, slot)?
        } else {
            let name = resolution.registry_name.as_deref().ok_or_else(|| {
                RegistryError::MalformedMeta {
                    path: "retained-source".into(),
                    reason: "retained registry has no owner".into(),
                }
            })?;
            let source = self.selected_registry_source(name)?;
            let RegistrySource::Local(local) = source.as_ref() else {
                return Err(RegistryError::MalformedMeta {
                    path: "retained-source".into(),
                    reason: "Git token has no observed metadata route".into(),
                });
            };
            if Manifest::read(resolution.resolved.source_dir.join(Manifest::FILENAME))? != *manifest
            {
                return Err(RegistryError::MalformedMeta {
                    path: resolution.resolved.source_dir.clone(),
                    reason: "retained local manifest changed".into(),
                });
            }
            let mut cached = local.registry.fetch(&resolution.resolved, store)?;
            cached.registry_name = resolution.registry_name.clone();
            if let Some(expected) = expected {
                crate::git_package_registry::verify_store_entry_against_pin(
                    &cached.cache_dir,
                    expected,
                    &resolution.resolved.group,
                    &resolution.resolved.name,
                    &resolution.resolved.version,
                )?;
            }
            return Ok(crate::PackageAcquisition::Cached(Box::new(cached)));
        };
        let route =
            resolution
                .selected_route
                .as_ref()
                .ok_or_else(|| RegistryError::MalformedMeta {
                    path: "retained-source".into(),
                    reason: "selected route disappeared".into(),
                })?;
        match acquisition {
            crate::PackageAcquisition::Cached(mut cached) => {
                let actual = cached
                    .fetched_from
                    .clone()
                    .unwrap_or_else(|| cached.source_uri.clone());
                cached.source_uri = resolution.source_url.clone();
                cached.source_ref = resolution.source_ref.clone();
                cached.registry_name = resolution.registry_name.clone();
                cached.via_redirect = resolution.via_redirect.clone();
                cached.fetched_from = (vibe_core::progress::transfer::safe_remote(&actual)
                    != vibe_core::progress::transfer::safe_remote(&cached.source_uri))
                .then(|| vibe_core::progress::transfer::safe_remote(&actual));
                cached.route_publication = route
                    .publication_scope()
                    .observed(route.selected_address(), &actual);
                Ok(crate::PackageAcquisition::Cached(cached))
            }
            crate::PackageAcquisition::InPlace(prepared) => Ok(crate::PackageAcquisition::InPlace(
                prepared
                    .with_registry_evidence(resolution)?
                    .with_publication(route.publication_scope(), route.selected_address())?,
            )),
        }
    }
}
