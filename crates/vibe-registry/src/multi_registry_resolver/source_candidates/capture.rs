//! Capture immutable metadata on the selected route before solver acquisition.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#SOURCE-POLICY");
use super::*;
impl MultiRegistryResolver {
    pub(super) fn retain_resolution(
        &self,
        mut resolution: MultiResolution,
        binding: Option<Binding>,
    ) -> Result<RetainedCandidateObservation, RegistryError> {
        // Supplied inventory identity must agree with the actual resolved owner
        // before even its manifest is read. A registry label cannot turn an
        // override/direct resolution into registry authority.
        let actual = self.binding_for_resolution(&resolution)?;
        if binding.as_ref().is_some_and(|supplied| supplied != &actual) {
            return Err(invalid_selection(
                &resolution,
                "candidate binding differs from the resolved source owner",
            ));
        }
        let binding = actual;
        let manifest = self.manifest_for_candidate(&mut resolution)?;
        let mut evidence = match &resolution.direct {
            Some(direct) => direct.evidence(),
            None => crate::binding_source_evidence(&binding),
        };
        // Metadata contact is not installed payload acquisition.
        evidence.acquisition = None;
        evidence.store_reused = None;
        evidence.source_url = Some(vibe_core::progress::transfer::safe_remote(
            &resolution.source_url,
        ));
        evidence.source_ref = resolution.source_ref.clone();
        evidence.via_redirect = resolution
            .via_redirect
            .as_deref()
            .map(vibe_core::progress::transfer::safe_remote);
        if resolution.from_store {
            evidence.store_reused = Some(true);
        }
        if let Some(route) = &resolution.selected_route {
            evidence.fetched_from = route
                .observed_endpoint()
                .filter(|actual| Some(actual.as_str()) != evidence.source_url.as_deref());
            evidence.resolved_commit = route.metadata_commit().map(str::to_owned);
        }
        let version = resolution.resolved.version.clone();
        let candidate = RetainedSourceCandidate::captured(
            &self.candidate_owner,
            binding.clone(),
            resolution,
            manifest,
            evidence,
        )?;
        Ok(RetainedCandidateObservation {
            candidate,
            observations: vec![SourceCandidateObservation {
                binding,
                observation: SourceCandidateObservationKind::Available {
                    versions: vec![version],
                },
                decision: SourceCandidateDecision::Eligible,
            }],
        })
    }

    fn manifest_for_candidate(
        &self,
        resolution: &mut MultiResolution,
    ) -> Result<Manifest, RegistryError> {
        if let Some(direct) = &resolution.direct {
            return Ok(direct.manifest().clone());
        }
        if let Some(locked) = &resolution.locked {
            return self.locked_manifest(resolution, locked);
        }
        if resolution.from_store || resolution.is_path_source {
            return Manifest::read(resolution.resolved.source_dir.join(Manifest::FILENAME))
                .map_err(RegistryError::from);
        }
        let group = &resolution.resolved.group;
        let name = &resolution.resolved.name;
        if resolution.via_redirect.is_some() {
            let reference = resolution.source_ref.as_deref().ok_or_else(|| {
                invalid_selection(resolution, "redirect selection has no reference")
            })?;
            let target = self.redirect_target_registry(
                group,
                name,
                &resolution.source_url,
                reference,
                resolution.redirect_target_auth,
                resolution.redirect_target_token_env.as_deref(),
            )?;
            let (manifest, route) = target.fetch_dep_manifest_observed(
                &resolution.resolved,
                resolution.selected_route.as_ref(),
            )?;
            resolution.selected_route = Some(route);
            return Ok(manifest);
        }
        if resolution.is_git_source || resolution.overridden {
            return Err(invalid_selection(
                resolution,
                "direct source selection has no retained acquisition lease",
            ));
        }
        let name = resolution.registry_name.as_deref().ok_or_else(|| {
            invalid_selection(
                resolution,
                "selected registry no longer belongs to the operation",
            )
        })?;
        let source = self.selected_registry_source(name)?;
        match source.as_ref() {
            RegistrySource::Git(registry) => {
                let (manifest, route) = registry.fetch_dep_manifest_observed(
                    &resolution.resolved,
                    resolution.selected_route.as_ref(),
                )?;
                resolution.selected_route = Some(route);
                Ok(manifest)
            }
            RegistrySource::Local(_) | RegistrySource::Monorepo(_) => {
                Manifest::read(resolution.resolved.source_dir.join(Manifest::FILENAME))
                    .map_err(RegistryError::from)
            }
        }
    }

    fn binding_for_resolution(
        &self,
        resolution: &MultiResolution,
    ) -> Result<Binding, RegistryError> {
        if let Some(locked) = &resolution.locked {
            return Ok(Binding::from_locked(locked.source_row()));
        }
        if resolution.is_path_source {
            return Ok(Binding::Path {
                root: resolution.resolved.source_dir.clone(),
            });
        }
        if resolution.is_git_source {
            return Ok(Binding::Git {
                source_url: resolution.source_url.clone().into(),
                source_ref: resolution.source_ref.clone(),
            });
        }
        if resolution.overridden {
            return Ok(Binding::Override {
                source_url: resolution.source_url.clone().into(),
                source_ref: resolution.source_ref.clone(),
            });
        }
        let name = resolution.registry_name.as_deref().ok_or_else(|| {
            invalid_selection(
                resolution,
                "retained selection has no canonical source binding",
            )
        })?;
        let source = self.selected_registry_source(name)?;
        Ok(Binding::Registry {
            registry: RegistryPin::new(source.name().to_owned())?,
            source_url: source.url().to_owned().into(),
        })
    }
}
