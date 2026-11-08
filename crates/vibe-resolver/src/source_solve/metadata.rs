//! Evaluate retained metadata without collapsing equal-version source identities.
specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#solver");
use super::*;
use crate::source_candidates::ConfigurationAdmission;
use vibe_registry::source_candidates::{RetainedSourceCandidate, SourceCandidateObservation};

pub(super) fn prepare(
    candidate: RetainedSourceCandidate,
    observations: Vec<SourceCandidateObservation>,
    request: &SourceSolveRequest<'_>,
) -> Result<CandidateMetadata, DepProviderError> {
    let (mut view, configuration, configuration_error, configuration_admission) =
        match request.configuration {
            Some(snapshot) => {
                let (view, configuration, error) = snapshot.retained_source(&candidate)?;
                (
                    view,
                    configuration,
                    error,
                    ConfigurationAdmission::WorkspaceInputs,
                )
            }
            None => (
                candidate.manifest().clone(),
                None,
                None,
                ConfigurationAdmission::Unconfigured,
            ),
        };
    let mut original_edges =
        crate::source_edges::original_requirements(&view, |requires, reference| {
            reference
                .group
                .as_ref()
                .and_then(|group| requires.declared_registry(group, reference.name.as_str()))
                .cloned()
        })?;
    let owner = format!("{}/{}", candidate.group(), candidate.name());
    let targets = request
        .blocked
        .iter()
        .filter(|(parent, _)| parent == &owner)
        .map(|(_, target)| target.clone())
        .collect::<BTreeSet<_>>();
    if !targets.is_empty() && configuration.is_some() {
        return Err(DepProviderError::Configuration(
            vibe_core::manifest::PackageConfigurationError::Unsupported {
                package: owner,
                reason: "visibility-masked configured declaration provenance is not connected"
                    .into(),
            },
        ));
    }
    for edge in &mut original_edges {
        edge.alternatives
            .retain(|choice| !targets.contains(&choice.reference.qualified_name()));
    }
    original_edges.retain(|edge| !edge.alternatives.is_empty());
    crate::mask_manifest_edges(&mut view, &targets);
    Ok(CandidateMetadata {
        candidate,
        view,
        original_edges,
        configuration,
        configuration_error,
        configuration_admission,
        observations,
    })
}
