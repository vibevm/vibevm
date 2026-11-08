//! `vibe update --all` (and a bare `vibe update`): which executed region
//! serves it.
//!
//! `--help` promises "Update every package in the lockfile", and that is a
//! SCOPED update whose roots are every declared root of the workspace: each
//! one re-resolved against its declared constraint, every version allowed to
//! move at its locked source, the transitive closure following. B-234 was the
//! whole-graph install substrate answering this request instead — it replays
//! every lock row, so a registry that gained a patch release was reported
//! `unchanged`.
//!
//! The install substrate still serves the shapes only it owns, unchanged:
//!
//! - **a resumed parked run** — the substrate services the persisted slot
//!   continuation the parking invocation left, under the adopted identity;
//! - **an empty or missing lock** — the empty-world regeneration, or a first
//!   resolution of whatever the manifest declares;
//! - **a declared root not in the lock** — installing it is install's job;
//! - **an unreadable manifest, tree or lock** — the substrate reports it in
//!   the words and at the point it always has.
//!
//! None of those has a locked version that could move, so routing them to the
//! substrate never hides a bump.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#command-summary");

use std::collections::BTreeMap;

use vibe_core::manifest::{LockedPackage, Lockfile};
use vibe_core::{PackageRef, VersionSpec};
use vibe_workspace::Workspace;

use super::Execution;

/// The region an `--all` request runs in.
#[derive(Debug)]
pub(super) enum AllRoute {
    /// The install substrate — see the module note for when.
    Whole,
    /// A scoped update over exactly these roots.
    Scoped(Vec<PackageRef>),
}

/// Which roots this scoped run re-resolves.
#[derive(Debug)]
pub(super) enum Roots {
    /// The named packages (`--sources-only`: or every locked package when none
    /// is named), read from the arguments.
    Named,
    /// `vibe update --all`: every declared root, already selected by
    /// [`route`]. Each moves within its declared constraint, and a
    /// source reselection covers every locked coordinate.
    All(Vec<PackageRef>),
}

/// Decide the route from the prepared epoch, without consuming it and without
/// failing: anything this cannot read is left for the substrate to report.
pub(super) fn route(execution: &Execution) -> AllRoute {
    if execution.adopted {
        return AllRoute::Whole;
    }
    let Some(workspace) = execution.selection.loaded_workspace() else {
        return AllRoute::Whole;
    };
    if execution.selection.parsed_ref().is_none() {
        return AllRoute::Whole;
    }
    let path = workspace.root.join(Lockfile::FILENAME);
    let Ok(lockfile) = Lockfile::read(&path) else {
        return AllRoute::Whole;
    };
    match declared_roots(workspace, &lockfile) {
        Some(roots) if !roots.is_empty() => AllRoute::Scoped(roots),
        _ => AllRoute::Whole,
    }
}

/// Every declared root of every workspace node, as the scoped update's root
/// requests: a registry root at its declared constraint, a git root pinned to
/// its locked version (its declared ref, not a version range, selects it).
///
/// `None` when a declared root is not locked, or one coordinate is declared
/// with two different constraints — the install substrate's own cases.
fn declared_roots(workspace: &Workspace, lockfile: &Lockfile) -> Option<Vec<PackageRef>> {
    let mut roots: BTreeMap<String, PackageRef> = BTreeMap::new();
    for (_, node) in workspace.iter_nodes() {
        for declared in &node.requires.packages {
            let group = declared.group.as_ref()?;
            lockfile.find(group, declared.name.as_str())?;
            insert(&mut roots, declared.clone())?;
        }
        for declared in &node.requires.git_packages {
            let row = lockfile.find(&declared.group, &declared.name)?;
            insert(&mut roots, exact_root(row)?)?;
        }
    }
    Some(roots.into_values().collect())
}

/// Insert one root; a second, DIFFERENT request for the same coordinate is
/// `None`.
fn insert(roots: &mut BTreeMap<String, PackageRef>, root: PackageRef) -> Option<()> {
    match roots.get(&root.qualified_name()) {
        Some(held) if held.version != root.version || held.kind != root.kind => None,
        Some(_) => Some(()),
        None => {
            roots.insert(root.qualified_name(), root);
            Some(())
        }
    }
}

fn exact_root(row: &LockedPackage) -> Option<PackageRef> {
    let exact = semver::VersionReq::parse(&format!("={}", row.version)).ok()?;
    PackageRef::new(
        None,
        Some(row.group.clone()),
        row.name.to_string(),
        VersionSpec::Req(exact),
    )
    .ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root(spec: &str) -> PackageRef {
        PackageRef::parse(spec).unwrap()
    }

    #[test]
    fn the_same_request_twice_is_one_root() {
        let mut roots = BTreeMap::new();
        assert!(insert(&mut roots, root("org.example/p@^0.1.0")).is_some());
        assert!(insert(&mut roots, root("org.example/p@^0.1.0")).is_some());
        assert_eq!(roots.len(), 1);
    }

    #[test]
    fn two_constraints_for_one_coordinate_leave_it_to_the_substrate() {
        let mut roots = BTreeMap::new();
        assert!(insert(&mut roots, root("org.example/p@^0.1.0")).is_some());
        assert!(insert(&mut roots, root("org.example/p@^0.2.0")).is_none());
    }
}
