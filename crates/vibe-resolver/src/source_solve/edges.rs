//! Original occurrence projection; historical roots do not re-evaluate predicates.
specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#solver");
use super::*;
use crate::source_candidates::{EdgeId, EdgeOwner};
use crate::source_edges::{OriginalRequirement, RequirementActivation, RequirementSite};
use model::{Choice, Edge};
use vibe_core::manifest::RegistryPin;

fn add(
    edges: &mut Vec<Edge>,
    world: &Universe,
    id: EdgeId,
    parent: Option<usize>,
    requirement: &OriginalRequirement<RegistryPin>,
    limits: SourceSolveLimits,
) -> Result<(), SolveError> {
    let prior_witnesses = edges
        .iter()
        .map(|edge| edge.choices.len().saturating_add(1))
        .fold(0usize, usize::saturating_add);
    let mut choices = Vec::new();
    for (alternative, choice) in requirement.alternatives.iter().enumerate() {
        let key = reference_coordinate(&choice.reference)?;
        for (target, entry) in world.entries.iter().enumerate() {
            if coordinate(&entry.metadata) != key
                || !choice
                    .reference
                    .version
                    .matches(entry.metadata.candidate.version())
            {
                continue;
            }
            if let Some(pin) = &choice.registry_pin
                && entry.metadata.candidate.binding().registry_name() != Some(pin.as_str())
            {
                continue;
            }
            checked(
                SourceModelResource::EdgeWitnesses,
                prior_witnesses
                    .saturating_add(choices.len())
                    .saturating_add(2),
                limits.edge_witnesses,
            )?;
            choices.push(Choice {
                target,
                alternative,
                pinned: choice.registry_pin.is_some(),
            });
        }
    }
    checked(
        SourceModelResource::EdgeWitnesses,
        edges.len().saturating_add(1),
        limits.edge_witnesses,
    )?;
    edges.push(Edge {
        id,
        parent,
        choices,
    });
    Ok(())
}
pub(super) fn build(
    world: &Universe,
    request: &SourceSolveRequest<'_>,
    recommended: &[RootRequirement],
) -> Result<Vec<Edge>, SolveError> {
    let mut edges = Vec::new();
    for root in request.roots.iter().chain(recommended) {
        add(
            &mut edges,
            world,
            root.id.clone(),
            None,
            &root.requirement,
            request.limits,
        )?;
    }
    for activation in request.activated {
        if activation.phase == 0 || activation.phase > 5 {
            return Err(invalid(
                "historical activation phase is outside the incumbent five passes",
            ));
        }
        let original = activation
            .parent
            .original_edges
            .iter()
            .find(|edge| edge.site == activation.edge.site)
            .ok_or_else(|| invalid("historical activation lost its exact original site"))?;
        if !matches!(
            original.activation,
            RequirementActivation::Conditional { .. }
        ) {
            return Err(invalid(
                "historical activation is not a conditional occurrence",
            ));
        }
        add(
            &mut edges,
            world,
            EdgeId {
                owner: EdgeOwner::Candidate(Box::new(activation.parent.key())),
                site: original.site.clone(),
            },
            None,
            original,
            request.limits,
        )?;
    }
    for (parent, entry) in world.entries.iter().enumerate() {
        for requirement in &entry.metadata.original_edges {
            if !matches!(requirement.activation, RequirementActivation::Mandatory) {
                continue;
            }
            add(
                &mut edges,
                world,
                EdgeId {
                    owner: EdgeOwner::Candidate(Box::new(entry.metadata.key())),
                    site: requirement.site.clone(),
                },
                Some(parent),
                requirement,
                request.limits,
            )?;
        }
        for (index, required) in entry.metadata.view.requires.capabilities.iter().enumerate() {
            let prior_witnesses = edges
                .iter()
                .map(|edge| edge.choices.len().saturating_add(1))
                .fold(0usize, usize::saturating_add);
            let mut choices = Vec::new();
            for (target, provider) in world.entries.iter().enumerate() {
                if provider
                    .metadata
                    .view
                    .provides
                    .capabilities
                    .iter()
                    .any(|provided| {
                        provided.qualified() == required.qualified()
                            && required.version.matches(
                                &crate::naive::capability_version_for_provider(
                                    provided,
                                    provider.metadata.candidate.version(),
                                ),
                            )
                    })
                {
                    checked(
                        SourceModelResource::EdgeWitnesses,
                        prior_witnesses
                            .saturating_add(choices.len())
                            .saturating_add(2),
                        request.limits.edge_witnesses,
                    )?;
                    choices.push(Choice {
                        target,
                        alternative: choices.len(),
                        pinned: false,
                    });
                }
            }
            // Preserve the incumbent typed post-solve CapabilityUnmet case.
            if !choices.is_empty() {
                checked(
                    SourceModelResource::EdgeWitnesses,
                    edges.len().saturating_add(1),
                    request.limits.edge_witnesses,
                )?;
                edges.push(Edge {
                    id: EdgeId {
                        owner: EdgeOwner::Candidate(Box::new(entry.metadata.key())),
                        site: RequirementSite::Capability(index),
                    },
                    parent: Some(parent),
                    choices,
                });
            }
        }
    }
    Ok(edges)
}
