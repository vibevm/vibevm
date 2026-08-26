//! The built-in gathered-document batch to multi-seed Closure lowering.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#IR-REFACTOR");

use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};

use crate::use_graph::{UseGraphError, topology, use_addresses};
use crate::{Authority, SpecAddress};

use super::embed_snapshot::EmbedResolutionSnapshot;
use super::ir::{
    AbsorptionState, ArtifactInput, ArtifactPlan, ClosureContribution, ClosureDocument,
    ClosureEdge, ClosureEdgeKind, ClosureIr, ClosureNodeId, DocumentAddress, DocumentIr, Documents,
    LinkState, QualificationState,
};
use super::pass::{Pass, PassName};
use super::source_snapshot::SourceResolutionSnapshot;
use super::worklist::document_key;

pub(crate) const CLOSE_PASS_NAME: &str = "close";

#[derive(Debug, Clone)]
struct LoadFailure {
    address: String,
    reason: String,
}

/// Scheduler observations replayed by the named close/merge/embed passes.
#[derive(Debug, Clone, Default)]
pub(crate) struct CloseState {
    failures: Arc<Mutex<HashMap<String, LoadFailure>>>,
    pending_sources: Arc<Mutex<Option<SourceResolutionSnapshot>>>,
    pending_embeds: Arc<Mutex<Option<EmbedResolutionSnapshot>>>,
}

impl CloseState {
    pub(crate) fn record_failure(&self, address: &SpecAddress, reason: String) {
        self.failures
            .lock()
            .expect("close discovery failure map is not poisoned")
            .entry(address.to_string())
            .or_insert_with(|| LoadFailure {
                address: address.to_string(),
                reason,
            });
    }

    fn failure(&self, address: &SpecAddress) -> Option<LoadFailure> {
        self.failures
            .lock()
            .expect("close discovery failure map is not poisoned")
            .get(&address.to_string())
            .cloned()
    }

    pub(crate) fn set_pending_sources(&self, snapshot: SourceResolutionSnapshot) {
        let previous = self
            .pending_sources
            .lock()
            .expect("close pending-source state is not poisoned")
            .replace(snapshot);
        assert!(previous.is_none(), "pending source snapshot is set once");
    }

    fn pending_sources(&self) -> SourceResolutionSnapshot {
        self.pending_sources
            .lock()
            .expect("close pending-source state is not poisoned")
            .clone()
            .unwrap_or_default()
    }

    pub(crate) fn set_pending_embeds(&self, snapshot: EmbedResolutionSnapshot) {
        let previous = self
            .pending_embeds
            .lock()
            .expect("close pending-embed state is not poisoned")
            .replace(snapshot);
        assert!(previous.is_none(), "pending embed snapshot is set once");
    }

    fn pending_embeds(&self) -> EmbedResolutionSnapshot {
        self.pending_embeds
            .lock()
            .expect("close pending-embed state is not poisoned")
            .clone()
            .unwrap_or_default()
    }
}

pub(crate) struct ClosePass {
    name: PassName,
    plan: ArtifactPlan,
    state: CloseState,
}

impl ClosePass {
    pub(crate) fn new(plan: ArtifactPlan, state: CloseState) -> Self {
        Self {
            name: PassName::new(CLOSE_PASS_NAME)
                .expect("the static built-in close pass name is non-blank"),
            plan,
            state,
        }
    }
}

impl Pass for ClosePass {
    type Input = Documents;
    type Output = ClosureIr;
    type Error = UseGraphError;

    fn name(&self) -> &PassName {
        &self.name
    }

    fn run(&self, input: Documents) -> Result<ClosureIr, UseGraphError> {
        close_documents(&self.plan, input, &self.state)
    }
}

fn close_documents(
    plan: &ArtifactPlan,
    input: Documents,
    state: &CloseState,
) -> Result<ClosureIr, UseGraphError> {
    let mut spec_documents = BTreeMap::new();
    let mut simple_documents = BTreeMap::new();
    for document in input {
        match document.source().address() {
            DocumentAddress::Spec(address) => {
                spec_documents.insert(address.without_pin(), document);
            }
            address @ DocumentAddress::StaticEntry { .. } => {
                simple_documents.insert(document_key(address), document);
            }
        }
    }

    let mut nodes = Vec::new();
    let mut node_ids = BTreeMap::new();
    let mut edges = Vec::new();
    let mut contributions = Vec::with_capacity(plan.contributions().len());

    for input in plan.contributions() {
        match input {
            ArtifactInput::Normal { meta, seed } => {
                let mut requests = BTreeMap::new();
                let order = topology::order_by(seed, |address| {
                    requests
                        .entry(address.without_pin())
                        .or_insert_with(|| address.clone());
                    if let Some(document) = spec_documents.get(&address.without_pin()) {
                        return Ok(use_addresses(document.tree().directives()));
                    }
                    let failure = state.failure(address).unwrap_or_else(|| {
                        panic!(
                            "close discovery omitted `{}` without recording a load failure",
                            address.without_pin()
                        )
                    });
                    Err(UseGraphError::Unresolved {
                        addr: failure.address,
                        reason: failure.reason,
                    })
                })?;

                let mut emission_order = Vec::with_capacity(order.len());
                for key in &order {
                    let id = ensure_node(key, &spec_documents, &mut nodes, &mut node_ids);
                    emission_order.push(super::ir::ClosureOccurrence {
                        node: id,
                        requested_address: requests[key].clone(),
                    });
                }
                for key in &order {
                    let from = node_ids[key];
                    let document = &spec_documents[key];
                    for target in use_addresses(document.tree().directives()) {
                        let edge = ClosureEdge {
                            from,
                            to: node_ids[&target.without_pin()],
                            kind: ClosureEdgeKind::Use,
                            requested_target: target,
                        };
                        if !edges.contains(&edge) {
                            edges.push(edge);
                        }
                    }
                }
                contributions.push(ClosureContribution::Normal {
                    meta: meta.clone(),
                    seed: node_ids[&seed.without_pin()],
                    seed_address: seed.clone(),
                    emission_order,
                });
            }
            ArtifactInput::Simple { meta, source } => {
                let key = document_key(source.address());
                let document = simple_documents
                    .get(&key)
                    .unwrap_or_else(|| panic!("gather omitted simple document `{key}`"));
                contributions.push(ClosureContribution::Simple {
                    meta: meta.clone(),
                    document: Box::new(close_document(document)),
                });
            }
            ArtifactInput::Elided { meta } => {
                contributions.push(ClosureContribution::Elided { meta: meta.clone() })
            }
            ArtifactInput::Hoisted { meta, target } => {
                contributions.push(ClosureContribution::Hoisted {
                    meta: meta.clone(),
                    target: target.clone(),
                });
            }
        }
    }

    Ok(ClosureIr::from_plan(
        plan,
        nodes,
        edges,
        contributions,
        Vec::new(),
        QualificationState::Pending(plan.context().mode()),
        AbsorptionState::Unplanned,
        LinkState::Unlinked,
        Some(state.pending_sources()),
        Some(state.pending_embeds()),
    ))
}

fn ensure_node(
    key: &str,
    documents: &BTreeMap<String, DocumentIr>,
    nodes: &mut Vec<ClosureDocument>,
    node_ids: &mut BTreeMap<String, ClosureNodeId>,
) -> ClosureNodeId {
    if let Some(id) = node_ids.get(key) {
        return *id;
    }
    let id = ClosureNodeId(nodes.len());
    nodes.push(close_document(
        documents
            .get(key)
            .expect("topology only returns gathered documents"),
    ));
    node_ids.insert(key.to_string(), id);
    id
}

fn close_document(document: &DocumentIr) -> ClosureDocument {
    let address = document.source().address().clone();
    let origin = match &address {
        DocumentAddress::Spec(address) => document_origin(address),
        DocumentAddress::StaticEntry { origin, .. } => origin.clone(),
    };
    ClosureDocument {
        address,
        origin,
        tree: document.tree().clone(),
        aliases: Default::default(),
    }
}

pub(crate) fn document_origin(address: &SpecAddress) -> String {
    match &address.authority {
        Authority::Host(host) => host.clone(),
        Authority::Package { group, name, .. } => format!("{group}/{name}"),
    }
}

#[cfg(test)]
#[path = "close/tests.rs"]
mod tests;
