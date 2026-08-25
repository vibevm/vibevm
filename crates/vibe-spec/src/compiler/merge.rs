//! The built-in recursive `#source` merge pass.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#IR-REFACTOR");

use std::collections::{HashMap, HashSet};

use crate::doctree::{DocTree, NodeId, NodeKind};
use crate::gate::{DuplicateId, first_duplicate};
use crate::merge::fold_sources;
use crate::use_graph::{UseGraphError, topology};
use crate::{DirectiveKind, SpecAddress, SpecAddressError};

use super::close::document_origin;
use super::ir::{
    ClosureContribution, ClosureDocument, ClosureEdge, ClosureEdgeKind, ClosureIr, ClosureNodeId,
    DocumentAddress,
};
use super::pass::{Pass, PassName};
use super::source_snapshot::{DocumentObservation, ExpansionObservation, SourceResolutionSnapshot};

pub(crate) const MERGE_PASS_NAME: &str = "merge";

pub(crate) struct MergePass {
    name: PassName,
}

impl MergePass {
    pub(crate) fn new() -> Self {
        Self {
            name: PassName::new(MERGE_PASS_NAME)
                .expect("the static built-in merge pass name is non-blank"),
        }
    }
}

impl Pass for MergePass {
    type Input = ClosureIr;
    type Output = ClosureIr;
    type Error = MergePassError;

    fn name(&self) -> &PassName {
        &self.name
    }

    fn run(&self, input: ClosureIr) -> Result<ClosureIr, MergePassError> {
        #[cfg(test)]
        MERGE_INVOCATIONS.with(|count| count.set(count.get() + 1));
        merge_closure(input)
    }
}

#[cfg(test)]
std::thread_local! {
    static MERGE_INVOCATIONS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
pub(crate) fn reset_merge_invocations() {
    MERGE_INVOCATIONS.with(|count| count.set(0));
}

#[cfg(test)]
pub(crate) fn merge_invocations() -> usize {
    MERGE_INVOCATIONS.with(std::cell::Cell::get)
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum MergePassError {
    #[error(transparent)]
    Address(#[from] SpecAddressError),
    #[error("use cycle: {}", .0.join(" -> "))]
    SourceCycle(Vec<String>),
    #[error("cannot load {addr}: {reason}")]
    Unresolved { addr: String, reason: String },
    #[error("merged {addr}: {dup}")]
    DuplicateId { addr: String, dup: DuplicateId },
    #[error(
        "merged {addr}: section `{anchor}` is defined by more than one source but not by the contract — a source section matching a contract anchor is an :add sum, but one the contract never declared is a definition, and a name has one definition"
    )]
    DuplicateSourceSection { addr: String, anchor: String },
}

impl MergePassError {
    pub(crate) fn into_compile_error(self) -> crate::pipeline::CompileError {
        match self {
            Self::Address(error) => crate::pipeline::CompileError::Address(error),
            Self::SourceCycle(path) => {
                crate::pipeline::CompileError::UseGraph(UseGraphError::Cycle(path))
            }
            Self::Unresolved { addr, reason } => {
                crate::pipeline::CompileError::Unresolved { addr, reason }
            }
            Self::DuplicateId { addr, dup } => {
                crate::pipeline::CompileError::DuplicateId { addr, dup }
            }
            Self::DuplicateSourceSection { addr, anchor } => {
                crate::pipeline::CompileError::DuplicateSourceSection { addr, anchor }
            }
        }
    }
}

fn merge_closure(mut closure: ClosureIr) -> Result<ClosureIr, MergePassError> {
    let snapshot = closure
        .pending_sources
        .take()
        .expect("close supplies one pending source snapshot");
    let current_trees: HashMap<String, DocTree> = closure
        .nodes
        .iter()
        .map(|node| match &node.address {
            DocumentAddress::Spec(address) => (address.without_pin(), node.tree.clone()),
            DocumentAddress::StaticEntry { .. } => {
                unreachable!("normal merge sees only spec-addressed nodes")
            }
        })
        .collect();
    let roots: Vec<ClosureNodeId> = closure
        .contributions
        .iter()
        .flat_map(|contribution| match contribution {
            ClosureContribution::Normal { emission_order, .. } => emission_order.clone(),
            ClosureContribution::Simple { .. } => Vec::new(),
        })
        .collect();

    let mut updates = HashMap::new();
    let mut source_order = Vec::new();
    let mut accepted_edges = Vec::new();
    for root in roots {
        let DocumentAddress::Spec(address) = &closure.nodes[root.0].address else {
            unreachable!("normal merge sees only spec-addressed nodes")
        };
        let folded = fold_root(address, &snapshot, &current_trees)?;
        for key in folded.order {
            if !source_order.contains(&key) {
                source_order.push(key);
            }
        }
        for edge in folded.edges {
            if !accepted_edges.contains(&edge) {
                accepted_edges.push(edge);
            }
        }
        updates.insert(root, folded.tree);
    }
    for (node, tree) in updates {
        closure.nodes[node.0].tree = tree;
    }
    let mut node_ids: HashMap<String, ClosureNodeId> = closure
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| match &node.address {
            DocumentAddress::Spec(address) => (address.without_pin(), ClosureNodeId(index)),
            DocumentAddress::StaticEntry { .. } => {
                unreachable!("normal merge sees only spec-addressed nodes")
            }
        })
        .collect();
    for key in &source_order {
        if snapshot.explicit_use_keys.contains(key) || node_ids.contains_key(key) {
            continue;
        }
        let Some(document) = snapshot.resolved(key) else {
            continue;
        };
        let DocumentAddress::Spec(address) = document.source().address() else {
            unreachable!("source snapshot contains only spec addresses")
        };
        let id = ClosureNodeId(closure.nodes.len());
        closure.nodes.push(ClosureDocument {
            address: DocumentAddress::Spec(address.clone()),
            origin: document_origin(address),
            tree: document.tree().clone(),
        });
        node_ids.insert(key.clone(), id);
    }

    closure
        .edges
        .extend(accepted_edges.into_iter().map(|(from, to)| ClosureEdge {
            from: node_ids[&from],
            to: node_ids[&to],
            kind: ClosureEdgeKind::Source,
        }));
    Ok(closure)
}

struct FoldRoot {
    tree: DocTree,
    order: Vec<String>,
    edges: Vec<(String, String)>,
}

fn fold_root(
    seed: &SpecAddress,
    snapshot: &SourceResolutionSnapshot,
    current_trees: &HashMap<String, DocTree>,
) -> Result<FoldRoot, MergePassError> {
    let mut targets_by_key = HashMap::new();
    let order = topology::order_by(seed, |address| {
        let tree = tree_for(address, snapshot, current_trees).map_err(as_use_graph_error)?;
        let targets = source_targets(tree, snapshot).map_err(as_use_graph_error)?;
        targets_by_key.insert(address.without_pin(), targets.clone());
        Ok(targets)
    })
    .map_err(|error| match error {
        UseGraphError::Cycle(path) => MergePassError::SourceCycle(path),
        UseGraphError::Unresolved { addr, reason } => MergePassError::Unresolved { addr, reason },
    })?;
    let seed_key = seed.without_pin();
    let accepted_edges: Vec<(String, String)> = order
        .iter()
        .flat_map(|key| {
            targets_by_key[key]
                .iter()
                .map(|target| (key.clone(), target.without_pin()))
        })
        .collect();
    if order.len() == 1 {
        return Ok(FoldRoot {
            tree: tree_for(seed, snapshot, current_trees)?.clone(),
            order,
            edges: accepted_edges,
        });
    }

    let mut folded: HashMap<String, DocTree> = HashMap::new();
    let mut included = HashSet::new();
    for key in &order {
        let address = SpecAddress::parse(key)?;
        let contract = tree_for(&address, snapshot, current_trees)?.clone();
        let members: Vec<DocTree> = targets_by_key[key]
            .iter()
            .cloned()
            .filter_map(|member| {
                let member_key = member.without_pin();
                if included.contains(&member_key) {
                    return None;
                }
                folded.get(&member_key).cloned().inspect(|_| {
                    included.insert(member_key);
                })
            })
            .collect();
        let member_refs: Vec<&DocTree> = members.iter().collect();
        let merged_text = fold_sources(&contract, &member_refs);
        let merged = DocTree::parse(&merged_text);
        if let Some(dup) = first_duplicate(&merged) {
            return Err(MergePassError::DuplicateId {
                addr: key.clone(),
                dup,
            });
        }
        if let Some(anchor) = first_source_section_collision(&contract, &member_refs) {
            return Err(MergePassError::DuplicateSourceSection {
                addr: key.clone(),
                anchor,
            });
        }
        folded.insert(key.clone(), merged);
    }
    Ok(FoldRoot {
        tree: folded
            .remove(&seed_key)
            .expect("seed is last in source fold order"),
        order,
        edges: accepted_edges,
    })
}

fn tree_for<'a>(
    address: &SpecAddress,
    snapshot: &'a SourceResolutionSnapshot,
    current_trees: &'a HashMap<String, DocTree>,
) -> Result<&'a DocTree, MergePassError> {
    let key = address.without_pin();
    if let Some(tree) = current_trees.get(&key) {
        return Ok(tree);
    }
    match snapshot.document(address) {
        Some(DocumentObservation::Resolved(document)) => Ok(document.tree()),
        Some(DocumentObservation::Failed { requested, reason }) => {
            Err(MergePassError::Unresolved {
                addr: requested.to_string(),
                reason: reason.clone(),
            })
        }
        None => panic!("source snapshot omitted `{key}`"),
    }
}

fn source_targets(
    tree: &DocTree,
    snapshot: &SourceResolutionSnapshot,
) -> Result<Vec<SpecAddress>, MergePassError> {
    let mut targets = Vec::new();
    for directive in &tree.directives().directives {
        if directive.kind != DirectiveKind::Source {
            continue;
        }
        match snapshot.expansion(&directive.address) {
            Some(ExpansionObservation::Resolved {
                targets: expanded, ..
            }) => targets.extend(expanded.clone()),
            Some(ExpansionObservation::Failed { requested, reason }) => {
                return Err(MergePassError::Unresolved {
                    addr: requested.to_string(),
                    reason: reason.clone(),
                });
            }
            None => panic!(
                "source snapshot omitted expansion `{}`",
                directive.address.without_pin()
            ),
        }
    }
    Ok(targets)
}

fn as_use_graph_error(error: MergePassError) -> UseGraphError {
    match error {
        MergePassError::Unresolved { addr, reason } => UseGraphError::Unresolved { addr, reason },
        other => panic!("topology callback returned non-resolution merge error: {other}"),
    }
}

fn first_source_section_collision(contract: &DocTree, members: &[&DocTree]) -> Option<String> {
    let contract_anchors: HashSet<&str> = contract
        .children(contract.root())
        .iter()
        .filter_map(|&child| heading_anchor(contract, child))
        .collect();
    let mut declared_by: HashMap<&str, usize> = HashMap::new();
    for member in members {
        let mut seen_here = HashSet::new();
        for &child in member.children(member.root()) {
            if let Some(anchor) = heading_anchor(member, child)
                && !contract_anchors.contains(anchor)
            {
                seen_here.insert(anchor);
            }
        }
        for anchor in seen_here {
            *declared_by.entry(anchor).or_insert(0) += 1;
        }
    }
    for member in members {
        for &child in member.children(member.root()) {
            if let Some(anchor) = heading_anchor(member, child)
                && declared_by.get(anchor).is_some_and(|&count| count >= 2)
            {
                return Some(anchor.to_string());
            }
        }
    }
    None
}

fn heading_anchor(tree: &DocTree, node: NodeId) -> Option<&str> {
    let node = tree.node(node);
    (node.kind == NodeKind::Heading)
        .then_some(node.id.as_deref())
        .flatten()
}

#[cfg(test)]
mod tests;
