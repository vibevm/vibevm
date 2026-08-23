specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-050#effective-set");

use std::collections::BTreeSet;

use vibe_core::manifest::{Manifest, Requires};
use vibe_core::visibility::{Analysis, EdgeDecl, NodeDecl, NodeId, VisibilityGraph, analyze};
use vibe_core::{Group, PackageRef};
use vibe_resolver::{
    DepProvider, DepProviderError, DepSolver, NaiveDepSolver, ResolvedGraph, ResolvoDepSolver,
    SolveError, VersionEnumerator,
};

use crate::InstallSource;
use crate::error::{Error, Result};
use crate::record::exact_pinned_pkgref;

/// Everything the projection needs from the solve side: the resolved graph,
/// the consumer root, and a metadata-only manifest oracle.
///
/// ```
/// use vibe_core::{PackageRef, manifest::Manifest};
/// use vibe_install::ProjectionInput;
/// use vibe_resolver::ResolvedGraph;
///
/// let root = Manifest::parse_str(
///     "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n",
/// ).unwrap();
/// let graph = ResolvedGraph::default();
/// let manifest_of = |_: &PackageRef| unreachable!("an empty graph asks for no package manifest");
/// let input = ProjectionInput {
///     root_manifest: &root,
///     root_id: "org.test/demo".into(),
///     graph: &graph,
///     manifest_of: &manifest_of,
/// };
/// assert!(input.graph.packages.is_empty());
/// ```
pub struct ProjectionInput<'a> {
    pub root_manifest: &'a Manifest,
    pub root_id: NodeId,
    pub graph: &'a ResolvedGraph,
    pub manifest_of: &'a dyn Fn(&PackageRef) -> std::result::Result<Manifest, SolveError>,
}

/// The pure visibility verdict and the solver mask it induces.
///
/// ```
/// use std::collections::BTreeSet;
/// use vibe_core::visibility::Analysis;
/// use vibe_install::Projection;
///
/// let projection = Projection {
///     analysis: Analysis::default(),
///     blocked_edges: BTreeSet::new(),
/// };
/// assert!(projection.blocked_edges.is_empty());
/// ```
pub struct Projection {
    pub analysis: Analysis,
    pub blocked_edges: BTreeSet<(NodeId, NodeId)>,
}

pub(crate) struct EffectiveResolution {
    pub graph: ResolvedGraph,
    pub analysis: Analysis,
    pub iterations: usize,
}

/// Project a solved graph onto the consumer root's effective set.
///
/// Package manifests are obtained only through `manifest_of`; projection
/// therefore performs metadata reads and never fetches package content.
///
/// ```
/// use vibe_core::{PackageRef, manifest::Manifest};
/// use vibe_install::{project, ProjectionInput};
/// use vibe_resolver::ResolvedGraph;
///
/// let root = Manifest::parse_str(
///     "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n",
/// ).unwrap();
/// let graph = ResolvedGraph::default();
/// let manifest_of = |_: &PackageRef| unreachable!("an empty graph asks for no package manifest");
/// let result = project(&ProjectionInput {
///     root_manifest: &root,
///     root_id: "org.test/demo".into(),
///     graph: &graph,
///     manifest_of: &manifest_of,
/// }).unwrap();
/// assert!(result.analysis.effective.is_empty());
/// ```
pub fn project(input: &ProjectionInput<'_>) -> Result<Projection> {
    let mut graph = VisibilityGraph::default();
    let mut root_decl = declaration_from_manifest(input.root_manifest)?;

    // Solver roots include CLI roots and conditional-dependency extras. They
    // are direct consumer choices even when they are not written in the root
    // manifest, so ensure every one has a root edge in the projection.
    for node in input.graph.roots() {
        let target = node_id(&node.group, &node.name);
        if !root_decl.edges.iter().any(|edge| edge.to == target) {
            root_decl.edges.push(EdgeDecl {
                to: target,
                ..EdgeDecl::default()
            });
        }
    }
    graph.nodes.insert(input.root_id.clone(), root_decl);

    for node in input.graph.iter() {
        let pkgref = exact_pinned_pkgref(node);
        let manifest = (input.manifest_of)(&pkgref)?;
        graph.nodes.insert(
            node_id(&node.group, &node.name),
            declaration_from_manifest(&manifest)?,
        );
    }

    let analysis = analyze(&graph, &input.root_id);
    let mut blocked_edges = BTreeSet::new();
    for node in input.graph.iter() {
        let parent = node_id(&node.group, &node.name);
        for dependency in &node.dependencies {
            let target = dependency.qualified_name();
            if !edge_contributes(&graph, &input.root_id, &parent, &target) {
                blocked_edges.insert((parent.clone(), target));
            }
        }
    }
    Ok(Projection {
        analysis,
        blocked_edges,
    })
}

fn edge_contributes(
    graph: &VisibilityGraph,
    root: &NodeId,
    parent: &NodeId,
    target: &NodeId,
) -> bool {
    let mut isolated = graph.clone();
    for (coordinate, declaration) in &mut isolated.nodes {
        if coordinate != parent {
            declaration
                .edges
                .retain(|edge| edge.to.as_str() != target.as_str());
        }
    }
    analyze(&isolated, root).effective.contains_key(target)
}

pub(crate) fn resolve_effective<S: InstallSource + ?Sized>(
    source: &S,
    roots: &[PackageRef],
    root_manifest: &Manifest,
    root_id: &str,
    initial_graph: ResolvedGraph,
) -> Result<EffectiveResolution> {
    const MAX_ITERATIONS: usize = 4;

    let mut graph = initial_graph;
    let mut blocked = BTreeSet::new();
    for iteration in 1..=MAX_ITERATIONS {
        let manifest_of = |pkgref: &PackageRef| source.manifest_of(pkgref);
        let projection = project(&ProjectionInput {
            root_manifest,
            root_id: root_id.to_string(),
            graph: &graph,
            manifest_of: &manifest_of,
        })?;
        let newly_blocked: BTreeSet<_> = projection
            .blocked_edges
            .difference(&blocked)
            .cloned()
            .collect();
        if newly_blocked.is_empty() {
            prune_graph(&mut graph, &projection.analysis);
            return Ok(EffectiveResolution {
                graph,
                analysis: projection.analysis,
                iterations: iteration,
            });
        }
        if iteration == MAX_ITERATIONS {
            let unstable: Vec<String> = newly_blocked
                .iter()
                .flat_map(|(from, to)| [from.clone(), to.clone()])
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect();
            return Err(visibility_error(format!(
                "visibility strictness did not converge within {MAX_ITERATIONS} iterations; \
                 unstable nodes: {unstable:?}"
            )));
        }
        blocked.extend(newly_blocked);
        graph = source.solve_masked(roots, &blocked)?;
    }
    Err(visibility_error(format!(
        "visibility strictness did not converge within {MAX_ITERATIONS} iterations"
    )))
}

fn declaration_from_manifest(manifest: &Manifest) -> Result<NodeDecl> {
    let edges = manifest
        .requires
        .iter_pkgrefs()
        .filter_map(|(group, name)| {
            group.map(|group| EdgeDecl {
                to: node_id(group, name),
                access: manifest.requires.declared_access(group, name),
                friend: manifest.requires.declared_friend(group, name),
                exclude: manifest.requires.excludes_for(group, name).to_vec(),
            })
        })
        .collect();
    let visibility = manifest.visibility.clone().unwrap_or_default();
    let overrides = manifest
        .override_table
        .as_ref()
        .map(|table| {
            table
                .targets()
                .map(|targets| {
                    targets
                        .into_iter()
                        .map(|(target, entry)| (target, entry.clone()))
                        .collect()
                })
                .map_err(|reason| {
                    visibility_error(format!(
                        "visibility projection could not read a declared override: {reason}"
                    ))
                })
        })
        .transpose()?
        .unwrap_or_default();
    Ok(NodeDecl {
        edges,
        friends: visibility.friends,
        unfriend: visibility.unfriend,
        allow_friends: visibility.allow_friends,
        overrides,
    })
}

fn node_id(group: &Group, name: &str) -> NodeId {
    format!("{group}/{name}")
}

fn visibility_error(reason: String) -> Error {
    Error::Solve(
        DepProviderError::Other(format!(
            "{reason} (violates spec://org.vibevm.core/vibevm/common/PROP-050#resolver; \
             fix: inspect the named packages' visibility edges and overrides)"
        ))
        .into(),
    )
}

fn prune_graph(graph: &mut ResolvedGraph, analysis: &Analysis) {
    graph.packages.retain(|node| {
        analysis
            .effective
            .contains_key(&node_id(&node.group, &node.name))
    });
    for node in &mut graph.packages {
        node.dependencies.retain(|dependency| {
            analysis
                .effective
                .contains_key(&dependency.qualified_name())
        });
    }
}

/// A metadata provider decorator that removes masked parent-to-target edges
/// before the selected solver cell reads a manifest.
///
/// ```
/// use std::collections::BTreeSet;
/// use vibe_install::FilteringDepProvider;
///
/// struct Provider;
/// let blocked = BTreeSet::new();
/// let filtered = FilteringDepProvider::new(&Provider, &blocked);
/// assert!(filtered.blocked_edges().is_empty());
/// ```
pub struct FilteringDepProvider<'a, P: ?Sized> {
    inner: &'a P,
    blocked: &'a BTreeSet<(String, String)>,
}

impl<'a, P: ?Sized> FilteringDepProvider<'a, P> {
    /// Wrap `inner` with a stable parent-to-target mask.
    ///
    /// ```
    /// use std::collections::BTreeSet;
    /// use vibe_install::FilteringDepProvider;
    ///
    /// let provider = ();
    /// let blocked = BTreeSet::from([("org.x/a".into(), "org.x/b".into())]);
    /// let filtered = FilteringDepProvider::new(&provider, &blocked);
    /// assert_eq!(filtered.blocked_edges().len(), 1);
    /// ```
    pub fn new(inner: &'a P, blocked: &'a BTreeSet<(String, String)>) -> Self {
        Self { inner, blocked }
    }

    /// Inspect the immutable mask supplied at construction.
    ///
    /// ```
    /// use std::collections::BTreeSet;
    /// use vibe_install::FilteringDepProvider;
    ///
    /// let provider = ();
    /// let blocked = BTreeSet::new();
    /// let filtered = FilteringDepProvider::new(&provider, &blocked);
    /// assert!(filtered.blocked_edges().is_empty());
    /// ```
    pub fn blocked_edges(&self) -> &BTreeSet<(String, String)> {
        self.blocked
    }
}

impl<P: DepProvider + ?Sized> DepProvider for FilteringDepProvider<'_, P> {
    fn resolve_version(&self, pkgref: &PackageRef) -> Result<semver::Version, DepProviderError> {
        self.inner.resolve_version(pkgref)
    }

    fn fetch_manifest(
        &self,
        group: &Group,
        name: &str,
        version: &semver::Version,
    ) -> Result<Manifest, DepProviderError> {
        let mut manifest = self.inner.fetch_manifest(group, name, version)?;
        let parent = node_id(group, name);
        let targets: BTreeSet<String> = self
            .blocked
            .iter()
            .filter(|(from, _)| from == &parent)
            .map(|(_, to)| to.clone())
            .collect();
        if !targets.is_empty() {
            prune_manifest(&mut manifest, &targets);
        }
        Ok(manifest)
    }
}

impl<P: VersionEnumerator + ?Sized> VersionEnumerator for FilteringDepProvider<'_, P> {
    fn list_versions(
        &self,
        group: &Group,
        name: &str,
    ) -> Result<Vec<semver::Version>, DepProviderError> {
        self.inner.list_versions(group, name)
    }
}

/// Read one exact package manifest through an arbitrary metadata provider.
///
/// ```
/// use vibe_core::{Group, PackageRef, manifest::Manifest};
/// use vibe_install::metadata_manifest;
/// use vibe_resolver::{DepProvider, DepProviderError};
/// struct Missing;
/// impl DepProvider for Missing {
///     fn resolve_version(&self, _: &PackageRef) -> Result<semver::Version, DepProviderError> {
///         Err(DepProviderError::Other("missing".into()))
///     }
///     fn fetch_manifest(&self, _: &Group, _: &str, _: &semver::Version)
///         -> Result<Manifest, DepProviderError> { unreachable!() }
/// }
/// assert!(metadata_manifest(&Missing, &PackageRef::parse("org.x/missing").unwrap()).is_err());
/// ```
pub fn metadata_manifest<P: DepProvider>(
    provider: &P,
    pkg: &PackageRef,
) -> std::result::Result<Manifest, SolveError> {
    let group = pkg.group.as_ref().ok_or_else(|| {
        DepProviderError::Other(format!(
            "metadata lookup requires a qualified package reference, got `{pkg}`"
        ))
    })?;
    let version = provider.resolve_version(pkg)?;
    Ok(provider.fetch_manifest(group, pkg.name.as_str(), &version)?)
}

/// Run a selected solver cell through a stable visibility edge mask.
///
/// ```
/// use std::collections::BTreeSet;
/// use vibe_core::{Group, PackageRef, manifest::Manifest};
/// use vibe_install::solve_with_visibility_mask;
/// use vibe_resolver::{DepProvider, DepProviderError, VersionEnumerator};
/// struct Empty;
/// impl DepProvider for Empty {
///     fn resolve_version(&self, _: &PackageRef) -> Result<semver::Version, DepProviderError> {
///         unreachable!()
///     }
///     fn fetch_manifest(&self, _: &Group, _: &str, _: &semver::Version)
///         -> Result<Manifest, DepProviderError> { unreachable!() }
/// }
/// impl VersionEnumerator for Empty {
///     fn list_versions(&self, _: &Group, _: &str)
///         -> Result<Vec<semver::Version>, DepProviderError> { unreachable!() }
/// }
/// let graph = solve_with_visibility_mask(Empty, None, &[], &BTreeSet::new()).unwrap();
/// assert!(graph.packages.is_empty());
/// ```
pub fn solve_with_visibility_mask<P: VersionEnumerator>(
    provider: P,
    solver: Option<&str>,
    roots: &[PackageRef],
    blocked: &BTreeSet<(String, String)>,
) -> std::result::Result<ResolvedGraph, SolveError> {
    let filtered = FilteringDepProvider::new(&provider, blocked);
    match solver.unwrap_or("resolvo") {
        "resolvo" => ResolvoDepSolver::new(filtered).solve(roots),
        "naive" => NaiveDepSolver::new(filtered).solve(roots),
        "sat" => vibe_resolver::sat::SatDepSolver::new(filtered).solve(roots),
        other => Err(DepProviderError::Other(format!(
            "unknown solver cell `{other}` while applying the visibility mask"
        ))
        .into()),
    }
}

fn prune_manifest(manifest: &mut Manifest, targets: &BTreeSet<String>) {
    prune_requires(&mut manifest.requires, targets);
    manifest
        .recommends
        .packages
        .retain(|dependency| !targets.contains(&dependency.qualified_name()));
    for disjunction in &mut manifest.requires_any {
        disjunction
            .one_of
            .retain(|dependency| !targets.contains(&dependency.qualified_name()));
    }
    manifest
        .requires_any
        .retain(|disjunction| !disjunction.one_of.is_empty());
    for conditional in manifest.conditional_deps.values_mut() {
        prune_requires(&mut conditional.dependencies, targets);
    }
}

fn prune_requires(requires: &mut Requires, targets: &BTreeSet<String>) {
    requires
        .packages
        .retain(|dependency| !targets.contains(&dependency.qualified_name()));
    requires
        .git_packages
        .retain(|dependency| !targets.contains(&node_id(&dependency.group, &dependency.name)));
    requires
        .path_packages
        .retain(|dependency| !targets.contains(&node_id(&dependency.group, &dependency.name)));
    requires
        .var_packages
        .retain(|dependency| !targets.contains(&node_id(&dependency.group, &dependency.name)));
    requires.links.retain(|target, _| !targets.contains(target));
    requires
        .accesses
        .retain(|target, _| !targets.contains(target));
    requires
        .friend_flags
        .retain(|target, _| !targets.contains(target));
    requires
        .excludes
        .retain(|target, _| !targets.contains(target));
}
