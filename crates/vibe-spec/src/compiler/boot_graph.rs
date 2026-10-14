//! Read-only projections of existing compiler carriers (PROP-072).
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-072#CAPTURED-GRAPH");

use super::{ir::*, pass::AnyIr};
use crate::DocTree;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReferenceState {
    Unknown,
    Resolved,
    Unresolved,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceIdentity {
    Spec(String),
    /// Closure arena identity local to this exact captured carrier.
    CapturedNode(usize),
    StaticEntry {
        origin: String,
        path: String,
    },
}

#[derive(Debug, Clone)]
pub struct GraphSource {
    pub identity: SourceIdentity,
    /// Existing tree preserves exact zero-based, half-open source line spans
    /// and parent/child structure. None means the input carried no tree.
    pub tree: Option<DocTree>,
}

#[derive(Debug, Clone)]
pub struct GraphEdge {
    pub from: usize,
    pub to: usize,
    pub kind: String,
    pub requested: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InclusionParent {
    Unknown,
    Root,
    Occurrence(usize),
}

#[derive(Debug, Clone)]
pub struct GraphOccurrence {
    pub source: usize,
    pub requested: Option<String>,
    pub state: ReferenceState,
    /// Parent occurrence, only where the capture actually records nesting.
    pub parent: InclusionParent,
    /// One-based inclusive artifact lines, not authored-source lines.
    pub artifact_lines: Option<(u64, u64)>,
}

#[derive(Debug, Clone)]
pub struct GraphContribution {
    pub origin: String,
    pub path: String,
    pub kind: String,
    pub seed: Option<usize>,
    pub occurrences: Vec<GraphOccurrence>,
}

#[derive(Debug, Clone, Default)]
pub struct BootGraph {
    pub sources: Vec<GraphSource>,
    pub contributions: Vec<GraphContribution>,
    pub edges: Vec<GraphEdge>,
    pub gaps: Vec<String>,
    /// Complete existing typed carrier, including link/absorption plans,
    /// pending resolution snapshots, chunk framing and emission digests.
    /// This is the compiler wire vocabulary, not a replacement IR.
    pub carrier: Option<vibe_wire::generated::compiler_ir::e1::ir::Ir>,
}

impl BootGraph {
    /// Deduplicate logical sources only; never contributions or occurrences.
    pub fn source(&mut self, identity: SourceIdentity, tree: Option<DocTree>) -> usize {
        if let Some(i) = self.sources.iter().position(|s| s.identity == identity) {
            return i;
        }
        let i = self.sources.len();
        self.sources.push(GraphSource { identity, tree });
        i
    }
}

fn identity(address: &DocumentAddress) -> SourceIdentity {
    match address {
        DocumentAddress::Spec(a) => SourceIdentity::Spec(a.to_string()),
        DocumentAddress::StaticEntry { origin, path } => SourceIdentity::StaticEntry {
            origin: origin.clone(),
            path: path.clone(),
        },
    }
}

/// Decode with the existing strict compiler reader and verifier. No providers
/// or compile entry points are reachable from this adapter.
pub fn inspect_boot_snapshot(bytes: &[u8]) -> Result<BootGraph, String> {
    let ir = super::wire::decode(bytes).map_err(|e| e.to_string())?;
    let mut graph = project(&ir);
    graph.carrier = Some(super::wire::encode_generated(&ir).map_err(|e| e.to_string())?);
    Ok(graph)
}

fn project(ir: &AnyIr) -> BootGraph {
    let mut graph = BootGraph::default();
    match ir {
        AnyIr::Closure(closure) => {
            // Keep the arena indices verbatim: edges and repeated occurrence
            // references are already the compiler's logical identity.
            graph.sources = closure
                .nodes
                .iter()
                .map(|d| GraphSource {
                    identity: identity(&d.address),
                    tree: Some(d.tree.clone()),
                })
                .collect();
            graph.edges = closure
                .edges
                .iter()
                .map(|e| GraphEdge {
                    from: e.from.0,
                    to: e.to.0,
                    kind: format!("{:?}", e.kind),
                    requested: e.requested_target.to_string(),
                })
                .collect();
            for c in &closure.contributions {
                let (meta, kind, seed, occurrences) = match c {
                    ClosureContribution::Normal {
                        meta,
                        seed,
                        emission_order,
                        ..
                    } => (
                        meta,
                        "normal",
                        Some(seed.0),
                        emission_order
                            .iter()
                            .map(|o| GraphOccurrence {
                                source: o.node.0,
                                requested: Some(o.requested_address.to_string()),
                                state: ReferenceState::Resolved,
                                parent: InclusionParent::Unknown,
                                artifact_lines: None,
                            })
                            .collect(),
                    ),
                    ClosureContribution::Simple { meta, document } => {
                        let source =
                            graph.source(identity(&document.address), Some(document.tree.clone()));
                        (
                            meta,
                            "simple",
                            Some(source),
                            vec![GraphOccurrence {
                                source,
                                requested: None,
                                state: ReferenceState::Resolved,
                                parent: InclusionParent::Unknown,
                                artifact_lines: None,
                            }],
                        )
                    }
                    ClosureContribution::Elided { meta } => (meta, "elided", None, vec![]),
                    ClosureContribution::Hoisted { meta, target } => {
                        let source = graph.source(SourceIdentity::Spec(target.to_string()), None);
                        (
                            meta,
                            "hoisted",
                            Some(source),
                            vec![GraphOccurrence {
                                source,
                                requested: Some(target.to_string()),
                                state: ReferenceState::Unknown,
                                parent: InclusionParent::Unknown,
                                artifact_lines: None,
                            }],
                        )
                    }
                };
                graph.contributions.push(GraphContribution {
                    origin: meta.origin.clone(),
                    path: meta.path.clone(),
                    kind: kind.into(),
                    seed,
                    occurrences,
                });
            }
            graph.gaps.push("ClosureOccurrence has node and requested_address but no parent occurrence or edge index; ClosureEdge has from/to/kind/requested_target but no directive source span. Exact occurrence why chains require these fields at capture time; dependency paths alone are ambiguous for repeated occurrences.".into());
        }
        AnyIr::Source(source) => {
            graph.source(identity(source.address()), None);
        }
        AnyIr::Document(doc) => {
            graph.source(identity(doc.source().address()), Some(doc.tree().clone()));
        }
        AnyIr::Documents(docs) => {
            for doc in docs.iter() {
                graph.source(identity(doc.source().address()), Some(doc.tree().clone()));
            }
        }
        AnyIr::Lane(lane) => project_lane(&mut graph, &lane.contributions),
        AnyIr::Emitted(emitted) => {
            for witness in &emitted.provenance.contributions {
                let (meta, kind, address) = match witness {
                    EmissionContributionWitness::Normal {
                        meta, seed_address, ..
                    } => (
                        meta,
                        "normal",
                        Some(SourceIdentity::Spec(seed_address.to_string())),
                    ),
                    EmissionContributionWitness::Simple { meta, address, .. } => {
                        (meta, "simple", Some(identity(address)))
                    }
                    EmissionContributionWitness::Elided { meta } => (meta, "elided", None),
                    EmissionContributionWitness::Hoisted { meta, target } => (
                        meta,
                        "hoisted",
                        Some(SourceIdentity::Spec(target.to_string())),
                    ),
                };
                let seed = address.map(|a| graph.source(a, None));
                graph.contributions.push(GraphContribution {
                    origin: meta.origin.clone(),
                    path: meta.path.clone(),
                    kind: kind.into(),
                    seed,
                    occurrences: vec![],
                });
            }
            graph.gaps.push("EmissionProvenance.contributions retains chunk_digest, not LaneContribution.chunks; occurrence sequence, parent edge and source spans cannot be recovered from that digest. Read the same scope's retained lane for occurrences; no cross-snapshot equality is assumed.".into());
        }
    }
    graph
}

fn project_lane(graph: &mut BootGraph, rows: &[LaneContribution]) {
    for row in rows {
        let (meta, kind, address, chunks) = match row {
            LaneContribution::Normal {
                meta, seed, chunks, ..
            } => (
                meta,
                "normal",
                Some(SourceIdentity::CapturedNode(seed.0)),
                chunks.as_slice(),
            ),
            LaneContribution::Simple {
                meta,
                address,
                chunks,
            } => (meta, "simple", Some(identity(address)), chunks.as_slice()),
            LaneContribution::Elided { meta } => (meta, "elided", None, &[][..]),
            LaneContribution::Hoisted { meta, target } => (
                meta,
                "hoisted",
                Some(SourceIdentity::Spec(target.to_string())),
                &[][..],
            ),
        };
        let seed = address.map(|a| graph.source(a, None));
        let mut occurrences = Vec::new();
        for chunk in chunks {
            if let LaneChunk::Node(node) = chunk {
                let (address, requested) = match node.as_ref() {
                    LaneNode::Normal {
                        node,
                        requested_address,
                        ..
                    } => (
                        SourceIdentity::CapturedNode(node.0),
                        Some(requested_address.to_string()),
                    ),
                    LaneNode::Simple { address, .. } => (identity(address), None),
                };
                let source = graph.source(address, None);
                occurrences.push(GraphOccurrence {
                    source,
                    requested,
                    state: ReferenceState::Resolved,
                    parent: InclusionParent::Unknown,
                    artifact_lines: None,
                });
            }
        }
        graph.contributions.push(GraphContribution {
            origin: meta.origin.clone(),
            path: meta.path.clone(),
            kind: kind.into(),
            seed,
            occurrences,
        });
    }
    graph.gaps.push("LaneNode retains contribution/occurrence/node ordinals, requested address, body and fences, but has no parent occurrence/edge or authored source span field. Chunk framing and node ids remain in carrier; requested addresses alone do not prove logical node equality across snapshots.".into());
}

#[cfg(test)]
mod m_06_b_1_cases;
