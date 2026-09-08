//! R6.4 production pass-tier scheduling and mandatory verification acceptance.

use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use vibe_core::lifecycle::ExtensionPoint;
use vibe_core::manifest::{
    ExtensionDecl, ExtensionHandler, ExtensionIrLevel, ExtensionPass, ExtensionPassKind,
    ExtensionsControl,
};
use vibe_extension_registry::{
    ExtensionRegistry, ExtensionWorld, HostExtensionSource, HostIdentity, HostProvider,
    collect_extensions,
};
use vibe_wire::generated::native::e1::compile_reply::{
    CompileReply, CompileReplyFail, CompileReplyOk,
};
use vibe_wire::generated::shared::{
    AbsorptionState, AbsorptionStateUnplanned, Ir, QualificationState, QualificationStatePending,
};

use super::execution::PassTierExecutionError;
use super::lowering::lower_effective_compile_rows;
use crate::SectionSource;
use crate::SpecAddress;
use crate::compiler::builtin::without_verify_each;
use crate::compiler::builtin::{
    ArtifactCompileError, BuiltinSchedule, compile_artifact, compile_artifact_native,
    compile_artifact_native_traced, compile_artifact_passes_for_test,
};
use crate::compiler::ir::{
    ArtifactInput, ArtifactPlan, ArtifactTarget, DocumentAddress, SourceFormatId, SourceIr,
};
use crate::compiler::pass::AnyIr;
use crate::compiler::trace::{CompileTraceSink, PassTraceEvent};
use crate::compiler::transform::native_manager::{
    CompilerNativeCall, CompilerNativeInvoker, CompilerNativeInvokerError,
};
use crate::compiler::wire;

fn transform(level: ExtensionIrLevel, after: &str) -> ExtensionPass {
    ExtensionPass {
        kind: ExtensionPassKind::Transform,
        level: Some(level),
        from: None,
        to: None,
        after: Some(after.to_owned()),
        before: None,
        replace: None,
        formats: None,
        artifact: None,
    }
}

fn before_transform(level: ExtensionIrLevel, before: &str) -> ExtensionPass {
    let mut pass = transform(level, "unused");
    pass.after = None;
    pass.before = Some(before.to_owned());
    pass
}

fn replacement(anchor: &str) -> ExtensionPass {
    ExtensionPass {
        kind: ExtensionPassKind::Lowering,
        level: None,
        from: None,
        to: None,
        after: None,
        before: None,
        replace: Some(anchor.to_owned()),
        formats: None,
        artifact: Some("static-xml".to_owned()),
    }
}

fn backend_for(artifact: &str) -> ExtensionPass {
    ExtensionPass {
        kind: ExtensionPassKind::Backend,
        level: None,
        from: None,
        to: None,
        after: None,
        before: None,
        replace: None,
        formats: None,
        artifact: Some(artifact.to_owned()),
    }
}

fn frontend_for(format: &str) -> ExtensionPass {
    ExtensionPass {
        kind: ExtensionPassKind::Frontend,
        level: None,
        from: None,
        to: None,
        after: None,
        before: None,
        replace: None,
        formats: Some(vec![format.to_owned()]),
        artifact: None,
    }
}

fn declaration(id: &str, pass: ExtensionPass) -> ExtensionDecl {
    ExtensionDecl {
        id: id.to_owned(),
        point: "compile:pass".parse::<ExtensionPoint>().unwrap(),
        handler: ExtensionHandler::Native {
            crate_dir: Some(PathBuf::from(format!("passes/{id}"))),
            prebuilt: None,
        },
        config: None,
        auto: None,
        inputs: None,
        applies_to: None,
        compiler_internals: Some(true),
        pass: Some(pass),
        when: None,
    }
}

fn registry(declarations: Vec<ExtensionDecl>) -> ExtensionRegistry {
    collect_extensions(ExtensionWorld {
        installed: Vec::new(),
        host: HostExtensionSource {
            provider: HostProvider {
                identity: HostIdentity::ungrouped_project("demo"),
                root: PathBuf::from("."),
                version: "0.1.0".into(),
                kind: None,
                content_hash: None,
            },
            declarations,
            controls: ExtensionsControl::default(),
            mechanisms: Vec::new(),
        },
        effective_stack: None,
    })
    .unwrap()
}

fn artifact_plan(declarations: Vec<ExtensionDecl>) -> ArtifactPlan {
    let registry = registry(declarations);
    let passes = lower_effective_compile_rows(&registry.enabled_compile_rows())
        .unwrap()
        .passes()
        .clone();
    ArtifactPlan::static_lane(
        ArtifactTarget::StaticXml,
        "vibevm/vibespecs/boot/STATIC.xml",
        "vibevm/vibespecs",
        inputs(),
    )
    .unwrap()
    .with_passes(passes)
}

fn inputs() -> Vec<ArtifactInput> {
    ["one", "two"]
        .into_iter()
        .map(|name| {
            ArtifactInput::normal(
                format!("org.demo/{name}"),
                format!("boot/{name}.md"),
                address(&format!("spec://org.demo/{name}/boot/{name}#root")),
            )
            .unwrap()
        })
        .collect()
}

fn address(value: &str) -> SpecAddress {
    SpecAddress::parse(value).unwrap()
}

#[derive(Default)]
struct Source {
    reads: AtomicUsize,
}

impl Source {
    fn reads(&self) -> usize {
        self.reads.load(Ordering::SeqCst)
    }
}

impl SectionSource for Source {
    fn section_text(&self, _address: &SpecAddress) -> Result<String, String> {
        self.reads.fetch_add(1, Ordering::SeqCst);
        Ok("# Root {#root}\n".to_owned())
    }
}

enum ReplyMode {
    Echo,
    Fail(String),
    WrongCarrier(Ir),
    DuplicateRoot,
    InvalidClosure(String),
}

struct Invoker {
    mode: ReplyMode,
    calls: Mutex<Vec<String>>,
}

impl Invoker {
    fn new(mode: ReplyMode) -> Self {
        Self {
            mode,
            calls: Mutex::new(Vec::new()),
        }
    }

    fn calls(&self) -> Vec<String> {
        self.calls.lock().unwrap().clone()
    }
}

impl CompilerNativeInvoker for Invoker {
    fn invoke(&self, call: CompilerNativeCall<'_>) -> Result<Vec<u8>, CompilerNativeInvokerError> {
        let key = call.key().as_str().to_owned();
        self.calls.lock().unwrap().push(key.clone());
        if matches!(&self.mode, ReplyMode::Fail(failed) if failed == &key) {
            return Ok(
                serde_json::to_vec(&CompileReply::Fail(Box::new(CompileReplyFail {
                    envelope: 1,
                    message: Some("deliberate".to_owned()),
                })))
                .unwrap(),
            );
        }
        let mut payload = match &self.mode {
            ReplyMode::WrongCarrier(payload) => payload.clone(),
            _ => call.into_payload(),
        };
        if matches!(&self.mode, ReplyMode::InvalidClosure(failed) if failed == &key) {
            let Ir::ClosureArtifact(closure) = &mut payload else {
                panic!("the invalid-closure fixture must run at closure level")
            };
            let mode = match &closure.closure.qualification {
                QualificationState::Applied(applied) => applied.mode.clone(),
                QualificationState::Pending(_) => {
                    panic!("the fixture reaches the pass after qualify")
                }
            };
            closure.closure.qualification =
                QualificationState::Pending(Box::new(QualificationStatePending { mode }));
            closure.closure.absorption =
                AbsorptionState::Unplanned(Box::new(AbsorptionStateUnplanned {}));
            closure.closure.renames.clear();
        }
        let reply = CompileReply::Ok(Box::new(CompileReplyOk {
            envelope: 1,
            payload,
            message: None,
        }));
        let mut raw = serde_json::to_string(&reply).unwrap();
        if matches!(self.mode, ReplyMode::DuplicateRoot) {
            raw = raw.replacen("\"envelope\":1", "\"envelope\":1,\"envelope\":1", 1);
        }
        Ok(raw.into_bytes())
    }
}

#[derive(Default)]
struct Trace(Mutex<Vec<String>>);

impl CompileTraceSink for Trace {
    fn record(&self, event: &PassTraceEvent<'_>) {
        self.0.lock().unwrap().push(event.pass().to_owned());
    }
}

fn pass_key(id: &str) -> String {
    format!("__host__/demo#{id}")
}

#[test]
fn document_and_artifact_passes_keep_cardinality_and_declaration_order() {
    let plan = artifact_plan(vec![
        declaration("document", transform(ExtensionIrLevel::Document, "parse")),
        declaration(
            "closure-one",
            transform(ExtensionIrLevel::Closure, "qualify"),
        ),
        declaration(
            "closure-two",
            transform(ExtensionIrLevel::Closure, "qualify"),
        ),
    ]);
    let source = Source::default();
    let invoker = Invoker::new(ReplyMode::Echo);
    compile_artifact_passes_for_test(plan, &source, &invoker, None).unwrap();

    assert_eq!(source.reads(), 2);
    assert_eq!(
        invoker.calls(),
        [
            pass_key("document"),
            pass_key("document"),
            pass_key("closure-one"),
            pass_key("closure-two"),
        ]
    );
}

#[test]
fn before_after_replace_and_artifact_filter_form_one_valid_schedule() {
    let plan = artifact_plan(vec![
        declaration(
            "before",
            before_transform(ExtensionIrLevel::Closure, "qualify"),
        ),
        declaration("after", transform(ExtensionIrLevel::Closure, "qualify")),
        declaration("other-artifact", backend_for("json")),
    ]);
    assert_eq!(plan.passes().len(), 2, "the JSON backend is filtered out");
    let invoker = Invoker::new(ReplyMode::Echo);
    compile_artifact_passes_for_test(plan, &Source::default(), &invoker, None).unwrap();
    assert_eq!(invoker.calls(), [pass_key("before"), pass_key("after")]);

    let replacement = artifact_plan(vec![declaration("replacement", replacement("merge"))]);
    let invoker = Invoker::new(ReplyMode::Echo);
    BuiltinSchedule::emitted_with_passes_for_test(&replacement, &invoker).unwrap();
    assert!(
        invoker.calls().is_empty(),
        "resolution never invokes a pass"
    );
}

#[test]
fn unknown_anchor_and_wrong_placement_shape_refuse_before_read_or_invoke() {
    for pass in [
        transform(ExtensionIrLevel::Closure, "unknown"),
        transform(ExtensionIrLevel::Closure, "assemble"),
    ] {
        let plan = artifact_plan(vec![declaration("bad", pass)]);
        let source = Source::default();
        let invoker = Invoker::new(ReplyMode::Echo);
        let error = compile_artifact_passes_for_test(plan, &source, &invoker, None).unwrap_err();
        assert!(matches!(error, ArtifactCompileError::PassTier(_)));
        assert_eq!(source.reads(), 0);
        assert!(invoker.calls().is_empty());
    }
}

#[test]
fn strict_reply_and_wrong_carrier_keep_the_qualified_pass_name() {
    let wrong = wire::encode_generated(&AnyIr::Source(SourceIr::reached(
        DocumentAddress::Spec(address("spec://org.demo/other/common/doc#root")),
        SourceFormatId::new("markdown").unwrap(),
        "# Other {#root}\n",
    )))
    .unwrap();
    for mode in [ReplyMode::DuplicateRoot, ReplyMode::WrongCarrier(wrong)] {
        let plan = artifact_plan(vec![declaration(
            "document",
            transform(ExtensionIrLevel::Document, "parse"),
        )]);
        let source = Source::default();
        let invoker = Invoker::new(mode);
        let error = compile_artifact_passes_for_test(plan, &source, &invoker, None).unwrap_err();
        assert!(matches!(
            error,
            ArtifactCompileError::Pass { ref pass, .. }
                if pass == &format!("pass:{}", pass_key("document"))
        ));
    }
}

#[test]
fn failure_and_trace_use_pass_qualified_key_identity() {
    let plan = artifact_plan(vec![declaration(
        "closure",
        transform(ExtensionIrLevel::Closure, "qualify"),
    )]);
    let trace = Trace::default();
    let invoker = Invoker::new(ReplyMode::Echo);
    compile_artifact_native_traced(plan, &Source::default(), &invoker, &trace).unwrap();
    let name = format!("pass:{}", pass_key("closure"));
    assert!(trace.0.lock().unwrap().contains(&name));

    let plan = artifact_plan(vec![declaration(
        "closure",
        transform(ExtensionIrLevel::Closure, "qualify"),
    )]);
    let invoker = Invoker::new(ReplyMode::Fail(pass_key("closure")));
    let error =
        compile_artifact_passes_for_test(plan, &Source::default(), &invoker, None).unwrap_err();
    assert!(matches!(
        error,
        ArtifactCompileError::Pass { pass, reason }
            if pass == name && reason.contains("deliberate")
    ));
}

#[test]
fn production_executes_valid_positioned_passes_and_preflights_invalid_schedules() {
    let invalid = artifact_plan(vec![declaration(
        "unknown",
        transform(ExtensionIrLevel::Closure, "unknown"),
    )]);
    let source = Source::default();
    let invoker = Invoker::new(ReplyMode::Echo);
    let error = compile_artifact_native(invalid, &source, &invoker).unwrap_err();
    assert!(matches!(
        error,
        ArtifactCompileError::PassTier(ref public)
            if matches!(public.inner(), PassTierExecutionError::Schedule { .. })
    ));
    assert_eq!(source.reads(), 0);
    assert!(invoker.calls().is_empty());

    let plan = artifact_plan(vec![declaration(
        "closure",
        transform(ExtensionIrLevel::Closure, "qualify"),
    )]);
    let source = Source::default();
    let invoker = Invoker::new(ReplyMode::Echo);
    compile_artifact_native(plan, &source, &invoker).unwrap();
    assert_eq!(source.reads(), 2);
    assert_eq!(invoker.calls(), [pass_key("closure")]);
}

#[test]
fn missing_invoker_and_unsupported_implementation_refuse_before_source() {
    let plan = artifact_plan(vec![declaration(
        "missing",
        transform(ExtensionIrLevel::Closure, "qualify"),
    )]);
    let source = Source::default();
    let error = compile_artifact(plan, &source).unwrap_err();
    assert!(matches!(
        error,
        ArtifactCompileError::PassTier(ref public)
            if matches!(public.inner(), PassTierExecutionError::MissingInvoker { key } if key.as_str() == pass_key("missing"))
    ));
    assert_eq!(source.reads(), 0);

    let mut builtin = declaration("builtin", transform(ExtensionIrLevel::Closure, "qualify"));
    builtin.handler = ExtensionHandler::Builtin {
        name: "unsupported-pass".into(),
    };
    let source = Source::default();
    let invoker = Invoker::new(ReplyMode::Echo);
    let error =
        compile_artifact_native(artifact_plan(vec![builtin]), &source, &invoker).unwrap_err();
    assert!(matches!(
        error,
        ArtifactCompileError::PassTier(ref public)
            if matches!(public.inner(), PassTierExecutionError::NativeExecution { key, .. } if key.as_str() == pass_key("builtin"))
    ));
    assert_eq!(source.reads(), 0);
    assert!(invoker.calls().is_empty());
}

#[test]
fn unused_frontends_do_not_affect_builtin_sources() {
    let invoker = Invoker::new(ReplyMode::Echo);
    let source = Source::default();
    compile_artifact_native(
        artifact_plan(vec![declaration("frontend", frontend_for("custom-markup"))]),
        &source,
        &invoker,
    )
    .unwrap();
    assert_eq!(source.reads(), 2);
    assert!(invoker.calls().is_empty());
}

#[test]
fn pass_tier_verifier_cannot_be_disabled_and_stops_before_the_next_pass() {
    let plan = artifact_plan(vec![declaration(
        "first",
        transform(ExtensionIrLevel::Closure, "qualify"),
    )]);
    let invoker = Invoker::new(ReplyMode::Echo);
    let schedule = without_verify_each(|| {
        BuiltinSchedule::emitted_with_passes_for_test(&plan, &invoker).unwrap()
    });
    assert!(schedule.pass_tier_verifier_enabled_for_test());

    let plan = artifact_plan(vec![
        declaration("first", transform(ExtensionIrLevel::Closure, "qualify")),
        declaration("second", transform(ExtensionIrLevel::Closure, "qualify")),
    ]);
    let invoker = Invoker::new(ReplyMode::InvalidClosure(pass_key("first")));
    let error = without_verify_each(|| {
        compile_artifact_passes_for_test(plan, &Source::default(), &invoker, None)
    })
    .unwrap_err();
    let first = format!("pass:{}", pass_key("first"));
    assert!(
        matches!(
        error,
        ArtifactCompileError::Pass { ref pass, ref reason }
            if pass == &first && reason.contains("inter-pass verification rejected the output")
        ),
        "{error:?}"
    );
    assert_eq!(invoker.calls(), [pass_key("first")]);
}

#[test]
fn empty_plan_installs_no_pass_tier_verifier() {
    let plan = artifact_plan(Vec::new());
    let invoker = Invoker::new(ReplyMode::Echo);
    let schedule = without_verify_each(|| {
        BuiltinSchedule::emitted_with_passes_for_test(&plan, &invoker).unwrap()
    });
    assert!(!schedule.pass_tier_verifier_enabled_for_test());
    assert!(invoker.calls().is_empty());
}
