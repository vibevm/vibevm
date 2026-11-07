//! Redirect targets use independently configured grants, not stub credentials.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-ROUTES");
use super::*;

impl MultiRegistryResolver {
    pub(in crate::multi_registry_resolver) fn redirect_target_registry(
        &self,
        group: &Group,
        name: &str,
        url: &str,
        reference: &str,
        requested_auth: vibe_core::manifest::AuthKind,
        token_env: Option<&str>,
    ) -> Result<GitPerPackageRegistry, RegistryError> {
        let synthetic = format!("redirect-target-{group}-{name}");
        if !self.backend.uses_native_materialisation() {
            return GitPerPackageRegistry::open_single_package(
                &synthetic,
                url,
                reference,
                &self.cache_root,
                Arc::clone(&self.backend),
                DEFAULT_FRESHNESS_SECS,
                requested_auth,
                token_env,
            );
        }
        // `sources` is the existing enabled/Locator-built registry walk; its
        // candidate chains already contain the selected remap/mirror policy.
        let mut grant = None;
        for source in &self.sources {
            if let RegistrySource::Git(registry) = source
                && registry.contains_candidate(group, name, url)?
            {
                if grant.is_some() {
                    return Err(RegistryError::MalformedMeta {
                        path: PathBuf::from("vibe-redirect.toml"),
                        reason: "redirect target has ambiguous configured credential grants".into(),
                    });
                }
                grant = Some(registry);
            }
        }
        if let Some(registry) = grant {
            return registry.configured_redirect(group, name, url, reference);
        }
        if !matches!(requested_auth, vibe_core::manifest::AuthKind::None) || token_env.is_some() {
            return Err(GitError::NativeContext{reason:"private redirect target requires an independently configured registry grant"}.into());
        }
        // A new unconfigured target has only anonymous authority. Marker text
        // never selects an environment token or the global SSH key/agent.
        let anonymous = self.backend.anonymized_for_public().ok_or_else(|| {
            RegistryError::Git(GitError::NativeContext {
                reason: "redirect target requires an anonymous bound capability",
            })
        })?;
        GitPerPackageRegistry::open_single_package(
            &synthetic,
            url,
            reference,
            &self.cache_root,
            anonymous,
            DEFAULT_FRESHNESS_SECS,
            vibe_core::manifest::AuthKind::None,
            None,
        )
    }

    pub(in crate::multi_registry_resolver) fn acquire_redirect(
        &self,
        resolution: &MultiResolution,
        store: &Path,
        expected: Option<&str>,
        slot: Option<&Path>,
    ) -> Result<crate::PackageAcquisition, RegistryError> {
        if !self.backend.uses_native_materialisation() {
            return self
                .fetch_via_redirect(resolution, store, expected)
                .map(|cached| crate::PackageAcquisition::Cached(Box::new(cached)));
        }
        let reference =
            resolution
                .source_ref
                .as_deref()
                .ok_or_else(|| RegistryError::MalformedMeta {
                    path: PathBuf::from("vibe-redirect.toml"),
                    reason: "redirect resolution has no selected ref".into(),
                })?;
        let target = self.redirect_target_registry(
            &resolution.resolved.group,
            &resolution.resolved.name,
            &resolution.source_url,
            reference,
            resolution.redirect_target_auth,
            resolution.redirect_target_token_env.as_deref(),
        )?;
        let route =
            resolution
                .selected_route
                .as_ref()
                .ok_or_else(|| RegistryError::MalformedMeta {
                    path: PathBuf::from("vibe-redirect.toml"),
                    reason: "native redirect has no retained target route".into(),
                })?;
        route.validate(&target, &resolution.resolved)?;
        route.validate_reference(reference)?;
        let registry =
            resolution
                .registry_name
                .clone()
                .ok_or_else(|| RegistryError::MalformedMeta {
                    path: PathBuf::from("vibe-redirect.toml"),
                    reason: "redirect resolution has no stub registry identity".into(),
                })?;
        let stub = resolution
            .via_redirect
            .clone()
            .ok_or_else(|| RegistryError::MalformedMeta {
                path: PathBuf::from("vibe-redirect.toml"),
                reason: "redirect resolution has no stub identity".into(),
            })?;
        let acquisition =
            target.fetch_on_route(&resolution.resolved, store, expected, slot, Some(route))?;
        match acquisition {
            crate::PackageAcquisition::Cached(mut cached) => {
                cached.registry_name = Some(registry);
                cached.via_redirect = Some(stub);
                Ok(crate::PackageAcquisition::Cached(cached))
            }
            crate::PackageAcquisition::InPlace(prepared) => Ok(crate::PackageAcquisition::InPlace(
                prepared.with_redirect_evidence(registry, stub)?,
            )),
        }
    }
    pub(in crate::multi_registry_resolver) fn prepare_redirect(
        &self,
        resolution: &MultiResolution,
        slot: &Path,
    ) -> Result<crate::PreparedInPlace, RegistryError> {
        let reference =
            resolution
                .source_ref
                .as_deref()
                .ok_or_else(|| RegistryError::MalformedMeta {
                    path: slot.into(),
                    reason: "redirect resolution has no selected ref".into(),
                })?;
        let target = self.redirect_target_registry(
            &resolution.resolved.group,
            &resolution.resolved.name,
            &resolution.source_url,
            reference,
            resolution.redirect_target_auth,
            resolution.redirect_target_token_env.as_deref(),
        )?;
        let route =
            resolution
                .selected_route
                .as_ref()
                .ok_or_else(|| RegistryError::MalformedMeta {
                    path: PathBuf::from("vibe-redirect.toml"),
                    reason: "native redirect has no retained target route".into(),
                })?;
        route.validate(&target, &resolution.resolved)?;
        route.validate_reference(reference)?;
        let registry =
            resolution
                .registry_name
                .clone()
                .ok_or_else(|| RegistryError::MalformedMeta {
                    path: slot.into(),
                    reason: "redirect has no stub registry identity".into(),
                })?;
        let stub = resolution
            .via_redirect
            .clone()
            .ok_or_else(|| RegistryError::MalformedMeta {
                path: slot.into(),
                reason: "redirect has no stub identity".into(),
            })?;
        target
            .prepare_declared_native_target(&resolution.resolved, slot)?
            .with_redirect_evidence(registry, stub)
    }
}
