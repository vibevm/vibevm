//! R6.3-D frontend/backend catalog acceptance.

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use vibe_core::lifecycle::ExtensionPoint;
use vibe_core::manifest::{
    ExtensionDecl, ExtensionHandler, ExtensionPass, ExtensionPassKind, ExtensionsControl,
};
use vibe_extension_registry::{
    ExtensionRegistry, ExtensionWorld, HostExtensionSource, HostIdentity, HostProvider,
    collect_extensions,
};

use super::catalog::{PassCatalogError, PassCatalogs};
use super::lowering::lower_effective_compile_rows;
use super::plan::PassPlan;
use crate::compiler::builtin::{ArtifactCompileError, compile_artifact_native};
use crate::compiler::ir::{
    ArtifactInput, ArtifactPlan, ArtifactTarget, DocumentIr, LaneIr, SourceIr,
};
use crate::compiler::pass::IrPayload;
use crate::compiler::transform::native_manager::{
    CompilerNativeCall, CompilerNativeInvoker, CompilerNativeInvokerError,
};
use crate::{SectionSource, SpecAddress};

fn frontend(id: &str, formats: &[&str]) -> ExtensionDecl {
    declaration(
        id,
        ExtensionPass {
            kind: ExtensionPassKind::Frontend,
            level: None,
            from: None,
            to: None,
            after: None,
            before: None,
            replace: None,
            formats: Some(formats.iter().map(|value| (*value).to_owned()).collect()),
            artifact: None,
        },
    )
}

fn backend(id: &str, artifact: &str) -> ExtensionDecl {
    declaration(
        id,
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
        },
    )
}

fn replacement(id: &str, artifact: &str) -> ExtensionDecl {
    declaration(
        id,
        ExtensionPass {
            kind: ExtensionPassKind::Lowering,
            level: None,
            from: None,
            to: None,
            after: None,
            before: None,
            replace: Some(format!("emit:{artifact}")),
            formats: None,
            artifact: Some(artifact.to_owned()),
        },
    )
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

fn pass_plan(declarations: Vec<ExtensionDecl>) -> PassPlan {
    let registry = registry(declarations);
    lower_effective_compile_rows(&registry.enabled_compile_rows())
        .unwrap()
        .passes()
        .clone()
}

fn key(id: &str) -> String {
    format!("__host__/demo#{id}")
}

#[test]
fn frontend_formats_map_to_the_parse_shape_and_qualified_identity() {
    let catalogs =
        PassCatalogs::resolve(&pass_plan(vec![frontend("docs", &["adoc", "rst"])])).unwrap();
    assert_eq!(catalogs.frontends().len(), 2);
    for format in ["adoc", "rst"] {
        let pass = catalogs.frontends().get(format).unwrap();
        assert_eq!(pass.key().as_str(), key("docs"));
        assert_eq!(pass.ordinal(), 0);
        assert_eq!(pass.entry().declaration().kind, ExtensionPassKind::Frontend);
        assert_eq!(
            pass.descriptor().name.as_str(),
            format!("pass:{}", key("docs"))
        );
        assert_eq!(pass.descriptor().input, SourceIr::SHAPE);
        assert_eq!(pass.descriptor().output, DocumentIr::SHAPE);
    }
    assert!(catalogs.positioned().is_empty());
}

#[test]
fn frontend_duplicates_and_builtin_markdown_xml_collisions_refuse() {
    let duplicate = PassCatalogs::resolve(&pass_plan(vec![
        frontend("one", &["adoc"]),
        frontend("two", &["adoc"]),
    ]))
    .unwrap_err();
    assert!(matches!(
        duplicate.inner(),
        PassCatalogError::DuplicateFormat { format, first, second }
            if format == "adoc" && first.as_str() == key("one") && second.as_str() == key("two")
    ));
    for format in ["markdown", "xml"] {
        let error =
            PassCatalogs::resolve(&pass_plan(vec![frontend("collision", &[format])])).unwrap_err();
        assert!(matches!(
            error.inner(),
            PassCatalogError::BuiltinFormat { format: actual, .. } if actual == format
        ));
    }
}

#[test]
fn backend_ids_map_to_lane_emitted_and_reject_duplicates_and_builtins() {
    let catalogs = PassCatalogs::resolve(&pass_plan(vec![backend("json", "lane-json")])).unwrap();
    let pass = catalogs.backends().get("lane-json").unwrap();
    assert_eq!(catalogs.backends().len(), 1);
    assert_eq!(
        pass.descriptor().name.as_str(),
        format!("pass:{}", key("json"))
    );
    assert_eq!(pass.descriptor().input, LaneIr::SHAPE);
    assert_eq!(
        pass.descriptor().output,
        crate::compiler::ir::EmittedIr::SHAPE
    );

    let duplicate = PassCatalogs::resolve(&pass_plan(vec![
        backend("one", "lane-json"),
        backend("two", "lane-json"),
    ]))
    .unwrap_err();
    assert!(matches!(
        duplicate.inner(),
        PassCatalogError::DuplicateBackend { .. }
    ));
    for builtin in ["static-md", "static-xml", "index", "inline"] {
        let error =
            PassCatalogs::resolve(&pass_plan(vec![backend("collision", builtin)])).unwrap_err();
        assert!(matches!(
            error.inner(),
            PassCatalogError::BuiltinBackend { backend, .. } if backend == builtin
        ));
    }
}

#[test]
fn explicit_lowering_replacement_is_the_only_builtin_displacement_path() {
    let catalogs =
        PassCatalogs::resolve(&pass_plan(vec![replacement("xml", "static-xml")])).unwrap();
    assert_eq!(catalogs.frontends().len(), 0);
    assert_eq!(catalogs.backends().len(), 0);
    assert_eq!(catalogs.positioned().len(), 1);
}

#[derive(Default)]
struct Guard {
    reads: AtomicUsize,
    calls: AtomicUsize,
}

impl SectionSource for Guard {
    fn section_text(&self, _address: &SpecAddress) -> Result<String, String> {
        self.reads.fetch_add(1, Ordering::SeqCst);
        Ok("# Root {#root}\n".to_owned())
    }
}

impl CompilerNativeInvoker for Guard {
    fn invoke(&self, _call: CompilerNativeCall<'_>) -> Result<Vec<u8>, CompilerNativeInvokerError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        panic!("catalog preflight must precede invocation")
    }
}

fn artifact(declarations: Vec<ExtensionDecl>) -> ArtifactPlan {
    let passes = pass_plan(declarations);
    ArtifactPlan::static_lane(
        ArtifactTarget::StaticXml,
        "vibevm/vibespecs/boot/STATIC.xml",
        "vibevm/vibespecs",
        vec![
            ArtifactInput::normal(
                "org.demo/pkg",
                "boot/doc.md",
                SpecAddress::parse("spec://org.demo/pkg/boot/doc#root").unwrap(),
            )
            .unwrap(),
        ],
    )
    .unwrap()
    .with_passes(passes)
}

#[test]
fn catalog_validation_precedes_reads_and_unused_frontends_leave_markdown_unchanged() {
    let guard = Guard::default();
    let error = compile_artifact_native(
        artifact(vec![frontend("one", &["adoc"]), frontend("two", &["adoc"])]),
        &guard,
        &guard,
    )
    .unwrap_err();
    assert!(matches!(error, ArtifactCompileError::PassCatalog(_)));
    assert_eq!(guard.reads.load(Ordering::SeqCst), 0);
    assert_eq!(guard.calls.load(Ordering::SeqCst), 0);

    compile_artifact_native(artifact(vec![frontend("docs", &["adoc"])]), &guard, &guard)
        .expect("an unused frontend does not affect Markdown sources");
    assert_ne!(guard.reads.load(Ordering::SeqCst), 0);
    assert_eq!(guard.calls.load(Ordering::SeqCst), 0);
}
