use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use vibe_core::lifecycle::ExtensionPoint;
use vibe_core::manifest::{
    ExtensionDecl, ExtensionHandler, ExtensionKey, ExtensionPass, ExtensionPassKind, ExtensionUse,
    ExtensionsControl,
};
use vibe_core::{ContentHash, Group, PackageKind, PackageName};
use vibe_extension_registry::{
    DependencyExtensionSource, DependencyProvider, DependencyProviderId, ExtensionWorld,
    HostExtensionSource, HostIdentity, HostProvider, collect_extensions,
};

use super::catalog::PassCatalogs;
use super::lowering::lower_effective_compile_rows;
use crate::compiler::ir::StaticCompileMode;
use crate::compiler::worklist::discover_with_formats;
use crate::{FileResolver, FsSectionSource, SectionSource, SelfCoordinate, SpecAddress};

fn frontend() -> ExtensionDecl {
    ExtensionDecl {
        id: "txt".to_owned(),
        point: "compile:pass".parse::<ExtensionPoint>().unwrap(),
        handler: ExtensionHandler::Native {
            crate_dir: Some(PathBuf::from("native")),
            prebuilt: None,
        },
        config: None,
        auto: None,
        inputs: None,
        applies_to: None,
        compiler_internals: Some(true),
        pass: Some(ExtensionPass {
            kind: ExtensionPassKind::Frontend,
            level: None,
            from: None,
            to: None,
            after: None,
            before: None,
            replace: None,
            formats: Some(vec!["txt".to_owned()]),
            artifact: None,
        }),
        when: None,
    }
}

fn plans(active: bool, disabled: bool) -> super::lowering::CompilePlans {
    let provider = DependencyProvider {
        id: DependencyProviderId::new(
            Group::parse("org.demo").unwrap(),
            PackageName::parse("formats").unwrap(),
        ),
        root: PathBuf::from("vibedeps/formats"),
        version: "1.0.0".into(),
        kind: PackageKind::Tool,
        content_hash: ContentHash::parse("sha256:aa").unwrap(),
    };
    let key = ExtensionKey::authored("org.demo/formats#txt");
    let registry = collect_extensions(ExtensionWorld {
        installed: vec![DependencyExtensionSource {
            provider,
            declarations: vec![frontend()],
            controls: ExtensionsControl::default(),
            mechanisms: Vec::new(),
        }],
        host: HostExtensionSource {
            provider: HostProvider {
                identity: HostIdentity::ungrouped_project("host"),
                root: PathBuf::from("."),
                version: "0.1.0".into(),
                kind: None,
                content_hash: None,
            },
            declarations: Vec::new(),
            controls: ExtensionsControl {
                uses: active
                    .then(|| ExtensionUse {
                        reference: key.clone(),
                        config: None,
                    })
                    .into_iter()
                    .collect(),
                disable: disabled.then_some(key).into_iter().collect(),
            },
            mechanisms: Vec::new(),
        },
        effective_stack: None,
    })
    .unwrap();
    lower_effective_compile_rows(&registry.enabled_compile_rows()).unwrap()
}

fn formats(active: bool, disabled: bool) -> Vec<String> {
    PassCatalogs::resolve(plans(active, disabled).passes())
        .unwrap()
        .frontend_formats()
}

fn address() -> SpecAddress {
    SpecAddress::parse("spec://org.demo/host/common/NOTE#root").unwrap()
}

fn resolver(root: &std::path::Path) -> FileResolver {
    FileResolver::new(
        root,
        SelfCoordinate::new(Some("org.demo".to_owned()), "host".to_owned()),
    )
}

#[test]
fn only_effective_activated_frontend_formats_resolve_or_read_txt() {
    let root = tempfile::tempdir().unwrap();
    let file = root.path().join("vibevm/vibespecs/common/NOTE.txt");
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(&file, b"disk bytes\n").unwrap();
    let overlay = BTreeMap::from([(file.clone(), Arc::<[u8]>::from(&b"overlay bytes\n"[..]))]);
    let source = FsSectionSource::with_overlay(resolver(root.path()), overlay);

    for inactive in [formats(false, false), formats(true, true)] {
        let error = source.resolved_source(&address(), &inactive).unwrap_err();
        assert!(error.contains("not found"), "inactive/disabled: {error}");
    }
    let active = source
        .resolved_source(&address(), &formats(true, false))
        .unwrap();
    assert_eq!(active.format(), "txt");
    assert_eq!(active.physical_stem(), "NOTE");
    assert_eq!(active.into_parts().0, "overlay bytes\n");
}

#[derive(Debug, PartialEq, Eq)]
struct FrontendBoundary(String, String, String);

struct GuardedSource(std::cell::Cell<usize>);

impl SectionSource for GuardedSource {
    fn section_text(&self, _addr: &SpecAddress) -> Result<String, String> {
        panic!("custom resolution must not use the Markdown reader")
    }

    fn resolved_source(
        &self,
        _addr: &SpecAddress,
        active_formats: &[String],
    ) -> Result<crate::embed::ResolvedSource, String> {
        if !active_formats.iter().any(|format| format == "txt") {
            return Err("txt is inactive".to_owned());
        }
        self.0.set(self.0.get() + 1);
        crate::embed::ResolvedSource::custom(
            "first\nsecond\n".to_owned(),
            "txt".to_owned(),
            "NOTE".to_owned(),
        )
    }
}

#[test]
fn worklist_reaches_a_raw_sentinel_and_inactive_formats_read_nothing() {
    let plan =
        crate::compiler::ir::ArtifactPlan::compatibility(address(), StaticCompileMode::Plain);
    for inactive in [formats(false, false), formats(true, true)] {
        let source = GuardedSource(std::cell::Cell::new(0));
        let worklist = discover_with_formats(
            &plan,
            &source,
            &inactive,
            |_input, _stem| -> Result<crate::compiler::ir::DocumentIr, FrontendBoundary> {
                panic!("inactive format must not reach the frontend boundary")
            },
            |_address, _reason| {},
        )
        .unwrap();
        assert!(worklist.documents.is_empty());
        assert_eq!(source.0.get(), 0);
    }

    let source = GuardedSource(std::cell::Cell::new(0));
    let error = discover_with_formats(
        &plan,
        &source,
        &formats(true, false),
        |input, stem| {
            Err::<crate::compiler::ir::DocumentIr, _>(FrontendBoundary(
                input.format().as_str().to_owned(),
                stem.to_owned(),
                input.text().to_owned(),
            ))
        },
        |_address, _reason| {},
    )
    .unwrap_err();
    assert_eq!(
        error,
        FrontendBoundary("txt".into(), "NOTE".into(), "first\nsecond\n".into())
    );
    assert_eq!(source.0.get(), 1);
}

#[test]
fn production_selects_txt_then_returns_typed_frontend_deferred() {
    let root = tempfile::tempdir().unwrap();
    let file = root.path().join("vibevm/vibespecs/common/NOTE.txt");
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(&file, "first\nsecond\n").unwrap();
    let source = FsSectionSource::new(resolver(root.path()));
    let plan = plans(true, false).attach_to(crate::compiler::ir::ArtifactPlan::compatibility(
        address(),
        StaticCompileMode::Plain,
    ));
    let error = crate::compiler::builtin::compile_artifact(plan, &source).unwrap_err();
    assert!(matches!(
        error,
        crate::compiler::builtin::ArtifactCompileError::Pass { pass, reason }
            if pass == "pass:org.demo/formats#txt"
                && reason.contains("provider pre-admission")
    ));
}
