//! Unit tests for the per-unit recursive compiler ([`super`]), out-of-line
//! per the file-length budget. The centrepiece is the owner's chain example
//! (PROP-038 §2.2): `root→A(dynamic)→B(static)→C(dynamic)→D(static-transitive)`.

use super::testkit::{gated_unit, id, node, table, unit};
use super::*;
use vibe_core::manifest::LinkType;

/// `resolve_zone` static members as a sorted pkgref list, for assertions.
#[cfg(test)]
fn static_names(m: &ZoneMembership) -> Vec<String> {
    let mut v: Vec<String> = m.static_members.iter().map(|(_, n)| n.clone()).collect();
    v.sort();
    v
}

#[cfg(test)]
fn dynamic_names(m: &ZoneMembership) -> Vec<String> {
    m.dynamic_edges
        .iter()
        .map(|((_, n), _)| n.clone())
        .collect()
}

#[cfg(test)]
fn topo_names(m: &ZoneMembership, units: &HashMap<UnitId, UnitInput>) -> Vec<String> {
    topo_zone(&m.static_members, units)
        .into_iter()
        .map(|(_, n)| n)
        .collect()
}

/// The owner's chain (PROP-038 §2.2): dynamic breaks the zone, static
/// compiles in, static-transitive forces the subtree.
#[cfg(test)]
fn chain() -> HashMap<UnitId, UnitInput> {
    table(vec![
        node("root", &[("a", LinkType::Dynamic)]),
        unit("a", &[("b", LinkType::Static)]),
        unit("b", &[("c", LinkType::Dynamic)]),
        unit("c", &[("d", LinkType::StaticTransitive)]),
        unit("d", &[("e", LinkType::Dynamic)]), // forced static under c→d
        unit("e", &[]),
    ])
}

#[test]
fn root_dynamic_edge_breaks_the_zone() {
    let units = chain();
    let m = resolve_zone(&id("root"), &units);
    // root→a is dynamic: nothing below a is compiled into root.
    assert_eq!(static_names(&m), vec!["root"]);
    assert_eq!(dynamic_names(&m), vec!["a"]);
}

#[test]
fn static_edge_compiles_child_but_dynamic_grandchild_breaks() {
    let units = chain();
    let m = resolve_zone(&id("a"), &units);
    // a→b static: b compiles into a. b→c dynamic: c surfaces, not compiled.
    assert_eq!(static_names(&m), vec!["a", "b"]);
    assert_eq!(dynamic_names(&m), vec!["c"]);
    // Dependency before dependent: b (a's dependency) precedes a.
    assert_eq!(topo_names(&m, &units), vec!["b", "a"]);
}

#[test]
fn static_transitive_forces_the_whole_subtree() {
    let units = chain();
    let m = resolve_zone(&id("c"), &units);
    // c→d static-transitive: d AND its dynamic child e are forced static.
    assert_eq!(static_names(&m), vec!["c", "d", "e"]);
    assert!(
        m.dynamic_edges.is_empty(),
        "forced subtree has no dynamic edges"
    );
    // e (deepest) → d → c.
    assert_eq!(
        topo_names(&units_zone(&id("c"), &units), &units),
        vec!["e", "d", "c"]
    );
}

#[cfg(test)]
fn units_zone(root: &UnitId, units: &HashMap<UnitId, UnitInput>) -> ZoneMembership {
    resolve_zone(root, units)
}

/// A package reachable through several static paths is compiled once
/// (PROP-034 §2.3 dedup, retained per unit).
#[test]
fn diamond_compiles_shared_package_once() {
    let units = table(vec![
        node("root", &[("a", LinkType::Static), ("b", LinkType::Static)]),
        unit("a", &[("c", LinkType::Static)]),
        unit("b", &[("c", LinkType::Static)]),
        unit("c", &[]),
    ]);
    let m = resolve_zone(&id("root"), &units);
    // c appears once despite two static paths.
    assert_eq!(static_names(&m), vec!["a", "b", "c", "root"]);
    // Topo: c before a and b; a before b (pkgref tie-break); root last.
    assert_eq!(topo_names(&m, &units), vec!["c", "a", "b", "root"]);
}

/// `static-hard` compiles the child in exactly like `static` — the two differ
/// only in hoisting (applied later), not in zone membership (PROP-038 §2.3).
#[test]
fn static_hard_compiles_the_child_in() {
    let units = table(vec![
        node("root", &[("a", LinkType::StaticHard)]),
        unit("a", &[]),
    ]);
    let m = resolve_zone(&id("root"), &units);
    assert_eq!(static_names(&m), vec!["a", "root"]);
    assert!(m.dynamic_edges.is_empty());
}

/// A `when`-gated package stays dynamic even under a static edge — OS
/// content must never reach the wrong OS (PROP-009 §2.4).
#[test]
fn when_gate_stays_dynamic_under_a_static_edge() {
    use vibe_core::manifest::{TargetOs, WhenCondition};
    let win = WhenCondition::Os(TargetOs::Windows);
    let units = table(vec![
        node("root", &[("win", LinkType::Static)]),
        gated_unit("win", win, &[]),
    ]);
    let m = resolve_zone(&id("root"), &units);
    // The static edge does not compile a gated target in.
    assert_eq!(static_names(&m), vec!["root"]);
    assert_eq!(dynamic_names(&m), vec!["win"]);
    assert_eq!(m.dynamic_edges[0].1, Some(win));
}
