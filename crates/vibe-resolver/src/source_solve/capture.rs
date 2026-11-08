//! Freeze each permitted candidate domain before resolvo's candidate cache exists.
specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#solver");
use super::*;
use std::collections::{HashMap, VecDeque};
use vibe_registry::source_candidates::{
    CandidateInventoryScope, SourceCandidateObservationKind, SourceInventoryLimits,
};

impl Universe {
    pub(super) fn capture<P: SourceCandidateProvider>(
        provider: &P,
        request: &SourceSolveRequest<'_>,
    ) -> Result<Self, SolveError> {
        let mut queue = VecDeque::new();
        let mut queued = BTreeSet::new();
        let mut enqueue = |reference: &PackageRef| -> Result<(), SolveError> {
            let key = reference_coordinate(reference)?;
            if queued.insert(key.clone()) {
                checked(
                    SourceModelResource::PackageNames,
                    queued.len(),
                    request.limits.package_names,
                )?;
                queue.push_back(key);
            }
            Ok(())
        };
        for root in request.roots {
            for choice in &root.requirement.alternatives {
                enqueue(&choice.reference)?;
            }
        }
        for activation in request.activated {
            for choice in &activation.edge.alternatives {
                enqueue(&choice.reference)?;
            }
        }
        let mut world = Self {
            entries: Vec::new(),
            incomplete: BTreeSet::new(),
            excluded: Vec::new(),
        };
        let mut keys = HashMap::<CandidateKey, String>::new();
        while let Some((group, name)) = queue.pop_front() {
            let inventory = match provider.source_inventory(
                &group,
                &name,
                CandidateInventoryScope::AllAllowed,
                &SourceInventoryLimits {
                    max_candidates: request.limits.source_candidates,
                },
            ) {
                Ok(value) => value,
                Err(
                    DepProviderError::UnknownPackage { .. }
                    | DepProviderError::NoMatchingVersion { .. }
                    | DepProviderError::AggregateNotFound { .. },
                ) => continue,
                Err(DepProviderError::SourcePolicy(
                    vibe_core::manifest::SourcePolicyError::ResourceLimit { actual, limit: cap },
                )) => return Err(limit(SourceModelResource::SourceCandidates, actual, cap)),
                Err(DepProviderError::SourcePolicy(error)) => {
                    return Err(DepProviderError::SourcePolicy(error).into());
                }
                Err(error) => return Err(error.into()),
            };
            if inventory.observations.iter().any(|row| {
                matches!(
                    row.observation,
                    SourceCandidateObservationKind::Unavailable { .. }
                )
            }) {
                world.incomplete.insert(format!("{group}/{name}"));
            }
            let excluded = excluded_endpoints(&inventory.observations);
            if !excluded.is_empty() {
                world
                    .excluded
                    .push(((group.clone(), name.clone()), excluded));
            }
            for record in inventory.candidates {
                let observed = world.entries.len().checked_add(1).ok_or_else(|| {
                    limit(
                        SourceModelResource::SourceCandidates,
                        usize::MAX,
                        request.limits.source_candidates,
                    )
                })?;
                checked(
                    SourceModelResource::SourceCandidates,
                    observed,
                    request.limits.source_candidates,
                )?;
                if record.candidate.group() != &group || record.candidate.name() != name {
                    return Err(invalid("candidate inventory returned another coordinate"));
                }
                if let Some(locked) = request
                    .authoritative_locked
                    .iter()
                    .find(|row| row.group == group && row.name == name)
                    && !provider.respects_locked_selection(&record.candidate, locked)?
                {
                    continue;
                }
                let key = CandidateKey::of(&record.candidate);
                if let Some(prior) = keys.insert(key, record.candidate.metadata_digest().into()) {
                    if prior != record.candidate.metadata_digest() {
                        return Err(invalid(
                            "candidate metadata changed inside one frozen source domain",
                        ));
                    }
                    continue;
                }
                let metadata = match metadata::prepare(
                    record.candidate,
                    inventory.observations.clone(),
                    request,
                ) {
                    Ok(value) => value,
                    Err(DepProviderError::Configuration(
                        vibe_core::manifest::PackageConfigurationError::Incompatible { .. },
                    )) => continue,
                    Err(error) => return Err(error.into()),
                };
                // Discovery is not activation. Only selected original witnesses
                // constrain the returned source choice.
                for edge in &metadata.original_edges {
                    if !matches!(
                        edge.activation,
                        crate::source_edges::RequirementActivation::Mandatory
                    ) {
                        continue;
                    }
                    for choice in &edge.alternatives {
                        let key = reference_coordinate(&choice.reference)?;
                        if queued.insert(key.clone()) {
                            checked(
                                SourceModelResource::PackageNames,
                                queued.len(),
                                request.limits.package_names,
                            )?;
                            queue.push_back(key);
                        }
                    }
                }
                for reference in &metadata.view.recommends.packages {
                    let key = reference_coordinate(reference)?;
                    if queued.insert(key.clone()) {
                        checked(
                            SourceModelResource::PackageNames,
                            queued.len(),
                            request.limits.package_names,
                        )?;
                        queue.push_back(key);
                    }
                }
                world.entries.push(Entry {
                    metadata,
                    default_eligible: record.default_eligible,
                    preference_rank: record.preference_rank,
                    registry_order: record.registry_order,
                });
            }
        }
        Ok(world)
    }
}

/// Endpoints refused by hosting exclusion, but only when no admitted source
/// observed any version: availability on an excluded endpoint is never
/// inferred, only that nothing admitted remains.
fn excluded_endpoints(
    observations: &[vibe_registry::source_candidates::SourceCandidateObservation],
) -> BTreeSet<String> {
    use vibe_registry::source_candidates::SourceCandidateDecision;
    let admitted = observations.iter().any(|row| {
        matches!(&row.observation, SourceCandidateObservationKind::Available { versions } if !versions.is_empty())
            && !matches!(row.decision, SourceCandidateDecision::ExcludedHost { .. })
    });
    if admitted {
        return BTreeSet::new();
    }
    observations
        .iter()
        .filter_map(|row| match &row.decision {
            SourceCandidateDecision::ExcludedHost { endpoint } => Some(endpoint.clone()),
            _ => None,
        })
        .collect()
}
