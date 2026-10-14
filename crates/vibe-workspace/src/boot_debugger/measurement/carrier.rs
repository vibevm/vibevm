//! Measurements of the existing typed carrier; no backend replay.
use super::*;
use vibe_wire::generated::compiler_ir::e1::ir as wire;

#[derive(Debug, Clone, Serialize)]
pub struct LinkedOccurrence {
    pub contribution: usize,
    pub occurrence: u32,
    pub body: Payload,
}
#[derive(Debug, Clone, Serialize)]
pub struct CarrierReport {
    pub capture_id: String,
    pub snapshot: Payload,
    pub recipe: String,
    pub graph: Lane,
    /// Linked body bytes exclude backend framing. They are NOT emitted bytes.
    pub linked_occurrences: Vec<LinkedOccurrence>,
    pub unique_linked_body_bytes: Count,
    pub repeated_linked_body_bytes: Count,
    pub emitted_artifact: Option<Payload>,
    pub emission_provenance: Option<wire::EmissionProvenance>,
    pub tokens: TokenMeasurement,
    pub ambient_tail: Count,
    pub gaps: Vec<String>,
}

/// The exact retained event identity and bytes stay attached to the calculation.
/// This function cannot acquire a writer, call a provider or compile anything.
pub fn measure_retained(capture: &super::super::retained::RetainedGraph) -> CarrierReport {
    let graph = &capture.graph;
    let mut raw = BTreeMap::new();
    let mut linked_occurrences = Vec::new();
    let mut emitted_artifact = None;
    let mut emission_provenance = None;
    let mut gaps = graph.gaps.clone();
    let mut source = |doc: &wire::SourceDoc| {
        let identity = match &doc.address {
            wire::DocumentAddress::Spec(a) => {
                vibe_spec::SourceIdentity::Spec(a.address.raw.clone())
            }
            wire::DocumentAddress::StaticEntry(a) => vibe_spec::SourceIdentity::StaticEntry {
                origin: a.origin.clone(),
                path: a.path.clone(),
            },
        };
        if let Some(index) = graph.sources.iter().position(|s| s.identity == identity) {
            raw.insert(index, text_payload(&doc.text));
        }
    };
    match graph.carrier.as_ref() {
        Some(wire::Ir::SourceDocument(ir)) => source(&ir.doc),
        Some(wire::Ir::DocumentDocument(ir)) => source(&ir.doc.source),
        Some(wire::Ir::DocumentsArtifact(ir)) => for doc in &ir.documents { source(&doc.source); },
        Some(wire::Ir::LaneArtifact(ir)) => {
            for (contribution, row) in ir.lane.contributions.iter().enumerate() {
                let chunks = match row {
                    wire::LaneContribution::Normal(row) => &row.chunks[..],
                    wire::LaneContribution::Simple(row) => &row.chunks[..],
                    _ => &[],
                };
                for chunk in chunks {
                    if let wire::LaneChunk::Node(node) = chunk {
                        let (occurrence, body) = match &node.node {
                            wire::LaneNode::Normal(node) => (node.occurrence, &node.body),
                            wire::LaneNode::Simple(node) => (node.occurrence, &node.body),
                        };
                        linked_occurrences.push(LinkedOccurrence { contribution, occurrence, body: text_payload(body) });
                    }
                }
            }
            gaps.push("Linked bodies are measured verbatim; backend framing and final emitted attribution are unavailable in a lane carrier.".into());
        }
        Some(wire::Ir::EmittedArtifact(ir)) => {
            // The strict captured reader already verifies the compiler's
            // domain-separated digest. Preserve it; plain payload SHA-256 is
            // a different measurement identity, never a replacement witness.
            emission_provenance = Some(ir.emitted.provenance.clone());
            match measure_reader(Base64Bytes::new(&ir.emitted.bytes_b64)) {
                Ok(payload) => emitted_artifact = Some(payload),
                Err(e) => gaps.push(format!("captured emitted bytes unreadable: {e}")),
            }
        }
        Some(wire::Ir::ClosureArtifact(_)) => gaps.push("Closure trees retain lines but not exact raw newline bytes; raw metrics unavailable rather than reconstructed.".into()),
        None => gaps.push("compiler carrier unavailable".into()),
    }
    let mut unique = BTreeMap::new();
    let values: Vec<_> = linked_occurrences
        .iter()
        .map(|o| {
            unique.insert((&o.body.sha256, o.body.bytes), o.body.bytes);
            Count::exact(o.body.bytes)
        })
        .collect();
    let linked_available = matches!(graph.carrier, Some(wire::Ir::LaneArtifact(_)));
    CarrierReport {
        capture_id: format!(
            "{}/{}/{}/{}/{}",
            capture.index.run_id,
            capture.scope,
            capture.sequence,
            capture.invocation,
            capture.snapshot
        ),
        snapshot: measure_reader(capture.bytes.as_slice()).expect("in-memory snapshot"),
        recipe: RECIPE.into(),
        graph: measure_graph(&capture.scope, graph, &raw, &BTreeMap::new()),
        unique_linked_body_bytes: if linked_available {
            Count::sum(
                unique
                    .values()
                    .map(|n| Count::exact(*n))
                    .collect::<Vec<_>>()
                    .iter(),
            )
        } else {
            Count::unavailable("not a linked lane capture")
        },
        repeated_linked_body_bytes: if linked_available {
            Count::sum(values.iter())
        } else {
            Count::unavailable("not a linked lane capture")
        },
        linked_occurrences,
        emitted_artifact,
        emission_provenance,
        tokens: TokenMeasurement {
            tokenizer: None,
            version: None,
            recipe: "unavailable: no tokenizer selected or executed".into(),
            count: Count::unavailable("no tokenizer selected or executed"),
        },
        ambient_tail: Count::unavailable("unknown: not captured"),
        gaps,
    }
}

/// Streaming adapter for the canonical padded base64 already owned/validated
/// by the compiler carrier. This decodes bytes only, never compiler semantics.
struct Base64Bytes<'a> {
    input: &'a [u8],
    output: [u8; 3],
    at: usize,
    len: usize,
}
impl<'a> Base64Bytes<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            input: input.as_bytes(),
            output: [0; 3],
            at: 0,
            len: 0,
        }
    }
}
impl Read for Base64Bytes<'_> {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        let invalid = || io::Error::new(io::ErrorKind::InvalidData, "noncanonical base64");
        let sextet = |b| match b {
            b'A'..=b'Z' => Some(b - b'A'),
            b'a'..=b'z' => Some(b - b'a' + 26),
            b'0'..=b'9' => Some(b - b'0' + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        };
        let mut written = 0;
        while written < out.len() {
            if self.at == self.len {
                if self.input.is_empty() {
                    break;
                }
                if self.input.len() < 4 {
                    return Err(invalid());
                }
                let q = &self.input[..4];
                let a = sextet(q[0]).ok_or_else(invalid)?;
                let b = sextet(q[1]).ok_or_else(invalid)?;
                let c = if q[2] == b'=' {
                    0
                } else {
                    sextet(q[2]).ok_or_else(invalid)?
                };
                let d = if q[3] == b'=' {
                    0
                } else {
                    sextet(q[3]).ok_or_else(invalid)?
                };
                self.len = if q[2] == b'=' {
                    if q[3] != b'=' || b & 15 != 0 {
                        return Err(invalid());
                    }
                    1
                } else if q[3] == b'=' {
                    if c & 3 != 0 {
                        return Err(invalid());
                    }
                    2
                } else {
                    3
                };
                if self.len < 3 && self.input.len() != 4 {
                    return Err(invalid());
                }
                self.output = [(a << 2) | (b >> 4), (b << 4) | (c >> 2), (c << 6) | d];
                self.input = &self.input[4..];
                self.at = 0;
            }
            let n = (self.len - self.at).min(out.len() - written);
            out[written..written + n].copy_from_slice(&self.output[self.at..self.at + n]);
            self.at += n;
            written += n;
        }
        Ok(written)
    }
}
