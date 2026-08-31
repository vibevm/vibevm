use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use base64::Engine as _;
use vibe_core::lifecycle::CompilePoint;
use vibe_wire::generated::native::e1::compile_reply::{
    CompileReply, CompileReplyFail, CompileReplyOk, CompileReplySkip,
};
use vibe_wire::generated::shared::{ArtifactFrame, Ir};

use crate::compiler::artifact_tests::{Fixture, fixture};
use crate::compiler::backend::BackendRegistry;
use crate::compiler::builtin::{ArtifactCompileError, compile_artifact_native_with_registries};
use crate::compiler::emit::emitted_bytes_digest;
use crate::compiler::ir::{DocumentSubject, SourceIr};
use crate::compiler::observer::{CompileObserver, EmissionEvent, StageDeltaEvent};
use crate::compiler::trace::{CompileTraceSink, PassTraceEvent};

use super::behavior::{TransformBehavior, TransformBehaviorError};
use super::lowering_worlds::{Declared, collected_host};
use super::native_identity::CompilerNativeImplementationDigest;
use super::native_manager::{
    CompilerNativeCall, CompilerNativeInvoker, CompilerNativeInvokerError,
    CompilerNativeInvokerErrorKind,
};
use super::plan::{TransformConfig, TransformPlan, TransformStage};
use super::registry::TransformRegistry;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CallRecord {
    pub(super) key: String,
    pub(super) point: CompilePoint,
    pub(super) order: u32,
    pub(super) config: BTreeMap<String, Option<serde_json::Value>>,
    pub(super) implementation: CompilerNativeImplementationDigest,
    pub(super) carrier: &'static str,
    pub(super) ir_schema: u32,
    pub(super) payload: Ir,
}

#[derive(Clone)]
pub(super) enum ReplyMode {
    Ok,
    SkipOrder(u32),
    SkipAll,
    FailOrder(u32),
    DuplicateRoot,
    DuplicateIr,
    DuplicateMap,
    UnknownRoot,
    UnknownStatus,
    IllegalSkipPayload,
    WrongEnvelope,
    WrongSchema,
    ReturnPayload(Ir),
    MalformedJson,
    InvalidUtf8,
    BuildableSourceUnavailable,
    BuildableOrder(u32),
    BuildableFirstCall(u32),
    BuildableAfterFirstCall(u32),
    InvocationFailed,
    ForgedSourceIdentity,
    ForgedDocumentIdentity,
    ForgedLaneProvenance,
    InvalidSource,
    LawfulSourceMutation,
    TemporaryEmittedProvenance,
    ChangedEmittedBytes,
}

pub(super) struct FakeInvoker {
    mode: ReplyMode,
    calls: Mutex<Vec<CallRecord>>,
    ordered: Option<Arc<Mutex<Vec<String>>>>,
}

impl FakeInvoker {
    pub(super) fn new(mode: ReplyMode) -> Self {
        Self {
            mode,
            calls: Mutex::new(Vec::new()),
            ordered: None,
        }
    }

    pub(super) fn with_ordered_log(mode: ReplyMode, ordered: Arc<Mutex<Vec<String>>>) -> Self {
        Self {
            mode,
            calls: Mutex::new(Vec::new()),
            ordered: Some(ordered),
        }
    }

    pub(super) fn records(&self) -> Vec<CallRecord> {
        self.calls.lock().unwrap().clone()
    }
}

impl CompilerNativeInvoker for FakeInvoker {
    fn invoke(&self, call: CompilerNativeCall<'_>) -> Result<Vec<u8>, CompilerNativeInvokerError> {
        let (carrier, ir_schema) = carrier(&call.payload().clone());
        let record = CallRecord {
            key: call.key().as_str().to_string(),
            point: call.point(),
            order: call.order(),
            config: call.config().clone(),
            implementation: call.implementation(),
            carrier,
            ir_schema,
            payload: call.payload().clone(),
        };
        let calls_for_order = {
            let mut calls = self.calls.lock().unwrap();
            calls.push(record);
            calls
                .iter()
                .filter(|record| record.order == call.order())
                .count()
        };
        if let Some(log) = &self.ordered {
            log.lock().unwrap().push(format!("native:{}", call.order()));
        }
        let buildable = matches!(self.mode, ReplyMode::BuildableSourceUnavailable)
            || matches!(self.mode, ReplyMode::BuildableOrder(order) if order == call.order())
            || matches!(self.mode, ReplyMode::BuildableFirstCall(order)
                if order == call.order() && calls_for_order == 1)
            || matches!(self.mode, ReplyMode::BuildableAfterFirstCall(order)
                if order == call.order() && calls_for_order > 1);
        if buildable {
            return Err(CompilerNativeInvokerError::new(
                CompilerNativeInvokerErrorKind::BuildableSourceUnavailable,
                "source exists but has not been built",
            ));
        }
        if matches!(self.mode, ReplyMode::InvocationFailed) {
            return Err(CompilerNativeInvokerError::new(
                CompilerNativeInvokerErrorKind::InvocationFailed,
                "ordinary invocation failure",
            ));
        }
        if matches!(self.mode, ReplyMode::MalformedJson) {
            return Ok(b"{".to_vec());
        }
        if matches!(self.mode, ReplyMode::InvalidUtf8) {
            return Ok(vec![0xff, b'{']);
        }

        let mut payload = match &self.mode {
            ReplyMode::ReturnPayload(payload) => payload.clone(),
            _ => call.payload().clone(),
        };
        mutate_payload(&self.mode, &mut payload);
        if matches!(self.mode, ReplyMode::IllegalSkipPayload) {
            return Ok(format!(
                "{{\"status\":\"skip\",\"envelope\":1,\"payload\":{}}}",
                serde_json::to_string(&payload).unwrap()
            )
            .into_bytes());
        }

        let mut reply = match self.mode {
            ReplyMode::SkipAll => CompileReply::Skip(Box::new(CompileReplySkip {
                envelope: 1,
                message: Some("not applicable".to_string()),
            })),
            ReplyMode::SkipOrder(order) if call.order() == order => {
                CompileReply::Skip(Box::new(CompileReplySkip {
                    envelope: 1,
                    message: Some("not applicable".to_string()),
                }))
            }
            ReplyMode::FailOrder(order) if call.order() == order => {
                CompileReply::Fail(Box::new(CompileReplyFail {
                    envelope: 1,
                    message: Some("x".repeat(64 * 1024)),
                }))
            }
            _ => CompileReply::Ok(Box::new(CompileReplyOk {
                envelope: 1,
                payload,
                message: None,
            })),
        };
        if matches!(self.mode, ReplyMode::WrongEnvelope) {
            set_envelope(&mut reply, 2);
        }
        let mut raw = serde_json::to_string(&reply).unwrap();
        match self.mode {
            ReplyMode::DuplicateRoot => {
                raw = raw.replacen("\"envelope\":1", "\"envelope\":1,\"envelope\":1", 1);
            }
            ReplyMode::DuplicateIr => {
                raw = raw.replacen("\"ir_schema\":1", "\"ir_schema\":1,\"ir_schema\":1", 1);
            }
            ReplyMode::DuplicateMap => raw = duplicate_anchor_member(&raw),
            ReplyMode::UnknownRoot => raw.insert_str(raw.len() - 1, ",\"future\":true"),
            ReplyMode::UnknownStatus => raw = raw.replacen("\"ok\"", "\"future\"", 1),
            _ => {}
        }
        Ok(raw.into_bytes())
    }
}

fn mutate_payload(mode: &ReplyMode, payload: &mut Ir) {
    match (mode, payload) {
        (ReplyMode::ForgedSourceIdentity, Ir::SourceDocument(value)) => {
            value.doc.subject.declared_path = "boot/forged.md".to_string();
        }
        (ReplyMode::ForgedDocumentIdentity, Ir::DocumentDocument(value)) => {
            value.doc.source.subject.declared_path = "boot/forged.md".to_string();
        }
        (ReplyMode::ForgedLaneProvenance, Ir::LaneArtifact(value)) => {
            value.lane.frame.generated_path = Some("forged.xml".to_string());
            if let ArtifactFrame::StaticLane(frame) = &mut value.lane.context.frame {
                frame.generated_path = "forged.xml".to_string();
            }
        }
        (ReplyMode::InvalidSource, Ir::SourceDocument(value)) => value.doc.format.clear(),
        (ReplyMode::LawfulSourceMutation, Ir::SourceDocument(value)) => {
            value.doc.text.push_str("\nLawful native body mutation.\n");
        }
        (ReplyMode::TemporaryEmittedProvenance, Ir::EmittedArtifact(value)) => value
            .emitted
            .provenance
            .emitted_transforms
            .push("plugin:temporary".to_string()),
        (ReplyMode::ChangedEmittedBytes, Ir::EmittedArtifact(value)) => {
            let mut bytes = base64::engine::general_purpose::STANDARD
                .decode(&value.emitted.bytes_b64)
                .unwrap();
            bytes.push(b'!');
            value.emitted.bytes_b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
            value.emitted.provenance.bytes_digest = hex(emitted_bytes_digest(&bytes));
            value
                .emitted
                .provenance
                .emitted_transforms
                .push("plugin:temporary".to_string());
        }
        (ReplyMode::WrongSchema, payload) => set_schema(payload, 2),
        _ => {}
    }
}

fn set_envelope(reply: &mut CompileReply, envelope: u32) {
    match reply {
        CompileReply::Ok(value) => value.envelope = envelope,
        CompileReply::Skip(value) => value.envelope = envelope,
        CompileReply::Fail(value) => value.envelope = envelope,
    }
}

fn set_schema(payload: &mut Ir, schema: u32) {
    match payload {
        Ir::SourceDocument(value) => value.ir_schema = schema,
        Ir::DocumentDocument(value) => value.ir_schema = schema,
        Ir::DocumentsArtifact(value) => value.ir_schema = schema,
        Ir::ClosureArtifact(value) => value.ir_schema = schema,
        Ir::LaneArtifact(value) => value.ir_schema = schema,
        Ir::EmittedArtifact(value) => value.ir_schema = schema,
    }
}

fn carrier(payload: &Ir) -> (&'static str, u32) {
    match payload {
        Ir::SourceDocument(value) => ("source-document", value.ir_schema),
        Ir::DocumentDocument(value) => ("document-document", value.ir_schema),
        Ir::DocumentsArtifact(value) => ("documents-artifact", value.ir_schema),
        Ir::ClosureArtifact(value) => ("closure-artifact", value.ir_schema),
        Ir::LaneArtifact(value) => ("lane-artifact", value.ir_schema),
        Ir::EmittedArtifact(value) => ("emitted-artifact", value.ir_schema),
    }
}

fn duplicate_anchor_member(raw: &str) -> String {
    let marker = "\"anchors\":{";
    let mut search = 0;
    while let Some(relative) = raw[search..].find(marker) {
        let object = search + relative + marker.len();
        if raw.as_bytes().get(object) == Some(&b'}') {
            search = object + 1;
            continue;
        }
        let tail = &raw[object..];
        let end = tail.find([',', '}']).unwrap_or(tail.len());
        let member = &tail[..end];
        return format!(
            "{}{member},{member}{}",
            &raw[..object],
            &raw[object + end..]
        );
    }
    raw.to_string()
}

fn hex(bytes: [u8; 32]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut value = String::with_capacity(64);
    for byte in bytes {
        value.push(HEX[(byte >> 4) as usize] as char);
        value.push(HEX[(byte & 0x0f) as usize] as char);
    }
    value
}

pub(super) fn plan(declarations: Vec<Declared>) -> crate::compiler::ir::ArtifactPlan {
    let registry = collected_host(declarations);
    let transforms = TransformPlan::from_effective_rows(&registry.enabled_compile_rows())
        .expect("native rows lower");
    fixture().plan.with_transforms(transforms)
}

pub(super) fn plan_with_registry(
    declarations: Vec<Declared>,
    registry: &TransformRegistry,
) -> crate::compiler::ir::ArtifactPlan {
    let collected = collected_host(declarations);
    let transforms =
        TransformPlan::from_effective_rows_with(&collected.enabled_compile_rows(), registry)
            .expect("mixed rows lower");
    fixture().plan.with_transforms(transforms)
}

pub(super) fn compile_mixed(
    plan: crate::compiler::ir::ArtifactPlan,
    world: &Fixture,
    registry: &TransformRegistry,
    invoker: &dyn CompilerNativeInvoker,
) -> Result<crate::compiler::ir::EmittedArtifact, ArtifactCompileError> {
    compile_artifact_native_with_registries(
        plan,
        &world.source,
        &BackendRegistry::builtins(),
        registry,
        invoker,
    )
}

pub(super) struct OrderedSourceBuiltin {
    pub(super) log: Arc<Mutex<Vec<String>>>,
    pub(super) forge_subject: bool,
}

impl TransformBehavior for OrderedSourceBuiltin {
    fn name(&self) -> &str {
        "test-native-adjacent"
    }

    fn epoch(&self) -> u32 {
        1
    }

    fn stage(&self) -> TransformStage {
        TransformStage::Source
    }

    fn run_source(
        &self,
        _config: Option<&TransformConfig>,
        input: SourceIr,
    ) -> Result<SourceIr, TransformBehaviorError> {
        self.log.lock().unwrap().push("builtin".to_string());
        if !self.forge_subject {
            return Ok(input);
        }
        Ok(SourceIr::new(
            input.address().clone(),
            input.format().clone(),
            DocumentSubject::declared(input.subject().provider().clone(), "boot/builtin-forged.md"),
            input.text().to_string(),
        ))
    }
}

pub(super) fn registry_with_ordered_builtin(
    log: Arc<Mutex<Vec<String>>>,
    forge_subject: bool,
) -> TransformRegistry {
    let mut registry = TransformRegistry::default();
    registry
        .register(Arc::new(OrderedSourceBuiltin { log, forge_subject }))
        .unwrap();
    registry
}

pub(super) struct Sink;

impl CompileTraceSink for Sink {
    fn record(&self, _event: &PassTraceEvent<'_>) {}
}

impl CompileObserver for Sink {
    fn emission(&self, _event: &EmissionEvent) {}

    fn stage_delta(&self, _event: &StageDeltaEvent) {}
}

pub(super) fn point_name(point: CompilePoint) -> &'static str {
    match point {
        CompilePoint::Source => "compile:source",
        CompilePoint::Document => "compile:document",
        CompilePoint::Lane => "compile:lane",
        CompilePoint::Emitted => "compile:emitted",
        CompilePoint::Pass => "compile:pass",
    }
}

pub(super) fn fixture_world() -> Fixture {
    fixture()
}
