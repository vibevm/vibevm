use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use serde_json::Value;
use vibe_core::lifecycle::ExtensionPoint;
use vibe_core::manifest::{
    ExtensionDecl, ExtensionHandler, ExtensionPass, ExtensionPassKind, ExtensionsControl,
};
use vibe_extension_registry::{
    ExtensionWorld, HostExtensionSource, HostIdentity, HostProvider, collect_extensions,
};
use vibe_wire::generated::native::e1::compile_reply::{CompileReply, CompileReplyOk};

use super::lowering::lower_effective_compile_rows;
use crate::compiler::builtin::{
    ArtifactCompileError, compile_artifact_native, compile_artifact_native_traced,
    without_verify_each,
};
use crate::compiler::ir::{
    ArtifactInput, ArtifactPlan, ArtifactTarget, DocumentAddress, DocumentIr, SourceIr,
};
use crate::compiler::pass::AnyIr;
use crate::compiler::trace::{CompileTraceSink, PassTraceEvent};
use crate::compiler::transform::native_identity::CompilerNativeImplementationDigest;
use crate::compiler::transform::native_manager::{
    CompilerNativeCall, CompilerNativeInvoker, CompilerNativeInvokerError,
    CompilerNativeInvokerErrorKind,
};
use crate::compiler::wire;
use crate::embed::ResolvedSource;
use crate::{DocTree, SectionSource, SpecAddress};

fn frontend() -> ExtensionDecl {
    declaration(
        "txt",
        ExtensionPass {
            kind: ExtensionPassKind::Frontend,
            level: None,
            from: None,
            to: None,
            after: None,
            before: None,
            replace: None,
            formats: Some(vec!["txt".to_owned()]),
            artifact: None,
        },
    )
}

fn document_transform() -> ExtensionDecl {
    declaration(
        "after",
        ExtensionPass {
            kind: ExtensionPassKind::Transform,
            level: Some(vibe_core::manifest::ExtensionIrLevel::Document),
            from: None,
            to: None,
            after: Some("parse".to_owned()),
            before: None,
            replace: None,
            formats: None,
            artifact: None,
        },
    )
}

fn declaration(id: &str, pass: ExtensionPass) -> ExtensionDecl {
    ExtensionDecl {
        id: id.to_owned(),
        point: "compile:pass".parse::<ExtensionPoint>().unwrap(),
        handler: ExtensionHandler::Native {
            crate_dir: Some(PathBuf::from(format!("native/{id}"))),
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

fn pass_key(id: &str) -> String {
    format!("__host__/demo#{id}")
}

fn address(name: &str) -> SpecAddress {
    SpecAddress::parse(&format!("spec://org.demo/{name}/common/NOTE#root")).unwrap()
}

fn plan(inputs: &[&str], after: bool) -> ArtifactPlan {
    let mut declarations = vec![frontend()];
    if after {
        declarations.push(document_transform());
    }
    let registry = collect_extensions(ExtensionWorld {
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
    .unwrap();
    let passes = lower_effective_compile_rows(&registry.enabled_compile_rows())
        .unwrap()
        .passes()
        .clone();
    ArtifactPlan::static_lane(
        ArtifactTarget::StaticMarkdown,
        "vibevm/vibespecs/boot/STATIC.md",
        "vibevm/vibedeps",
        inputs
            .iter()
            .map(|name| {
                ArtifactInput::normal(format!("org.demo/{name}"), "common/NOTE.txt", address(name))
                    .unwrap()
            })
            .collect(),
    )
    .unwrap()
    .with_passes(passes)
}

#[derive(Default)]
struct Source {
    custom_reads: AtomicUsize,
}

impl SectionSource for Source {
    fn section_text(&self, _address: &SpecAddress) -> Result<String, String> {
        Ok("# Markdown {#root}\n".to_owned())
    }

    fn source_metadata(
        &self,
        address: &SpecAddress,
        _formats: &[String],
    ) -> Result<crate::ResolvedSourceMetadata, String> {
        if address.to_string().contains("/markdown/") {
            return crate::ResolvedSourceMetadata::new("NOTE.md", "markdown".into(), "NOTE".into());
        }
        crate::ResolvedSourceMetadata::new("common/NOTE.txt", "txt".into(), "NOTE".into())
    }

    fn read_resolved_source(
        &self,
        _address: &SpecAddress,
        metadata: crate::ResolvedSourceMetadata,
    ) -> Result<ResolvedSource, String> {
        if metadata.format() == "markdown" {
            return Ok(ResolvedSource::markdown(
                "# Markdown {#root}\n".into(),
                "NOTE".into(),
            ));
        }
        self.custom_reads.fetch_add(1, Ordering::SeqCst);
        ResolvedSource::custom("first\nsecond\n".into(), "txt".into(), "NOTE".into())
    }
}

#[derive(Clone, Copy)]
enum Mode {
    Valid,
    InvalidDocument,
    Admission(CompilerNativeInvokerErrorKind),
}

struct Invoker {
    mode: Mode,
    admissions: Mutex<Vec<String>>,
    calls: Mutex<Vec<String>>,
    stems: Mutex<Vec<Option<String>>>,
}

impl Invoker {
    fn new(mode: Mode) -> Self {
        Self {
            mode,
            admissions: Mutex::new(Vec::new()),
            calls: Mutex::new(Vec::new()),
            stems: Mutex::new(Vec::new()),
        }
    }
}

impl CompilerNativeInvoker for Invoker {
    fn admit_frontend(
        &self,
        key: &vibe_core::manifest::ExtensionKey,
        _order: u32,
        _config: &BTreeMap<String, Option<Value>>,
        _implementation: CompilerNativeImplementationDigest,
    ) -> Result<(), CompilerNativeInvokerError> {
        self.admissions.lock().unwrap().push(key.to_string());
        match self.mode {
            Mode::Admission(kind) => Err(CompilerNativeInvokerError::new(kind, "not ready")),
            Mode::Valid | Mode::InvalidDocument => Ok(()),
        }
    }

    fn invoke(&self, call: CompilerNativeCall<'_>) -> Result<Vec<u8>, CompilerNativeInvokerError> {
        let key = call.key().to_string();
        self.calls.lock().unwrap().push(key.clone());
        let frontend_stem = call.frontend_physical_stem().map(str::to_owned);
        self.stems.lock().unwrap().push(frontend_stem.clone());
        let input = wire::decode_generated(&call.into_payload()).unwrap();
        let output = match input {
            AnyIr::Source(source) => {
                let source = if matches!(self.mode, Mode::InvalidDocument) {
                    let (_, format, _, text) = source.into_parts();
                    SourceIr::reached(DocumentAddress::Spec(address("retargeted")), format, text)
                } else {
                    source
                };
                let tree = frontend_stem.map_or_else(
                    || DocTree::parse(source.text()),
                    |stem| DocTree::parse(&format!("# {stem} {{#root}}\n\n{}", source.text())),
                );
                AnyIr::Document(DocumentIr::new(source, tree))
            }
            AnyIr::Document(document) => AnyIr::Document(document),
            other => other,
        };
        let payload = wire::encode_generated(&output).unwrap();
        Ok(
            serde_json::to_vec(&CompileReply::Ok(Box::new(CompileReplyOk {
                envelope: 1,
                payload,
                message: None,
            })))
            .unwrap(),
        )
    }
}

#[derive(Default)]
struct Trace(Mutex<Vec<String>>);

impl CompileTraceSink for Trace {
    fn record(&self, event: &PassTraceEvent<'_>) {
        self.0.lock().unwrap().push(event.pass().to_owned());
    }
}

#[test]
fn selected_frontend_admits_once_then_invokes_once_per_custom_document_in_mixed_order() {
    let source = Source::default();
    let invoker = Invoker::new(Mode::Valid);
    let trace = Trace::default();
    compile_artifact_native_traced(
        plan(&["markdown", "txt", "txt-two"], false),
        &source,
        &invoker,
        &trace,
    )
    .unwrap();

    assert_eq!(*invoker.admissions.lock().unwrap(), [pass_key("txt")]);
    assert_eq!(
        *invoker.calls.lock().unwrap(),
        [pass_key("txt"), pass_key("txt")]
    );
    assert_eq!(source.custom_reads.load(Ordering::SeqCst), 2);
    assert_eq!(
        *invoker.stems.lock().unwrap(),
        [Some("NOTE".to_owned()), Some("NOTE".to_owned())]
    );
    let document_trace = trace
        .0
        .lock()
        .unwrap()
        .iter()
        .filter(|name| name.as_str() == "parse" || name.starts_with("pass:"))
        .cloned()
        .collect::<Vec<_>>();
    assert_eq!(
        document_trace,
        [
            "parse".to_owned(),
            format!("pass:{}", pass_key("txt")),
            format!("pass:{}", pass_key("txt")),
        ]
    );
}

#[test]
fn missing_or_source_built_frontend_refuses_before_custom_read_or_invoke() {
    for kind in [
        CompilerNativeInvokerErrorKind::BuildableSourceUnavailable,
        CompilerNativeInvokerErrorKind::InvocationFailed,
    ] {
        let source = Source::default();
        let invoker = Invoker::new(Mode::Admission(kind));
        let error = compile_artifact_native(plan(&["txt"], false), &source, &invoker).unwrap_err();
        assert!(matches!(
            error,
            ArtifactCompileError::Pass { pass, .. } if pass == format!("pass:{}", pass_key("txt"))
        ));
        assert_eq!(source.custom_reads.load(Ordering::SeqCst), 0);
        assert!(invoker.calls.lock().unwrap().is_empty());
    }
}

#[derive(Default)]
struct UnselectedSource(AtomicUsize);

impl SectionSource for UnselectedSource {
    fn section_text(&self, _address: &SpecAddress) -> Result<String, String> {
        panic!("custom source cannot use the Markdown reader")
    }

    fn source_metadata(
        &self,
        _address: &SpecAddress,
        _formats: &[String],
    ) -> Result<crate::ResolvedSourceMetadata, String> {
        crate::ResolvedSourceMetadata::new("NOTE.ghost", "ghost".into(), "NOTE".into())
    }

    fn read_resolved_source(
        &self,
        _address: &SpecAddress,
        _metadata: crate::ResolvedSourceMetadata,
    ) -> Result<ResolvedSource, String> {
        self.0.fetch_add(1, Ordering::SeqCst);
        unreachable!("an unselected custom format refuses before read")
    }
}

#[test]
fn unselected_custom_format_refuses_before_alternate_source_read() {
    let source = UnselectedSource::default();
    let invoker = Invoker::new(Mode::Valid);
    let error = compile_artifact_native(plan(&["txt"], false), &source, &invoker).unwrap_err();
    assert!(matches!(
        error,
        ArtifactCompileError::Manager { reason }
            if reason.contains("custom source format `ghost` has no selected frontend seat")
    ));
    assert_eq!(source.0.load(Ordering::SeqCst), 0);
    assert!(invoker.admissions.lock().unwrap().is_empty());
    assert!(invoker.calls.lock().unwrap().is_empty());
}

#[test]
fn invalid_frontend_output_stops_document_pass_and_next_document() {
    let source = Source::default();
    let invoker = Invoker::new(Mode::InvalidDocument);
    let error = without_verify_each(|| {
        compile_artifact_native(plan(&["txt", "txt-two"], true), &source, &invoker)
    })
    .unwrap_err();
    assert!(
        matches!(
            &error,
            ArtifactCompileError::Pass { pass, reason }
                if pass == &format!("pass:{}", pass_key("txt"))
                    && reason.contains("inter-pass verification")
        ),
        "{error:?}"
    );
    assert_eq!(source.custom_reads.load(Ordering::SeqCst), 1);
    assert_eq!(*invoker.calls.lock().unwrap(), [pass_key("txt")]);
}
