//! Candidate-bound explicit workspace inputs for the existing resolvo solver.
//! One operation retains original metadata and its evaluated dependency view
//! across held-pin, recommendation and visibility attempts.
use crate::{
    DepProvider, DepProviderError, DepSolver, ResolvedGraph, ResolvoDepSolver, SolveError,
    VersionEnumerator,
};
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use vibe_core::{
    Group, PackageRef,
    manifest::{
        ConfigurationIntentDecl, Manifest, PackageConfiguration, PackageConfigurationError,
    },
};

type Coordinate = (Group, String, semver::Version);
type VersionObservation = Result<Vec<semver::Version>, DepProviderError>;
/// Actual incoming declarations grouped by the existing shared package identity.
/// A missing answer is retained, not replaced by default consent.
#[specmark::spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-069#AUTHORED-INTENT")]
pub type PackageConfigurationInputs =
    BTreeMap<String, BTreeMap<String, Option<ConfigurationIntentDecl>>>;

#[derive(Clone)]
struct Candidate {
    original: Manifest,
    evaluated: Result<Option<PackageConfiguration>, PackageConfigurationError>,
    view: Result<Manifest, PackageConfigurationError>,
}
/// Read-only-to-callers operation snapshot. It is neither a durable catalog nor
/// a source authority; the selected source supplies observations on demand.
#[specmark::spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-069#CANDIDATE-BOUNDARY")]
pub struct ConfigurationSnapshot {
    inputs: PackageConfigurationInputs,
    versions: RefCell<BTreeMap<(Group, String), VersionObservation>>,
    candidates: RefCell<BTreeMap<Coordinate, Candidate>>,
    source_errors: RefCell<BTreeMap<Coordinate, DepProviderError>>,
    source_candidates: RefCell<std::collections::HashMap<(crate::CandidateKey, String), Candidate>>,
}
impl ConfigurationSnapshot {
    /// A selected package edge into a coordinate that carries consumer
    /// configuration input cannot be merged yet: refuse it instead of
    /// letting the bare edge disappear behind the authored configuration
    /// (PROP-069 `#CONSUMER-MERGE`). Every solve path applies this check.
    pub(crate) fn refuse_unmerged_incoming_edges(
        &self,
        graph: &ResolvedGraph,
    ) -> Result<(), DepProviderError> {
        for parent in graph.iter() {
            for edge in &parent.dependencies {
                if self
                    .inputs
                    .get(&edge.qualified_name())
                    .is_some_and(|inputs| inputs.values().any(Option::is_some))
                {
                    return Err(DepProviderError::Configuration(
                        PackageConfigurationError::Unsupported {
                            package: edge.qualified_name(),
                            reason: format!(
                                "incoming package edge from `{}/{}` needs candidate-conditioned consumer merging",
                                parent.group, parent.name
                            ),
                        },
                    ));
                }
            }
        }
        Ok(())
    }
    fn evaluate_candidate(&self, group: &Group, name: &str, original: Manifest) -> Candidate {
        let package = format!("{group}/{name}");
        let supplied = self.inputs.get(&package);
        let unsupported = |reason: &str| PackageConfigurationError::Unsupported {
            package: package.clone(),
            reason: reason.into(),
        };
        let evaluated = if original.configuration.is_some() {
            match supplied {
                None => Err(unsupported(
                    "no fixed actual workspace consumer configures this candidate",
                )),
                Some(inputs) if inputs.values().any(Option::is_none) => Err(unsupported(
                    "an actual incoming consumer has no configuration answer",
                )),
                Some(inputs) => {
                    let inputs = inputs
                        .iter()
                        .filter_map(|(consumer, input)| {
                            input.clone().map(|i| (consumer.clone(), i))
                        })
                        .collect();
                    PackageConfiguration::evaluate(&original, &inputs).map(Some)
                }
            }
        } else if supplied.is_some_and(|inputs| inputs.values().any(Option::is_some)) {
            Err(unsupported(
                "incoming configuration targets a package without a common definition",
            ))
        } else if original.requires.optional.values().any(|value| *value)
            || !original.requires.configurations.is_empty()
        {
            Err(unsupported(
                "optional or configured outgoing declarations lack a supported owner configuration",
            ))
        } else {
            Ok(None)
        };
        let view = match &evaluated {
            Ok(Some(record)) => record.project(&original),
            Ok(None) => Ok(original.clone()),
            Err(PackageConfigurationError::NeedsSelection { .. }) => {
                // Candidate viability uses only unconditional requirements.
                // No partial selection is ever returned to an effect owner.
                let mut mandatory = original.clone();
                mandatory.requires.packages.retain(|p| {
                    !original
                        .requires
                        .optional
                        .get(&p.qualified_name())
                        .copied()
                        .unwrap_or(false)
                });
                mandatory.configuration = None;
                Ok(mandatory)
            }
            Err(error) => Err(error.clone()),
        };
        Candidate {
            original,
            evaluated,
            view,
        }
    }
    /// Source-aware metadata is keyed by both retained binding and manifest witness.
    pub(crate) fn retained_source(
        &self,
        source: &vibe_registry::source_candidates::RetainedSourceCandidate,
    ) -> Result<
        (
            Manifest,
            Option<PackageConfiguration>,
            Option<PackageConfigurationError>,
        ),
        DepProviderError,
    > {
        let key = (
            crate::CandidateKey::of(source),
            source.metadata_digest().to_owned(),
        );
        if !self.source_candidates.borrow().contains_key(&key) {
            let candidate =
                self.evaluate_candidate(source.group(), source.name(), source.manifest().clone());
            self.source_candidates
                .borrow_mut()
                .insert(key.clone(), candidate);
        }
        let candidates = self.source_candidates.borrow();
        let candidate = candidates.get(&key).ok_or_else(|| {
            DepProviderError::Other("retained configured candidate disappeared".into())
        })?;
        let view = match project(candidate) {
            Ok(view) => view,
            Err(DepProviderError::Configuration(
                error @ PackageConfigurationError::Incompatible { .. },
            )) => return Err(DepProviderError::Configuration(error)),
            Err(DepProviderError::Configuration(_)) => {
                // Candidate viability uses known unconditional requirements;
                // the selected result must still surface the retained error.
                let mut mandatory = candidate.original.clone();
                mandatory.requires.packages.retain(|reference| {
                    !candidate
                        .original
                        .requires
                        .optional
                        .get(&reference.qualified_name())
                        .copied()
                        .unwrap_or(false)
                });
                mandatory.configuration = None;
                mandatory
            }
            Err(error) => return Err(error),
        };
        Ok((
            view,
            candidate.evaluated.clone().ok().flatten(),
            candidate.evaluated.clone().err(),
        ))
    }

    pub fn new(inputs: PackageConfigurationInputs) -> Self {
        Self {
            inputs,
            versions: RefCell::new(BTreeMap::new()),
            candidates: RefCell::new(BTreeMap::new()),
            source_errors: RefCell::new(BTreeMap::new()),
            source_candidates: RefCell::new(std::collections::HashMap::new()),
        }
    }
    /// The same snapshot backs the provider callbacks and selected output. Only
    /// the existing resolvo engine performs version selection/backtracking.
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-069#CANDIDATE-BOUNDARY"
    )]
    pub fn solve<P: VersionEnumerator>(
        &self,
        provider: P,
        roots: &[PackageRef],
        blocked: &BTreeSet<(String, String)>,
    ) -> Result<ResolvedGraph, SolveError> {
        ResolvoDepSolver::new(ConfiguredProvider {
            source: provider,
            snapshot: self,
            blocked,
        })
        .solve(roots)
    }
    /// Selected evaluated record; rejected candidates never contribute state.
    pub fn configuration(
        &self,
        group: &Group,
        name: &str,
        version: &semver::Version,
    ) -> Result<Option<PackageConfiguration>, SolveError> {
        let candidates = self.candidates.borrow();
        let candidate = candidates
            .get(&(group.clone(), name.into(), version.clone()))
            .ok_or_else(|| {
                SolveError::Provider(DepProviderError::Other(
                    "selected configuration candidate was not observed".into(),
                ))
            })?;
        candidate
            .evaluated
            .clone()
            .map_err(|error| SolveError::Provider(DepProviderError::Configuration(error)))
    }
    /// Actual captured metadata for the final acquisition drift check.
    pub fn original(
        &self,
        group: &Group,
        name: &str,
        version: &semver::Version,
    ) -> Option<Manifest> {
        self.candidates
            .borrow()
            .get(&(group.clone(), name.into(), version.clone()))
            .map(|c| c.original.clone())
    }
    /// Visibility consumes the same selected active-declaration view as solving.
    pub fn manifest(&self, package: &PackageRef) -> Result<Manifest, SolveError> {
        let candidates = self.candidates.borrow();
        let found = candidates
            .iter()
            .filter(|((g, n, v), _)| {
                package.group.as_ref() == Some(g)
                    && package.name.as_str() == n
                    && package.version.matches(v)
            })
            .collect::<Vec<_>>();
        if found.len() != 1 {
            return Err(DepProviderError::Other(format!(
                "expected one observed exact candidate for `{package}`"
            ))
            .into());
        }
        let candidate = found[0].1;
        project(candidate).map_err(Into::into)
    }
}
fn project(candidate: &Candidate) -> Result<Manifest, DepProviderError> {
    candidate
        .view
        .clone()
        .map_err(DepProviderError::Configuration)
}
struct ConfiguredProvider<'a, P> {
    source: P,
    snapshot: &'a ConfigurationSnapshot,
    blocked: &'a BTreeSet<(String, String)>,
}
impl<P: VersionEnumerator> DepProvider for ConfiguredProvider<'_, P> {
    fn resolve_version(&self, package: &PackageRef) -> Result<semver::Version, DepProviderError> {
        self.source.resolve_version(package)
    }
    fn validate_selected(&self, graph: &ResolvedGraph) -> Result<(), DepProviderError> {
        for node in graph.iter() {
            let candidates = self.snapshot.candidates.borrow();
            let candidate = candidates
                .get(&(node.group.clone(), node.name.clone(), node.version.clone()))
                .ok_or_else(|| {
                    DepProviderError::Other("selected candidate was not captured".into())
                })?;
            if let Err(error) = &candidate.evaluated {
                return Err(DepProviderError::Configuration(error.clone()));
            }
        }
        self.snapshot.refuse_unmerged_incoming_edges(graph)?;
        self.source.validate_selected(graph)
    }
    fn fetch_manifest(
        &self,
        group: &Group,
        name: &str,
        version: &semver::Version,
    ) -> Result<Manifest, DepProviderError> {
        let key = (group.clone(), name.to_owned(), version.clone());
        if let Some(error) = self.snapshot.source_errors.borrow().get(&key) {
            return Err(error.clone());
        }
        if !self.snapshot.candidates.borrow().contains_key(&key) {
            let original = match self.source.fetch_manifest(group, name, version) {
                Ok(manifest) => manifest,
                Err(error) => {
                    self.snapshot
                        .source_errors
                        .borrow_mut()
                        .insert(key.clone(), error.clone());
                    return Err(error);
                }
            };
            let candidate = self.snapshot.evaluate_candidate(group, name, original);
            self.snapshot
                .candidates
                .borrow_mut()
                .insert(key.clone(), candidate);
        }
        let mut manifest = project(&self.snapshot.candidates.borrow()[&key])?;
        let owner = format!("{group}/{name}");
        let targets: BTreeSet<_> = self
            .blocked
            .iter()
            .filter(|(parent, _)| parent == &owner)
            .map(|(_, target)| target.clone())
            .collect();
        if !targets.is_empty()
            && matches!(
                self.snapshot.candidates.borrow()[&key].evaluated,
                Ok(Some(_))
            )
        {
            return Err(DepProviderError::Configuration(
                PackageConfigurationError::Unsupported {
                    package: owner,
                    reason: "visibility-masked configured declaration provenance is not connected"
                        .into(),
                },
            ));
        }
        crate::mask_manifest_edges(&mut manifest, &targets);
        Ok(manifest)
    }
}
impl<P: VersionEnumerator> VersionEnumerator for ConfiguredProvider<'_, P> {
    fn list_versions(
        &self,
        group: &Group,
        name: &str,
    ) -> Result<Vec<semver::Version>, DepProviderError> {
        let key = (group.clone(), name.to_owned());
        if let Some(versions) = self.snapshot.versions.borrow().get(&key) {
            return versions.clone();
        }
        let versions = self.source.list_versions(group, name);
        self.snapshot
            .versions
            .borrow_mut()
            .insert(key, versions.clone());
        versions
    }
}

#[cfg(test)]
#[path = "configuration/tests.rs"]
mod tests;
