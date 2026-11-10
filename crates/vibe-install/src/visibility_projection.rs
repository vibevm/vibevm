specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-050#effective-set");

use std::collections::BTreeSet;

use vibe_core::manifest::Manifest;
use vibe_core::visibility::{Analysis, EdgeDecl, NodeDecl, NodeId, VisibilityGraph, analyze};
use vibe_core::{Group, PackageRef};
use vibe_resolver::mask_manifest_edges as prune_manifest;
use vibe_resolver::{
    DepProvider, DepProviderError, NaiveDepSolver, ResolvedGraph, ResolvoDepSolver, SolveError,
    VersionEnumerator,
};

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

fn declaration_from_manifest(manifest: &Manifest) -> Result<NodeDecl> {
    let mut edges: Vec<EdgeDecl> = manifest
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
    // An application's installer requirements are resolver edges too
    // (PROP-065 `#app-dependencies`), with the default access.
    for reference in manifest.resolution_requires() {
        let Some(group) = reference.group.as_ref() else {
            continue;
        };
        let to = node_id(group, reference.name.as_str());
        if !edges.iter().any(|edge| edge.to == to) {
            edges.push(EdgeDecl {
                to,
                access: None,
                friend: None,
                exclude: Vec::new(),
            });
        }
    }
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

/// Solve `roots` through `source` and keep exactly the consumer root's
/// effective set (PROP-050 `#EFFECTIVE-SET`): a private (or not befriended
/// friends-only) requirement of a non-root package is masked and the
/// selected cell re-solved until the mask is stable, then the graph is
/// pruned. The same law the install planner applies, for a caller that
/// solves an explicit root set (`vibe update <pkg>`, `--sources-only`).
///
/// ```
/// use std::collections::BTreeSet;
/// use vibe_core::{PackageRef, manifest::Manifest};
/// use vibe_resolver::{ResolvedGraph, SolveError};
/// # struct Empty;
/// # impl vibe_install::InstallSource for Empty {
/// #     fn resolve_and_fetch(&self, _: &PackageRef, _: &std::path::Path, _: Option<&str>)
/// #         -> Result<vibe_registry::CachedPackage, vibe_registry::RegistryError> { unreachable!() }
/// #     fn solve(&self, _: &[PackageRef]) -> Result<ResolvedGraph, SolveError> { Ok(ResolvedGraph::default()) }
/// #     fn manifest_of(&self, _: &PackageRef) -> Result<Manifest, SolveError> { unreachable!() }
/// #     fn solve_masked(&self, _: &[PackageRef], _: &BTreeSet<(String, String)>)
/// #         -> Result<ResolvedGraph, SolveError> { unreachable!() }
/// #     fn materialise_in_place(&self, _: &PackageRef, _: &std::path::Path)
/// #         -> Result<vibe_registry::InPlaceMaterialised, vibe_registry::RegistryError> { unreachable!() }
/// # }
/// let root = Manifest::parse_str("[project]\nname = \"demo\"\nversion = \"0.1.0\"\n").unwrap();
/// let graph = vibe_install::solve_visible_graph(&Empty, &root, &[], true).unwrap();
/// assert!(graph.packages.is_empty());
/// ```
pub fn solve_visible_graph<S: crate::InstallSource + ?Sized>(
    source: &S,
    root_manifest: &Manifest,
    roots: &[PackageRef],
    recommends: bool,
) -> Result<ResolvedGraph> {
    const MAX_VISIBILITY_PASSES: usize = 4;
    let root_id = crate::plan::visibility_root_id(root_manifest);
    let mut blocked = BTreeSet::new();
    let mut graph = if recommends {
        source.solve(roots)?
    } else {
        source.solve_without_recommends(roots)?
    };
    for _ in 0..MAX_VISIBILITY_PASSES {
        let manifest_of = |reference: &PackageRef| source.manifest_of(reference);
        let projection = project(&ProjectionInput {
            root_manifest,
            root_id: root_id.clone(),
            graph: &graph,
            manifest_of: &manifest_of,
        })?;
        let newly_blocked = projection
            .blocked_edges
            .difference(&blocked)
            .cloned()
            .collect::<BTreeSet<_>>();
        if newly_blocked.is_empty() {
            prune_graph(&mut graph, &projection.analysis);
            return Ok(graph);
        }
        blocked.extend(newly_blocked);
        graph = if recommends {
            source.solve_masked(roots, &blocked)?
        } else {
            source.solve_masked_without_recommends(roots, &blocked)?
        };
    }
    Err(visibility_error(format!(
        "visibility strictness did not converge within {MAX_VISIBILITY_PASSES} iterations"
    )))
}

pub(crate) fn visibility_error(reason: String) -> Error {
    Error::Solve(
        DepProviderError::Other(format!(
            "{reason} (violates spec://org.vibevm.core/vibevm/common/PROP-050#resolver; \
             fix: inspect the named packages' visibility edges and overrides)"
        ))
        .into(),
    )
}

pub(crate) fn prune_graph(graph: &mut ResolvedGraph, analysis: &Analysis) {
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
    fn validate_selected(
        &self,
        graph: &ResolvedGraph,
    ) -> std::result::Result<(), DepProviderError> {
        self.inner.validate_selected(graph)
    }

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
/// let graph = solve_with_visibility_mask(Empty, None, &[], &BTreeSet::new(), true).unwrap();
/// assert!(graph.packages.is_empty());
/// ```
pub fn solve_with_visibility_mask<P: VersionEnumerator>(
    provider: P,
    solver: Option<&str>,
    roots: &[PackageRef],
    blocked: &BTreeSet<(String, String)>,
    recommends: bool,
) -> std::result::Result<ResolvedGraph, SolveError> {
    let filtered = FilteringDepProvider::new(&provider, blocked);
    let solve = |solver: &dyn vibe_resolver::DepSolver| {
        if recommends {
            solver.solve(roots)
        } else {
            solver.solve_without_recommends(roots)
        }
    };
    match solver.unwrap_or("resolvo") {
        "resolvo" => solve(&ResolvoDepSolver::new(filtered)),
        "naive" => solve(&NaiveDepSolver::new(filtered)),
        "sat" => solve(&vibe_resolver::sat::SatDepSolver::new(filtered)),
        other => Err(DepProviderError::Other(format!(
            "unknown solver cell `{other}` while applying the visibility mask"
        ))
        .into()),
    }
}
