//! The built-in explicit-`#use` document-batch to closure lowering.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#IR-REFACTOR");

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::use_graph::{UseGraphError, topology, use_addresses};
use crate::{Authority, SpecAddress};

use super::ir::{
    ArtifactId, ClosureContribution, ClosureDocument, ClosureEdge, ClosureEdgeKind, ClosureIr,
    ClosureNodeId, ContributionMeta, DocumentAddress, DocumentIr, Documents,
};
use super::pass::{Pass, PassName};
use super::source_snapshot::SourceResolutionSnapshot;

pub(crate) const CLOSE_PASS_NAME: &str = "close";

#[derive(Debug, Clone)]
struct LoadFailure {
    address: String,
    reason: String,
}

/// Scheduler load observations required for close to replay use-graph errors.
///
/// Discovery records failed loads but never judges them. Close traverses the
/// gathered graph itself and surfaces the first failure or cycle at the exact
/// point the canonical DFS reaches it.
#[derive(Debug, Clone, Default)]
pub(crate) struct CloseState {
    failures: Arc<Mutex<HashMap<String, LoadFailure>>>,
    pending_sources: Arc<Mutex<Option<SourceResolutionSnapshot>>>,
}

impl CloseState {
    pub(crate) fn record_failure(&self, address: &SpecAddress, reason: String) {
        self.failures
            .lock()
            .expect("close discovery failure map is not poisoned")
            .entry(address.without_pin())
            .or_insert_with(|| LoadFailure {
                address: address.to_string(),
                reason,
            });
    }

    fn failure(&self, address: &SpecAddress) -> Option<LoadFailure> {
        self.failures
            .lock()
            .expect("close discovery failure map is not poisoned")
            .get(&address.without_pin())
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
}

pub(crate) struct ClosePass {
    name: PassName,
    seed: SpecAddress,
    state: CloseState,
}

impl ClosePass {
    pub(crate) fn new(seed: SpecAddress, state: CloseState) -> Self {
        Self {
            name: PassName::new(CLOSE_PASS_NAME)
                .expect("the static built-in close pass name is non-blank"),
            seed,
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
        close_documents(&self.seed, input, &self.state)
    }
}

fn close_documents(
    seed: &SpecAddress,
    input: Documents,
    state: &CloseState,
) -> Result<ClosureIr, UseGraphError> {
    let mut documents: HashMap<String, DocumentIr> = input
        .into_iter()
        .map(|document| {
            let key = match document.source().address() {
                DocumentAddress::Spec(address) => address.without_pin(),
                DocumentAddress::StaticEntry { .. } => {
                    unreachable!("the one-seed close worklist contains only spec addresses")
                }
            };
            (key, document)
        })
        .collect();

    let order = topology::order_by(seed, |address| {
        if let Some(document) = documents.get(&address.without_pin()) {
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

    let edge_keys: Vec<(String, Vec<String>)> = order
        .iter()
        .map(|key| {
            let document = documents
                .get(key)
                .expect("topology only returns gathered documents");
            let targets = use_addresses(document.tree().directives())
                .into_iter()
                .map(|address| address.without_pin())
                .collect();
            (key.clone(), targets)
        })
        .collect();

    let mut nodes = Vec::with_capacity(order.len());
    for key in &order {
        let document = documents
            .remove(key)
            .expect("topology only returns gathered documents");
        let (source, tree) = document.into_parts();
        let (address, _format, _raw_text) = source.into_parts();
        let DocumentAddress::Spec(address) = address else {
            unreachable!("the one-seed close worklist contains only spec addresses")
        };
        nodes.push(ClosureDocument {
            origin: document_origin(&address),
            address: DocumentAddress::Spec(address),
            tree,
        });
    }

    let node_ids: HashMap<String, ClosureNodeId> = order
        .iter()
        .enumerate()
        .map(|(index, key)| (key.clone(), ClosureNodeId(index)))
        .collect();
    let mut edges = Vec::new();
    for (from, targets) in edge_keys {
        let from = node_ids[&from];
        for target in targets {
            edges.push(ClosureEdge {
                from,
                to: node_ids[&target],
                kind: ClosureEdgeKind::Use,
            });
        }
    }

    let seed_key = seed.without_pin();
    let seed_id = node_ids[&seed_key];
    let origin = document_origin(seed);
    Ok(ClosureIr {
        artifact: ArtifactId::new("static-fragment")
            .expect("the static compatibility artifact id is non-blank"),
        nodes,
        edges,
        contributions: vec![ClosureContribution::Normal {
            meta: ContributionMeta {
                origin,
                path: seed.doc_path.clone(),
            },
            seed: seed_id,
            emission_order: (0..order.len()).map(ClosureNodeId).collect(),
        }],
        renames: Vec::new(),
        pending_sources: Some(state.pending_sources()),
    })
}

pub(crate) fn document_origin(address: &SpecAddress) -> String {
    match &address.authority {
        Authority::Host(host) => host.clone(),
        Authority::Package { group, name, .. } => format!("{group}/{name}"),
    }
}

#[cfg(test)]
mod tests;
