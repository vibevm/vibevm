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
mod provider;
mod version_set;

use std::collections::{HashMap, HashSet};

use resolvo::{Problem, Solver, UnsolvableOrCancelled};
use specmark::{cell, spec};
use vibe_core::{Group, PackageRef};

use crate::{
    Chosen, DepProviderError, DepSolver, ResolvedGraph, SolveError, VersionEnumerator,
    build_resolved_graph,
};
use provider::VibevmResolvoProvider;

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
#[spec(implements = "spec://vibevm/modules/vibe-resolver/PROP-017#architecture")]
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
    #[spec(implements = "spec://vibevm/modules/vibe-resolver/PROP-017#dominance")]
    #[spec(implements = "spec://vibevm/modules/vibe-registry/PROP-002#lockfile")]
    fn solve(&self, roots: &[PackageRef]) -> Result<ResolvedGraph, SolveError> {
        // Hard solve, then a best-effort greedy expansion over
        // `[recommends]` (PROP-003 §2.3.3): each recommended package is
        // tried via a re-solve and kept only if the graph stays
        // satisfiable; a recommend that conflicts is silently dropped,
        // never a failure. Recommended packages enter the graph but are
        // not roots. resolvo's `soft_requirements` is root-level only, so
        // per-package recommends ride this re-solve loop — batching them
        // as soft is a future optimisation.
        let mut extra: Vec<PackageRef> = Vec::new();
        let mut tried: HashSet<(Group, String)> = HashSet::new();
        let mut graph = self.solve_hard(roots, &extra)?;
        loop {
            let recommends = self.collect_recommends(&graph, &tried)?;
            let Some(rec) = recommends.into_iter().next() else {
                break;
            };
            if let Some(group) = rec.group.clone() {
                tried.insert((group, rec.name.to_string()));
            }
            let mut trial = extra.clone();
            trial.push(rec);
            if let Ok(extended) = self.solve_hard(roots, &trial) {
                extra = trial;
                graph = extended;
            }
        }
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
    ) -> Result<ResolvedGraph, SolveError> {
        // Pre-scan the package closure (roots + extra) for capability
        // providers (PROP-017 §3) before building the resolvo provider.
        let scan_roots: Vec<PackageRef> = roots.iter().chain(extra.iter()).cloned().collect();
        let cap_index = capabilities::prescan(&self.provider, &scan_roots);
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
                for id in solvables {
                    let Some((group, name, version)) = rp.solvable_parts(id) else {
                        return Err(SolveError::Provider(DepProviderError::Other(
                            "internal: a chosen solvable did not resolve".to_string(),
                        )));
                    };
                    let is_root = root_order.iter().any(|(g, n)| g == &group && n == &name);
                    let manifest = rp
                        .manifest_of(id, &group, &name, &version)
                        .map_err(SolveError::Provider)?;
                    let direct_deps = manifest.requires.packages.clone();
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
                capabilities::verify(&selected)?;
                Ok(build_resolved_graph(&root_order, chosen, &obsolete))
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
    ) -> Result<Vec<PackageRef>, SolveError> {
        let mut out = Vec::new();
        let mut seen: HashSet<(Group, String)> = HashSet::new();
        for node in graph.iter() {
            let manifest = self
                .provider
                .fetch_manifest(&node.group, &node.name, &node.version)
                .map_err(SolveError::Provider)?;
            for rec in &manifest.recommends.packages {
                let Some(group) = rec.group.clone() else {
                    continue;
                };
                let key = (group.clone(), rec.name.to_string());
                if tried.contains(&key) || graph.find(&group, rec.name.as_str()).is_some() {
                    continue;
                }
                if seen.insert(key) {
                    out.push(rec.clone());
                }
            }
        }
        Ok(out)
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
