specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-050#example");

use crate::manifest::{AccessLevel, AllowFriendsOverride, OverrideEntry, OverrideTarget};

use super::*;

fn id(name: &str) -> NodeId {
    format!("org.x/{name}")
}

fn edge(name: &str, access: Option<AccessLevel>, friend: Option<bool>) -> EdgeDecl {
    EdgeDecl {
        to: id(name),
        access,
        friend,
        exclude: Vec::new(),
        weak: false,
    }
}

/// An accepted recommendation: the ordinary defaults, plus `weak`.
fn weak_edge(name: &str) -> EdgeDecl {
    EdgeDecl {
        to: id(name),
        weak: true,
        ..EdgeDecl::default()
    }
}

fn node(edges: Vec<EdgeDecl>) -> NodeDecl {
    NodeDecl {
        edges,
        ..NodeDecl::default()
    }
}

fn edge_override(from: &str, to: &str, access: AccessLevel) -> (OverrideTarget, OverrideEntry) {
    (
        OverrideTarget::Edge {
            from: id(from),
            to: id(to),
        },
        OverrideEntry {
            access: Some(access),
            ..OverrideEntry::default()
        },
    )
}

fn rejected(analysis: &Analysis, from: &str, to: &str) -> bool {
    analysis.diagnostics.contains(&Diagnostic::RejectedGrant {
        from: id(from),
        to: id(to),
    })
}

#[test]
fn owner_chain_a_b_c_d() {
    let mut graph = VisibilityGraph::default();
    graph
        .nodes
        .insert(id("a"), node(vec![edge("b", None, Some(true))]));
    graph.nodes.insert(
        id("b"),
        node(vec![edge("c", Some(AccessLevel::FriendsOnly), None)]),
    );
    graph.nodes.insert(
        id("c"),
        node(vec![edge("d", Some(AccessLevel::FriendsOnly), None)]),
    );
    graph.nodes.insert(id("d"), NodeDecl::default());
    let result = analyze(&graph, &id("a"));
    assert!(
        [id("b"), id("c"), id("d")]
            .iter()
            .all(|member| result.closure.contains(member))
    );
    assert_eq!(
        result.effective.get(&id("d")).map(|p| p.rule),
        Some(ProvenanceRule::FriendsChain)
    );
}

#[test]
fn matrix_public_default() {
    let mut graph = VisibilityGraph::default();
    graph
        .nodes
        .insert(id("r"), node(vec![edge("p", None, None)]));
    graph
        .nodes
        .insert(id("p"), node(vec![edge("q", None, None)]));
    graph.nodes.insert(
        id("q"),
        node(vec![edge("inner", Some(AccessLevel::FriendsOnly), None)]),
    );
    let result = analyze(&graph, &id("r"));
    assert!(result.effective.contains_key(&id("q")));
    assert!(!result.closure.contains(&id("q")));
    assert!(!result.effective.contains_key(&id("inner")));
}

#[test]
fn matrix_public_friend_true() {
    let mut graph = VisibilityGraph::default();
    graph
        .nodes
        .insert(id("r"), node(vec![edge("p", None, None)]));
    graph
        .nodes
        .insert(id("p"), node(vec![edge("q", None, Some(true))]));
    graph.nodes.insert(
        id("q"),
        node(vec![edge("inner", Some(AccessLevel::FriendsOnly), None)]),
    );
    let own = analyze(&graph, &id("p"));
    assert!(own.closure.contains(&id("q")) && own.effective.contains_key(&id("inner")));
    let foreign = analyze(&graph, &id("r"));
    assert!(foreign.effective.contains_key(&id("q")));
    assert!(!foreign.closure.contains(&id("q")));
    assert!(!foreign.effective.contains_key(&id("inner")));
}

#[test]
fn matrix_friends_only_vouched() {
    let mut graph = VisibilityGraph::default();
    graph
        .nodes
        .insert(id("r"), node(vec![edge("p", None, Some(true))]));
    graph.nodes.insert(
        id("p"),
        node(vec![edge("q", Some(AccessLevel::FriendsOnly), None)]),
    );
    let result = analyze(&graph, &id("r"));
    assert!(result.effective.contains_key(&id("q")));
    assert!(result.closure.contains(&id("p")) && result.closure.contains(&id("q")));
}

#[test]
fn matrix_friends_only_no_vouch() {
    let mut graph = VisibilityGraph::default();
    graph
        .nodes
        .insert(id("r"), node(vec![edge("p", None, Some(true))]));
    graph.nodes.insert(
        id("p"),
        node(vec![edge("q", Some(AccessLevel::FriendsOnly), Some(false))]),
    );
    let result = analyze(&graph, &id("r"));
    assert!(result.effective.contains_key(&id("q")));
    assert!(!result.closure.contains(&id("q")));
}

#[test]
fn matrix_private_both() {
    let mut graph = VisibilityGraph::default();
    graph
        .nodes
        .insert(id("r"), node(vec![edge("p", None, None)]));
    graph.nodes.insert(
        id("p"),
        node(vec![
            edge("plain", Some(AccessLevel::Private), Some(false)),
            edge("deep", Some(AccessLevel::Private), Some(true)),
        ]),
    );
    graph.nodes.insert(
        id("deep"),
        node(vec![edge("inner", Some(AccessLevel::FriendsOnly), None)]),
    );
    let foreign = analyze(&graph, &id("r"));
    assert!(!foreign.effective.contains_key(&id("plain")));
    assert!(!foreign.effective.contains_key(&id("deep")));
    let own = analyze(&graph, &id("p"));
    assert!(own.effective.contains_key(&id("plain")));
    assert!(own.effective.contains_key(&id("deep")));
    assert!(own.effective.contains_key(&id("inner")));
}

#[test]
fn unfriend_breaks_at_declarant_only() {
    let mut graph = VisibilityGraph::default();
    graph
        .nodes
        .insert(id("a"), node(vec![edge("b", None, Some(true))]));
    graph.nodes.insert(
        id("b"),
        NodeDecl {
            edges: vec![edge("c", Some(AccessLevel::FriendsOnly), None)],
            unfriend: vec![id("c")],
            ..NodeDecl::default()
        },
    );
    graph.nodes.insert(
        id("c"),
        node(vec![edge("d", Some(AccessLevel::FriendsOnly), None)]),
    );
    let broken = analyze(&graph, &id("a"));
    assert!(!broken.closure.contains(&id("c")));
    assert!(!broken.effective.contains_key(&id("d")));
    graph
        .nodes
        .get_mut(&id("a"))
        .into_iter()
        .for_each(|a| a.edges.push(edge("b2", None, Some(true))));
    graph.nodes.insert(
        id("b2"),
        node(vec![edge("c", Some(AccessLevel::FriendsOnly), None)]),
    );
    let diamond = analyze(&graph, &id("a"));
    assert!(diamond.closure.contains(&id("c")));
    assert!(diamond.effective.contains_key(&id("d")));
}

#[test]
fn exclude_prunes_subtree_diamond() {
    let mut graph = VisibilityGraph::default();
    let mut through_b = edge("b", None, None);
    through_b.exclude.push(id("d"));
    graph
        .nodes
        .insert(id("a"), node(vec![through_b, edge("c", None, None)]));
    graph
        .nodes
        .insert(id("b"), node(vec![edge("d", None, None)]));
    graph
        .nodes
        .insert(id("c"), node(vec![edge("d", None, None)]));
    let result = analyze(&graph, &id("a"));
    let path = result.effective.get(&id("d")).map(|p| p.path.clone());
    assert_eq!(path, Some(vec![id("a"), id("c"), id("d")]));
}

#[test]
fn dev_world_flip() {
    let mut graph = VisibilityGraph::default();
    graph
        .nodes
        .insert(id("r"), node(vec![edge("p", None, None)]));
    graph.nodes.insert(
        id("p"),
        node(vec![edge("tool", Some(AccessLevel::Private), None)]),
    );
    assert!(
        !analyze(&graph, &id("r"))
            .effective
            .contains_key(&id("tool"))
    );
    assert!(
        analyze(&graph, &id("p"))
            .effective
            .contains_key(&id("tool"))
    );
}

#[test]
fn override_root_beats_midgraph() {
    let mut graph = VisibilityGraph::default();
    graph.nodes.insert(
        id("r"),
        NodeDecl {
            edges: vec![edge("n", None, None)],
            overrides: vec![edge_override("x", "y", AccessLevel::Public)],
            ..NodeDecl::default()
        },
    );
    graph.nodes.insert(
        id("n"),
        NodeDecl {
            edges: vec![edge("x", None, None)],
            overrides: vec![edge_override("x", "y", AccessLevel::Private)],
            ..NodeDecl::default()
        },
    );
    graph.nodes.insert(
        id("x"),
        node(vec![edge("y", Some(AccessLevel::Private), None)]),
    );
    assert!(analyze(&graph, &id("r")).effective.contains_key(&id("y")));
}

#[test]
fn override_midgraph_scopes_to_its_chains() {
    let mut graph = VisibilityGraph::default();
    graph.nodes.insert(
        id("r"),
        node(vec![edge("n", None, None), edge("m", None, None)]),
    );
    graph.nodes.insert(
        id("n"),
        NodeDecl {
            edges: vec![edge("x", None, None)],
            overrides: vec![edge_override("x", "y", AccessLevel::Private)],
            ..NodeDecl::default()
        },
    );
    graph
        .nodes
        .insert(id("m"), node(vec![edge("x", None, None)]));
    graph
        .nodes
        .insert(id("x"), node(vec![edge("y", None, None)]));
    let result = analyze(&graph, &id("r"));
    assert_eq!(
        result.effective.get(&id("y")).map(|p| p.path.clone()),
        Some(vec![id("r"), id("m"), id("x"), id("y")])
    );
}

#[test]
fn override_expands_private_edge() {
    let mut graph = VisibilityGraph::default();
    graph.nodes.insert(
        id("r"),
        NodeDecl {
            edges: vec![edge("x", None, None)],
            overrides: vec![edge_override("x", "y", AccessLevel::Public)],
            ..NodeDecl::default()
        },
    );
    graph.nodes.insert(
        id("x"),
        node(vec![edge("y", Some(AccessLevel::Private), None)]),
    );
    let result = analyze(&graph, &id("r"));
    assert_eq!(
        result
            .effective
            .get(&id("y"))
            .and_then(|p| p.via_override.clone()),
        Some(id("r"))
    );
}

#[test]
fn override_node_rewrites_allow_friends() {
    let mut graph = VisibilityGraph::default();
    graph
        .nodes
        .insert(id("r"), node(vec![edge("sealed", None, Some(true))]));
    graph.nodes.insert(
        id("sealed"),
        NodeDecl {
            edges: vec![edge("inner", Some(AccessLevel::FriendsOnly), None)],
            allow_friends: Some(Vec::new()),
            ..NodeDecl::default()
        },
    );
    let sealed = analyze(&graph, &id("r"));
    assert!(rejected(&sealed, "r", "sealed"));
    assert!(!sealed.effective.contains_key(&id("inner")));
    let open = OverrideEntry {
        allow_friends: Some(AllowFriendsOverride::Everyone("*".into())),
        ..OverrideEntry::default()
    };
    if let Some(root) = graph.nodes.get_mut(&id("r")) {
        root.overrides
            .push((OverrideTarget::Node(id("sealed")), open));
    }
    let broken_seal = analyze(&graph, &id("r"));
    assert!(broken_seal.closure.contains(&id("sealed")));
    assert!(broken_seal.effective.contains_key(&id("inner")));
}

#[test]
fn allow_friends_three_states() {
    let mut graph = VisibilityGraph::default();
    graph.nodes.insert(
        id("root"),
        NodeDecl {
            friends: ["open", "sealed", "exact", "pattern", "denied"]
                .into_iter()
                .map(id)
                .collect(),
            ..NodeDecl::default()
        },
    );
    graph.nodes.insert(id("open"), NodeDecl::default());
    graph.nodes.insert(
        id("sealed"),
        NodeDecl {
            allow_friends: Some(Vec::new()),
            ..NodeDecl::default()
        },
    );
    graph.nodes.insert(
        id("exact"),
        NodeDecl {
            allow_friends: Some(vec![id("root")]),
            ..NodeDecl::default()
        },
    );
    graph.nodes.insert(
        id("pattern"),
        NodeDecl {
            allow_friends: Some(vec!["org.x/*".into()]),
            ..NodeDecl::default()
        },
    );
    graph.nodes.insert(
        id("denied"),
        NodeDecl {
            allow_friends: Some(vec!["org.y/root".into()]),
            ..NodeDecl::default()
        },
    );
    let result = analyze(&graph, &id("root"));
    assert!(
        ["open", "exact", "pattern"]
            .into_iter()
            .all(|name| result.closure.contains(&id(name)))
    );
    assert!(!result.closure.contains(&id("sealed")));
    assert!(!result.closure.contains(&id("denied")));
    assert!(rejected(&result, "root", "sealed") && rejected(&result, "root", "denied"));
}

#[test]
fn determinism_repeat() {
    let mut graph = VisibilityGraph::default();
    graph.nodes.insert(
        id("r"),
        NodeDecl {
            edges: vec![edge("p", None, Some(true)), edge("x", None, None)],
            overrides: vec![edge_override("x", "y", AccessLevel::Public)],
            ..NodeDecl::default()
        },
    );
    graph.nodes.insert(
        id("p"),
        node(vec![edge("q", Some(AccessLevel::FriendsOnly), None)]),
    );
    graph.nodes.insert(
        id("x"),
        node(vec![edge("y", Some(AccessLevel::Private), None)]),
    );
    assert_eq!(analyze(&graph, &id("r")), analyze(&graph, &id("r")));
}

/// A weak edge is traversed exactly like the ordinary edge it stands beside,
/// so a package recommended by a project dependency reaches the consumer root
/// — and the arrival says it is weak (PROP-050
/// `#WEAK-EDGES-ARE-ORDINARY-EDGES`).
#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-050#WEAK-EDGES-ARE-ORDINARY-EDGES")]
fn a_weak_edge_admits_its_target_like_an_ordinary_one() {
    let mut graph = VisibilityGraph::default();
    graph
        .nodes
        .insert(id("root"), node(vec![edge("dep", None, None)]));
    graph.nodes.insert(
        id("dep"),
        node(vec![weak_edge("companion"), edge("hard", None, None)]),
    );
    graph
        .nodes
        .insert(id("companion"), node(vec![edge("below", None, None)]));
    graph.nodes.insert(id("hard"), NodeDecl::default());
    graph.nodes.insert(id("below"), NodeDecl::default());
    let result = analyze(&graph, &id("root"));

    let companion = result
        .effective
        .get(&id("companion"))
        .expect("the recommended package is in the effective set");
    assert!(companion.weak, "and its chain is marked weak");
    assert_eq!(companion.rule, ProvenanceRule::PublicChain);
    assert_eq!(companion.path, vec![id("root"), id("dep"), id("companion")]);
    // What the recommendation stands on arrives with it, by the ordinary
    // public rule, and is weak because nothing but the recommendation leads
    // there.
    assert!(
        result
            .effective
            .get(&id("below"))
            .is_some_and(|witness| witness.weak)
    );
    // A declared edge beside it is untouched.
    assert!(
        result
            .effective
            .get(&id("hard"))
            .is_some_and(|witness| !witness.weak)
    );
}

/// A weak edge grants no friendship: `friend` defaults to `false` and a
/// recommendation carries no access mark to imply it, so the recommended
/// package's own friends-only substance stays shut.
#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-050#WEAK-EDGES-ARE-ORDINARY-EDGES")]
fn a_weak_edge_grants_no_friendship() {
    let mut graph = VisibilityGraph::default();
    graph
        .nodes
        .insert(id("root"), node(vec![weak_edge("companion")]));
    graph.nodes.insert(
        id("companion"),
        node(vec![edge("inner", Some(AccessLevel::FriendsOnly), None)]),
    );
    graph.nodes.insert(id("inner"), NodeDecl::default());
    let result = analyze(&graph, &id("root"));

    assert!(result.effective.contains_key(&id("companion")));
    assert!(!result.closure.contains(&id("companion")));
    assert!(!result.effective.contains_key(&id("inner")));
}

/// A package reachable both weakly and strongly records the STRONG chain,
/// whatever order the declarations are walked in — «the strong edge decides»,
/// as the application plane's reverse edge already answers.
#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-050#WEAK-EDGES-ARE-ORDINARY-EDGES")]
fn a_strong_chain_beats_a_weak_one_in_either_declaration_order() {
    for weak_first in [true, false] {
        let mut graph = VisibilityGraph::default();
        let edges = if weak_first {
            vec![edge("recommender", None, None), edge("owner", None, None)]
        } else {
            vec![edge("owner", None, None), edge("recommender", None, None)]
        };
        graph.nodes.insert(id("root"), node(edges));
        graph
            .nodes
            .insert(id("recommender"), node(vec![weak_edge("shared")]));
        graph
            .nodes
            .insert(id("owner"), node(vec![edge("shared", None, None)]));
        graph.nodes.insert(id("shared"), NodeDecl::default());
        let result = analyze(&graph, &id("root"));
        let shared = result
            .effective
            .get(&id("shared"))
            .expect("present either way");
        assert!(
            !shared.weak,
            "a coordinate something requires is never reported weak (weak_first = {weak_first})"
        );
        assert_eq!(shared.path, vec![id("root"), id("owner"), id("shared")]);
    }
}

/// An exclusion kills a weak chain as it kills any other: a weak edge is
/// narrowable by the consumer exactly like the ordinary edge it imitates.
#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-050#WEAK-EDGES-ARE-ORDINARY-EDGES")]
fn a_weak_target_is_excludable_through_the_declaring_edge() {
    let mut graph = VisibilityGraph::default();
    graph.nodes.insert(
        id("root"),
        node(vec![EdgeDecl {
            to: id("dep"),
            exclude: vec![id("companion")],
            ..EdgeDecl::default()
        }]),
    );
    graph
        .nodes
        .insert(id("dep"), node(vec![weak_edge("companion")]));
    graph.nodes.insert(id("companion"), NodeDecl::default());
    let result = analyze(&graph, &id("root"));

    assert!(result.effective.contains_key(&id("dep")));
    assert!(!result.effective.contains_key(&id("companion")));
}
