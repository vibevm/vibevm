//! Unit tests for soft-hoisting analysis ([`super`]) — who is shared and
//! therefore hoisted (PROP-038 §2.4).

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
fn shared_names(t: &HashMap<UnitId, UnitInput>) -> Vec<String> {
    let mut v: Vec<String> = shared_packages(t).into_iter().map(|(_, n)| n).collect();
    v.sort();
    v
}

/// A package statically linked by exactly one unit stays local — not hoisted.
#[test]
fn single_use_static_is_not_shared() {
    let t = table(vec![
        unit("root", &[("a", LinkType::Static)]),
        unit("a", &[]),
    ]);
    assert!(shared_names(&t).is_empty());
}

/// A package statically linked by two units is shared → hoisted (§2.4).
#[test]
fn two_static_consumers_make_a_package_shared() {
    let t = table(vec![
        unit("root", &[("a", LinkType::Static), ("e", LinkType::Static)]),
        unit("a", &[("shared", LinkType::Static)]),
        unit("e", &[("shared", LinkType::Static)]),
        unit("shared", &[]),
    ]);
    // Only `shared` is pulled twice; a and e are each pulled once (by root).
    assert_eq!(shared_names(&t), vec!["shared"]);
}

/// `static-hard` never counts toward sharing — two hard consumers still leave
/// the package local in each (PROP-038 §2.3).
#[test]
fn static_hard_opts_out_of_hoisting() {
    let t = table(vec![
        unit("root", &[("a", LinkType::Static), ("e", LinkType::Static)]),
        unit("a", &[("x", LinkType::StaticHard)]),
        unit("e", &[("x", LinkType::StaticHard)]),
        unit("x", &[]),
    ]);
    assert!(shared_names(&t).is_empty());
}

/// A nested static chain does not over-hoist — each package is pulled by
/// exactly one direct consumer, so nothing is shared.
#[test]
fn nested_static_chain_does_not_over_hoist() {
    let t = table(vec![
        unit("root", &[("a", LinkType::Static)]),
        unit("a", &[("b", LinkType::Static)]),
        unit("b", &[("c", LinkType::Static)]),
        unit("c", &[]),
    ]);
    assert!(shared_names(&t).is_empty());
}

/// A `static-transitive` edge's forced members count as pulled by the
/// declaring unit (§5.2): `b` is forced by root's transitive over `a` AND
/// directly linked by `e` → shared.
#[test]
fn static_transitive_forced_members_count_toward_sharing() {
    let t = table(vec![
        unit(
            "root",
            &[("a", LinkType::StaticTransitive), ("e", LinkType::Dynamic)],
        ),
        unit("a", &[("b", LinkType::Static)]),
        unit("e", &[("b", LinkType::Static)]),
        unit("b", &[]),
    ]);
    assert_eq!(shared_names(&t), vec!["b"]);
}
