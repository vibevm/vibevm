//! Unit tests for boot-graph fingerprints ([`super`]) — the Merkle change
//! detector that drives dirty-subgraph regeneration (PROP-038 §2.7).

use super::super::{UnitEdge, UnitInput};
use super::*;
use vibe_core::Group;
use vibe_core::manifest::LinkType;

#[cfg(test)]
fn org() -> Group {
    Group::parse("org.vibevm").unwrap()
}

#[cfg(test)]
fn id(name: &str) -> UnitId {
    (org(), name.to_string())
}

#[cfg(test)]
fn unit(name: &str, edges: &[(&str, LinkType)]) -> (UnitId, UnitInput) {
    (
        id(name),
        UnitInput {
            own_boot_path: Some(format!("vibedeps/flow-{name}/1.0.0/boot.md")),
            origin: format!("org.vibevm/{name}"),
            when: None,
            edges: edges
                .iter()
                .map(|(t, l)| UnitEdge {
                    target: id(t),
                    link: *l,
                })
                .collect(),
        },
    )
}

#[cfg(test)]
fn table(v: Vec<(UnitId, UnitInput)>) -> HashMap<UnitId, UnitInput> {
    v.into_iter().collect()
}

#[cfg(test)]
fn vers(names: &[&str]) -> HashMap<UnitId, String> {
    names.iter().map(|n| (id(n), "1.0.0".to_string())).collect()
}

#[test]
fn fingerprint_is_deterministic() {
    let t = table(vec![
        unit("root", &[("a", LinkType::Static)]),
        unit("a", &[]),
    ]);
    let v = vers(&["root", "a"]);
    assert_eq!(fingerprints(&t, &v), fingerprints(&t, &v));
}

#[test]
fn a_version_change_flips_the_fingerprint_up_the_static_chain() {
    let t = table(vec![
        unit("root", &[("a", LinkType::Static)]),
        unit("a", &[]),
    ]);
    let v1 = vers(&["root", "a"]);
    let mut v2 = v1.clone();
    v2.insert(id("a"), "2.0.0".to_string());
    let f1 = fingerprints(&t, &v1);
    let f2 = fingerprints(&t, &v2);
    assert_ne!(f1[&id("a")], f2[&id("a")]);
    assert_ne!(
        f1[&id("root")],
        f2[&id("root")],
        "a static parent propagates the change"
    );
}

#[test]
fn a_link_type_switch_flips_the_fingerprint() {
    let t1 = table(vec![
        unit("root", &[("a", LinkType::Static)]),
        unit("a", &[]),
    ]);
    let t2 = table(vec![
        unit("root", &[("a", LinkType::Dynamic)]),
        unit("a", &[]),
    ]);
    let v = vers(&["root", "a"]);
    assert_ne!(
        fingerprints(&t1, &v)[&id("root")],
        fingerprints(&t2, &v)[&id("root")],
        "a dynamic<->static switch flips the parent"
    );
}

#[test]
fn a_dynamic_boundary_isolates_a_change_behind_it() {
    // root→a dynamic, a→b static. A change to b flips a (its static parent)
    // but NOT root — the dynamic edge to a breaks propagation (PROP-038 §2.7).
    let t = table(vec![
        unit("root", &[("a", LinkType::Dynamic)]),
        unit("a", &[("b", LinkType::Static)]),
        unit("b", &[]),
    ]);
    let v1 = vers(&["root", "a", "b"]);
    let mut v2 = v1.clone();
    v2.insert(id("b"), "2.0.0".to_string());
    let f1 = fingerprints(&t, &v1);
    let f2 = fingerprints(&t, &v2);
    assert_ne!(f1[&id("a")], f2[&id("a")], "b's static parent a changes");
    assert_eq!(
        f1[&id("root")],
        f2[&id("root")],
        "the dynamic edge breaks propagation"
    );
}

#[test]
fn adding_a_static_edge_flips_the_fingerprint() {
    let t1 = table(vec![unit("root", &[]), unit("a", &[])]);
    let t2 = table(vec![
        unit("root", &[("a", LinkType::Static)]),
        unit("a", &[]),
    ]);
    let v = vers(&["root", "a"]);
    assert_ne!(
        fingerprints(&t1, &v)[&id("root")],
        fingerprints(&t2, &v)[&id("root")]
    );
}
