//! Shared in-memory unit-table builder for the hybrid linker's test suites
//! (PROP-038; scaffold-h). ONE declarative way to spell a boot graph: name each
//! node and its `(target, link)` edges and get back exactly the
//! `HashMap<UnitId, UnitInput>` the compiler consumes — a runnable reference
//! model of the unit table. The recursion (`super::tests`), hoisting
//! (`super::hoist::tests`), and fingerprint (`super::fingerprint::tests`) suites
//! all build their graphs through this one place instead of re-deriving the
//! same `org` / `id` / `unit` / `table` boilerplate three times over. The
//! snippet path and `origin` marker follow the materialised-slot convention, so
//! fingerprints and provenance read as they would in a real workspace.
//!
//! Every helper carries `#[cfg(test)]` (matching `super::fuzz`) so the
//! discipline scanner reads them as test scaffolding, not domain logic.

use std::collections::HashMap;

use vibe_core::Group;
use vibe_core::manifest::{LinkType, WhenCondition};

use super::{UnitEdge, UnitId, UnitInput};

/// The canonical first-party test `Group` (`org.vibevm`).
#[cfg(test)]
pub(crate) fn org() -> Group {
    Group::parse("org.vibevm").unwrap()
}

/// A `(group, name)` unit id in the test group.
#[cfg(test)]
pub(crate) fn id(name: &str) -> UnitId {
    (org(), name.to_string())
}

/// The `(target, link)` pairs as [`UnitEdge`]s in the test group.
#[cfg(test)]
fn mk_edges(edges: &[(&str, LinkType)]) -> Vec<UnitEdge> {
    edges
        .iter()
        .map(|(t, link)| UnitEdge {
            target: id(t),
            link: *link,
        })
        .collect()
}

/// A package unit — its own boot snippet plus `(target, link)` edges.
#[cfg(test)]
pub(crate) fn unit(name: &str, edges: &[(&str, LinkType)]) -> (UnitId, UnitInput) {
    (
        id(name),
        UnitInput {
            own_boot_path: Some(format!("vibedeps/flow-{name}/1.0.0/boot.md")),
            origin: format!("org.vibevm/{name}"),
            when: None,
            edges: mk_edges(edges),
        },
    )
}

/// A boot-less workspace-node-like unit (no own snippet) — it still threads the
/// ordering but contributes no static body.
#[cfg(test)]
pub(crate) fn node(name: &str, edges: &[(&str, LinkType)]) -> (UnitId, UnitInput) {
    let (uid, mut u) = unit(name, edges);
    u.own_boot_path = None;
    (uid, u)
}

/// A `when`-gated package unit — the gate that keeps it dynamic under any
/// static edge (PROP-009 §2.4). Fluent, so the gate is expressed at
/// construction rather than by post-hoc mutation of the built table.
#[cfg(test)]
pub(crate) fn gated_unit(
    name: &str,
    when: WhenCondition,
    edges: &[(&str, LinkType)],
) -> (UnitId, UnitInput) {
    let (uid, mut u) = unit(name, edges);
    u.when = Some(when);
    (uid, u)
}

/// Assemble a unit table from `(id, input)` pairs.
#[cfg(test)]
pub(crate) fn table(units: Vec<(UnitId, UnitInput)>) -> HashMap<UnitId, UnitInput> {
    units.into_iter().collect()
}
