//! Property-based characterization of the solver core (card
//! scaffold-d-differential-oracle; adopt-v0.3 Phase 4).
//!
//! Two artifacts in one file:
//!
//! 1. **The behavior net.** proptest generates random acyclic package
//!    worlds and pins the solver's observable contract: determinism,
//!    dependency closure, roots-first ordering, exact pinning. These
//!    properties test behavior the author never enumerated case by
//!    case — the modification safety net a weak reader cannot derive.
//! 2. **The differential harness.** [`assert_solvers_agree`] drives
//!    two `DepSolver` cells over the same world and demands identical
//!    normalized graphs. Today it smoke-tests naive-vs-naive (proving
//!    the harness itself); Phase 7 plugs the SAT solver into the same
//!    socket per GUIDE §9 / R-040 — DBT-0011's landing pad.
//!
//! The in-memory [`WorldProvider`] is deliberately also a Class-H
//! fake: the external dependency (a registry) reasoned about offline.

use std::collections::HashMap;

use proptest::prelude::*;
use specmark::verifies;
use vibe_core::manifest::Manifest;
use vibe_core::{Group, PackageRef, VersionSpec};
use vibe_resolver::{
    DepProvider, DepProviderError, DepSolver, NaiveDepSolver, ResolvedGraph, SolveError,
};

fn org() -> Group {
    Group::parse("org.vibevm").unwrap()
}

/// One generated package world: an acyclic dependency universe.
/// Package `i` may depend only on packages with larger indices, so a
/// cycle cannot be expressed by construction.
#[derive(Debug, Clone)]
struct World {
    /// `packages[i]` = list of (version, deps) for package `p<i>`;
    /// deps are indices into `packages` (always > their owner's).
    packages: Vec<Vec<(semver::Version, Vec<usize>)>>,
}

impl World {
    fn name(i: usize) -> String {
        format!("p{i}")
    }

    fn manifest_toml(&self, i: usize, version: &semver::Version, deps: &[usize]) -> String {
        let mut s = format!(
            "[package]\ngroup = \"org.vibevm\"\nname = \"{}\"\nkind = \"flow\"\nversion = \"{version}\"\n",
            Self::name(i)
        );
        if !deps.is_empty() {
            s.push_str("\n[requires.packages]\n");
            for d in deps {
                // Caret on the dep's lowest existing major keeps every
                // edge satisfiable within the world.
                let lowest = &self.packages[*d].first().expect("dep has versions").0;
                s.push_str(&format!(
                    "\"org.vibevm/{}\" = \"^{}\"\n",
                    Self::name(*d),
                    lowest
                ));
            }
        }
        s
    }
}

/// In-memory `DepProvider` over a [`World`] — the registry fake.
struct WorldProvider {
    entries: HashMap<String, Vec<(semver::Version, Manifest)>>,
}

impl WorldProvider {
    fn new(world: &World) -> Self {
        let mut entries: HashMap<String, Vec<(semver::Version, Manifest)>> = HashMap::new();
        for (i, versions) in world.packages.iter().enumerate() {
            for (version, deps) in versions {
                let m = Manifest::parse_str(&world.manifest_toml(i, version, deps))
                    .expect("generated manifest parses");
                entries
                    .entry(World::name(i))
                    .or_default()
                    .push((version.clone(), m));
            }
        }
        WorldProvider { entries }
    }
}

impl DepProvider for WorldProvider {
    fn resolve_version(&self, pkgref: &PackageRef) -> Result<semver::Version, DepProviderError> {
        let candidates =
            self.entries
                .get(&pkgref.name)
                .ok_or_else(|| DepProviderError::UnknownPackage {
                    group: org(),
                    name: pkgref.name.clone(),
                })?;
        let mut versions: Vec<&semver::Version> = candidates.iter().map(|(v, _)| v).collect();
        versions.sort();
        versions
            .iter()
            .rev()
            .find(|v| pkgref.version.matches(v))
            .map(|v| (*v).clone())
            .ok_or_else(|| DepProviderError::NoMatchingVersion {
                group: org(),
                name: pkgref.name.clone(),
                constraint: format!("{}", pkgref.version),
            })
    }

    fn fetch_manifest(
        &self,
        _group: &Group,
        name: &str,
        version: &semver::Version,
    ) -> Result<Manifest, DepProviderError> {
        self.entries
            .get(name)
            .and_then(|c| c.iter().find(|(v, _)| v == version))
            .map(|(_, m)| m.clone())
            .ok_or_else(|| DepProviderError::Other(format!("no manifest for {name}@{version}")))
    }
}

/// Deterministic, comparison-friendly rendering of a resolved graph.
fn normalize(graph: &ResolvedGraph) -> Vec<String> {
    graph
        .packages
        .iter()
        .map(|n| {
            let mut deps: Vec<String> = n
                .dependencies
                .iter()
                .map(|d| format!("{}@{}", d.name, d.version))
                .collect();
            deps.sort();
            format!(
                "{}/{}@{} root={} deps=[{}]",
                n.group,
                n.name,
                n.version,
                n.is_root,
                deps.join(", ")
            )
        })
        .collect()
}

/// The differential socket (R-040): both solvers walk the same world
/// from the same roots and must produce the identical normalized
/// graph — or fail identically.
fn assert_solvers_agree<A: DepSolver, B: DepSolver>(
    a: &A,
    b: &B,
    roots: &[PackageRef],
) -> Result<(), TestCaseError> {
    let ga = a.solve(roots);
    let gb = b.solve(roots);
    match (ga, gb) {
        (Ok(ga), Ok(gb)) => {
            prop_assert_eq!(normalize(&ga), normalize(&gb), "graphs diverge");
        }
        (Err(ea), Err(eb)) => {
            prop_assert_eq!(
                std::mem::discriminant(&ea),
                std::mem::discriminant(&eb),
                "error classes diverge: {} vs {}",
                ea,
                eb
            );
        }
        (Ok(_), Err(e)) => return Err(TestCaseError::fail(format!("only B failed: {e}"))),
        (Err(e), Ok(_)) => return Err(TestCaseError::fail(format!("only A failed: {e}"))),
    }
    Ok(())
}

/// Strategy: an acyclic world of 1..=6 packages, each with 1..=2
/// versions and 0..=3 deps pointing strictly forward.
fn world_strategy() -> impl Strategy<Value = World> {
    (1usize..=6)
        .prop_flat_map(|n| {
            let pkg = move |i: usize| {
                let max_deps = n.saturating_sub(i + 1).min(3);
                let dep_pool: Vec<usize> = ((i + 1)..n).collect();
                let versions = prop::collection::vec(
                    (
                        1u64..=3,
                        prop::sample::subsequence(dep_pool.clone(), 0..=max_deps),
                    ),
                    1..=2,
                );
                versions.prop_map(move |vs| {
                    let mut out: Vec<(semver::Version, Vec<usize>)> = vs
                        .into_iter()
                        .map(|(major, deps)| (semver::Version::new(major, 0, 0), deps))
                        .collect();
                    out.sort_by(|a, b| a.0.cmp(&b.0));
                    out.dedup_by(|a, b| a.0 == b.0);
                    out
                })
            };
            (0..n)
                .map(pkg)
                .collect::<Vec<_>>()
                .prop_map(|packages| World { packages })
        })
        .prop_filter("worlds must have at least one package", |w| {
            !w.packages.is_empty()
        })
}

fn root_refs(world: &World, picks: &[usize]) -> Vec<PackageRef> {
    picks
        .iter()
        .map(|i| {
            PackageRef::parse(&format!(
                "org.vibevm/{}",
                World::name(*i % world.packages.len())
            ))
            .unwrap()
        })
        .collect()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// Determinism (PROP-003 #determinism): the same world and roots
    /// solve to a byte-identical normalized graph, twice.
    #[test]
    #[verifies("spec://vibevm/modules/vibe-resolver/PROP-003#determinism")]
    fn solve_is_deterministic(world in world_strategy(), picks in prop::collection::vec(0usize..6, 1..=3)) {
        let provider = WorldProvider::new(&world);
        let solver = NaiveDepSolver::new(provider);
        let roots = root_refs(&world, &picks);
        let first = solver.solve(&roots);
        let second = solver.solve(&roots);
        match (first, second) {
            (Ok(a), Ok(b)) => prop_assert_eq!(normalize(&a), normalize(&b)),
            (Err(a), Err(b)) => prop_assert_eq!(a.to_string(), b.to_string()),
            _ => return Err(TestCaseError::fail("two runs disagreed on success")),
        }
    }

    /// Closure: every dependency edge of every resolved node points at
    /// a node present in the graph whose version satisfies the pinned
    /// constraint.
    #[test]
    fn solve_output_is_closed(world in world_strategy(), picks in prop::collection::vec(0usize..6, 1..=3)) {
        let provider = WorldProvider::new(&world);
        let solver = NaiveDepSolver::new(provider);
        let roots = root_refs(&world, &picks);
        if let Ok(graph) = solver.solve(&roots) {
            for node in graph.iter() {
                for dep in &node.dependencies {
                    let target = graph
                        .find(dep.group.as_ref().unwrap_or(&org()), &dep.name)
                        .ok_or_else(|| TestCaseError::fail(format!(
                            "dep `{}` of `{}` missing from the graph",
                            dep.name, node.name
                        )))?;
                    prop_assert!(
                        dep.version.matches(&target.version),
                        "pinned constraint `{}` does not match resolved `{}@{}`",
                        dep.version, target.name, target.version
                    );
                }
            }
        }
    }

    /// Roots-first: the resolved graph lists every requested root as
    /// `is_root = true` and roots form a prefix of the package list.
    #[test]
    fn roots_are_marked_and_prefix(world in world_strategy(), picks in prop::collection::vec(0usize..6, 1..=3)) {
        let provider = WorldProvider::new(&world);
        let solver = NaiveDepSolver::new(provider);
        let roots = root_refs(&world, &picks);
        if let Ok(graph) = solver.solve(&roots) {
            let first_non_root = graph.packages.iter().position(|n| !n.is_root);
            if let Some(idx) = first_non_root {
                prop_assert!(
                    graph.packages[idx..].iter().all(|n| !n.is_root),
                    "a root-flagged node appears after a non-root node"
                );
            }
            for r in &roots {
                prop_assert!(
                    graph.packages.iter().any(|n| n.name == r.name && n.is_root),
                    "requested root `{}` not marked in the graph", r.name
                );
            }
        }
    }

    /// Exact pinning: every dependency reference in the output carries
    /// an `=x.y.z` constraint (the lockfile reproducibility contract).
    #[test]
    #[verifies("spec://vibevm/modules/vibe-registry/PROP-002#lockfile")]
    fn dependencies_are_exact_pinned(world in world_strategy(), picks in prop::collection::vec(0usize..6, 1..=3)) {
        let provider = WorldProvider::new(&world);
        let solver = NaiveDepSolver::new(provider);
        let roots = root_refs(&world, &picks);
        if let Ok(graph) = solver.solve(&roots) {
            for node in graph.iter() {
                for dep in &node.dependencies {
                    let VersionSpec::Req(req) = &dep.version else {
                        return Err(TestCaseError::fail(format!(
                            "dep `{}` of `{}` is not version-pinned at all",
                            dep.name, node.name
                        )));
                    };
                    prop_assert!(
                        req.to_string().starts_with('='),
                        "dep `{}` of `{}` pinned loosely: `{}`",
                        dep.name, node.name, req
                    );
                }
            }
        }
    }

    /// The differential harness proves itself: a solver agrees with a
    /// second instance of itself on every world. Phase 7 swaps one
    /// side for the SAT solver (DBT-0011's landing pad).
    #[test]
    #[verifies("spec://vibevm/modules/vibe-resolver/PROP-003#solver-upgrade")]
    fn differential_harness_smoke(world in world_strategy(), picks in prop::collection::vec(0usize..6, 1..=3)) {
        let a = NaiveDepSolver::new(WorldProvider::new(&world));
        let b = NaiveDepSolver::new(WorldProvider::new(&world));
        let roots = root_refs(&world, &picks);
        assert_solvers_agree(&a, &b, &roots)?;
    }
}
