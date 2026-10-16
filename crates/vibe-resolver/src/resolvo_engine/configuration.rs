//! Candidate-local B1/B2 evaluation feeding the incumbent resolvo provider.
//!
//! This entry accepts fixed incoming consumer edges for a single resolution
//! domain. It does not infer consumer edges from installed state. Definitions
//! are admitted inputs, just as at the B2 seam, but must bind a C1 receipt.
use super::{ResolvoDepSolver, TransactionCatalog};
use crate::features::configuration::{ConfigurationDefinition, resolve_consumer_configuration};
use crate::{
    DepProvider, DepProviderError, FeatureExpansion, ResolvedGraph, SolveError, VersionEnumerator,
};
use std::collections::{BTreeMap, BTreeSet};
use vibe_core::manifest::{Manifest, feature_constraints::*, feature_effects::*};
use vibe_core::{Group, PackageRef};
use vibe_registry::CandidateIdentity;
use vibe_wire::behaviour::consumer_configuration as codec;
use vibe_wire::generated::consumer_configuration::e1::{request::Request, result::OutcomeStatus};

type Coordinate = (Group, String, semver::Version);

/// Admitted normalized definition and the actual catalog receipt it interprets.
#[derive(Debug, Clone)]
pub struct CandidateConfiguration {
    pub package: PackageRef,
    pub version: semver::Version,
    pub identity: CandidateIdentity,
    pub definition: ConfigurationDefinition,
}

/// Explicit authorization of defaults from one shown definition for one edge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefaultBinding {
    edge: usize,
    identity: CandidateIdentity,
    shown: ConfigurationDefinition,
    request: Request,
}
impl DefaultBinding {
    /// Bind permission to both the C1 receipt and the full admitted definition
    /// shown to the consumer, including constraints, defaults and declarations.
    pub fn shown(edge: usize, request: &Request, candidate: &CandidateConfiguration) -> Self {
        Self {
            edge,
            identity: candidate.identity.clone(),
            shown: candidate.definition.clone(),
            request: request.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateReason {
    pub coordinate: String,
    pub identity: CandidateIdentity,
    pub edges: Vec<usize>,
    pub reason: String,
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigurationSolveError {
    #[error(transparent)]
    Source(DepProviderError),
    #[error("unsupported configured solve: {0}")]
    Unsupported(String),
    #[error("consumer configuration conflict: {solver}; candidate reasons: {candidates:?}")]
    Conflict {
        request: Request,
        candidates: Vec<CandidateReason>,
        solver: SolveError,
    },
    #[error("selected configuration needs consumer selection: {candidates:?}")]
    NeedsSelection {
        request: Request,
        candidates: Vec<CandidateReason>,
    },
    #[error("selected definition requires rediscovery: {candidates:?}")]
    Rediscovery {
        request: Request,
        candidates: Vec<CandidateReason>,
    },
}
impl From<DepProviderError> for ConfigurationSolveError {
    fn from(error: DepProviderError) -> Self {
        Self::Source(error)
    }
}

/// Complete selected graph and every selected candidate's activation effects.
/// The original request is retained losslessly; no install mutations occur here.
pub struct ConfiguredResolution {
    pub graph: ResolvedGraph,
    pub request: Request,
    pub effects: BTreeMap<String, FeatureExpansion>,
    pub providers: BTreeMap<String, Vec<String>>,
    pub rejected: Vec<CandidateReason>,
    pub choices: Vec<super::SelectedChoice>,
    pub manifests: BTreeMap<String, Manifest>,
}

enum Activation {
    Ready(FeatureExpansion),
    Rejected(String),
    NeedsSelection(String),
    Rediscovery(String),
}
struct Candidate {
    manifest: Manifest,
    activation: Activation,
    reason: CandidateReason,
}
struct ConfiguredCatalog<'a> {
    catalog: &'a TransactionCatalog,
    candidates: BTreeMap<Coordinate, Candidate>,
    blocked: &'a BTreeSet<(String, String)>,
    retained: &'a [super::SelectedChoice],
}

impl ResolvoDepSolver<&TransactionCatalog> {
    /// Solve fixed, explicit incoming consumers over immutable candidates.
    /// Unsupported native/multi-domain/forwarding combinations refuse before
    /// solving. Choice/default needs are observed only in the selected graph.
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/modules/vibe-resolver/PROP-003#M05C2-CONDITIONING"
    )]
    pub fn solve_configuration(
        &self,
        roots: &[PackageRef],
        request: &Request,
        definitions: &[CandidateConfiguration],
        defaults: &[DefaultBinding],
    ) -> Result<ConfiguredResolution, ConfigurationSolveError> {
        self.solve_configuration_routes(
            roots,
            request,
            definitions,
            defaults,
            &BTreeSet::new(),
            &[],
        )
    }

    pub fn solve_configuration_routes(
        &self,
        roots: &[PackageRef],
        request: &Request,
        definitions: &[CandidateConfiguration],
        defaults: &[DefaultBinding],
        blocked: &BTreeSet<(String, String)>,
        retained: &[super::SelectedChoice],
    ) -> Result<ConfiguredResolution, ConfigurationSolveError> {
        let edges = codec::admit_request(request).map_err(ConfigurationSolveError::Unsupported)?;
        // The graph projection does not retain complete activation support
        // through obsoletes/conditional targets. Refuse
        // those combinations explicitly at this complete-effects entrypoint.
        // Ordinary DepSolver routes keep their existing behavior.
        for (coordinate, _) in &self.provider.inspect().candidates {
            let (qualified, version) = coordinate.rsplit_once('@').expect("C1 coordinate");
            let package = PackageRef::parse(qualified).expect("C1 qualified package");
            let version = semver::Version::parse(version).expect("C1 version");
            let manifest = &self
                .provider
                .metadata(
                    package.group.as_ref().expect("C1 group"),
                    package.name.as_str(),
                    &version,
                )?
                .manifest;
            if !manifest.conditional_deps.is_empty()
                || !manifest.obsoletes.packages.is_empty()
                || !manifest.requires.git_packages.is_empty()
                || !manifest.requires.path_packages.is_empty()
                || !manifest.requires.var_packages.is_empty()
            {
                return Err(ConfigurationSolveError::Unsupported(format!(
                    "{coordinate}: configured solve requires registry package edges without conditional targets or obsoletes"
                )));
            }
        }
        let mut by_package: BTreeMap<(Group, String), Vec<usize>> = BTreeMap::new();
        for (i, edge) in edges.iter().enumerate() {
            let PackageIdentity::Vibe(package) = &edge.instance.package else {
                return Err(ConfigurationSolveError::Unsupported(
                    "native candidate selection requires its native backend".into(),
                ));
            };
            let group = package.group.clone().ok_or_else(|| {
                ConfigurationSolveError::Unsupported("unqualified consumer package".into())
            })?;
            let indices = by_package
                .entry((group, package.name.to_string()))
                .or_default();
            if indices.iter().any(|&j| edges[j].instance != edge.instance) {
                return Err(ConfigurationSolveError::Unsupported("multiple configuration instances of one package require a domain-specific solve".into()));
            }
            indices.push(i);
        }
        for (i, binding) in defaults.iter().enumerate() {
            if binding.edge >= edges.len() || defaults[..i].iter().any(|b| b.edge == binding.edge) {
                return Err(ConfigurationSolveError::Unsupported(
                    "invalid or duplicate default binding".into(),
                ));
            }
        }
        let mut provider = ConfiguredCatalog {
            catalog: self.provider,
            candidates: BTreeMap::new(),
            blocked,
            retained,
        };
        // Validate all admitted definitions before classifying incompatibility.
        // Malformed/dangling source data must never become a SAT rejection.
        for d in definitions {
            let group = d.package.group.as_ref().ok_or_else(|| {
                ConfigurationSolveError::Unsupported("unqualified definition".into())
            })?;
            let metadata = self
                .provider
                .metadata(group, d.package.name.as_str(), &d.version)?;
            if metadata.identity != d.identity {
                return Err(DepProviderError::Other(
                    "configuration definition differs from immutable catalog identity".into(),
                )
                .into());
            }
            d.definition
                .constraints
                .order(&d.definition.program, &d.definition.declarations)
                .map_err(|e| {
                    ConfigurationSolveError::Source(DepProviderError::Other(e.to_string()))
                })?;
            if d.definition
                .program
                .features
                .values()
                .flatten()
                .any(|e| matches!(e, FeatureEffect::TargetFeature { .. }))
            {
                return Err(ConfigurationSolveError::Unsupported("cross-package feature forwarding needs explicit candidate-conditioned consumer edges; not supported by this fixed-consumer entry".into()));
            }
            if d.definition
                .declarations
                .dependencies
                .values()
                .any(|d| !matches!(d.source, DependencySource::Registry(_)))
            {
                return Err(ConfigurationSolveError::Unsupported(
                    "configured dependencies must use catalog registry coordinates".into(),
                ));
            }
        }
        for ((group, name), indices) in &by_package {
            let local_request = Request {
                epoch: request.epoch.clone(),
                edges: indices.iter().map(|&i| request.edges[i].clone()).collect(),
            };
            for version in self.provider.list_versions(group, name)? {
                let matching: Vec<_> = definitions
                    .iter()
                    .filter(|d| {
                        d.package.group.as_ref() == Some(group)
                            && d.package.name.as_str() == name
                            && d.version == version
                    })
                    .collect();
                if matching.len() != 1 {
                    return Err(DepProviderError::Other(format!(
                        "missing or ambiguous configured definition for {group}/{name}@{version}"
                    ))
                    .into());
                }
                let d = matching[0];
                if d.definition.instance != edges[indices[0]].instance {
                    return Err(DepProviderError::Other(
                        "configured definition instance disagrees with admitted consumers".into(),
                    )
                    .into());
                }
                let mut manifest = self
                    .provider
                    .metadata(group, name, &version)?
                    .manifest
                    .clone();
                // Every base dependency must have exactly the same declaration;
                // optionality is an explicit normalized-definition input.
                let base =
                    FeatureDeclarations::from_requires(&manifest.requires, &Default::default())
                        .map_err(|e| {
                            ConfigurationSolveError::Source(DepProviderError::Other(e.to_string()))
                        })?;
                for (id, original) in &base.dependencies {
                    let Some(declared) = d.definition.declarations.dependencies.get(id) else {
                        return Err(DepProviderError::Other(format!(
                            "missing source dependency declaration {}",
                            id.as_str()
                        ))
                        .into());
                    };
                    let mut comparable = declared.clone();
                    comparable.optional = original.optional;
                    if &comparable != original {
                        return Err(DepProviderError::Other(format!(
                            "changed source dependency declaration {}",
                            id.as_str()
                        ))
                        .into());
                    }
                }
                // The entire candidate-local dependency set is rebuilt, never
                // appended to a previous candidate's expansion.
                let required: Vec<_> = d
                    .definition
                    .declarations
                    .dependencies
                    .values()
                    .filter(|d| !d.optional)
                    .filter_map(|d| match &d.source {
                        DependencySource::Registry(p) => Some(p.clone()),
                        _ => None,
                    })
                    .collect();
                manifest.requires.packages = required;
                let mut reason = CandidateReason {
                    coordinate: format!("{group}/{name}@{version}"),
                    identity: d.identity.clone(),
                    edges: indices.clone(),
                    reason: String::new(),
                };
                let version_conflict = indices.iter().any(|&i| match &edges[i].instance.package {
                    PackageIdentity::Vibe(p) => {
                        !p.version.matches(&version)
                            || p.kind.is_some_and(|k| {
                                manifest.package.as_ref().is_some_and(|m| k != m.kind)
                            })
                    }
                    _ => unreachable!(),
                });
                // Authorization is about omitted intent and the shown definition,
                // not just defaults that happen to exist in today's outcome.
                let bound = |i: usize| {
                    defaults.iter().any(|b| {
                        b.edge == i
                            && b.identity == d.identity
                            && b.shown == d.definition
                            && b.request == *request
                    })
                };
                let default_governed = |i: usize| {
                    let edge = &edges[i];
                    let omitted_default = |definition: &ConfigurationDefinition| {
                        definition.constraints.groups.iter().any(|(name, group)| {
                            group.defaults.is_some()
                                && edge
                                    .selections
                                    .get(name)
                                    .is_none_or(|s| matches!(s.selection, Selection::Omitted))
                        })
                    };
                    edge.policy == DefaultPolicy::UseDeclaredDefaults
                        && (omitted_default(&d.definition)
                            || defaults
                                .iter()
                                .any(|b| b.edge == i && omitted_default(&b.shown)))
                };
                let stale_defaults = indices.iter().any(|&i| default_governed(i) && !bound(i));
                // A changed/unbound default cannot supply even internal solver
                // implications. Reuse B2 to retain only authorized selections.
                let mut authorized_request = local_request.clone();
                if stale_defaults {
                    for (local, &i) in indices.iter().enumerate() {
                        if !bound(i) {
                            authorized_request.edges[local].policy =
                                vibe_wire::generated::consumer_configuration::e1::request::EdgePolicy::RequireSelection;
                        }
                    }
                }
                let mut known = None;
                let activation = if version_conflict {
                    Activation::Rejected(
                        "original incoming consumer version/kind constraint".into(),
                    )
                } else {
                    match resolve_consumer_configuration(
                        &authorized_request,
                        std::slice::from_ref(&d.definition),
                    ) {
                        Err(error) => Activation::Rejected(error),
                        Ok(resolution) => {
                            let outcome = &resolution.result.outcomes[0];
                            if outcome.status == OutcomeStatus::Conflict {
                                Activation::Rejected(format!(
                                    "conflicting consumer selections: {:?}",
                                    outcome
                                        .conflicting_edges
                                        .iter()
                                        .map(|&i| indices[i as usize])
                                        .collect::<Vec<_>>()
                                ))
                            } else if stale_defaults
                                || outcome.status == OutcomeStatus::NeedsSelection
                            {
                                // B2 intentionally withholds a full expansion for
                                // unresolved groups. B1 can still derive the known
                                // monotone closure, without choosing missing members.
                                let selected: BTreeSet<String> =
                                    outcome.selected.values().flatten().cloned().collect();
                                let mut features = selected.clone();
                                for &i in indices {
                                    features.extend(
                                        edges[i].features.iter().map(|f| f.as_str().to_owned()),
                                    );
                                    if edges[i].all_features {
                                        features.extend(
                                            d.definition
                                                .program
                                                .features
                                                .keys()
                                                .filter(|f| !f.as_str().starts_with('_'))
                                                .map(|f| f.as_str().to_owned()),
                                        );
                                    }
                                }
                                match crate::features::expand_feature_effects(
                                    &d.definition.program, &d.definition.declarations,
                                    &crate::FeatureRequest { explicit: features.into_iter().collect(), no_defaults: true, all: false },
                                    &BTreeSet::new(),
                                ) {
                                    Err(error) => Activation::Rejected(error.to_string()),
                                    Ok(expansion) if d.definition.constraints.members().iter().any(|m|
                                        expansion.active_features.contains(m.as_str()) && !selected.contains(m.as_str())) => {
                                        Activation::Rejected("feature closure implicitly selects a constrained member".into())
                                    }
                                    Ok(expansion) => {
                                        known = Some(expansion);
                                        if stale_defaults {
                                            Activation::Rediscovery("declared default is not bound to this exact shown definition".into())
                                        } else {
                                            Activation::NeedsSelection(format!("mandatory groups: {:?}", outcome.needs))
                                        }
                                    }
                                }
                            } else {
                                let expansion = resolution
                                    .expansions
                                    .into_iter()
                                    .next()
                                    .flatten()
                                    .expect("resolved B2 outcome has expansion");
                                Activation::Ready(expansion)
                            }
                        }
                    }
                };
                // These requirements remain attached to this candidate even
                // when its final effects cannot yet be returned to the caller.
                if let Some(expansion) = match &activation {
                    Activation::Ready(expansion) => Some(expansion),
                    Activation::NeedsSelection(_) | Activation::Rediscovery(_) => known.as_ref(),
                    Activation::Rejected(_) => None,
                } {
                    manifest.requires.packages.clear();
                    for id in &expansion.active_deps {
                        let declaration = &d.definition.declarations.dependencies
                            [&DependencyDeclarationId::new(id).expect("validated ID")];
                        let DependencySource::Registry(dep) = &declaration.source else {
                            unreachable!()
                        };
                        let dep_group = dep.group.as_ref().ok_or_else(|| {
                            ConfigurationSolveError::Unsupported(
                                "unqualified induced dependency".into(),
                            )
                        })?;
                        // Missing catalog closure is an operational
                        // error, not a candidate incompatibility.
                        match self.provider.list_versions(dep_group, dep.name.as_str()) {
                            Err(error @ DepProviderError::Other(_)) => {
                                return Err(error.into());
                            }
                            _ => {}
                        }
                        manifest.requires.packages.push(dep.clone());
                    }
                }
                match &activation {
                    Activation::Ready(_) => {
                        reason.reason = format!(
                            "candidate-local dependencies: {:?}",
                            manifest.requires.packages
                        )
                    }
                    Activation::Rejected(s)
                    | Activation::NeedsSelection(s)
                    | Activation::Rediscovery(s) => reason.reason = s.clone(),
                }
                provider.candidates.insert(
                    (group.clone(), name.clone(), version),
                    Candidate {
                        manifest,
                        activation,
                        reason,
                    },
                );
            }
        }
        let rejected: Vec<_> = provider
            .candidates
            .values()
            .filter(|c| matches!(c.activation, Activation::Rejected(_)))
            .map(|c| c.reason.clone())
            .collect();
        let candidate_reasons = provider
            .candidates
            .values()
            .map(|c| c.reason.clone())
            .collect();
        let solver = ResolvoDepSolver::new(provider);
        let recorded = match solver.solve_recorded_transaction(roots) {
            Ok(graph) => graph,
            Err(SolveError::Provider(error)) => return Err(error.into()),
            Err(solver) => {
                return Err(ConfigurationSolveError::Conflict {
                    request: request.clone(),
                    candidates: candidate_reasons,
                    solver,
                });
            }
        };
        let graph = recorded.graph;
        let provider = solver.into_inner();
        let mut manifests = BTreeMap::new();
        let mut needs = Vec::new();
        let mut rediscovery = Vec::new();
        let mut effects = BTreeMap::new();
        let mut providers = BTreeMap::new();
        for node in graph.iter() {
            let key = (node.group.clone(), node.name.clone(), node.version.clone());
            let coordinate = format!("{}/{}@{}", node.group, node.name, node.version);
            let metadata = self
                .provider
                .metadata(&node.group, &node.name, &node.version)?;
            providers.insert(
                coordinate.clone(),
                metadata
                    .manifest
                    .provides
                    .capabilities
                    .iter()
                    .map(ToString::to_string)
                    .collect(),
            );
            manifests.insert(
                coordinate.clone(),
                provider.fetch_manifest(&node.group, &node.name, &node.version)?,
            );
            let expansion = if let Some(candidate) = provider.candidates.get(&key) {
                match &candidate.activation {
                    Activation::Ready(expansion) => expansion.clone(),
                    Activation::NeedsSelection(_) => {
                        needs.push(candidate.reason.clone());
                        FeatureExpansion::default()
                    }
                    Activation::Rediscovery(_) => {
                        rediscovery.push(candidate.reason.clone());
                        FeatureExpansion::default()
                    }
                    Activation::Rejected(_) => {
                        return Err(DepProviderError::Other(
                            "solver selected rejected candidate".into(),
                        )
                        .into());
                    }
                }
            } else {
                crate::features::expand_manifest_features(
                    &metadata.manifest,
                    &crate::FeatureRequest {
                        no_defaults: true,
                        ..Default::default()
                    },
                )
                .map_err(|e| {
                    ConfigurationSolveError::Source(DepProviderError::Other(e.to_string()))
                })?
            };
            effects.insert(coordinate, expansion);
        }
        if !rediscovery.is_empty() {
            return Err(ConfigurationSolveError::Rediscovery {
                request: request.clone(),
                candidates: rediscovery,
            });
        }
        if !needs.is_empty() {
            return Err(ConfigurationSolveError::NeedsSelection {
                request: request.clone(),
                candidates: needs,
            });
        }
        Ok(ConfiguredResolution {
            graph,
            request: request.clone(),
            effects,
            providers,
            rejected,
            choices: recorded.choices,
            manifests,
        })
    }
}

impl DepProvider for ConfiguredCatalog<'_> {
    fn resolve_version(&self, package: &PackageRef) -> Result<semver::Version, DepProviderError> {
        self.catalog.resolve_version(package)
    }
    fn fetch_manifest(
        &self,
        group: &Group,
        name: &str,
        version: &semver::Version,
    ) -> Result<Manifest, DepProviderError> {
        match self
            .candidates
            .get(&(group.clone(), name.into(), version.clone()))
        {
            Some(candidate) => Ok(candidate.manifest.clone()),
            None => self.catalog.fetch_manifest(group, name, version),
        }
    }
}
impl VersionEnumerator for ConfiguredCatalog<'_> {
    fn edge_blocked(&self, parent: &str, target: &str) -> bool {
        self.blocked.contains(&(parent.into(), target.into()))
    }
    fn retained_choices(&self) -> &[super::SelectedChoice] {
        self.retained
    }

    fn cached_capabilities(&self) -> Option<&super::CapIndex> {
        self.catalog.cached_capabilities()
    }
    fn list_versions(
        &self,
        group: &Group,
        name: &str,
    ) -> Result<Vec<semver::Version>, DepProviderError> {
        self.catalog.list_versions(group, name)
    }
    fn candidate_rejection(
        &self,
        group: &Group,
        name: &str,
        version: &semver::Version,
    ) -> Option<String> {
        self.candidates
            .get(&(group.clone(), name.into(), version.clone()))
            .and_then(|c| match &c.activation {
                Activation::Rejected(reason) => Some(format!(
                    "{}: {reason}; original consumer edges {:?}; definition {}",
                    c.reason.coordinate, c.reason.edges, c.reason.identity.definition
                )),
                _ => None,
            })
    }
}
