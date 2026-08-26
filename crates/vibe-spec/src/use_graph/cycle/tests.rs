//! The order-independence reds for the shared SCC cycle law.

use super::first_illegal_cycle;

fn always_illegal(_node: usize) -> bool {
    false
}

/// Ascending keys, so a fixture that says nothing about identity still exercises
/// the key-ordered law (`keys[i]` sorts exactly like `i`).
fn keys(count: usize) -> Vec<String> {
    (0..count).map(|node| format!("k{node:04}")).collect()
}

#[test]
fn an_acyclic_relation_has_no_cycle() {
    assert_eq!(
        first_illegal_cycle(&keys(3), &[(0, 1), (1, 2), (0, 2)], always_illegal),
        None
    );
}

#[test]
fn a_self_loop_is_a_cycle() {
    assert_eq!(
        first_illegal_cycle(&keys(2), &[(1, 1)], always_illegal),
        Some(vec![1, 1])
    );
}

#[test]
fn a_two_node_cycle_closes_on_its_smallest_member() {
    assert_eq!(
        first_illegal_cycle(&keys(2), &[(0, 1), (1, 0)], always_illegal),
        Some(vec![0, 1, 0])
    );
}

#[test]
fn an_admitted_component_is_not_reported() {
    assert_eq!(
        first_illegal_cycle(&keys(2), &[(0, 1), (1, 0)], |_| true),
        None
    );
}

/// One non-admitting member is enough, and the reported path is anchored on it
/// so the rendering shows why the component is illegal.
#[test]
fn the_path_is_anchored_on_the_node_that_makes_the_cycle_illegal() {
    let path = first_illegal_cycle(&keys(3), &[(0, 1), (1, 2), (2, 0)], |node| node != 2)
        .expect("node 2 does not admit cycles");
    assert_eq!(path, vec![2, 0, 1, 2]);
}

/// The whole point of the SCC law. `w` (index 1) sits on a cycle whose members
/// are not all admitted, but a three-colour DFS that finishes `u` through `x`
/// first marks `u` black and never examines `w -> u`, so it reports only the
/// admitted loop. The component answer depends on neither root nor edge order.
#[test]
fn a_masked_second_cycle_is_reported_from_every_edge_order() {
    // 0 = u, 1 = w (the only inadmissible node), 2 = x, 3 = v.
    // v -> x, v -> w, x -> u, w -> u, u -> v.
    let declared = [(3, 2), (3, 1), (2, 0), (1, 0), (0, 3)];
    let reversed = [(0, 3), (1, 0), (2, 0), (3, 1), (3, 2)];
    let admits = |node: usize| node != 1;

    let from_declared = first_illegal_cycle(&keys(4), &declared, admits);
    let from_reversed = first_illegal_cycle(&keys(4), &reversed, admits);
    assert_eq!(
        from_declared, from_reversed,
        "neither the verdict nor the path may depend on edge order"
    );
    let path = from_declared.expect("the component holds the inadmissible node");
    assert_eq!(path.first(), path.last(), "the path closes on itself");
    assert_eq!(path.first(), Some(&1), "anchored on the offending node");
}

/// Two different loops through the witness close it equally well, so which one
/// is rendered would otherwise be decided by the order the edges were declared.
#[test]
fn the_reported_path_does_not_depend_on_edge_order() {
    let declared = [(0, 1), (1, 0), (0, 2), (2, 0)];
    let reversed = [(0, 2), (2, 0), (0, 1), (1, 0)];
    assert_eq!(
        first_illegal_cycle(&keys(3), &declared, always_illegal),
        Some(vec![0, 1, 0])
    );
    assert_eq!(
        first_illegal_cycle(&keys(3), &reversed, always_illegal),
        Some(vec![0, 1, 0]),
        "the rendered path must not follow declaration order"
    );
}

/// Identity is the *key*, never the arena position. The same relation numbered
/// two ways must report the same node and the same path, spelled in keys — an
/// id-ordered law would answer `0` both times and so name two different nodes.
#[test]
fn the_offender_and_path_are_chosen_by_key_not_by_node_id() {
    let alpha = ["a".to_string(), "b".to_string(), "c".to_string()];
    let renumbered = ["c".to_string(), "b".to_string(), "a".to_string()];
    let edges = [(0, 1), (1, 0), (0, 2), (2, 0)];
    // Same graph, ids handed out in the reverse order: 0<->1 and 0<->2 becomes
    // 2<->1 and 2<->0 once the keys are read off.
    let mirrored = [(2, 1), (1, 2), (2, 0), (0, 2)];

    let spell = |keys: &[String], edges: &[(usize, usize)]| {
        first_illegal_cycle(keys, edges, always_illegal)
            .expect("every node is inadmissible")
            .into_iter()
            .map(|node| keys[node].clone())
            .collect::<Vec<_>>()
    };
    assert_eq!(spell(&alpha, &edges), vec!["a", "b", "a"]);
    assert_eq!(
        spell(&renumbered, &mirrored),
        vec!["a", "b", "a"],
        "renumbering the same relation must not move the offender"
    );
}

/// Two disjoint illegal components: the key-least one is reported whatever
/// order the edges arrive in.
#[test]
fn the_smallest_illegal_component_is_reported_first() {
    let forward = [(2, 3), (3, 2), (0, 1), (1, 0)];
    let backward = [(0, 1), (1, 0), (2, 3), (3, 2)];
    assert_eq!(
        first_illegal_cycle(&keys(4), &forward, always_illegal),
        Some(vec![0, 1, 0])
    );
    assert_eq!(
        first_illegal_cycle(&keys(4), &backward, always_illegal),
        Some(vec![0, 1, 0])
    );
}

/// Admission is judged per component, never over the union of every cycle: one
/// legal contract cycle beside one illegal cycle reports only the illegal one.
#[test]
fn admission_is_judged_per_component() {
    let edges = [(0, 1), (1, 0), (2, 3), (3, 2)];
    let admits = |node: usize| node < 2;
    assert_eq!(
        first_illegal_cycle(&keys(4), &edges, admits),
        Some(vec![2, 3, 2])
    );
}

#[test]
fn out_of_range_endpoints_are_ignored_rather_than_panicking() {
    assert_eq!(
        first_illegal_cycle(&keys(1), &[(0, 9), (9, 0)], always_illegal),
        None
    );
}

/// A long chain must not overflow the stack inside the law itself.
#[test]
fn a_deep_chain_is_decided_iteratively() {
    let depth = 50_000;
    let spelling = keys(depth + 1);
    let mut edges: Vec<(usize, usize)> = (0..depth).map(|node| (node, node + 1)).collect();
    assert_eq!(first_illegal_cycle(&spelling, &edges, always_illegal), None);
    edges.push((depth, 0));
    let path = first_illegal_cycle(&spelling, &edges, always_illegal)
        .expect("closing the chain makes one giant component");
    assert_eq!(path.first(), Some(&0));
    assert_eq!(path.last(), Some(&0));
}
