//! R6.5-C2 native backend execution and manager-owned provenance acceptance.

use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use vibe_core::lifecycle::{CompilePoint, ExtensionPoint};
use vibe_core::manifest::{
    ExtensionDecl, ExtensionHandler, ExtensionPass, ExtensionPassKind, ExtensionsControl,
};
use vibe_extension_registry::{
    ExtensionWorld, HostExtensionSource, HostIdentity, HostProvider, collect_extensions,
};
use vibe_wire::behaviour::native_backend::{REPLY_CAP_BYTES, ok_reply};
use vibe_wire::generated::native::e1::compile_reply::{CompileReply, CompileReplyOk};

use super::lowering::lower_effective_compile_rows;
use crate::compiler::builtin::{
    ArtifactCompileError, compile_artifact, compile_artifact_native, compile_artifact_native_traced,
};
use crate::compiler::ir::{ArtifactInput, ArtifactPlan};
use crate::compiler::trace::{CompileTraceSink, PassTraceEvent};
use crate::compiler::transform::native_manager::{
    CompilerNativeCall, CompilerNativeInvoker, CompilerNativeInvokerError,
};
use crate::{SectionSource, SpecAddress};

const SELECTED: &str = "opaque-bin";

fn pass(kind: ExtensionPassKind, artifact: Option<&str>) -> ExtensionPass {
    ExtensionPass {
        kind,
        level: None,
        from: None,
        to: None,
        after: None,
        before: None,
        replace: None,
        formats: None,
        artifact: artifact.map(str::to_owned),
    }
}

fn declaration(id: &str, point: CompilePoint, pass: Option<ExtensionPass>) -> ExtensionDecl {
    ExtensionDecl {
        id: id.to_owned(),
        point: ExtensionPoint::Compile(point),
        handler: ExtensionHandler::Native {
            crate_dir: Some(PathBuf::from(format!("passes/{id}"))),
            prebuilt: None,
        },
        config: None,
        auto: None,
        inputs: None,
        applies_to: None,
        compiler_internals: (point == CompilePoint::Pass).then_some(true),
        pass,
        when: None,
    }
}

fn plans(declarations: Vec<ExtensionDecl>) -> super::lowering::CompilePlans {
    let registry = collect_extensions(ExtensionWorld {
        installed: Vec::new(),
        host: HostExtensionSource {
            provider: HostProvider {
                identity: HostIdentity::ungrouped_project("demo"),
                root: PathBuf::from("."),
                version: "1.0.0".into(),
                kind: None,
                content_hash: None,
            },
            declarations,
            controls: ExtensionsControl::default(),
            mechanisms: Vec::new(),
        },
        effective_stack: None,
    })
    .unwrap();
    lower_effective_compile_rows(&registry.enabled_compile_rows()).unwrap()
}

fn artifact(declarations: Vec<ExtensionDecl>, target: &'static str) -> ArtifactPlan {
    plans(declarations).attach_to(
        ArtifactPlan::custom_for_test(
            target,
            vec![
                ArtifactInput::normal(
                    "org.demo/root",
                    "boot/root.md",
                    SpecAddress::parse("spec://org.demo/root/boot/root#root").unwrap(),
                )
                .unwrap(),
            ],
        )
        .unwrap(),
    )
}

#[derive(Default)]
struct Source {
    reads: AtomicUsize,
    events: Mutex<Vec<String>>,
}

impl SectionSource for Source {
    fn section_text(&self, _address: &SpecAddress) -> Result<String, String> {
        self.reads.fetch_add(1, Ordering::SeqCst);
        self.events.lock().unwrap().push("read".into());
        Ok("# Root {#root}\n".into())
    }
}

enum Reply {
    Bytes(Vec<u8>),
    Wrong,
    Oversize,
}

struct Invoker<'a> {
    source: &'a Source,
    reply: Reply,
    calls: Mutex<Vec<String>>,
}

impl CompilerNativeInvoker for Invoker<'_> {
    fn admit_backend(
        &self,
        key: &vibe_core::manifest::ExtensionKey,
        _order: u32,
        _config: &std::collections::BTreeMap<String, Option<serde_json::Value>>,
        _implementation: crate::CompilerNativeImplementationDigest,
        backend: &str,
    ) -> Result<(), CompilerNativeInvokerError> {
        self.source
            .events
            .lock()
            .unwrap()
            .push(format!("admit:{backend}:{}", key.as_str()));
        Ok(())
    }

    fn invoke(&self, call: CompilerNativeCall<'_>) -> Result<Vec<u8>, CompilerNativeInvokerError> {
        self.calls.lock().unwrap().push(format!(
            "{}:{}",
            call.backend().unwrap_or("transform"),
            call.key().as_str()
        ));
        if call.backend().is_none() {
            let mut returned = crate::compiler::wire::decode_generated(call.payload()).unwrap();
            let crate::compiler::pass::AnyIr::Emitted(emitted) = &mut returned else {
                panic!("the emitted transform receives Emitted IR")
            };
            emitted.bytes.push(b'!');
            emitted.provenance.bytes_digest =
                crate::compiler::emit::emitted_bytes_digest(&emitted.bytes);
            return Ok(
                serde_json::to_vec(&CompileReply::Ok(Box::new(CompileReplyOk {
                    envelope: 1,
                    payload: crate::compiler::wire::encode_generated(&returned).unwrap(),
                    message: None,
                })))
                .unwrap(),
            );
        }
        Ok(match &self.reply {
            Reply::Bytes(bytes) => serde_json::to_vec(&ok_reply(bytes, None).unwrap()).unwrap(),
            Reply::Wrong => br#"{"status":"skip","envelope":1}"#.to_vec(),
            Reply::Oversize => vec![b'x'; REPLY_CAP_BYTES + 1],
        })
    }
}

#[derive(Default)]
struct Trace(Mutex<Vec<(String, bool)>>);

impl CompileTraceSink for Trace {
    fn record(&self, event: &PassTraceEvent<'_>) {
        self.0
            .lock()
            .unwrap()
            .push((event.pass().to_owned(), event.verify_duration().is_some()));
    }
}

fn backend(id: &str, artifact: &str) -> ExtensionDecl {
    declaration(
        id,
        CompilePoint::Pass,
        Some(pass(ExtensionPassKind::Backend, Some(artifact))),
    )
}

fn emitted_transform(id: &str) -> ExtensionDecl {
    declaration(id, CompilePoint::Emitted, None)
}

#[test]
fn selected_backend_is_admitted_before_read_and_reconstructs_exact_provenance() {
    let source = Source::default();
    let bytes = vec![0, 255, b'{', b'\n'];
    let invoker = Invoker {
        source: &source,
        reply: Reply::Bytes(bytes.clone()),
        calls: Mutex::new(Vec::new()),
    };
    let trace = Trace::default();
    let artifact = compile_artifact_native_traced(
        artifact(
            vec![backend("other", "other-bin"), backend("selected", SELECTED)],
            SELECTED,
        ),
        &source,
        &invoker,
        &trace,
    )
    .unwrap();

    assert_eq!(artifact.bytes(), bytes);
    assert_eq!(artifact.provenance().backend_id(), SELECTED);
    assert_eq!(
        artifact.provenance().producer(),
        "pass:__host__/demo#selected"
    );
    assert_eq!(
        source.events.lock().unwrap().as_slice(),
        ["admit:opaque-bin:__host__/demo#selected", "read"]
    );
    assert_eq!(
        invoker.calls.lock().unwrap().as_slice(),
        ["opaque-bin:__host__/demo#selected"]
    );
    let trace = trace.0.lock().unwrap();
    let backend = trace
        .iter()
        .find(|(pass, _)| pass == "pass:__host__/demo#selected")
        .unwrap();
    assert!(backend.1, "backend output is verified before any next pass");
}

#[test]
fn verified_backend_precedes_the_normal_emitted_transform_schedule() {
    let source = Source::default();
    let invoker = Invoker {
        source: &source,
        reply: Reply::Bytes(vec![0, 255, b'x']),
        calls: Mutex::new(Vec::new()),
    };
    let trace = Trace::default();
    let artifact = compile_artifact_native_traced(
        artifact(
            vec![backend("selected", SELECTED), emitted_transform("post")],
            SELECTED,
        ),
        &source,
        &invoker,
        &trace,
    )
    .unwrap();

    assert_eq!(artifact.bytes(), [0, 255, b'x', b'!']);
    assert_eq!(artifact.provenance().backend_id(), SELECTED);
    assert_eq!(
        artifact
            .provenance()
            .emitted_transforms
            .iter()
            .map(|name| name.as_str())
            .collect::<Vec<_>>(),
        ["transform:emitted:__host__/demo#post"]
    );
    assert_eq!(
        invoker.calls.lock().unwrap().as_slice(),
        [
            "opaque-bin:__host__/demo#selected",
            "transform:__host__/demo#post"
        ]
    );
    let trace = trace.0.lock().unwrap();
    let backend = trace
        .iter()
        .position(|(pass, verified)| pass == "pass:__host__/demo#selected" && *verified)
        .unwrap();
    let post = trace
        .iter()
        .position(|(pass, _)| pass == "transform:emitted:__host__/demo#post")
        .unwrap();
    assert!(backend < post);
}

#[test]
fn absent_invoker_and_wrong_or_oversize_replies_are_qualified_refusals() {
    let source = Source::default();
    let error = compile_artifact(
        artifact(vec![backend("selected", SELECTED)], SELECTED),
        &source,
    )
    .unwrap_err();
    assert!(
        matches!(error, ArtifactCompileError::Backend { ref pass, .. }
        if pass == "pass:__host__/demo#selected")
    );
    assert_eq!(source.reads.load(Ordering::SeqCst), 0);

    for reply in [Reply::Wrong, Reply::Oversize] {
        let source = Source::default();
        let invoker = Invoker {
            source: &source,
            reply,
            calls: Mutex::new(Vec::new()),
        };
        let error = compile_artifact_native(
            artifact(vec![backend("selected", SELECTED)], SELECTED),
            &source,
            &invoker,
        )
        .unwrap_err();
        assert!(matches!(error, ArtifactCompileError::Pass { ref pass, .. }
            if pass == "pass:__host__/demo#selected"));
        assert_eq!(invoker.calls.lock().unwrap().len(), 1);
    }
}

#[test]
fn backend_catalog_for_another_target_is_zero_call_on_builtin_emission() {
    let source = Source::default();
    let invoker = Invoker {
        source: &source,
        reply: Reply::Wrong,
        calls: Mutex::new(Vec::new()),
    };
    let plans = plans(vec![backend("other", "other-bin")]);
    let plan = plans.attach_to(
        ArtifactPlan::static_lane(
            crate::compiler::ir::ArtifactTarget::StaticMarkdown,
            "STATIC.md",
            "deps",
            vec![
                ArtifactInput::normal(
                    "org.demo/root",
                    "boot/root.md",
                    SpecAddress::parse("spec://org.demo/root/boot/root#root").unwrap(),
                )
                .unwrap(),
            ],
        )
        .unwrap(),
    );
    compile_artifact_native(plan, &source, &invoker).unwrap();
    assert!(invoker.calls.lock().unwrap().is_empty());
    assert!(
        source
            .events
            .lock()
            .unwrap()
            .iter()
            .all(|event| event == "read")
    );
}
