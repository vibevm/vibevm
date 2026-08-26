//! Shared machinery for the BUILTIN CLOSE ORDER oracle
//! (`x-corpus-producer-oracles`), used by `compiler_ir_producer_laws.rs` and
//! `compiler_ir_close_replay.rs`. Not a test binary of its own, and NOT a
//! conversion gate: it says what THIS corpus's builtin `close` emitted, and a
//! plugin may legally return a verifier-valid closure it would reject.
//!
//! The oracle owns BOTH witnesses, in this order:
//!
//!   1. EDGE WITNESS — reconstruct the `use` edge vector the way
//!      `close.rs:146-197` mints it: for every normal contribution in
//!      contribution order, walk the reached nodes in traversal order, take
//!      each node's OWN carried `tree.directives.directives`, keep
//!      `kind == use`, sort by `line` exactly as `use_graph::use_addresses`
//!      does (stably, so equal lines keep declaration order), resolve each
//!      target by PINLESS address identity to the one carried spec node, and
//!      append `(from, to, requested_target)` unless that exact tuple is
//!      already present. Compare to the carrier's `kind == use` edges in exact
//!      order and exact requested-target fields.
//!   2. ORDER WITNESS — replay `topology::order_by` on the adjacency DERIVED
//!      in step 1, never on the carrier's edges, and compare node-for-node
//!      with the pre-absorb order.
//!
//! Deriving the adjacency from the directives is what makes step 2 mean
//! something: replaying DFS over the carrier's own edges would only prove the
//! order is consistent with whatever edges the carrier happens to claim.

#![allow(dead_code)]

use vibe_wire::generated::compiler_ir::e1::ir::{
    AbsorptionState, Authority, ClosureContribution, ClosureEdgeKind, ClosureIr,
    ContributionAbsorption, DirectiveKind, DocumentAddress, SpecAddress,
};

/// One arena node as the oracle sees it: its own pinless key, whether its doc
/// path makes it a `contract/…` document (the ONLY thing
/// `topology.rs::is_contract` asks), and its carried `#use` declarations.
#[derive(Clone)]
pub struct Node {
    pub key: String,
    pub contract: bool,
    pub uses: Vec<Use>,
}

/// One `#use` directive carried by a node's own tree.
#[derive(Clone)]
pub struct Use {
    pub line: u32,
    pub target: SpecAddress,
}

/// One reconstructed or carried `use` edge.
#[derive(Clone, PartialEq)]
pub struct Edge {
    pub from: u32,
    pub to: u32,
    pub target: SpecAddress,
}

impl Edge {
    fn show(&self) -> String {
        format!("{} -> {} ({})", self.from, self.to, self.target.raw)
    }
}

/// `SpecAddress::without_pin` — the identity `close` keys every node and every
/// resolved target by.
pub fn without_pin(address: &SpecAddress) -> String {
    let head = match &address.authority {
        Authority::Host(host) => host.name.clone(),
        Authority::Package(package) => match &package.version {
            Some(version) => format!("{}/{}@{version}", package.group, package.name),
            None => format!("{}/{}", package.group, package.name),
        },
    };
    let mut out = format!("spec://{head}/{}", address.doc_path);
    if !address.anchor.is_empty() {
        out.push('#');
        out.push_str(&address.anchor.join("."));
    }
    out
}

/// `use_graph::use_addresses` — `kind == use`, sorted by line, stably.
fn declared_uses(node: &Node) -> Vec<&Use> {
    let mut out: Vec<&Use> = node.uses.iter().collect();
    out.sort_by_key(|entry| entry.line);
    out
}

/// Resolve a directive target to the one carried spec node. A missing or
/// ambiguous target is an oracle FAULT, never an index.
fn resolve(nodes: &[Node], target: &SpecAddress) -> Result<u32, String> {
    let key = without_pin(target);
    let matches: Vec<usize> = nodes
        .iter()
        .enumerate()
        .filter(|(_, node)| node.key == key)
        .map(|(index, _)| index)
        .collect();
    match matches.as_slice() {
        [one] => Ok(*one as u32),
        [] => Err(format!("`#use {key}` resolves to no carried node")),
        many => Err(format!("`#use {key}` resolves to {} nodes", many.len())),
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Colour {
    Gray,
    Black,
}

/// Replay `use_graph/topology.rs::order_by` EXACTLY — declaration-order DFS,
/// postorder output, contract-only Gray-revisit admission.
///
/// Iterative with an explicit `(node, next child)` stack so it mirrors the
/// recursion without one: every node is pushed at most once (the colour table)
/// and every adjacency entry consumed at most once, so a malicious arena can
/// neither spin nor overflow. Bounds are the caller's job.
pub fn replay_order_by(
    seed: u32,
    adjacency: &[Vec<u32>],
    contract: &[bool],
) -> Result<Vec<u32>, String> {
    let mut colour: Vec<Option<Colour>> = vec![None; adjacency.len()];
    let mut order = Vec::new();
    let mut path: Vec<u32> = Vec::new();
    let mut stack: Vec<(u32, usize)> = Vec::new();
    colour[seed as usize] = Some(Colour::Gray);
    path.push(seed);
    stack.push((seed, 0));
    while let Some((node, cursor)) = stack.pop() {
        let children = &adjacency[node as usize];
        if cursor < children.len() {
            stack.push((node, cursor + 1));
            let target = children[cursor];
            match colour[target as usize] {
                Some(Colour::Black) => continue,
                Some(Colour::Gray) => {
                    // `topology.rs:41-49`: the loop suffix starts at the
                    // target's own position on the path, and the revisit is
                    // admitted only when the target AND every node on that
                    // suffix is a `contract/…` document.
                    let start = path.iter().position(|entry| *entry == target).unwrap_or(0);
                    let admitted = contract[target as usize]
                        && path[start..].iter().all(|entry| contract[*entry as usize]);
                    if admitted {
                        continue;
                    }
                    let mut cycle: Vec<u32> = path[start..].to_vec();
                    cycle.push(target);
                    return Err(format!("`use` cycle {cycle:?} touches a non-contract node"));
                }
                None => {
                    colour[target as usize] = Some(Colour::Gray);
                    path.push(target);
                    stack.push((target, 0));
                }
            }
        } else {
            colour[node as usize] = Some(Colour::Black);
            path.pop();
            order.push(node);
        }
    }
    Ok(order)
}

/// The ordered adjacency DERIVED from each node's own `#use` directives.
fn derived_adjacency(nodes: &[Node]) -> Result<Vec<Vec<u32>>, String> {
    nodes
        .iter()
        .map(|node| {
            declared_uses(node)
                .into_iter()
                .map(|entry| resolve(nodes, &entry.target))
                .collect::<Result<Vec<u32>, String>>()
        })
        .collect()
}

/// The whole CLOSE ORDER oracle: edge witness first, then order witness over
/// the derived adjacency. `orders` is the pre-absorb `(seed, order)` per normal
/// contribution, in contribution order; `edges` is the carrier's `kind == use`
/// vector.
pub fn close_faults(nodes: &[Node], edges: &[Edge], orders: &[(u32, Vec<u32>)]) -> Vec<String> {
    let mut out = Vec::new();
    let count = nodes.len();
    for edge in edges {
        if (edge.from as usize) >= count || (edge.to as usize) >= count {
            out.push(format!("edge {} is out of arena bounds", edge.show()));
        }
    }
    for (seed, order) in orders {
        if (*seed as usize) >= count {
            out.push(format!("seed {seed} is out of arena bounds"));
        }
        for node in order {
            if (*node as usize) >= count {
                out.push(format!("occurrence {node} is out of arena bounds"));
            }
        }
    }
    // Nothing below indexes until every id is in range.
    if !out.is_empty() {
        return out;
    }
    let adjacency = match derived_adjacency(nodes) {
        Ok(adjacency) => adjacency,
        Err(fault) => return vec![fault],
    };
    let contract: Vec<bool> = nodes.iter().map(|node| node.contract).collect();

    // ── Witness 1 · the edge vector `close` would have minted ───────────────
    let mut expected: Vec<Edge> = Vec::new();
    let mut replayed: Vec<Option<Vec<u32>>> = Vec::new();
    for (seed, _) in orders {
        match replay_order_by(*seed, &adjacency, &contract) {
            Ok(order) => {
                for node in &order {
                    // `declared_uses` and `adjacency[node]` are the same list
                    // in the same order — the second is just the first
                    // resolved — so zipping them is `close.rs`'s inner loop.
                    let declared = declared_uses(&nodes[*node as usize]);
                    for (entry, to) in declared.into_iter().zip(&adjacency[*node as usize]) {
                        let edge = Edge {
                            from: *node,
                            to: *to,
                            target: entry.target.clone(),
                        };
                        if !expected.contains(&edge) {
                            expected.push(edge);
                        }
                    }
                }
                replayed.push(Some(order));
            }
            Err(fault) => {
                out.push(fault);
                replayed.push(None);
            }
        }
    }
    if expected.len() != edges.len()
        || expected.iter().zip(edges).any(|(want, got)| {
            want.from != got.from || want.to != got.to || want.target != got.target
        })
    {
        out.push(format!(
            "the `use` edge vector is {:?}, but the carried directives mint {:?}",
            edges.iter().map(Edge::show).collect::<Vec<_>>(),
            expected.iter().map(Edge::show).collect::<Vec<_>>()
        ));
    }

    // ── Witness 2 · the traversal order, over the DERIVED adjacency ─────────
    for ((seed, order), expected_order) in orders.iter().zip(&replayed) {
        if order.is_empty() {
            out.push(format!("close order for seed {seed} is empty"));
            continue;
        }
        let mut unique = order.clone();
        unique.sort_unstable();
        unique.dedup();
        if unique.len() != order.len() {
            out.push(format!("close order {order:?} repeats a key"));
        }
        if let Some(expected_order) = expected_order
            && expected_order != order
        {
            out.push(format!(
                "close order {order:?} differs from the declaration-order DFS \
                 of seed {seed}, which yields {expected_order:?}"
            ));
        }
    }
    out
}

/// Project a decoded closure into the oracle's view.
pub fn view(closure: &ClosureIr) -> (Vec<Node>, Vec<Edge>, Vec<(u32, Vec<u32>)>) {
    let nodes = closure
        .nodes
        .iter()
        .map(|node| {
            let (key, contract) = match &node.address {
                DocumentAddress::Spec(spec) => (
                    without_pin(&spec.address),
                    spec.address
                        .doc_path
                        .split('/')
                        .any(|segment| segment == "contract"),
                ),
                DocumentAddress::StaticEntry(entry) => {
                    (format!("static:{}\0{}", entry.origin, entry.path), false)
                }
            };
            Node {
                key,
                contract,
                uses: node
                    .tree
                    .directives
                    .directives
                    .iter()
                    .filter(|directive| matches!(directive.kind, DirectiveKind::Use))
                    .map(|directive| Use {
                        line: directive.line,
                        target: directive.address.clone(),
                    })
                    .collect(),
            }
        })
        .collect();
    let edges = closure
        .edges
        .iter()
        .filter(|edge| matches!(edge.kind, ClosureEdgeKind::Use))
        .map(|edge| Edge {
            from: edge.from,
            to: edge.to,
            target: edge.requested_target.clone(),
        })
        .collect();
    (nodes, edges, pre_absorb_orders(closure))
}

/// The PRE-absorb close order per normal contribution, in contribution order:
/// the applied plan's occurrences when absorb has run (absorb only filters
/// that order), else the live `emission_order`.
pub fn pre_absorb_orders(closure: &ClosureIr) -> Vec<(u32, Vec<u32>)> {
    if let AbsorptionState::Applied(state) = &closure.absorption {
        return state
            .plan
            .contributions
            .iter()
            .filter_map(|entry| match entry {
                ContributionAbsorption::Normal(normal) => Some((
                    normal.seed,
                    normal.occurrences.iter().map(|e| e.node).collect(),
                )),
                _ => None,
            })
            .collect();
    }
    closure
        .contributions
        .iter()
        .filter_map(|entry| match entry {
            ClosureContribution::Normal(normal) => Some((
                normal.seed,
                normal.emission_order.iter().map(|e| e.node).collect(),
            )),
            _ => None,
        })
        .collect()
}

pub fn close_violations(closure: &ClosureIr) -> Vec<String> {
    let (nodes, edges, orders) = view(closure);
    close_faults(&nodes, &edges, &orders)
}
