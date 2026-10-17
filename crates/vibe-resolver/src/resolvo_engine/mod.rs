//! `ResolvoDepSolver` — the production `DepSolver` cell: resolvo (CDCL
//! SAT) behind vibevm's solver seam (PROP-017).
//!
//! `solve` builds a [`VibevmResolvoProvider`] from the roots and the
//! vibevm [`VersionEnumerator`], runs `resolvo::Solver`, and maps the
//! chosen solvables back through the shared [`build_resolved_graph`]
//! output builder — so a resolvo graph is byte-identical to a naive one
//! wherever naive also solves (the differential oracle pins this).
//!
//! resolvo's provider is lazy: a package's versions are fetched only
//! when the search first asks, a manifest only when a solvable is
//! explored. The solver runs on the default `NowOrNeverRuntime`, so the
//! synchronous vibevm provider drives it with no async runtime.

mod capabilities;
mod catalog;
pub mod configuration;

#[cfg(test)]
mod m_05_f_1_cases;
pub mod request;
mod snapshot;
pub use capabilities::{CapIndex, CapProvider};
pub use catalog::{CatalogInspection, TransactionCatalog};
mod provider;
mod version_set;

use std::collections::{HashMap, HashSet};

use resolvo::{Problem, Solver, UnsolvableOrCancelled};
use specmark::{cell, spec};
use vibe_core::{Group, PackageRef};

use crate::{
    Chosen, DepProviderError, DepSolver, ResolvedGraph, SolveError, VersionEnumerator,
    assert_kind_matches, build_resolved_graph,
};
pub use provider::SelectedChoice;
use provider::VibevmResolvoProvider;

/// Selected SAT witnesses accompany the package projection. They are captured
/// during the solve, never guessed from package presence afterward.
#[derive(Debug, Clone)]
pub struct RecordedResolution {
    pub graph: ResolvedGraph,
    pub choices: Vec<SelectedChoice>,
}

/// The resolvo-backed `DepSolver`. Construct it the way the other solver
/// cells are constructed and call [`DepSolver::solve`]:
///
/// ```
/// use vibe_core::{Group, PackageRef, manifest::Manifest};
/// use vibe_resolver::{DepProvider, DepProviderError, DepSolver, VersionEnumerator,
///     ResolvoDepSolver};
///
/// struct OnePackage(Manifest);
/// impl DepProvider for OnePackage {
///     fn resolve_version(&self, _: &PackageRef)
///         -> Result<semver::Version, DepProviderError>
///     { Ok(self.0.require_package().unwrap().version.clone()) }
///     fn fetch_manifest(&self, _: &Group, _: &str, _: &semver::Version)
///         -> Result<Manifest, DepProviderError>
///     { Ok(self.0.clone()) }
/// }
/// impl VersionEnumerator for OnePackage {
///     fn list_versions(&self, _: &Group, _: &str)
///         -> Result<Vec<semver::Version>, DepProviderError>
///     { Ok(vec![self.0.require_package().unwrap().version.clone()]) }
/// }
///
/// let m = Manifest::parse_str(
///     "[package]\ngroup = \"org.vibevm\"\nname = \"wal\"\nkind = \"flow\"\nversion = \"0.1.0\"\n",
/// ).unwrap();
/// let solver = ResolvoDepSolver::new(OnePackage(m));
/// let graph = solver
///     .solve(&[PackageRef::parse("org.vibevm/wal").unwrap()])
///     .unwrap();
/// assert_eq!(graph.packages.len(), 1);
/// assert!(graph.packages[0].is_root);
/// ```
#[cell(seam = "DepSolver", variant = "resolvo")]
#[spec(implements = "spec://org.vibevm.core/vibevm/modules/vibe-resolver/PROP-017#architecture")]
pub struct ResolvoDepSolver<P: VersionEnumerator> {
    provider: P,
}

impl<P: VersionEnumerator> ResolvoDepSolver<P> {
    pub fn new(provider: P) -> Self {
        ResolvoDepSolver { provider }
    }

    pub fn into_inner(self) -> P {
        self.provider
    }
}

impl<P: VersionEnumerator> DepSolver for ResolvoDepSolver<P> {
    #[spec(implements = "spec://org.vibevm.core/vibevm/modules/vibe-resolver/PROP-017#dominance")]
    #[spec(implements = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#lockfile")]
    fn solve(&self, roots: &[PackageRef]) -> Result<ResolvedGraph, SolveError> {
        let transaction = ResolvoDepSolver::new(snapshot::MetadataSnapshot::new(&self.provider));
        transaction.solve_transaction(roots)
    }
}

impl<P: VersionEnumerator> ResolvoDepSolver<P> {
    fn solve_transaction(&self, roots: &[PackageRef]) -> Result<ResolvedGraph, SolveError> {
        Ok(self.solve_recorded_transaction(roots)?.graph)
    }

    pub fn solve_recorded(&self, roots: &[PackageRef]) -> Result<RecordedResolution, SolveError> {
        let transaction = ResolvoDepSolver::new(snapshot::MetadataSnapshot::new(&self.provider));
        transaction.solve_recorded_transaction(roots)
    }

    fn solve_recorded_transaction(
        &self,
        roots: &[PackageRef],
    ) -> Result<RecordedResolution, SolveError> {
        // Hard solve, then a best-effort greedy expansion over
        // `[recommends]` (PROP-003 §2.3.3): each recommended package is
        // tried via a re-solve and kept only if the graph stays
        // satisfiable; a recommend that conflicts is silently dropped,
        // never a failure. Recommended packages enter the graph but are
        // not roots. resolvo's `soft_requirements` is root-level only, so
        // per-package recommends ride this re-solve loop — batching them
        // as soft is a future optimisation.
        let mut extra: Vec<PackageRef> = Vec::new();
        let mut recommendation_edges: Vec<SelectedChoice> = Vec::new();
        let mut tried: HashSet<(Group, String)> = HashSet::new();
        let mut graph = self.solve_hard(roots, &extra)?;
        loop {
            let recommends = self.collect_recommends(&graph.graph, &tried)?;
            let Some((rec, witness)) = recommends.into_iter().next() else {
                break;
            };
            if let Some(group) = rec.group.clone() {
                tried.insert((group, rec.name.to_string()));
            }
            let mut trial = extra.clone();
            trial.push(rec);
            match self.solve_hard(roots, &trial) {
                Ok(extended) => {
                    // A recommendation belongs to its declaring candidate.
                    // Do not carry it across a trial that discards that candidate.
                    if recommendation_edges
                        .iter()
                        .chain(std::iter::once(&witness))
                        .all(|e| {
                            extended.graph.iter().any(|n| {
                                e.parent.group.as_ref() == Some(&n.group)
                                    && e.parent.name == n.name
                                    && e.parent.version.matches(&n.version)
                            })
                        })
                    {
                        recommendation_edges.push(witness);
                        extra = trial;
                        graph = extended;
                    }
                }
                Err(SolveError::Provider(error)) => return Err(error.into()),
                Err(_) => {}
            }
        }
        for mut edge in recommendation_edges {
            let target = graph
                .graph
                .iter()
                .find(|n| {
                    edge.provider.group.as_ref() == Some(&n.group) && edge.provider.name == n.name
                })
                .expect("accepted recommendation");
            edge.provider = PackageRef::parse(&format!(
                "{}/{}@={}",
                target.group, target.name, target.version
            ))
            .expect("selected coordinate");
            let parent = graph
                .graph
                .packages
                .iter_mut()
                .find(|n| {
                    edge.parent.group.as_ref() == Some(&n.group) && edge.parent.name == n.name
                })
                .expect("retained recommending candidate");
            if !parent.dependencies.contains(&edge.provider) {
                parent.dependencies.push(edge.provider.clone());
            }
            graph.choices.push(edge);
        }
        graph
            .choices
            .sort_by_key(|e| (e.parent.to_string(), e.declaration.clone(), e.alternative));
        Ok(graph)
    }
}

impl<P: VersionEnumerator> ResolvoDepSolver<P> {
    /// The hard solve — resolve `roots` plus any `extra` (recommends
    /// already accepted) into a graph. `roots` are the user's roots
    /// (marked `is_root`, validated up front); `extra` are pulled in but
    /// never roots and never pre-validated, so an absent recommend just
    /// yields no candidates and fails its trial.
    fn solve_hard(
        &self,
        roots: &[PackageRef],
        extra: &[PackageRef],
    ) -> Result<RecordedResolution, SolveError> {
        // Pre-scan the package closure (roots + extra) for capability
        // providers (PROP-017 §3) before building the resolvo provider.
        let scan_roots: Vec<PackageRef> = roots.iter().chain(extra.iter()).cloned().collect();
        let cap_index = capabilities::prescan(&self.provider, &scan_roots)?;
        let rp = VibevmResolvoProvider::new(&self.provider, cap_index);

        // Root requirements + the (group, name) root order (deduplicated,
        // input order preserved) the output builder needs.
        let mut requirements = Vec::with_capacity(roots.len() + extra.len());
        let mut root_order: Vec<(Group, String)> = Vec::with_capacity(roots.len());
        let mut seen = HashSet::new();
        for r in roots {
            let Some(group) = r.group.clone() else {
                return Err(SolveError::Provider(DepProviderError::Other(format!(
                    "root `{r}` is not group-qualified — resolution needs `<group>/<name>`"
                ))));
            };
            let vs = rp.intern_version_set(&group, r.name.as_str(), &r.version);
            requirements.push(vs.into());
            let key = (group, r.name.to_string());
            if seen.insert(key.clone()) {
                root_order.push(key);
            }
        }
        // Accepted recommends ride in as requirements, never roots. An
        // unqualified extra is skipped rather than erroring.
        for e in extra {
            if let Some(group) = e.group.clone() {
                let vs = rp.intern_version_set(&group, e.name.as_str(), &e.version);
                requirements.push(vs.into());
            }
        }

        // Validate user-named roots up front: a typo'd or absent root
        // gives a clean "not found" rather than a SAT-conflict derivation.
        for (group, name) in &root_order {
            if let Err(e) = self.provider.list_versions(group, name) {
                return Err(SolveError::Provider(e));
            }
        }

        let problem = Problem::new().requirements(requirements);
        let mut solver = Solver::new(rp);
        let outcome = solver.solve(problem);

        // A provider failure mid-solve (unknown package, fetch error)
        // trumps whatever resolvo concluded — surface it with its rich
        // message, matching the naive cell's discriminant.
        if let Some(err) = solver.provider().take_error() {
            return Err(SolveError::Provider(err));
        }

        match outcome {
            Ok(solvables) => {
                let rp = solver.provider();
                let mut chosen: HashMap<(Group, String), Chosen> = HashMap::new();
                let mut obsolete: HashSet<(Group, String)> = HashSet::new();
                let mut selected = Vec::new();
                let mut choices = rp.selected_choices(&solvables)?;
                for id in solvables {
                    if rp.is_choice(id) {
                        continue;
                    }
                    let Some((group, name, version)) = rp.solvable_parts(id) else {
                        return Err(SolveError::Provider(DepProviderError::Other(
                            "internal: a chosen solvable did not resolve".to_string(),
                        )));
                    };
                    let is_root = root_order.iter().any(|(g, n)| g == &group && n == &name);
                    let manifest = rp
                        .manifest_of(id, &group, &name, &version)
                        .map_err(SolveError::Provider)?;
                    // PROP-008 §2.4 KIND-VALIDATION — a root's kind prefix
                    // (if any) must match the resolved manifest's declared
                    // kind. A resolvo solvable is interned by
                    // `(group, name, version)`, so a transitive dep's prefix
                    // is dropped at intern time and only a root's prefix
                    // survives to be checked here; the naive cell (shared
                    // with `sat`) additionally checks transitive prefixes
                    // via the pkgref it carries through `process_one`.
                    if is_root && let Some(actual) = manifest.package.as_ref().map(|p| p.kind) {
                        for root in roots
                            .iter()
                            .filter(|r| r.group.as_ref() == Some(&group) && r.name == name)
                        {
                            assert_kind_matches(root.kind, actual, &format!("{group}/{name}"))?;
                        }
                    }
                    let mut direct_deps: Vec<_> = manifest
                        .requires
                        .packages
                        .iter()
                        .filter(|dep| {
                            !self
                                .provider
                                .edge_blocked(&format!("{group}/{name}"), &dep.qualified_name())
                        })
                        .cloned()
                        .collect();
                    for edge in choices.iter().filter(|e| {
                        e.parent.group.as_ref() == Some(&group) && e.parent.name == name
                    }) {
                        if !direct_deps.contains(&edge.provider) {
                            direct_deps.push(edge.provider.clone());
                        }
                    }

                    // `[obsoletes]` → drop the superseded node from the
                    // output, mirroring the naive cell (PROP-017 §3).
                    for ob in &manifest.obsoletes.packages {
                        if let Some(g) = ob.group.clone() {
                            obsolete.insert((g, ob.name.to_string()));
                        }
                    }
                    selected.push((format!("{group}/{name}"), version.clone(), manifest));
                    chosen.insert(
                        (group, name),
                        Chosen {
                            version,
                            direct_deps,
                            is_root,
                        },
                    );
                }
                // Capability requirements of the selected packages are
                // verified against the selected set — the clean
                // CapabilityUnmet verdict (PROP-017 §3).
                // Masked capability declarations have already been discharged
                // by the route mask; unmasked ones still require a witness.
                for (parent, _, manifest) in &mut selected {
                    let mut index = 0;
                    manifest.requires.capabilities.retain(|cap| {
                        let declaration = format!("requires.capabilities[{index}]:{cap}");
                        index += 1;
                        !self.provider.retained_choices().iter().any(|e| {
                            e.parent.qualified_name() == *parent
                                && e.declaration == declaration
                                && self
                                    .provider
                                    .edge_blocked(parent, &e.provider.qualified_name())
                        })
                    });
                }
                capabilities::verify(&selected)?;
                let graph = build_resolved_graph(&root_order, chosen, &obsolete);
                for edge in &choices {
                    if graph
                        .find(
                            edge.parent.group.as_ref().expect("qualified witness"),
                            edge.parent.name.as_str(),
                        )
                        .is_some()
                        && graph
                            .find(
                                edge.provider.group.as_ref().expect("qualified witness"),
                                edge.provider.name.as_str(),
                            )
                            .is_none()
                    {
                        return Err(DepProviderError::Other(
                            "obsolete projection removed a selected choice provider".into(),
                        )
                        .into());
                    }
                }
                for node in graph.iter() {
                    let manifest =
                        self.provider
                            .fetch_manifest(&node.group, &node.name, &node.version)?;
                    for (index, dep) in manifest.requires.packages.iter().enumerate() {
                        if self.provider.edge_blocked(
                            &format!("{}/{}", node.group, node.name),
                            &dep.qualified_name(),
                        ) {
                            continue;
                        }
                        if dep
                            .group
                            .as_ref()
                            .is_some_and(|g| obsolete.contains(&(g.clone(), dep.name.to_string())))
                        {
                            continue;
                        }
                        let Some(target) = dep
                            .group
                            .as_ref()
                            .and_then(|g| graph.find(g, dep.name.as_str()))
                        else {
                            return Err(DepProviderError::Other(
                                "graph projection lost a selected dependency".into(),
                            )
                            .into());
                        };
                        choices.push(SelectedChoice {
                            parent: PackageRef::parse(&format!(
                                "{}/{}@={}",
                                node.group, node.name, node.version
                            ))
                            .expect("selected coordinate"),
                            declaration: format!("requires.packages[{index}]"),
                            alternative: 0,
                            provider: PackageRef::parse(&format!(
                                "{}/{}@={}",
                                target.group, target.name, target.version
                            ))
                            .expect("selected coordinate"),
                        });
                    }
                }
                choices
                    .sort_by_key(|e| (e.parent.to_string(), e.declaration.clone(), e.alternative));
                Ok(RecordedResolution { graph, choices })
            }
            Err(UnsolvableOrCancelled::Unsolvable(conflict)) => Err(SolveError::Unsatisfiable {
                explanation: conflict.display_user_friendly(&solver).to_string(),
            }),
            Err(UnsolvableOrCancelled::Cancelled(_)) => Err(SolveError::Provider(
                DepProviderError::Other("resolvo solve was cancelled".to_string()),
            )),
        }
    }

    /// Collect, from the current graph, the `[recommends]` package refs
    /// not already installed and not yet tried — the candidates the
    /// greedy best-effort expansion attempts next.
    fn collect_recommends(
        &self,
        graph: &ResolvedGraph,
        tried: &HashSet<(Group, String)>,
    ) -> Result<Vec<(PackageRef, SelectedChoice)>, SolveError> {
        let mut out = Vec::new();
        let mut seen: HashSet<(Group, String)> = HashSet::new();
        for node in graph.iter() {
            let manifest = self
                .provider
                .fetch_manifest(&node.group, &node.name, &node.version)
                .map_err(SolveError::Provider)?;
            for (index, rec) in manifest.recommends.packages.iter().enumerate() {
                if self.provider.edge_blocked(
                    &format!("{}/{}", node.group, node.name),
                    &rec.qualified_name(),
                ) {
                    continue;
                }
                let Some(group) = rec.group.clone() else {
                    continue;
                };
                let key = (group.clone(), rec.name.to_string());
                if tried.contains(&key) || graph.find(&group, rec.name.as_str()).is_some() {
                    continue;
                }
                if seen.insert(key) {
                    out.push((
                        rec.clone(),
                        SelectedChoice {
                            parent: PackageRef::parse(&format!(
                                "{}/{}@={}",
                                node.group, node.name, node.version
                            ))
                            .expect("selected coordinate"),
                            declaration: format!("recommends.packages[{index}]"),
                            alternative: 0,
                            provider: rec.clone(),
                        },
                    ));
                }
            }
        }
        Ok(out)
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

#[cfg(test)]
mod m_05_c_1_cases;

#[cfg(test)]
mod m_05_c_2_cases;

pub use semver::Version as CandidateVersion;

#[cfg(test)]
mod m_05_c_3_cases;
