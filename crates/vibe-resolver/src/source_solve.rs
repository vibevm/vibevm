//! Source-aware constraints in the existing resolvo engine, with lazy legacy parity.
specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#solver");
use crate::source_candidates::SourceModelResource;
use crate::{
    CandidateKey, CandidateMetadata, DepProviderError, ResolvedGraph, RootRequirement, SolveError,
    SourceCandidateProvider, SourceResolvedGraph, SourceSolveLimits, SourceSolveRequest,
};
use std::collections::{BTreeMap, BTreeSet};
use vibe_core::{Group, PackageRef, manifest::Manifest};
use vibe_registry::source_candidates::SourceChoiceMode;
mod capture;
mod decisions;
mod edges;
mod legacy;
mod metadata;
mod model;
mod output;
mod provider;

type Coordinate = (Group, String);
struct Entry {
    metadata: CandidateMetadata,
    default_eligible: bool,
    preference_rank: usize,
    registry_order: usize,
}
struct Universe {
    entries: Vec<Entry>,
    incomplete: BTreeSet<String>,
    /// Coordinates whose only observed sources were refused by hosting
    /// exclusion, in capture order (roots first), with those endpoints and
    /// the registries that were in play beside them.
    excluded: Vec<(Coordinate, capture::ExcludedSplit)>,
}
impl Universe {
    /// A demanded coordinate with no admitted entry at all, known only from
    /// excluded hosting; a coordinate with any admitted entry is never
    /// reported here, so real version conflicts keep their proof.
    fn excluded_only(&self) -> Option<vibe_core::manifest::SourcePolicyError> {
        self.excluded.iter().find_map(|(key, split)| {
            let package = format!("{}/{}", key.0, key.1);
            let admitted = self
                .entries
                .iter()
                .any(|entry| coordinate(&entry.metadata) == *key);
            (!admitted && !self.incomplete.contains(&package)).then(|| {
                vibe_core::manifest::SourcePolicyError::ExcludedOnly {
                    package,
                    endpoints: split.endpoints.iter().cloned().collect(),
                    admitted: split.admitted.iter().cloned().collect(),
                }
            })
        })
    }
}
fn coordinate(metadata: &CandidateMetadata) -> Coordinate {
    (
        metadata.candidate.group().clone(),
        metadata.candidate.name().into(),
    )
}
fn reference_coordinate(reference: &PackageRef) -> Result<Coordinate, SolveError> {
    reference
        .group
        .clone()
        .map(|group| (group, reference.name.to_string()))
        .ok_or_else(|| invalid("source requirement is not group-qualified"))
}
fn invalid(reason: impl Into<String>) -> SolveError {
    SolveError::SourceModelInvalid {
        reason: reason.into(),
    }
}
fn limit(resource: SourceModelResource, observed: usize, limit: usize) -> SolveError {
    SolveError::SourceModelLimit {
        resource,
        observed,
        limit,
    }
}
fn checked(
    resource: SourceModelResource,
    observed: usize,
    limit_value: usize,
) -> Result<(), SolveError> {
    if observed > limit_value {
        Err(limit(resource, observed, limit_value))
    } else {
        Ok(())
    }
}

pub(crate) fn solve<P: SourceCandidateProvider>(
    provider: &P,
    request: &SourceSolveRequest<'_>,
) -> Result<SourceResolvedGraph, SolveError> {
    let explicit_pin = request
        .roots
        .iter()
        .flat_map(|root| &root.requirement.alternatives)
        .chain(
            request
                .activated
                .iter()
                .flat_map(|activation| &activation.edge.alternatives),
        )
        .any(|choice| choice.registry_pin.is_some());
    if !explicit_pin
        && provider.source_choice_mode() != SourceChoiceMode::CandidateRequired
        && let Some(graph) = legacy::solve(provider, request, legacy::LegacyCell::Resolvo)?
    {
        return Ok(graph);
    }
    validate_pins(provider, request)?;
    let world = Universe::capture(provider, request)?;
    let mut recommended = Vec::new();
    let mut tried = BTreeSet::new();
    let mut selected = hard(provider, &world, request, &recommended)?;
    // `--no-recommends` is the hard solve and nothing else: the expansion
    // below is not entered at all (`PROP-077#NO-RECOMMENDS-FLAG`).
    if !request.recommends {
        return Ok(selected);
    }
    // Keep the incumbent greedy recommends policy: each trial owns fresh
    // assumptions/model; a declined trial cannot leak an active pin or parent.
    loop {
        let mut next = None;
        for node in &selected.graph.packages {
            let Some(metadata) = selected
                .selected
                .get(&(node.group.clone(), node.name.clone()))
            else {
                return Err(invalid("selected recommendation owner metadata missing"));
            };
            for (index, reference) in metadata.view.recommends.packages.iter().enumerate() {
                let key = reference_coordinate(reference)?;
                if tried.contains(&key) || selected.graph.find(&key.0, &key.1).is_some() {
                    continue;
                }
                next = Some((
                    key,
                    RootRequirement {
                        id: crate::source_candidates::EdgeId {
                            owner: crate::source_candidates::EdgeOwner::Candidate(Box::new(
                                metadata.key(),
                            )),
                            site: crate::source_edges::RequirementSite::Recommendation(index),
                        },
                        requirement: crate::source_edges::OriginalRequirement {
                            site: crate::source_edges::RequirementSite::Recommendation(index),
                            activation: crate::source_edges::RequirementActivation::Mandatory,
                            alternatives: vec![crate::source_edges::RequirementAlternative {
                                reference: reference.clone(),
                                registry_pin: None,
                            }],
                        },
                    },
                ));
                break;
            }
            if next.is_some() {
                break;
            }
        }
        let Some((key, next)) = next else {
            break;
        };
        tried.insert(key);
        let mut trial = recommended.clone();
        trial.push(next);
        match hard(provider, &world, request, &trial) {
            Ok(graph) => {
                recommended = trial;
                selected = graph;
            }
            Err(SolveError::SourceModelLimit {
                resource,
                observed,
                limit,
            }) => {
                return Err(SolveError::SourceModelLimit {
                    resource,
                    observed,
                    limit,
                });
            }
            Err(SolveError::SourceModelInvalid { reason }) => {
                return Err(SolveError::SourceModelInvalid { reason });
            }
            Err(_) => {}
        }
    }
    Ok(selected)
}
/// Authored pins are admitted before capture so an unknown or disabled
/// registry surfaces as its typed policy error, never as an empty domain.
fn validate_pins<P: SourceCandidateProvider>(
    provider: &P,
    request: &SourceSolveRequest<'_>,
) -> Result<(), SolveError> {
    let choices = request
        .roots
        .iter()
        .flat_map(|root| &root.requirement.alternatives)
        .chain(
            request
                .activated
                .iter()
                .flat_map(|activation| &activation.edge.alternatives),
        );
    for choice in choices {
        if let Some(pin) = &choice.registry_pin {
            let (group, name) = reference_coordinate(&choice.reference)?;
            provider.validate_registry_pin(&group, &name, pin)?;
        }
    }
    Ok(())
}
fn hard<P: SourceCandidateProvider>(
    provider: &P,
    world: &Universe,
    request: &SourceSolveRequest<'_>,
    recommended: &[RootRequirement],
) -> Result<SourceResolvedGraph, SolveError> {
    let edges = edges::build(world, request, recommended)?;
    let model = model::Model::new(world, edges, request)?;
    let roots = model.roots.iter().copied().map(Into::into).collect();
    let constraints = model.root_constraints.clone();
    let mut solver = resolvo::Solver::new(provider::Provider {
        model,
        fault: std::cell::RefCell::new(None),
    });
    let outcome = solver.solve(
        resolvo::Problem::new()
            .requirements(roots)
            .constraints(constraints),
    );
    if let Some(reason) = solver.provider().fault.borrow_mut().take() {
        return Err(invalid(reason));
    }
    match outcome {
        Ok(ids) => output::decode(provider, &solver.provider().model, &ids, request),
        Err(resolvo::UnsolvableOrCancelled::Cancelled(_)) => Err(DepProviderError::Other(
            "source-aware solve was cancelled; no unsatisfiability proof".into(),
        )
        .into()),
        Err(resolvo::UnsolvableOrCancelled::Unsolvable(conflict)) => {
            if let Some(error) = world.excluded_only() {
                return Err(DepProviderError::SourcePolicy(error).into());
            }
            if !world.incomplete.is_empty() {
                return Err(SolveError::SourceModelIncomplete {
                    packages: world.incomplete.iter().cloned().collect(),
                });
            }
            Err(SolveError::Unsatisfiable {
                explanation: conflict.display_user_friendly(&solver).to_string(),
            })
        }
    }
}

pub(crate) fn solve_legacy_cell<P: SourceCandidateProvider>(
    provider: &P,
    request: &SourceSolveRequest<'_>,
    sat: bool,
) -> Result<SourceResolvedGraph, SolveError> {
    let solver = if sat { "sat" } else { "naive" };
    let pinned = request
        .roots
        .iter()
        .flat_map(|root| &root.requirement.alternatives)
        .chain(request.activated.iter().flat_map(|a| &a.edge.alternatives))
        .any(|choice| choice.registry_pin.is_some());
    if pinned || provider.source_choice_mode() == SourceChoiceMode::CandidateRequired {
        return Err(SolveError::SourceSolverUnsupported { solver });
    }
    legacy::solve(
        provider,
        request,
        if sat {
            legacy::LegacyCell::Sat
        } else {
            legacy::LegacyCell::Naive
        },
    )?
    .ok_or(SolveError::SourceSolverUnsupported { solver })
}
