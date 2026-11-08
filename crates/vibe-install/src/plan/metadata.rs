//! One metadata-only closure phase shared by preview and installation.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#OPERATION-EFFECTS");

use crate::{
    InstallSource, PlanEvent, PlanObserver,
    error::{Error, Result},
    fetched::{build_activation_context_from_manifests, tailor_feature_request},
    visibility_projection::{ProjectionInput, project, prune_graph, visibility_error},
};
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::path::Path;
use vibe_core::visibility::Analysis;
use vibe_core::{
    Group, PackageRef,
    manifest::{Lockfile, Manifest},
};
use vibe_resolver::{
    CandidateMetadata, DepProviderError, FeatureExpansion, FeatureRequest, HistoricalActivation,
    ResolvedGraph, RootRequirement, SourceResolvedGraph, SourceSolveRequest,
    conditional::ConditionalPredicate, expand_features, source_edges::RequirementActivation,
};
use vibe_workspace::Workspace;

pub(super) struct MetadataPlanningInput<'a> {
    pub workspace: &'a Workspace,
    pub root_manifest: &'a Manifest,
    pub roots: &'a [PackageRef],
    pub root_edges: &'a [RootRequirement],
    pub held_roots: &'a [PackageRef],
    pub lockfile: &'a Lockfile,
    pub root_features: &'a FeatureRequest,
    pub language_chain: &'a [String],
    pub project_root: &'a Path,
    pub root_id: &'a str,
    pub observer: &'a dyn PlanObserver,
}

#[derive(Debug)]
pub(super) struct MetadataClosure {
    pub graph: ResolvedGraph,
    pub selected: BTreeMap<(Group, String), CandidateMetadata>,
    pub feature_expansions: BTreeMap<(Group, String), FeatureExpansion>,
    pub visibility_analysis: Analysis,
}

pub(super) fn resolve_metadata_closure<S: InstallSource + ?Sized>(
    source: &S,
    input: MetadataPlanningInput<'_>,
) -> Result<MetadataClosure> {
    // Preserve the operation's existing five-pass monotone activation law.
    // A historical edge is retained only after observing its selected parent
    // and its actual predicate, never because a target was discovered elsewhere.
    const MAX_CONDITIONAL_PASSES: usize = 5;
    let limits = source.source_solve_limits()?;
    let mut held = input.held_roots.to_vec();
    let mut activated = Vec::<HistoricalActivation>::new();
    let progress = input.observer.progress();
    let mut conditional_pass = 0usize;
    let root_count = input
        .roots
        .iter()
        .map(|reference| (&reference.group, &reference.name))
        .collect::<BTreeSet<_>>()
        .len();
    input
        .observer
        .on(PlanEvent::ResolvingRoots { roots: root_count });
    let _bound_inputs = (input.workspace, input.lockfile);
    loop {
        progress.checkpoint()?;
        let (solved, visibility_analysis) =
            solve_visible(source, &input, &mut held, &activated, limits)?;
        if conditional_pass == 0 && solved.graph.packages.len() > root_count {
            input.observer.on(PlanEvent::GraphSolved {
                roots: root_count,
                total: solved.graph.packages.len(),
            });
        }
        let context = build_activation_context_from_manifests(
            solved.selected.values().map(|metadata| &metadata.view),
            input.project_root,
            input.language_chain,
        )?;
        let activated_sites = activated
            .iter()
            .map(|prior| (prior.parent.key(), prior.edge.site.clone()))
            .collect::<HashSet<_>>();
        let mut extra = Vec::new();
        for node in solved.graph.iter() {
            let metadata = solved
                .selected
                .get(&(node.group.clone(), node.name.clone()))
                .ok_or_else(|| missing("conditional parent metadata"))?;
            progress.checkpoint()?;
            let mut parent = None;
            let mut predicates = BTreeMap::new();
            for edge in &metadata.original_edges {
                let RequirementActivation::Conditional {
                    predicate,
                    optional,
                } = &edge.activation
                else {
                    continue;
                };
                if activated_sites.contains(&(metadata.key(), edge.site.clone())) {
                    continue;
                }
                // The evaluated view determines whether an optional declaration
                // actually participates; the raw original still supplies its pin/site.
                if *optional && !optional_selected(metadata, edge, predicate) {
                    continue;
                }
                let enabled = *predicates.entry(predicate.as_str()).or_insert_with(|| {
                    match ConditionalPredicate::parse(predicate) {
                        Ok(condition) => condition.evaluate(&context),
                        Err(error) => {
                            tracing::warn!(package = %format!("{}/{}", metadata.candidate.group(), metadata.candidate.name()),
                                predicate = %predicate, error = %error,
                                "conditional-dep predicate could not be parsed; skipping");
                            false
                        }
                    }
                });
                if enabled {
                    // Every active original declaration contributes its full
                    // constraint, including an already-present coordinate.
                    let observed = activated
                        .len()
                        .checked_add(extra.len())
                        .and_then(|count| count.checked_add(1))
                        .unwrap_or(usize::MAX);
                    if observed > limits.edge_witnesses {
                        return Err(vibe_resolver::SolveError::SourceModelLimit {
                            resource: vibe_resolver::source_candidates::SourceModelResource::EdgeWitnesses,
                            observed, limit: limits.edge_witnesses,
                        }.into());
                    }
                    let introduced_targets = edge
                        .alternatives
                        .iter()
                        .filter_map(|alternative| {
                            let group = alternative.reference.group.as_ref()?;
                            let key = (group.clone(), alternative.reference.name.to_string());
                            (!solved.selected.contains_key(&key)).then_some(key)
                        })
                        .collect();
                    let parent =
                        parent.get_or_insert_with(|| std::sync::Arc::new(metadata.clone()));
                    extra.push(HistoricalActivation {
                        parent: std::sync::Arc::clone(parent),
                        edge: edge.clone(),
                        phase: conditional_pass + 1,
                        introduced_targets,
                    });
                }
            }
        }
        if extra.is_empty() {
            let mut feature_expansions = BTreeMap::new();
            for node in solved.graph.iter() {
                let key = (node.group.clone(), node.name.clone());
                let metadata = solved
                    .selected
                    .get(&key)
                    .ok_or_else(|| missing("selected node metadata"))?;
                let expansion = match &metadata.configuration {
                    Some(record) => FeatureExpansion {
                        active_features: record.features.iter().cloned().collect(),
                        ..Default::default()
                    },
                    None => {
                        let request = if node.is_root {
                            tailor_feature_request(
                                input.root_features,
                                &metadata.candidate.manifest().features,
                            )
                        } else {
                            FeatureRequest::default()
                        };
                        expand_features(&metadata.candidate.manifest().features, &request)?
                    }
                };
                feature_expansions.insert(key, expansion);
            }
            return Ok(MetadataClosure {
                graph: solved.graph,
                selected: solved.selected,
                feature_expansions,
                visibility_analysis,
            });
        }
        if conditional_pass == MAX_CONDITIONAL_PASSES {
            return Err(Error::ConditionalDepRunaway {
                iterations: MAX_CONDITIONAL_PASSES,
                pending: extra
                    .iter()
                    .flat_map(|activation| {
                        activation
                            .edge
                            .alternatives
                            .iter()
                            .map(|alternative| alternative.reference.qualified_name())
                    })
                    .collect(),
            });
        }
        conditional_pass += 1;
        let introduced = extra
            .iter()
            .flat_map(|activation| activation.introduced_targets.iter())
            .collect::<BTreeSet<_>>()
            .len();
        input.observer.on(PlanEvent::ConditionalIteration {
            iteration: conditional_pass,
            extras: introduced,
        });
        activated.extend(extra);
    }
}

fn solve_visible<S: InstallSource + ?Sized>(
    source: &S,
    input: &MetadataPlanningInput<'_>,
    held: &mut Vec<PackageRef>,
    activated: &[HistoricalActivation],
    limits: vibe_resolver::SourceSolveLimits,
) -> Result<(SourceResolvedGraph, Analysis)> {
    const MAX_VISIBILITY_PASSES: usize = 4;
    let mut blocked = BTreeSet::new();
    for iteration in 1..=MAX_VISIBILITY_PASSES {
        input.observer.progress().checkpoint()?;
        let request = SourceSolveRequest {
            roots: input.root_edges,
            held_roots: held,
            activated,
            limits,
            configuration: None,
            blocked: &blocked,
            authoritative_locked: &input.lockfile.packages,
        };
        let mut solved = match source.solve_metadata(&request) {
            Ok(solved) => solved,
            // PROP-011 §5.3 — the pin-held set over-constrained: a changed
            // dependency is incompatible with a held pin. Retry once, freely,
            // over every authored requirement. Locked-source authority is not
            // a held pin: `authoritative_locked` still binds the retry, so a
            // replayed source can never be replaced through this fallback.
            Err(error) if held.as_slice() != input.roots && held_conflict(&error) => {
                input.observer.on(PlanEvent::HeldPinsConflicted {
                    error: error.to_string(),
                });
                *held = input.roots.to_vec();
                source
                    .solve_metadata(&SourceSolveRequest {
                        roots: input.root_edges,
                        held_roots: held,
                        activated,
                        limits,
                        configuration: None,
                        blocked: &blocked,
                        authoritative_locked: &input.lockfile.packages,
                    })
                    .map_err(|error| explain_held_failure(source, input, error))?
            }
            Err(error) => return Err(explain_held_failure(source, input, error)),
        };
        let manifest_of = |reference: &PackageRef| {
            let group = reference.group.as_ref().ok_or_else(|| {
                vibe_resolver::SolveError::from(DepProviderError::Other(
                    "unqualified selected metadata".into(),
                ))
            })?;
            let metadata = solved
                .selected
                .get(&(group.clone(), reference.name.to_string()))
                .ok_or_else(|| {
                    vibe_resolver::SolveError::from(DepProviderError::Other(
                        "visibility requested metadata outside the selected graph".into(),
                    ))
                })?;
            if !reference.version.matches(metadata.candidate.version()) {
                return Err(DepProviderError::Other(
                    "visibility metadata version differs from graph".into(),
                )
                .into());
            }
            Ok(metadata.view.clone())
        };
        let projection = project(&ProjectionInput {
            root_manifest: input.root_manifest,
            root_id: input.root_id.to_owned(),
            graph: &solved.graph,
            manifest_of: &manifest_of,
        })?;
        let newly_blocked = projection
            .blocked_edges
            .difference(&blocked)
            .cloned()
            .collect::<BTreeSet<_>>();
        if newly_blocked.is_empty() {
            prune_graph(&mut solved.graph, &projection.analysis);
            let retained = solved
                .graph
                .iter()
                .map(|node| (node.group.clone(), node.name.clone()))
                .collect::<BTreeSet<_>>();
            solved.selected.retain(|key, _| retained.contains(key));
            return Ok((solved, projection.analysis));
        }
        if iteration == MAX_VISIBILITY_PASSES {
            return Err(visibility_error(format!(
                "visibility strictness did not converge within {MAX_VISIBILITY_PASSES} iterations; unstable edges: {newly_blocked:?}"
            )));
        }
        blocked.extend(newly_blocked);
    }
    Err(visibility_error(
        "visibility did not produce a stable selected graph".into(),
    ))
}

fn optional_selected(
    metadata: &CandidateMetadata,
    edge: &vibe_resolver::source_edges::OriginalRequirement<vibe_core::manifest::RegistryPin>,
    predicate: &str,
) -> bool {
    let Some(target) = metadata.view.conditional_deps.get(predicate) else {
        return false;
    };
    edge.alternatives.iter().any(|alternative| {
        target
            .dependencies
            .packages
            .contains(&alternative.reference)
            && !target
                .dependencies
                .optional
                .get(&alternative.reference.qualified_name())
                .copied()
                .unwrap_or(false)
    })
}

fn missing(what: &str) -> Error {
    Error::Solve(DepProviderError::Other(format!("metadata closure lost {what}")).into())
}

fn held_conflict(error: &vibe_resolver::SolveError) -> bool {
    use vibe_resolver::SolveError;
    matches!(
        error,
        SolveError::Unsatisfiable { .. }
            | SolveError::VersionConflict { .. }
            | SolveError::ConflictsDeclared { .. }
            | SolveError::CapabilityUnmet { .. }
            | SolveError::DisjunctionUnsatisfiable { .. }
            | SolveError::Provider(DepProviderError::NoMatchingVersion { .. })
    )
}

// Explain an existing failed solve using a known mandatory held constraint.
// This neither preselects a candidate nor changes solver/fallback behavior.
fn explain_held_failure<S: InstallSource + ?Sized>(
    source: &S,
    input: &MetadataPlanningInput<'_>,
    error: vibe_resolver::SolveError,
) -> Error {
    if held_conflict(&error) {
        for root in input.root_edges {
            if !matches!(
                &root.requirement.activation,
                RequirementActivation::Mandatory
            ) || root.requirement.alternatives.len() != 1
            {
                continue;
            }
            let reference = &root.requirement.alternatives[0].reference;
            if !source.replays_locked_source(reference) {
                // An explicit version move stays at the held source; when
                // that source cannot serve the request, name it and the fix.
                if let Ok(Some(held)) = source.locked_update_availability(reference)
                    && !held.available
                {
                    return Error::HeldSourceVersionUnavailable {
                        package: reference.qualified_name(),
                        requirement: reference.version.to_string(),
                        locked_source: held.source,
                        source: Box::new(error),
                    };
                }
                continue;
            }
            if let Some(group) = &reference.group
                && let Some(held) = input.lockfile.find(group, reference.name.as_str())
                && !reference.version.matches(&held.version)
            {
                return Error::HeldSourceConstraint {
                    package: reference.qualified_name(),
                    held_version: held.version.to_string(),
                    requirement: reference.version.to_string(),
                    source: Box::new(error),
                };
            }
        }
    }
    error.into()
}
