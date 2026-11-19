//! Top-level source selection remains in the existing cell registry.
use super::*;
/// Select one top-level metadata token through the existing provider composition.
pub(crate) fn select_top_level_cell(
    resource: ProviderResource<'_>,
    reference: &vibe_core::PackageRef,
) -> Result<vibe_registry::source_candidates::RetainedSourceCandidate, vibe_resolver::SolveError> {
    fn select(
        provider: &impl vibe_resolver::SourceCandidateProvider,
        reference: &vibe_core::PackageRef,
    ) -> Result<vibe_registry::source_candidates::RetainedSourceCandidate, vibe_resolver::SolveError>
    {
        let group = reference.group.as_ref().ok_or_else(|| {
            vibe_resolver::DepProviderError::Other("top-level reference must be qualified".into())
        })?;
        let version = provider.resolve_version(reference)?;
        let candidate = provider
            .retain_default_candidate(group, reference.name.as_str(), &version)?
            .candidate;
        let actual = candidate
            .manifest()
            .require_package()
            .map_err(|error| vibe_resolver::DepProviderError::Other(error.to_string()))?
            .kind;
        if let Some(requested) = reference.kind
            && requested != actual
        {
            return Err(vibe_resolver::SolveError::KindMismatch {
                package: format!("{group}/{}", reference.name),
                requested,
                actual,
            });
        }
        Ok(candidate)
    }
    match resource {
        ProviderResource::Local(local) => select(&super::local_provider(local), reference),
        ProviderResource::Multi(owner) => select(&super::multi_provider(owner), reference),
        ProviderResource::Embedded {
            locals,
            declared,
            precedence,
            short_circuit,
        } => select(
            &embedded_dep_provider(locals, declared, precedence, short_circuit),
            reference,
        ),
    }
}

/// Admission is minted by the execution owner's real policy inventory; matching
/// identity alone never creates eligibility. The solver still checks pins/locks.
pub(crate) fn readmit_top_level_cell(
    source: &crate::InstallResolver,
    original: &vibe_registry::source_candidates::RetainedSourceCandidate,
) -> Result<vibe_registry::source_candidates::RetainedSourceCandidate, vibe_resolver::SolveError> {
    fn admit(
        provider: &impl vibe_resolver::SourceCandidateProvider,
        original: &vibe_registry::source_candidates::RetainedSourceCandidate,
    ) -> Result<vibe_registry::source_candidates::RetainedSourceCandidate, vibe_resolver::SolveError>
    {
        let inventory = provider.source_inventory(
            original.group(),
            original.name(),
            vibe_registry::source_candidates::CandidateInventoryScope::AllAllowed,
            &vibe_registry::source_candidates::SourceInventoryLimits {
                max_candidates: 512,
            },
        )?;
        inventory.candidates.into_iter().find(|r| r.candidate.same_selected_source(original))
            .map(|r| r.candidate).ok_or_else(|| vibe_resolver::SolveError::SourceModelInvalid { reason:"routing source is not admitted by the personal project's ordinary source policy; resolve the conflicting registry/lock explicitly".into() })
    }
    use vibe_registry::source_candidates::LocalCandidateKind;
    match source {
        crate::InstallResolver::Local(local, _) => admit(
            &LocalRegistryDepProvider::with_source_context(
                local,
                LocalCandidateKind::ExplicitRegistry,
                None,
            ),
            original,
        ),
        crate::InstallResolver::LockedLocal { local, held, .. } => admit(
            &LocalRegistryDepProvider::with_source_context(
                local,
                LocalCandidateKind::ExplicitRegistry,
                Some(held),
            ),
            original,
        ),
        crate::InstallResolver::Multi(owner, _) => admit(&super::multi_provider(owner), original),
        crate::InstallResolver::Embedded {
            locals,
            project_local_count,
            declared,
            precedence,
            short_circuit,
            ..
        } => admit(
            &super::embedded_provider(
                super::local_composite_provider(
                    locals
                        .iter()
                        .enumerate()
                        .map(|(i, local)| {
                            LocalRegistryDepProvider::with_source_context(
                                local,
                                if i < *project_local_count {
                                    LocalCandidateKind::ProjectLocal
                                } else {
                                    LocalCandidateKind::Embedded
                                },
                                declared.as_deref(),
                            )
                        })
                        .collect(),
                ),
                declared.as_deref().map(super::multi_provider),
                *precedence,
                *short_circuit,
            ),
            original,
        ),
    }
}
