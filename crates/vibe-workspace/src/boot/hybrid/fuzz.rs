//! Property-based fuzzing of the hybrid boot graph (PROP-038 §3, DEF-5).
//!
//! `proptest` generates random acyclic unit tables algorithmically — the
//! machine-generated form of the differential oracle. The properties pin the
//! change detector's core invariants under mutation, so the owner's fear ("не
//! потеряли / не забыли перегенерить") is checked across thousands of random
//! shapes rather than a handful of hand-written cases.

use std::collections::{HashMap, HashSet};

use proptest::prelude::*;
use vibe_core::Group;
use vibe_core::manifest::LinkType;

use super::fingerprint::fingerprints;
use super::{UnitEdge, UnitId, UnitInput};

#[cfg(test)]
fn org() -> Group {
    Group::parse("org.vibevm").unwrap()
}

#[cfg(test)]
fn uid(i: usize) -> UnitId {
    (org(), format!("u{i}"))
}

/// A random link mode, or no edge (weighted toward sparser graphs).
#[cfg(test)]
fn arb_edge() -> impl Strategy<Value = Option<LinkType>> {
    prop_oneof![
        3 => Just(None),
        1 => Just(Some(LinkType::Static)),
        1 => Just(Some(LinkType::Dynamic)),
        1 => Just(Some(LinkType::StaticTransitive)),
        1 => Just(Some(LinkType::StaticHard)),
    ]
}

/// A random **acyclic** unit table of 2..8 units: unit `i` may edge only to a
/// higher-indexed unit `j > i`, so the graph is a DAG by construction. Returns
/// the table and a matching versions map.
#[cfg(test)]
fn arb_table() -> impl Strategy<Value = (HashMap<UnitId, UnitInput>, HashMap<UnitId, String>)> {
    (2usize..8)
        .prop_flat_map(|n| {
            (
                Just(n),
                prop::collection::vec(prop::collection::vec(arb_edge(), n), n),
            )
        })
        .prop_map(|(n, matrix)| {
            let mut table = HashMap::new();
            let mut versions = HashMap::new();
            for (i, row) in matrix.iter().enumerate() {
                let edges: Vec<UnitEdge> = (i + 1..n)
                    .filter_map(|j| {
                        row[j].map(|link| UnitEdge {
                            target: uid(j),
                            link,
                        })
                    })
                    .collect();
                table.insert(
                    uid(i),
                    UnitInput {
                        own_boot_path: Some(format!("vibedeps/u{i}/1.0.0/boot.md")),
                        origin: format!("org.vibevm/u{i}"),
                        when: None,
                        edges,
                        format: Default::default(),
                    },
                );
                versions.insert(uid(i), "1.0.0".to_string());
            }
            (table, versions)
        })
}

/// The static-ancestors of `target`: `target` plus every unit with a
/// continuous chain of **static** (non-gated) edges down to it. This is exactly
/// the set whose fingerprint depends on `target`'s own boot content
/// (PROP-038 §2.7) — a change to that content must flip these and no others.
#[cfg(test)]
fn static_ancestors(target: &UnitId, table: &HashMap<UnitId, UnitInput>) -> HashSet<UnitId> {
    // reverse index of static edges: child -> its static parents.
    let mut parents: HashMap<UnitId, Vec<UnitId>> = HashMap::new();
    for (id, unit) in table {
        for edge in &unit.edges {
            let gated = table.get(&edge.target).and_then(|u| u.when).is_some();
            let is_static = matches!(
                edge.link,
                LinkType::Static | LinkType::StaticTransitive | LinkType::StaticHard
            );
            if is_static && !gated {
                parents
                    .entry(edge.target.clone())
                    .or_default()
                    .push(id.clone());
            }
        }
    }
    let mut seen = HashSet::new();
    let mut stack = vec![target.clone()];
    while let Some(u) = stack.pop() {
        if !seen.insert(u.clone()) {
            continue;
        }
        if let Some(ps) = parents.get(&u) {
            stack.extend(ps.iter().cloned());
        }
    }
    seen
}

proptest! {
    /// A unit's fingerprint does not depend on the order its edges were
    /// declared — the compiler sorts them, so reversing changes nothing.
    #[test]
    fn fingerprints_are_edge_order_invariant((table, versions) in arb_table()) {
        let base = fingerprints(&table, &versions);
        let mut reversed = table.clone();
        for unit in reversed.values_mut() {
            unit.edges.reverse();
        }
        prop_assert_eq!(base, fingerprints(&reversed, &versions));
    }

    /// A change to a unit's own boot content flips the fingerprints of exactly
    /// its static-ancestors — nothing lost (every static dependent regenerates)
    /// and nothing spurious (a dynamic boundary isolates the change). The
    /// owner's core invariant, fuzzed over random DAGs.
    #[test]
    fn a_content_change_flips_exactly_the_static_ancestors((table, versions) in arb_table()) {
        let base = fingerprints(&table, &versions);
        for target in table.keys() {
            let mut mutated = table.clone();
            mutated.get_mut(target).unwrap().own_boot_path = Some("MUTATED".to_string());
            let after = fingerprints(&mutated, &versions);
            let changed: HashSet<UnitId> = base
                .keys()
                .filter(|k| base[*k] != after[*k])
                .cloned()
                .collect();
            let expected = static_ancestors(target, &table);
            prop_assert_eq!(
                &changed,
                &expected,
                "target {:?}: the dirty set must equal its static ancestors",
                target
            );
        }
    }
}
