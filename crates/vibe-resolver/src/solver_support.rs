//! Solver-shared state and the single output builder — the helpers every
//! solver cell (naive, resolvo, sat) routes through, split from `lib.rs`
//! along the seam between the public provider/solver contract and the
//! machinery behind it.

use std::collections::{HashMap, HashSet, VecDeque};

use vibe_core::manifest::Manifest;
use vibe_core::{Group, PackageRef, VersionSpec};

use crate::{ResolvedGraph, ResolvedNode};

// ---------------------------------------------------------------------------
// Helpers used by both the naive impl and (eventually) the resolvo impl.
// ---------------------------------------------------------------------------

/// Per-(group,name) state the solver accumulates as it walks. `pub(crate)`
/// rather than module-private so the multiple solver impls in this crate
/// share one definition.
pub(crate) struct SolverState {
    pub chosen: HashMap<(Group, String), ChosenEntry>,
    pub providers_index: HashMap<String, Vec<(Group, String, semver::Version)>>,
    pub declared_conflicts: HashSet<(Group, String)>,
    pub declared_obsolete: HashSet<(Group, String)>,
    pub queue: VecDeque<EnqueuedPkg>,
}

pub(crate) struct ChosenEntry {
    pub version: semver::Version,
    pub manifest: Manifest,
    pub direct_deps: Vec<PackageRef>,
    pub is_root: bool,
}

pub(crate) struct EnqueuedPkg {
    pub pkgref: PackageRef,
    pub via: Option<String>,
    pub is_root: bool,
}

impl SolverState {
    pub(crate) fn new() -> Self {
        SolverState {
            chosen: HashMap::new(),
            providers_index: HashMap::new(),
            declared_conflicts: HashSet::new(),
            declared_obsolete: HashSet::new(),
            queue: VecDeque::new(),
        }
    }
}

/// `true` iff the version satisfies the spec.
pub(crate) fn version_satisfies(spec: &VersionSpec, version: &semver::Version) -> bool {
    spec.matches(version)
}

/// One solver-chosen package, reduced to what the output builder needs.
/// Both `NaiveDepSolver` and `ResolvoDepSolver` collapse their internal
/// state into this before calling [`build_resolved_graph`].
#[derive(Clone)]
pub(crate) struct Chosen {
    pub version: semver::Version,
    pub direct_deps: Vec<PackageRef>,
    pub is_root: bool,
}

/// Build the final [`ResolvedGraph`] from a solver's choices — the single
/// home of the output contract (PROP-017 §2.3): drop obsoleted entries,
/// list roots first in input order then the rest sorted for determinism,
/// and exact-pin every dependency edge to the version chosen for it. Both
/// solver cells route through here, so they produce byte-identical graphs
/// — which is exactly what the differential oracle asserts.
pub(crate) fn build_resolved_graph(
    root_order: &[(Group, String)],
    mut chosen: HashMap<(Group, String), Chosen>,
    obsolete: &HashSet<(Group, String)>,
) -> ResolvedGraph {
    for ob in obsolete {
        chosen.remove(ob);
    }

    let mut packages: Vec<ResolvedNode> = Vec::with_capacity(chosen.len());
    // Roots first, input order preserved.
    for (group, name) in root_order {
        if let Some(entry) = chosen.remove(&(group.clone(), name.clone())) {
            packages.push(node_from_chosen(group.clone(), name.clone(), entry, true));
        }
    }
    // The rest, sorted by (group, name) for a deterministic tail.
    let mut rest: Vec<((Group, String), Chosen)> = chosen.into_iter().collect();
    rest.sort_by(|a, b| (a.0.0.as_str(), a.0.1.as_str()).cmp(&(b.0.0.as_str(), b.0.1.as_str())));
    for ((group, name), entry) in rest {
        packages.push(node_from_chosen(group, name, entry, false));
    }

    // Exact-pin every dependency reference to the chosen version, so a
    // later `vibe install` off this lockfile reproduces it bit-for-bit.
    let resolved_versions: HashMap<(Group, String), semver::Version> = packages
        .iter()
        .map(|n| ((n.group.clone(), n.name.clone()), n.version.clone()))
        .collect();
    for node in packages.iter_mut() {
        for dep in node.dependencies.iter_mut() {
            if let Some(group) = dep.group.clone()
                && let Some(version) = resolved_versions.get(&(group, dep.name.to_string()))
                && let Ok(req) = semver::VersionReq::parse(&format!("={version}"))
            {
                *dep = PackageRef {
                    kind: dep.kind,
                    group: dep.group.clone(),
                    name: dep.name.clone(),
                    version: VersionSpec::Req(req),
                };
            }
        }
        // Two requirements of one package pin to the same reference.
        let mut seen = HashSet::new();
        node.dependencies.retain(|dep| seen.insert(dep.to_string()));
    }

    ResolvedGraph { packages }
}

fn node_from_chosen(group: Group, name: String, entry: Chosen, is_root: bool) -> ResolvedNode {
    ResolvedNode {
        group,
        name,
        version: entry.version,
        dependencies: entry.direct_deps,
        is_root: is_root || entry.is_root,
    }
}
