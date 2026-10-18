//! Host-bound immutable inspection and explicit prepared builtin previews.
//! Preparation is a separate host-admitted operation; inspection has no provider calls.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-072#DEBUGGER-PROTOCOL");
use super::stream_message::{self, ExpectedBinding};
use crate::{
    document_analysis::{AnalysisCapture, DocumentAnalysisHost, DocumentClient, ResourceId},
    machine_view::*,
};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, VecDeque},
    path::Path,
    sync::{Arc, Mutex},
};
use vibe_core::machine_view::OpaqueText;
pub use vibe_wire::generated::boot_debugger::e1::request as debugger_request;
pub use vibe_wire::generated::boot_debugger::e1::request::Request as BootDebuggerRequest;
pub use vibe_wire::generated::boot_debugger::e1::result as debugger_result;
use vibe_wire::generated::{
    boot_debugger::e1::{error as e, request as q, result as r},
    machine::e1::{
        bound_page as page, entity_reference as entity, operation_envelope as env,
        operation_stream_message as stream,
    },
};
pub use vibe_workspace::boot_debugger::hypothetical;
pub use vibe_workspace::boot_debugger::protocol::CapturedKind;
use vibe_workspace::boot_debugger::{
    measurement,
    protocol::{self, Projection},
};

pub const OPERATION: &str = "rust:vibe-workspace:boot-debugger";
pub const SOURCE: &str = "captured-boot-debugger/1";
type Answer<T> = Result<T, e::Error>;

fn fault(code: e::ErrorCode, message: &str) -> e::Error {
    e::Error {
        schema: 1,
        context_id: None,
        code,
        message: message.into(),
    }
}
fn internal() -> e::Error {
    fault(e::ErrorCode::Internal, "inspection owner invariant failed")
}
fn text(s: &str) -> OpaqueText {
    OpaqueText::new(s).expect("owner nonblank identity")
}
fn hash(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}
fn json<T: serde::Serialize>(value: &T) -> Answer<String> {
    serde_json::to_string(value).map_err(|_| internal())
}
fn exact(n: u64) -> r::Count {
    r::Count {
        exact: Some(n.to_string()),
        unavailable: None,
    }
}
fn unavailable(reason: &str) -> r::Count {
    r::Count {
        exact: None,
        unavailable: Some(reason.into()),
    }
}
fn decimal(s: &str) -> Answer<u64> {
    let n = s.parse::<u64>().map_err(|_| {
        fault(
            e::ErrorCode::InvalidRequest,
            "expected canonical decimal u64",
        )
    })?;
    if n.to_string() != s {
        return Err(fault(e::ErrorCode::InvalidRequest, "noncanonical decimal"));
    }
    Ok(n)
}

struct Snapshot {
    capture: AnalysisCapture,
    projection: Option<Projection>,
    resources: Vec<r::Resource>,
    nodes: Vec<r::Node>,
    sources: Vec<r::Source>,
    gaps: Vec<String>,
}
struct Inspection {
    snapshot: String,
    filter: String,
    projection: String,
    stamps: Vec<AxisStamp>,
    pages: PageSession,
    stopped: bool,
    terminal: bool,
    sequence: u64,
    delivered: u64,
    events: VecDeque<stream::EventRecord>,
    result_id: String,
}
struct State {
    client: DocumentClient,
    snapshots: BTreeMap<String, Arc<Snapshot>>,
    inspections: BTreeMap<String, Inspection>,
    next: u64,
    scenarios: BTreeMap<String, hypothetical::PreparedScenario>,
    preparations: BTreeMap<String, Arc<preparation::PreparationService>>,
}

/// The embedding chooses one already admitted boot artifact and its reader.
/// All resources remain owned by the supplied document host; tokens cannot
/// introduce another root, resource, overlay client or writer.
pub struct BootDebuggerRoute {
    host: Arc<DocumentAnalysisHost>,
    artifact: ResourceId,
    kind: CapturedKind,
    context: String,
    state: Mutex<State>,
}
impl BootDebuggerRoute {
    pub fn new(
        host: Arc<DocumentAnalysisHost>,
        artifact: ResourceId,
        kind: CapturedKind,
    ) -> Answer<Self> {
        let client = host
            .client()
            .map_err(|_| fault(e::ErrorCode::Unavailable, "document client unavailable"))?;
        let first = client
            .freeze()
            .map_err(|_| fault(e::ErrorCode::Unavailable, "document capture unavailable"))?;
        if first.resource(artifact).is_none() {
            return Err(fault(
                e::ErrorCode::UnknownResource,
                "artifact not admitted by host",
            ));
        }
        let context = format!("boot-context:{}", first.context());
        let route = Self {
            host,
            artifact,
            kind,
            context,
            state: Mutex::new(State {
                client,
                snapshots: BTreeMap::new(),
                inspections: BTreeMap::new(),
                next: 1,
                scenarios: BTreeMap::new(),
                preparations: BTreeMap::new(),
            }),
        };
        Ok(route)
    }
    pub fn root(&self) -> &Path {
        self.host.root()
    }
    pub fn context_id(&self) -> &str {
        &self.context
    }
    /// Extract only an already retained, host-admitted finite inventory.
    /// This does not freeze live files or grant a source/provider capability.
    pub fn hypothetical_host(&self, id: &str) -> Result<hypothetical::HostCapture, String> {
        let state = self
            .state
            .lock()
            .map_err(|_| "debugger state unavailable")?;
        let snapshot = state
            .snapshots
            .get(id)
            .ok_or("missing retained host capture")?;
        let files: BTreeMap<_, _> = snapshot
            .capture
            .resources()
            .filter_map(|(_, r)| r.raw().map(|bytes| (r.path().to_owned(), bytes.to_vec())))
            .collect();
        let text = files
            .get("vibe.toml")
            .ok_or("missing host manifest capture: vibe.toml")?;
        let manifest = vibe_core::manifest::Manifest::parse_str(
            std::str::from_utf8(text).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        Ok(hypothetical::HostCapture {
            id: id.into(),
            manifest,
            files,
        })
    }
    /// Normal composition seam: the package source binds both real resolver
    /// worlds before transport can request pure computation. Tokens cannot
    /// supply a graph, source path or compiler/provider implementation.
    pub fn retain_hypothetical(
        &self,
        scenario: hypothetical::PreparedScenario,
    ) -> Result<q::Request, String> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| "debugger state unavailable")?;
        for world in [scenario.baseline(), scenario.candidate()] {
            if !state.snapshots.contains_key(world.capture()) {
                return Err("missing or foreign retained source capture".into());
            }
        }
        let request = q::Request {
            schema: 1,
            context_id: self.context.clone(),
            command: q::Command::Preview(Box::new(q::CommandPreview {
                scenario_id: scenario.identity().into(),
                expected_baseline: scenario.baseline().identity().into(),
                expected_candidate: scenario.candidate().identity().into(),
                expected_catalog: scenario.candidate().catalog().into(),
                loading_overrides: BTreeMap::new(),
            })),
        };
        state.scenarios.insert(scenario.identity().into(), scenario);
        Ok(request)
    }
    /// Explicit host composition can retain another registered client's frozen
    /// overlay. Wire requests cannot construct a capture or acquire that client.
    pub fn retain_capture(&self, capture: AnalysisCapture) -> Answer<String> {
        if format!("boot-context:{}", capture.context()) != self.context {
            return Err(fault(
                e::ErrorCode::WrongContext,
                "capture belongs to another host",
            ));
        }
        let id = capture.id().to_owned();
        let mut state = self.state.lock().map_err(|_| internal())?;
        if state.snapshots.contains_key(&id) {
            return Err(fault(
                e::ErrorCode::InvalidRequest,
                "capture already retained",
            ));
        }
        state
            .snapshots
            .insert(id.clone(), Arc::new(self.snapshot(capture)));
        Ok(id)
    }
    pub fn compare_context_root(&self, root: &Path) -> Answer<()> {
        if self.root() != root {
            return Err(fault(e::ErrorCode::WrongContext, "host root mismatch"));
        }
        Ok(())
    }
    fn snapshot(&self, capture: AnalysisCapture) -> Snapshot {
        let resources = capture
            .resources()
            .enumerate()
            .map(|(i, (_, value))| r::Resource {
                resource_id: format!("{}:resource-{i}", capture.id()),
                path: value.path().into(),
                digest: value.content_digest(),
                buffer_version: value.buffer_version().map(|v| v.to_string()),
                basis: r::ResourceBasis::OriginalBytes,
                bytes: value
                    .raw()
                    .map(|b| exact(b.len() as u64))
                    .unwrap_or_else(|| unavailable("captured resource has no active bytes")),
            })
            .collect();
        let mut gaps = vec!["Finite per-file capture; not atomic all-files, source authentication or hostile-writer proof.".into()];
        let projection = match capture.resource(self.artifact).and_then(|r| r.raw()) {
            Some(bytes) => match protocol::project(capture.id(), self.kind, bytes) {
                Ok(p) => Some(p),
                Err(error) => {
                    gaps.push(format!("structured artifact unavailable: {error}"));
                    None
                }
            },
            None => {
                gaps.push("captured artifact bytes unavailable".into());
                None
            }
        };
        let mut nodes = vec![];
        let mut sources = vec![];
        let artifact = capture
            .resources()
            .enumerate()
            .find(|(_, (id, _))| *id == self.artifact)
            .map(|(i, (_, resource))| {
                (
                    format!("{}:resource-{i}", capture.id()),
                    resource.content_digest(),
                )
            });
        if let Some(p) = &projection {
            gaps.extend(p.graph.gaps.clone());
            let (artifact_id, artifact_digest) = artifact.as_ref().expect("admitted artifact");
            let artifact_digest = artifact_digest.as_ref().expect("projected bytes");
            for (index, source) in p.graph.sources.iter().enumerate() {
                let identity = match &source.identity {
                    vibe_spec::SourceIdentity::Spec(address) => {
                        r::SourceIdentity::Spec(Box::new(r::SourceIdentitySpec {
                            address: address.clone(),
                        }))
                    }
                    vibe_spec::SourceIdentity::CapturedNode(index) => {
                        r::SourceIdentity::CapturedNode(Box::new(r::SourceIdentityCapturedNode {
                            index: index.to_string(),
                        }))
                    }
                    vibe_spec::SourceIdentity::StaticEntry { origin, path } => {
                        r::SourceIdentity::StaticEntry(Box::new(r::SourceIdentityStaticEntry {
                            origin: origin.clone(),
                            path: path.clone(),
                        }))
                    }
                };
                sources.push(r::Source {
                    source: index as u32,
                    identity,
                    snapshot_id: capture.id().into(),
                    artifact_resource_id: artifact_id.clone(),
                    artifact_digest: artifact_digest.clone(),
                    // A graph identity is not an authenticated join to a host path.
                    authored_resource_id: None,
                });
            }
            for (c, row) in p.graph.contributions.iter().enumerate() {
                for (i, node) in row.occurrences.iter().enumerate() {
                    nodes.push(r::Node {
                        artifact_lines: node.artifact_lines.map(|(start, end)| r::ArtifactLines {
                            basis: r::ArtifactLinesBasis::CapturedEmittedArtifactOneBasedInclusiveLines,
                            snapshot_id: capture.id().into(), resource_id: artifact_id.clone(),
                            digest: artifact_digest.clone(), start_line: start.to_string(), end_line: end.to_string(),
                        }),
                        contribution: c as u32,
                        occurrence: i as u32,
                        source: node.source as u32,
                        path: row.path.clone(),
                        origin: row.origin.clone(),
                        requested: node.requested.clone(),
                        parent: match node.parent {
                            vibe_spec::InclusionParent::Occurrence(p) => Some(p as u32),
                            _ => None,
                        },
                        root: node.parent == vibe_spec::InclusionParent::Root,
                    });
                }
            }
        }
        Snapshot {
            capture,
            projection,
            resources,
            nodes,
            sources,
            gaps,
        }
    }
    fn result(&self, snapshot: Option<String>, body: r::Body) -> Answer<r::Result> {
        let result_id = hash(json(&(&self.context, &snapshot, &body))?.as_bytes());
        Ok(r::Result {
            schema: 1,
            context_id: self.context.clone(),
            snapshot_id: snapshot,
            result_id,
            body,
        })
    }
    pub fn request(&self, request: q::Request) -> Answer<r::Result> {
        if request.schema != 1 {
            return Err(fault(
                e::ErrorCode::UnsupportedEpoch,
                "expected debugger epoch 1",
            ));
        }
        if request.context_id != self.context {
            return Err(fault(e::ErrorCode::WrongContext, "foreign context"));
        }
        let mut state = self.state.lock().map_err(|_| internal())?;
        self.dispatch(&mut state, request.command)
            .map_err(|mut error| {
                error.context_id = Some(self.context.clone());
                error
            })
    }
    /// Decode consumers must check the wrapper and the incumbent stream owner,
    /// not merely trust the embedded JSON string's schema annotation.
    pub fn validate_progress(
        &self,
        progress: &vibe_wire::generated::boot_debugger::e1::progress::Progress,
    ) -> Answer<()> {
        if progress.schema != 1 {
            return Err(fault(
                e::ErrorCode::UnsupportedEpoch,
                "unsupported progress epoch",
            ));
        }
        if progress.context_id != self.context
            || progress.stream_format_id != "machine-operation-stream-message"
        {
            return Err(fault(
                e::ErrorCode::WrongContext,
                "foreign progress binding",
            ));
        }
        let state = self.state.lock().map_err(|_| internal())?;
        let op = state
            .inspections
            .get(&progress.inspection_id)
            .ok_or_else(|| {
                fault(
                    e::ErrorCode::UnknownInspection,
                    "unknown progress inspection",
                )
            })?;
        if op.snapshot != progress.snapshot_id {
            return Err(fault(
                e::ErrorCode::UnknownSnapshot,
                "mixed progress snapshot",
            ));
        }
        stream_message::parse(
            progress.stream_json.as_bytes(),
            self.binding(&progress.inspection_id),
        )
        .map_err(|_| {
            fault(
                e::ErrorCode::InvalidRequest,
                "invalid nested progress stream",
            )
        })?;
        Ok(())
    }
    /// Strict generated structured input through the incumbent operation envelope.
    pub fn exchange(&self, bytes: &[u8]) -> env::OperationEnvelope {
        let mut preparation_requested = false;
        let answer = (|| {
            if bytes.len() > stream_message::MAX_FRAME_BYTES {
                return Err(fault(
                    e::ErrorCode::InvalidRequest,
                    "input frame exceeds bound",
                ));
            }
            let input: env::OperationEnvelope = serde_json::from_slice(bytes)
                .map_err(|_| fault(e::ErrorCode::InvalidRequest, "invalid operation envelope"))?;
            if input.schema != 1
                || input.operation_id.as_deref() != Some(OPERATION)
                || input.domain_owner.as_deref() != Some(OPERATION)
                || input.source_id.as_deref() != Some(SOURCE)
                || input.evidence_id.is_some()
            {
                return Err(fault(
                    e::ErrorCode::InvalidRequest,
                    "foreign outer operation binding",
                ));
            }
            let env::Body::Input(body) = input.body else {
                return Err(fault(e::ErrorCode::InvalidRequest, "expected input stage"));
            };
            if body.format_id != "boot-debugger-request" {
                return Err(fault(
                    e::ErrorCode::InvalidRequest,
                    "foreign request format",
                ));
            }
            let request: q::Request = serde_json::from_str(&body.payload_json)
                .map_err(|_| fault(e::ErrorCode::InvalidRequest, "invalid generated request"))?;
            preparation_requested = matches!(&request.command, q::Command::Prepare(_));
            self.request(request)
        })();
        let body = match answer {
            Ok(result) => {
                if let r::Body::Replay(replay) = &result.body {
                    let progress = vibe_wire::generated::boot_debugger::e1::progress::Progress {
                        schema: 1,
                        context_id: result.context_id.clone(),
                        snapshot_id: result
                            .snapshot_id
                            .clone()
                            .expect("retained replay snapshot"),
                        inspection_id: replay.inspection_id.clone(),
                        stream_format_id: replay.stream_format_id.clone(),
                        stream_json: replay.stream_json.clone(),
                    };
                    if let Err(error) = self.validate_progress(&progress) {
                        return envelope(env::Body::Error(Box::new(env::BodyError {
                            format_id: Some("boot-debugger-error".into()),
                            payload_json: Some(
                                serde_json::to_string(&error).expect("generated error"),
                            ),
                            effect_state: env::BodyErrorEffectState::NoEffects,
                            completeness: env::BodyErrorCompleteness::Unavailable,
                            provenance_id: result.snapshot_id,
                        })));
                    }
                    env::Body::Event(Box::new(env::BodyEvent {
                        event_id: result.result_id.clone(),
                        format_id: Some("boot-debugger-progress".into()),
                        payload_json: Some(
                            serde_json::to_string(&progress).expect("generated progress"),
                        ),
                        effect_state: env::BodyEventEffectState::NoEffects,
                        completeness: env::BodyEventCompleteness::Complete,
                        provenance_id: result.snapshot_id,
                    }))
                } else {
                    let partial = match &result.body {
                        r::Body::Inspect(v) => !v.gaps.is_empty(),
                        r::Body::Explain(v) => v.unavailable.is_some(),
                        r::Body::Cost(_) | r::Body::Tree(_) => true,
                        r::Body::Preparation(p) => !matches!(
                            p.status,
                            r::BodyPreparationStatus::Complete | r::BodyPreparationStatus::Planned
                        ),
                        _ => false,
                    };
                    env::Body::Result(Box::new(env::BodyResult {
                        format_id: Some("boot-debugger-result".into()),
                        payload_json: Some(
                            serde_json::to_string(&result).expect("generated result"),
                        ),
                        effect_state: if preparation_requested
                            && matches!(&result.body, r::Body::Preparation(p) if p.outcome_json.is_some())
                        {
                            if matches!(&result.body, r::Body::Preparation(p) if p.status == r::BodyPreparationStatus::RecoveryRequired)
                            {
                                env::BodyResultEffectState::Uncertain
                            } else {
                                env::BodyResultEffectState::Completed
                            }
                        } else {
                            env::BodyResultEffectState::NoEffects
                        },
                        completeness: if partial {
                            env::BodyResultCompleteness::Partial
                        } else {
                            env::BodyResultCompleteness::Complete
                        },
                        provenance_id: result.snapshot_id,
                    }))
                }
            }
            Err(error) => env::Body::Error(Box::new(env::BodyError {
                format_id: Some("boot-debugger-error".into()),
                payload_json: Some(serde_json::to_string(&error).expect("generated error")),
                effect_state: if preparation_requested {
                    env::BodyErrorEffectState::Uncertain
                } else {
                    env::BodyErrorEffectState::NoEffects
                },
                completeness: env::BodyErrorCompleteness::Complete,
                provenance_id: None,
            })),
        };
        envelope(body)
    }
}
pub fn envelope(body: env::Body) -> env::OperationEnvelope {
    env::OperationEnvelope {
        schema: 1,
        operation_id: Some(OPERATION.into()),
        domain_owner: Some(OPERATION.into()),
        source_id: Some(SOURCE.into()),
        evidence_id: None,
        body,
    }
}
pub fn input(request: &q::Request) -> env::OperationEnvelope {
    envelope(env::Body::Input(Box::new(env::BodyInput {
        format_id: "boot-debugger-request".into(),
        payload_json: serde_json::to_string(request).expect("generated request"),
    })))
}

mod dispatch;
#[cfg(test)]
mod m_06_d_2_cases;
pub mod preparation;
mod preview;
#[cfg(test)]
mod tests;
