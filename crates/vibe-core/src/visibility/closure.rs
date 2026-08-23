specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-050#closure");

use std::collections::{BTreeMap, BTreeSet};

use crate::manifest::{AccessLevel, AllowFriendsOverride, OverrideEntry, OverrideTarget};

use super::{Analysis, Diagnostic, EdgeDecl, NodeId, Provenance, ProvenanceRule, VisibilityGraph};

#[derive(Debug, Clone, PartialEq, Eq)]
struct MaskLayer {
    declared_by: NodeId,
    entries: Vec<(OverrideTarget, OverrideEntry)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct State {
    node: NodeId,
    path: Vec<NodeId>,
    layers: Vec<MaskLayer>,
    excluded: BTreeSet<NodeId>,
}

impl State {
    fn same_key(&self, other: &Self) -> bool {
        self.node == other.node && self.layers == other.layers && self.excluded == other.excluded
    }
}

#[derive(Default)]
struct Walk {
    states: Vec<State>,
    effective: BTreeMap<NodeId, Provenance>,
    seen_nodes: BTreeSet<NodeId>,
    seen_edges: BTreeSet<(NodeId, NodeId)>,
}

#[derive(Debug, Clone, Copy)]
struct EffectiveEdge {
    access: AccessLevel,
    friend: bool,
    killed: bool,
}

/// Compute the least friend-closure/effective-set fixpoint for `root`.
///
/// Static mask layers and a finite node set give finitely many simple path
/// states; repeated normalized states are deduplicated. Both result sets grow
/// monotonically, so alternating traversal and closure expansion terminates.
///
/// ```
/// use vibe_core::visibility::{analyze, EdgeDecl, NodeDecl, VisibilityGraph};
///
/// let root = "org.x/root".to_string();
/// let mut graph = VisibilityGraph::default();
/// graph.nodes.insert(root.clone(), NodeDecl {
///     edges: vec![EdgeDecl { to: "org.x/api".into(), ..EdgeDecl::default() }],
///     ..NodeDecl::default()
/// });
/// assert!(analyze(&graph, &root).effective.contains_key("org.x/api"));
/// ```
pub fn analyze(graph: &VisibilityGraph, root: &NodeId) -> Analysis {
    let mut closure = BTreeSet::new();
    loop {
        let walk = walk_graph(graph, root, &closure);
        let expanded = expand_closure(graph, root, &closure, &walk);
        if expanded == closure {
            return finish_analysis(graph, root, closure, walk);
        }
        closure = expanded;
    }
}

fn walk_graph(graph: &VisibilityGraph, root: &NodeId, closure: &BTreeSet<NodeId>) -> Walk {
    let initial = State {
        node: root.clone(),
        path: vec![root.clone()],
        layers: layer_for(graph, root).into_iter().collect(),
        excluded: BTreeSet::new(),
    };
    let mut walk = Walk {
        states: vec![initial],
        ..Walk::default()
    };
    walk.seen_nodes.insert(root.clone());
    let mut cursor = 0;
    while cursor < walk.states.len() {
        let state = walk.states[cursor].clone();
        cursor += 1;
        let Some(decl) = graph.nodes.get(&state.node) else {
            continue;
        };
        for edge in &decl.edges {
            walk.seen_nodes.insert(edge.to.clone());
            walk.seen_edges
                .insert((state.node.clone(), edge.to.clone()));
            let mut excluded = state.excluded.clone();
            excluded.extend(edge.exclude.iter().cloned());
            let attributes = effective_edge(&state.layers, &state.node, edge);
            if attributes.killed || excluded.contains(&edge.to) {
                continue;
            }
            let rule = if state.node == *root {
                Some(ProvenanceRule::RootEdge)
            } else if attributes.access == AccessLevel::Public {
                Some(ProvenanceRule::PublicChain)
            } else if attributes.access == AccessLevel::FriendsOnly && closure.contains(&state.node)
            {
                Some(ProvenanceRule::FriendsChain)
            } else {
                None
            };
            let Some(rule) = rule else {
                continue;
            };
            let mut path = state.path.clone();
            path.push(edge.to.clone());
            let mut layers = state.layers.clone();
            if let Some(layer) = layer_for(graph, &edge.to) {
                layers.push(layer);
            }
            let next = State {
                node: edge.to.clone(),
                path: path.clone(),
                layers,
                excluded,
            };
            let via_override = access_override(&state.layers, &state.node, &edge.to)
                .map(|(declared_by, _)| declared_by.clone());
            walk.effective.entry(edge.to.clone()).or_insert(Provenance {
                rule,
                path,
                via_override,
            });
            let cycle = next.path[..next.path.len() - 1].contains(&next.node);
            if !cycle && !walk.states.iter().any(|known| known.same_key(&next)) {
                walk.states.push(next);
            }
        }
    }
    walk
}

fn expand_closure(
    graph: &VisibilityGraph,
    root: &NodeId,
    closure: &BTreeSet<NodeId>,
    walk: &Walk,
) -> BTreeSet<NodeId> {
    let mut expanded = closure.clone();
    for state in &walk.states {
        let is_seed = state.node == *root;
        let is_grow = closure.contains(&state.node);
        if !is_seed && !is_grow {
            continue;
        }
        for grant in grants_for_state(graph, state) {
            if !is_seed && !friends_only_grant_edge(graph, state, &grant) {
                continue;
            }
            if grant_allowed(graph, &state.layers, &state.node, &grant) {
                expanded.insert(grant);
            }
        }
    }
    expanded
}

fn grants_for_state(graph: &VisibilityGraph, state: &State) -> BTreeSet<NodeId> {
    let Some(node) = graph.nodes.get(&state.node) else {
        return BTreeSet::new();
    };
    let mut grants: BTreeSet<NodeId> = node
        .edges
        .iter()
        .filter(|edge| effective_edge(&state.layers, &state.node, edge).friend)
        .map(|edge| edge.to.clone())
        .chain(node.friends.iter().cloned())
        .collect();
    for target in &node.unfriend {
        grants.remove(target);
    }
    grants
}

fn friends_only_grant_edge(graph: &VisibilityGraph, state: &State, grant: &NodeId) -> bool {
    graph.nodes.get(&state.node).is_some_and(|node| {
        node.edges.iter().any(|edge| {
            edge.to == *grant
                && effective_edge(&state.layers, &state.node, edge).access
                    == AccessLevel::FriendsOnly
        })
    })
}

fn effective_edge(layers: &[MaskLayer], from: &NodeId, edge: &EdgeDecl) -> EffectiveEdge {
    let target = OverrideTarget::Edge {
        from: from.clone(),
        to: edge.to.clone(),
    };
    let access = first_override(layers, &target, |entry| entry.access)
        .map_or_else(|| edge.effective_access(), |(_, value)| value);
    let friend = first_override(layers, &target, |entry| entry.friend).map_or_else(
        || edge.friend.unwrap_or(access == AccessLevel::FriendsOnly),
        |(_, value)| value,
    );
    let killed =
        first_override(layers, &target, |entry| entry.exclude).is_some_and(|(_, value)| value);
    EffectiveEdge {
        access,
        friend,
        killed,
    }
}

fn access_override<'a>(
    layers: &'a [MaskLayer],
    from: &NodeId,
    to: &NodeId,
) -> Option<(&'a NodeId, AccessLevel)> {
    let target = OverrideTarget::Edge {
        from: from.clone(),
        to: to.clone(),
    };
    first_override(layers, &target, |entry| entry.access)
}

fn first_override<'a, T: Copy>(
    layers: &'a [MaskLayer],
    target: &OverrideTarget,
    field: impl Fn(&OverrideEntry) -> Option<T>,
) -> Option<(&'a NodeId, T)> {
    for layer in layers {
        for (candidate, entry) in &layer.entries {
            if candidate == target
                && let Some(value) = field(entry)
            {
                return Some((&layer.declared_by, value));
            }
        }
    }
    None
}

fn effective_allow_friends(
    graph: &VisibilityGraph,
    layers: &[MaskLayer],
    target: &NodeId,
) -> Option<Vec<NodeId>> {
    let node_target = OverrideTarget::Node(target.clone());
    for layer in layers {
        for (candidate, entry) in &layer.entries {
            if candidate == &node_target
                && let Some(replacement) = &entry.allow_friends
            {
                return match replacement {
                    AllowFriendsOverride::Everyone(_) => None,
                    AllowFriendsOverride::List(values) => Some(values.clone()),
                };
            }
        }
    }
    graph
        .nodes
        .get(target)
        .and_then(|node| node.allow_friends.clone())
}

fn grant_allowed(
    graph: &VisibilityGraph,
    layers: &[MaskLayer],
    from: &NodeId,
    target: &NodeId,
) -> bool {
    effective_allow_friends(graph, layers, target).is_none_or(|circle| {
        circle.iter().any(|allowed| {
            allowed == from
                || allowed.strip_suffix("/*").is_some_and(|group| {
                    from.strip_prefix(group)
                        .is_some_and(|name| name.starts_with('/') && !name[1..].contains('/'))
                })
        })
    })
}

fn layer_for(graph: &VisibilityGraph, node: &NodeId) -> Option<MaskLayer> {
    graph.nodes.get(node).and_then(|declaration| {
        (!declaration.overrides.is_empty()).then(|| MaskLayer {
            declared_by: node.clone(),
            entries: declaration.overrides.clone(),
        })
    })
}

fn finish_analysis(
    graph: &VisibilityGraph,
    root: &NodeId,
    closure: BTreeSet<NodeId>,
    walk: Walk,
) -> Analysis {
    let mut diagnostics = rejected_grants(graph, root, &closure, &walk);
    append_dead_diagnostics(graph, &walk, &mut diagnostics);
    Analysis {
        closure,
        effective: walk.effective,
        diagnostics,
    }
}

fn rejected_grants(
    graph: &VisibilityGraph,
    root: &NodeId,
    closure: &BTreeSet<NodeId>,
    walk: &Walk,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for state in &walk.states {
        if state.node != *root && !closure.contains(&state.node) {
            continue;
        }
        let is_seed = state.node == *root;
        for grant in grants_for_state(graph, state) {
            if (!is_seed && !friends_only_grant_edge(graph, state, &grant))
                || grant_allowed(graph, &state.layers, &state.node, &grant)
            {
                continue;
            }
            let warning = Diagnostic::RejectedGrant {
                from: state.node.clone(),
                to: grant,
            };
            if !diagnostics.contains(&warning) {
                diagnostics.push(warning);
            }
        }
    }
    diagnostics
}

fn append_dead_diagnostics(
    graph: &VisibilityGraph,
    walk: &Walk,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for (declared_by, node) in &graph.nodes {
        for (target, _) in &node.overrides {
            let seen = match target {
                OverrideTarget::Edge { from, to } => {
                    walk.seen_edges.contains(&(from.clone(), to.clone()))
                }
                OverrideTarget::Node(target) => walk.seen_nodes.contains(target),
            };
            if !seen {
                diagnostics.push(Diagnostic::DeadOverrideEntry {
                    declared_by: declared_by.clone(),
                    target: target.clone(),
                });
            }
        }
        for target in &node.friends {
            if !walk.seen_nodes.contains(target) {
                diagnostics.push(Diagnostic::DeadFriendsEntry {
                    declared_by: declared_by.clone(),
                    target: target.clone(),
                });
            }
        }
        for target in &node.unfriend {
            if !walk.seen_nodes.contains(target) {
                diagnostics.push(Diagnostic::DeadUnfriendEntry {
                    declared_by: declared_by.clone(),
                    target: target.clone(),
                });
            }
        }
    }
}
