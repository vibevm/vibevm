//! Immutable admitted intent shared by production transaction routes.
use super::configuration::{CandidateConfiguration, DefaultBinding};
use super::{RecordedResolution, SelectedChoice, TransactionCatalog};
use crate::{
    DepProvider, DepProviderError, FeatureExpansion, FeatureRequest, SolveError, VersionEnumerator,
};
use std::collections::{BTreeMap, BTreeSet};
use vibe_core::{Group, PackageRef, manifest::Manifest};
use vibe_registry::CandidateIdentity;

#[derive(Debug, Clone)]
pub struct ConfigurationInput {
    pub request: vibe_wire::generated::consumer_configuration::e1::request::Request,
    pub definitions: Vec<CandidateConfiguration>,
    pub defaults: Vec<DefaultBinding>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RootConstraint {
    pub constraint: PackageRef,
    pub consumer: String,
    pub declaration: String,
}
#[derive(Debug, Clone)]
pub struct NormalizedRequest {
    pub roots: Vec<PackageRef>,
    pub root_origins: Vec<RootConstraint>,
    pub features: FeatureRequest,
    pub configuration: Option<ConfigurationInput>,
}
impl NormalizedRequest {
    pub fn legacy(roots: &[PackageRef], features: &FeatureRequest) -> Self {
        Self {
            roots: roots.to_vec(),
            root_origins: roots
                .iter()
                .enumerate()
                .map(|(index, p)| RootConstraint {
                    constraint: p.clone(),
                    consumer: "caller".into(),
                    declaration: format!("roots[{index}]"),
                })
                .collect(),
            features: features.clone(),
            configuration: None,
        }
    }
    pub fn catalog_roots(&self) -> Vec<PackageRef> {
        let mut roots = self.roots.clone();
        if let Some(configuration) = &self.configuration {
            for definition in &configuration.definitions {
                roots.push(definition.package.clone());
                for declaration in definition.definition.declarations.dependencies.values() {
                    if let vibe_core::manifest::feature_effects::DependencySource::Registry(p) =
                        &declaration.source
                    {
                        roots.push(p.clone());
                    }
                }
            }
        }
        roots
    }
    /// Add route constraints; never deduplicate or substitute a coordinate.
    pub fn all_roots(&self, route: &[PackageRef]) -> Vec<PackageRef> {
        let mut roots = self.roots.clone();
        roots.extend_from_slice(route);
        roots
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EdgeProvenance {
    pub selected: SelectedChoice,
    pub declaration_identity: CandidateIdentity,
    pub provider_identity: CandidateIdentity,
    pub configured_definition: Option<crate::features::configuration::ConfigurationDefinition>,
}
#[derive(Debug, Clone)]
pub struct RouteResolution {
    pub recorded: RecordedResolution,
    pub provenance: Vec<EdgeProvenance>,
    pub manifests: BTreeMap<String, Manifest>,
    pub effects: BTreeMap<String, FeatureExpansion>,
}
impl RouteResolution {
    pub fn bind(
        catalog: &TransactionCatalog,
        request: &NormalizedRequest,
        recorded: RecordedResolution,
        manifests: BTreeMap<String, Manifest>,
        effects: BTreeMap<String, FeatureExpansion>,
    ) -> Result<Self, SolveError> {
        let identity = |p: &PackageRef| -> Result<CandidateIdentity, SolveError> {
            let version = catalog.resolve_version(p)?;
            Ok(catalog
                .metadata(
                    p.group.as_ref().ok_or_else(|| {
                        DepProviderError::Other("unqualified selected edge".into())
                    })?,
                    p.name.as_str(),
                    &version,
                )?
                .identity
                .clone())
        };
        let mut provenance = Vec::new();
        for selected in &recorded.choices {
            let definition = request.configuration.as_ref().and_then(|c| {
                c.definitions.iter().find(|d| {
                    d.package.group == selected.parent.group
                        && d.package.name == selected.parent.name
                        && selected.parent.version.matches(&d.version)
                })
            });
            let mut selected = selected.clone();
            if selected.declaration.starts_with("requires.packages[") {
                if let Some(definition) = definition {
                    // ConfiguredCatalog emits requirements in active declaration
                    // order. Bind that exact slot, even when two declarations
                    // intentionally target the same package coordinate.
                    let index = selected
                        .declaration
                        .strip_prefix("requires.packages[")
                        .and_then(|s| s.strip_suffix(']'))
                        .and_then(|s| s.parse::<usize>().ok());
                    let key = format!(
                        "{}@{}",
                        selected.parent.qualified_name(),
                        catalog.resolve_version(&selected.parent)?
                    );
                    let id = index.and_then(|i| effects.get(&key)?.active_deps.iter().nth(i));
                    let declaration = id.and_then(|id| {
                        definition
                            .definition
                            .declarations
                            .dependencies
                            .iter()
                            .find(|(key, _)| key.as_str() == id)
                    });
                    let Some((id, declaration)) = declaration else {
                        return Err(SolveError::UnsupportedRoute(
                            "selected configured dependency lost its declaration slot".into(),
                        ));
                    };
                    if !matches!(&declaration.source,
                        vibe_core::manifest::feature_effects::DependencySource::Registry(p)
                            if p.group == selected.provider.group && p.name == selected.provider.name)
                    {
                        return Err(SolveError::UnsupportedRoute(
                            "selected configured dependency differs from its declaration".into(),
                        ));
                    }
                    selected.declaration = format!("configuration.dependencies[{}]", id.as_str());
                }
            }
            provenance.push(EdgeProvenance {
                selected: selected.clone(),
                declaration_identity: identity(&selected.parent)?,
                provider_identity: identity(&selected.provider)?,
                configured_definition: definition.map(|d| d.definition.clone()),
            });
        }
        for node in recorded.graph.iter() {
            for dep in &node.dependencies {
                if !recorded.choices.iter().any(|e| {
                    e.parent.group.as_ref() == Some(&node.group)
                        && e.parent.name == node.name
                        && e.parent.version.matches(&node.version)
                        && e.provider.group == dep.group
                        && e.provider.name == dep.name
                        && recorded
                            .graph
                            .find(
                                dep.group.as_ref().expect("qualified graph edge"),
                                dep.name.as_str(),
                            )
                            .is_some_and(|n| {
                                e.provider.version.matches(&n.version)
                                    && dep.version.matches(&n.version)
                            })
                }) {
                    return Err(SolveError::UnsupportedRoute(
                        "selected graph edge has no declaration/provider witness".into(),
                    ));
                }
            }
        }
        Ok(Self {
            recorded,
            provenance,
            manifests,
            effects,
        })
    }
}

pub struct RouteProvider<'a> {
    pub catalog: &'a TransactionCatalog,
    pub blocked: &'a BTreeSet<(String, String)>,
    pub retained: &'a [SelectedChoice],
}
impl DepProvider for RouteProvider<'_> {
    fn resolve_version(&self, p: &PackageRef) -> Result<semver::Version, DepProviderError> {
        self.catalog.resolve_version(p)
    }
    fn fetch_manifest(
        &self,
        g: &Group,
        n: &str,
        v: &semver::Version,
    ) -> Result<Manifest, DepProviderError> {
        self.catalog.required_source_manifest(g, n, v)
    }
}
impl VersionEnumerator for RouteProvider<'_> {
    fn list_versions(&self, g: &Group, n: &str) -> Result<Vec<semver::Version>, DepProviderError> {
        self.catalog.list_versions(g, n)
    }
    fn cached_capabilities(&self) -> Option<&super::CapIndex> {
        self.catalog.cached_capabilities()
    }
    fn edge_blocked(&self, p: &str, t: &str) -> bool {
        self.blocked.contains(&(p.into(), t.into()))
    }
    fn retained_choices(&self) -> &[SelectedChoice] {
        self.retained
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConditionalEdge {
    pub parent: PackageRef,
    pub declaration: String,
    pub dependency: PackageRef,
}

pub use vibe_wire::behaviour::consumer_configuration as consumer_codec;
pub use vibe_wire::generated::consumer_configuration::e1::request as consumer_wire;

/// Metadata projection for the incumbent naive/SAT cells. Previously selected
/// witnesses become exact requirements; no alternative is chosen here.
pub struct LegacyRouteProvider<'a>(pub RouteProvider<'a>);
impl LegacyRouteProvider<'_> {
    fn project(
        &self,
        g: &Group,
        n: &str,
        v: &semver::Version,
    ) -> Result<(Manifest, BTreeMap<String, (String, Vec<usize>)>), DepProviderError> {
        let mut manifest = self.0.fetch_manifest(g, n, v)?;
        let parent = format!("{g}/{n}");
        let retained = |declaration: &str| {
            self.0.retained.iter().find(|e| {
                e.parent.group.as_ref() == Some(g)
                    && e.parent.name == n
                    && e.parent.version.matches(v)
                    && e.declaration == declaration
            })
        };
        let mut map = BTreeMap::new();
        let mut packages = Vec::new();
        for (index, dep) in manifest.requires.packages.iter().enumerate() {
            if self.0.edge_blocked(&parent, &dep.qualified_name()) {
                continue;
            }
            map.insert(
                format!("requires.packages[{}]", packages.len()),
                (format!("requires.packages[{index}]"), vec![0]),
            );
            let mut dep = dep.clone();
            if let Some(edge) = retained(&format!("requires.packages[{index}]")) {
                dep.version = edge.provider.version.clone();
            }
            packages.push(dep);
        }
        let mut capabilities = Vec::new();
        for (index, cap) in manifest.requires.capabilities.iter().enumerate() {
            let declaration = format!("requires.capabilities[{index}]:{cap}");
            if let Some(edge) = retained(&declaration) {
                if self
                    .0
                    .edge_blocked(&parent, &edge.provider.qualified_name())
                {
                    continue;
                }
                map.insert(
                    format!("requires.packages[{}]", packages.len()),
                    (declaration, vec![edge.alternative]),
                );
                packages.push(edge.provider.clone());
            } else {
                map.insert(
                    format!("requires.capabilities[{}]:{cap}", capabilities.len()),
                    (declaration, vec![0]),
                );
                capabilities.push(cap.clone());
            }
        }
        manifest.requires.packages = packages;
        manifest.requires.capabilities = capabilities;
        let mut disjunctions = Vec::new();
        for (index, original) in manifest.requires_any.iter().enumerate() {
            let declaration = format!("requires_any[{index}]");
            let mut disjunction = original.clone();
            let indices = if let Some(edge) = retained(&declaration) {
                if self
                    .0
                    .edge_blocked(&parent, &edge.provider.qualified_name())
                {
                    continue;
                }
                disjunction.one_of = vec![edge.provider.clone()];
                vec![edge.alternative]
            } else {
                let mut indices = Vec::new();
                let mut alternatives = Vec::new();
                for (i, alt) in original.one_of.iter().enumerate() {
                    if !self.0.edge_blocked(&parent, &alt.qualified_name()) {
                        indices.push(i);
                        alternatives.push(alt.clone());
                    }
                }
                disjunction.one_of = alternatives;
                indices
            };
            if disjunction.one_of.is_empty() {
                continue;
            }
            map.insert(
                format!("requires_any[{}]", disjunctions.len()),
                (declaration, indices),
            );
            disjunctions.push(disjunction);
        }
        manifest.requires_any = disjunctions;
        Ok((manifest, map))
    }
    pub fn restore_provenance(&self, result: &mut RecordedResolution) -> Result<(), SolveError> {
        for edge in &mut result.choices {
            let version = self.0.catalog.resolve_version(&edge.parent)?;
            let (_, map) = self.project(
                edge.parent.group.as_ref().expect("qualified selected edge"),
                edge.parent.name.as_str(),
                &version,
            )?;
            let (declaration, indices) = map.get(&edge.declaration).ok_or_else(|| {
                SolveError::UnsupportedRoute(
                    "legacy route lost selected declaration mapping".into(),
                )
            })?;
            edge.alternative = *indices.get(edge.alternative).ok_or_else(|| {
                SolveError::UnsupportedRoute(
                    "legacy route lost selected alternative mapping".into(),
                )
            })?;
            edge.declaration = declaration.clone();
        }
        result
            .choices
            .sort_by_key(|e| (e.parent.to_string(), e.declaration.clone(), e.alternative));
        Ok(())
    }
}
impl DepProvider for &LegacyRouteProvider<'_> {
    fn resolve_version(&self, p: &PackageRef) -> Result<semver::Version, DepProviderError> {
        self.0.catalog.resolve_version(p)
    }
    fn fetch_manifest(
        &self,
        g: &Group,
        n: &str,
        v: &semver::Version,
    ) -> Result<Manifest, DepProviderError> {
        Ok(self.project(g, n, v)?.0)
    }
}
