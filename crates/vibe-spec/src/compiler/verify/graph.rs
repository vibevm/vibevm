//! Closure-level invariants: bounds, identity, reachability, per-edge-kind
//! cycle law, and qualification/absorption/link typestate.
//!
//! The checks run in one fixed order (structure → duplicate identity → tree
//! well-formedness → DuplicateId → cycles → reachability → simple coherence →
//! typestate) so a mutation test never depends on hash iteration, and a
//! bounds error always precedes anything that would index invalid data.

use std::collections::HashMap;

use crate::use_graph::{cycle, topology};

use super::super::absorb::validate_applied_absorption;
use super::super::close::document_origin;
use super::super::ir::{
    AbsorptionState, ClosureContribution, ClosureEdgeKind, ClosureIr, DocumentAddress, LinkState,
    QualificationState,
};
use super::super::link::validate_linked;
use super::super::qualify::validate_planned_absorption;
use super::{VerificationError, address_label};

/// The whole closure contract, in its fixed deterministic order.
pub(super) fn verify_closure(closure: &ClosureIr) -> Result<(), VerificationError> {
    verify_bounds(closure)?;
    verify_node_identity(closure)?;
    verify_request_identity(closure)?;
    verify_trees(closure)?;
    verify_cycles(closure)?;
    verify_reachability(closure)?;
    verify_simple(closure)?;
    verify_typestate(closure)
}

/// Every `ClosureNodeId` is bounds-checked before any indexing runs.
fn verify_bounds(closure: &ClosureIr) -> Result<(), VerificationError> {
    let len = closure.nodes.len();
    for edge in &closure.edges {
        if edge.from.0 >= len {
            return Err(VerificationError::InvalidNodeId {
                site: "edge source",
                index: edge.from.0,
                len,
            });
        }
        if edge.to.0 >= len {
            return Err(VerificationError::InvalidNodeId {
                site: "edge target",
                index: edge.to.0,
                len,
            });
        }
    }
    for contribution in &closure.contributions {
        if let ClosureContribution::Normal {
            seed,
            emission_order,
            ..
        } = contribution
        {
            if seed.0 >= len {
                return Err(VerificationError::InvalidNodeId {
                    site: "normal seed",
                    index: seed.0,
                    len,
                });
            }
            for occurrence in emission_order {
                if occurrence.node.0 >= len {
                    return Err(VerificationError::InvalidNodeId {
                        site: "emission occurrence node",
                        index: occurrence.node.0,
                        len,
                    });
                }
            }
        }
    }
    Ok(())
}

/// Graph nodes are spec-addressed, canonically unique, and owned by the
/// authority their address names. Simple documents stay inside their simple
/// contribution; they are never smuggled into the graph.
fn verify_node_identity(closure: &ClosureIr) -> Result<(), VerificationError> {
    let mut first_position: HashMap<String, usize> = HashMap::new();
    for (index, node) in closure.nodes.iter().enumerate() {
        let DocumentAddress::Spec(address) = &node.address else {
            return Err(VerificationError::NodeAddressKind { index });
        };
        let key = address.without_pin();
        if let Some(first) = first_position.insert(key.clone(), index) {
            return Err(VerificationError::DuplicateNodeAddress {
                first,
                second: index,
                key,
            });
        }
        let expected = document_origin(address);
        if node.origin != expected {
            return Err(VerificationError::NodeOriginMismatch {
                index,
                expected,
                actual: node.origin.clone(),
            });
        }
    }
    Ok(())
}

/// Every recorded *request* still names the node it was resolved to.
///
/// A contribution's `seed_address`, an occurrence's `requested_address` and an
/// edge's `requested_target` are the authored spellings close resolved; the node
/// id beside each is the resolution. A pass that retargets a request while
/// keeping the node id — or renames a node while keeping the request — leaves a
/// carrier whose provenance lies. Downstream, `qualify`'s absorption analysis
/// only `debug_assert_eq!`s this, so without a real check the compiler behaves
/// one way in debug and another in release; here it is a typed failure under the
/// name of the pass that produced it, in both profiles.
fn verify_request_identity(closure: &ClosureIr) -> Result<(), VerificationError> {
    for (index, edge) in closure.edges.iter().enumerate() {
        let actual = node_key(closure, edge.to.0);
        if edge.requested_target.without_pin() != actual {
            return Err(VerificationError::EdgeTargetMismatch {
                edge: index,
                expected: edge.requested_target.without_pin(),
                actual,
            });
        }
    }
    for (contribution, entry) in closure.contributions.iter().enumerate() {
        let ClosureContribution::Normal {
            seed,
            seed_address,
            emission_order,
            ..
        } = entry
        else {
            continue;
        };
        let actual = node_key(closure, seed.0);
        if seed_address.without_pin() != actual {
            return Err(VerificationError::SeedAddressMismatch {
                contribution,
                expected: seed_address.without_pin(),
                actual,
            });
        }
        for (occurrence, current) in emission_order.iter().enumerate() {
            let actual = node_key(closure, current.node.0);
            if current.requested_address.without_pin() != actual {
                return Err(VerificationError::OccurrenceAddressMismatch {
                    contribution,
                    occurrence,
                    expected: current.requested_address.without_pin(),
                    actual,
                });
            }
        }
    }
    Ok(())
}

/// Tree shape first for every carrier document, then the anchor gate — the
/// two phases never interleave, keeping first-failure order deterministic.
fn verify_trees(closure: &ClosureIr) -> Result<(), VerificationError> {
    for (address, tree) in carrier_documents(closure) {
        tree.verify_structure()
            .map_err(|source| VerificationError::DocTree {
                address: address_label(&address),
                source,
            })?;
    }
    for (address, tree) in carrier_documents(closure) {
        if let Some(duplicate) = crate::gate::first_duplicate(tree) {
            return Err(VerificationError::DuplicateId {
                address: address_label(&address),
                duplicate,
            });
        }
    }
    Ok(())
}

/// Graph documents in node order, then simple documents in contribution
/// order — the deterministic carrier order for tree checks.
fn carrier_documents(
    closure: &ClosureIr,
) -> impl Iterator<Item = (DocumentAddress, &crate::DocTree)> {
    closure
        .nodes
        .iter()
        .map(|node| (node.address.clone(), &node.tree))
        .chain(closure.contributions.iter().filter_map(|contribution| {
            let ClosureContribution::Simple { document, .. } = contribution else {
                return None;
            };
            Some((document.address.clone(), &document.tree))
        }))
}

/// Cycles are judged per edge kind, never on the union: Embed is strictly
/// acyclic; Use and Source may hold only contract-only forward-declaration
/// cycles (PROP-035 §9); a mixed-kind union cycle whose individual relations are
/// acyclic is legal retained provenance, not one recursive evaluator.
///
/// The verdict comes from the same [`cycle::first_illegal_cycle`] law the
/// engine walkers use, over whole strongly connected components, keyed by the
/// nodes' stable pinless addresses. A DFS-shaped rule would answer from
/// whichever loop its traversal happened to discover, and an id-keyed one from
/// whichever order handed out the ids — this cell numbers nodes in arena order
/// while `topology::order_by` numbers them in discovery order, so only a
/// component law over semantic keys makes the two agree on every graph.
fn verify_cycles(closure: &ClosureIr) -> Result<(), VerificationError> {
    let keys: Vec<String> = (0..closure.nodes.len())
        .map(|index| node_key(closure, index))
        .collect();
    for kind in [
        ClosureEdgeKind::Use,
        ClosureEdgeKind::Source,
        ClosureEdgeKind::Embed,
    ] {
        let relation: Vec<(usize, usize)> = closure
            .edges
            .iter()
            .filter(|edge| edge.kind == kind)
            .map(|edge| (edge.from.0, edge.to.0))
            .collect();
        let offender = cycle::first_illegal_cycle(&keys, &relation, |index| match kind {
            ClosureEdgeKind::Embed => false,
            ClosureEdgeKind::Use | ClosureEdgeKind::Source => is_contract_node(closure, index),
        });
        if let Some(path) = offender {
            return Err(VerificationError::IllegalCycle {
                kind,
                path: path.into_iter().map(|index| keys[index].clone()).collect(),
            });
        }
    }
    Ok(())
}

fn is_contract_node(closure: &ClosureIr, index: usize) -> bool {
    match &closure.nodes[index].address {
        DocumentAddress::Spec(address) => topology::is_contract_address(address),
        DocumentAddress::StaticEntry { .. } => false,
    }
}

fn node_key(closure: &ClosureIr, index: usize) -> String {
    match &closure.nodes[index].address {
        DocumentAddress::Spec(address) => address.without_pin(),
        DocumentAddress::StaticEntry { origin, path } => format!("static:{origin}:{path}"),
    }
}

/// Every graph node is reachable from at least one normal seed over the union
/// of retained edges. A source/embed-only node that a live root reaches is
/// valid even though no emission order lists it — absence from an order is not
/// orphaning.
fn verify_reachability(closure: &ClosureIr) -> Result<(), VerificationError> {
    let len = closure.nodes.len();
    let mut adjacency = vec![Vec::new(); len];
    for edge in &closure.edges {
        adjacency[edge.from.0].push(edge.to.0);
    }
    let mut reached = vec![false; len];
    let mut stack: Vec<usize> = Vec::new();
    for contribution in &closure.contributions {
        if let ClosureContribution::Normal { seed, .. } = contribution
            && !reached[seed.0]
        {
            reached[seed.0] = true;
            stack.push(seed.0);
        }
    }
    while let Some(node) = stack.pop() {
        for &next in &adjacency[node] {
            if !reached[next] {
                reached[next] = true;
                stack.push(next);
            }
        }
    }
    if let Some(index) = reached.iter().position(|seen| !seen) {
        return Err(VerificationError::UnreachableNode { index });
    }
    Ok(())
}

/// Simple contributions carry exactly their one static entry, owned by the
/// origin the address names.
fn verify_simple(closure: &ClosureIr) -> Result<(), VerificationError> {
    for (contribution, entry) in closure.contributions.iter().enumerate() {
        let ClosureContribution::Simple { document, .. } = entry else {
            continue;
        };
        let DocumentAddress::StaticEntry { origin, .. } = &document.address else {
            return Err(VerificationError::SimpleAddressKind { contribution });
        };
        if &document.origin != origin {
            return Err(VerificationError::SimpleOriginMismatch {
                contribution,
                expected: origin.clone(),
                actual: document.origin.clone(),
            });
        }
    }
    Ok(())
}

/// Qualification/absorption/link typestate, judged from the carrier alone: a
/// pass is never credited by its name, only by the state it left behind.
fn verify_typestate(closure: &ClosureIr) -> Result<(), VerificationError> {
    let qualification = match closure.qualification {
        QualificationState::Pending(_) => "pending",
        QualificationState::Applied(_) => "applied",
    };
    match (&closure.qualification, &closure.absorption) {
        (QualificationState::Pending(_), AbsorptionState::Unplanned) => {
            if !closure.renames.is_empty() {
                return Err(VerificationError::PendingRenames {
                    count: closure.renames.len(),
                });
            }
        }
        (QualificationState::Pending(_), AbsorptionState::Planned(_)) => {
            return Err(VerificationError::MisalignedState {
                qualification,
                absorption: "planned",
            });
        }
        (QualificationState::Pending(_), AbsorptionState::Applied(_)) => {
            return Err(VerificationError::MisalignedState {
                qualification,
                absorption: "applied",
            });
        }
        (QualificationState::Applied(_), AbsorptionState::Unplanned) => {
            return Err(VerificationError::MisalignedState {
                qualification,
                absorption: "unplanned",
            });
        }
        (QualificationState::Applied(_), AbsorptionState::Planned(plan)) => {
            verify_snapshots_consumed(closure)?;
            validate_planned_absorption(plan, closure).map_err(|source| {
                VerificationError::AbsorptionPlanned {
                    source: Box::new(source),
                }
            })?;
        }
        (QualificationState::Applied(_), AbsorptionState::Applied(_)) => {
            verify_snapshots_consumed(closure)?;
            validate_applied_absorption(closure).map_err(|source| {
                VerificationError::AbsorptionApplied {
                    source: Box::new(source),
                }
            })?;
        }
    }
    if let LinkState::Linked(_) = closure.link {
        validate_linked(closure).map_err(|source| VerificationError::LinkReplay {
            source: Box::new(source),
        })?;
    }
    Ok(())
}

fn verify_snapshots_consumed(closure: &ClosureIr) -> Result<(), VerificationError> {
    if closure.pending_sources.is_some() {
        return Err(VerificationError::PendingSnapshotsLive { kind: "source" });
    }
    if closure.pending_embeds.is_some() {
        return Err(VerificationError::PendingSnapshotsLive { kind: "embed" });
    }
    Ok(())
}
