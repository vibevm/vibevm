//! Lazy retention around the incumbent no-policy solver, with whole-attempt restart.
specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#solver");
use super::*;
use crate::{DepProvider, DepSolver, ResolvoDepSolver, VersionEnumerator};
use std::cell::{Cell, RefCell};
use vibe_registry::source_candidates::SourceChoiceMode;

#[derive(Clone, Copy)]
pub(super) enum LegacyCell {
    Resolvo,
    Naive,
    Sat,
}

type LegacyKey = (Group, String, semver::Version);
struct Retained<'a, P> {
    provider: &'a P,
    request: &'a SourceSolveRequest<'a>,
    cache: RefCell<BTreeMap<LegacyKey, CandidateMetadata>>,
    before: RefCell<Option<ResolvedGraph>>,
    restart: Cell<bool>,
}
impl<P: SourceCandidateProvider> Retained<'_, P> {
    fn check_mode(&self) -> Result<(), DepProviderError> {
        if self.restart.get()
            || self.provider.source_choice_mode() == SourceChoiceMode::CandidateRequired
        {
            self.restart.set(true);
            return Err(DepProviderError::SourceAwareRestart);
        }
        Ok(())
    }
    fn selected(
        &self,
        graph: &ResolvedGraph,
    ) -> Result<BTreeMap<Coordinate, CandidateMetadata>, DepProviderError> {
        let cache = self.cache.borrow();
        let mut selected = BTreeMap::new();
        for node in &graph.packages {
            let key = (node.group.clone(), node.name.clone(), node.version.clone());
            let metadata = cache.get(&key).ok_or_else(|| {
                DepProviderError::Other("selected legacy metadata was not retained".into())
            })?;
            selected.insert((node.group.clone(), node.name.clone()), metadata.clone());
        }
        Ok(selected)
    }
}
impl<P: SourceCandidateProvider> DepProvider for Retained<'_, P> {
    fn resolve_version(&self, reference: &PackageRef) -> Result<semver::Version, DepProviderError> {
        let version = self.provider.resolve_version(reference)?;
        self.check_mode()?;
        Ok(version)
    }
    fn fetch_manifest(
        &self,
        group: &Group,
        name: &str,
        version: &semver::Version,
    ) -> Result<Manifest, DepProviderError> {
        self.check_mode()?;
        let key = (group.clone(), name.to_owned(), version.clone());
        if let Some(metadata) = self.cache.borrow().get(&key) {
            return Ok(metadata.view.clone());
        }
        let observed = self.cache.borrow().len().saturating_add(1);
        if observed > self.request.limits.source_candidates {
            return Err(DepProviderError::SourceMetadataLimit {
                observed,
                limit: self.request.limits.source_candidates,
            });
        }
        let observation = self
            .provider
            .retain_default_candidate(group, name, version)?;
        self.check_mode()?;
        let candidate = &observation.candidate;
        if candidate.group() != group || candidate.name() != name || candidate.version() != version
        {
            return Err(DepProviderError::Other(
                "retained metadata changed the requested coordinate/version".into(),
            ));
        }
        if let Some(locked) = self
            .request
            .authoritative_locked
            .iter()
            .find(|row| row.group == *group && row.name == name)
            && !self.provider.respects_locked_selection(candidate, locked)?
        {
            return Err(DepProviderError::Other(
                "retained source violates authoritative locked selection".into(),
            ));
        }
        let original = crate::source_edges::original_requirements(
            candidate.manifest(),
            |requires, reference| {
                reference
                    .group
                    .as_ref()
                    .and_then(|group| requires.declared_registry(group, reference.name.as_str()))
                    .cloned()
            },
        )?;
        if original
            .iter()
            .flat_map(|edge| &edge.alternatives)
            .any(|choice| choice.registry_pin.is_some())
        {
            self.restart.set(true);
            return Err(DepProviderError::SourceAwareRestart);
        }
        let metadata = metadata::prepare(
            observation.candidate,
            observation.observations,
            self.request,
        )?;
        if let Some(error) = &metadata.configuration_error
            && !matches!(
                error,
                vibe_core::manifest::PackageConfigurationError::NeedsSelection { .. }
            )
        {
            return Err(DepProviderError::Configuration(error.clone()));
        }
        let view = metadata.view.clone();
        self.cache.borrow_mut().insert(key, metadata);
        Ok(view)
    }
    fn validate_selected(&self, graph: &ResolvedGraph) -> Result<(), DepProviderError> {
        for metadata in self.selected(graph)?.values() {
            if let Some(error) = &metadata.configuration_error {
                return Err(DepProviderError::Configuration(error.clone()));
            }
        }
        if let Some(configuration) = self.request.configuration {
            configuration.refuse_unmerged_incoming_edges(graph)?;
        }
        self.provider.validate_selected(graph)
    }
    fn observe_selected_before_projection(
        &self,
        graph: &ResolvedGraph,
    ) -> Result<(), DepProviderError> {
        self.selected(graph)?;
        self.provider.observe_selected_before_projection(graph)?;
        *self.before.borrow_mut() = Some(graph.clone());
        Ok(())
    }
}
impl<P: SourceCandidateProvider> VersionEnumerator for Retained<'_, P> {
    fn list_versions(
        &self,
        group: &Group,
        name: &str,
    ) -> Result<Vec<semver::Version>, DepProviderError> {
        let versions = self.provider.list_versions(group, name)?;
        self.check_mode()?;
        Ok(versions)
    }
}

pub(super) fn solve<'a, P: SourceCandidateProvider>(
    provider: &'a P,
    request: &'a SourceSolveRequest<'a>,
    cell: LegacyCell,
) -> Result<Option<SourceResolvedGraph>, SolveError> {
    let mut roots = Vec::new();
    for root in request.roots {
        if root.requirement.alternatives.len() != 1 {
            return Ok(None);
        }
        roots.push(root.requirement.alternatives[0].reference.clone());
    }
    for activated in request.activated {
        if activated.edge.alternatives.len() != 1 {
            return Ok(None);
        }
        roots.push(activated.edge.alternatives[0].reference.clone());
    }
    // Extra held constraints retain the old duplicate-root range intersection.
    roots.extend_from_slice(request.held_roots);
    let retained = Retained {
        provider,
        request,
        cache: RefCell::new(BTreeMap::new()),
        before: RefCell::new(None),
        restart: Cell::new(false),
    };
    let (outcome, retained) = match cell {
        LegacyCell::Resolvo => {
            let solver = ResolvoDepSolver::new(retained);
            (solver.solve(&roots), solver.into_inner())
        }
        LegacyCell::Naive => {
            let solver = crate::NaiveDepSolver::new(retained);
            (solver.solve(&roots), solver.into_inner())
        }
        LegacyCell::Sat => {
            let solver = crate::sat::SatDepSolver::new(retained);
            (solver.solve(&roots), solver.into_inner())
        }
    };
    if retained.restart.get()
        || provider.source_choice_mode() == SourceChoiceMode::CandidateRequired
    {
        return Ok(None);
    }
    let graph = normalize_roots(outcome?, request)?;
    for metadata in retained.selected(&graph)?.values() {
        if let Some(error) = &metadata.configuration_error {
            return Err(DepProviderError::Configuration(error.clone()).into());
        }
    }
    let mut selected = retained.selected(&graph)?;
    decisions::annotate(&mut selected, &[], request, None)?;
    let before = retained
        .before
        .borrow()
        .clone()
        .unwrap_or_else(|| graph.clone());
    let mut selected_before_projection = retained.selected(&before)?;
    decisions::annotate(&mut selected_before_projection, &[], request, None)?;
    Ok(Some(SourceResolvedGraph {
        graph,
        selected,
        selected_before_projection,
        active_edges: Vec::new(),
    }))
}

fn normalize_roots(
    graph: ResolvedGraph,
    request: &SourceSolveRequest<'_>,
) -> Result<ResolvedGraph, SolveError> {
    let mut order = Vec::new();
    let mut seen = BTreeSet::new();
    for root in request.roots {
        for choice in &root.requirement.alternatives {
            let key = reference_coordinate(&choice.reference)?;
            if seen.insert(key.clone()) {
                order.push(key);
            }
        }
    }
    for activation in request.activated {
        for choice in &activation.edge.alternatives {
            let key = reference_coordinate(&choice.reference)?;
            if activation.introduced_targets.contains(&key) && seen.insert(key.clone()) {
                order.push(key);
            }
        }
    }
    let chosen = graph
        .packages
        .into_iter()
        .map(|node| {
            (
                (node.group, node.name),
                crate::Chosen {
                    version: node.version,
                    direct_deps: node.dependencies,
                    is_root: false,
                },
            )
        })
        .collect();
    Ok(crate::build_resolved_graph(
        &order,
        chosen,
        &Default::default(),
    ))
}
