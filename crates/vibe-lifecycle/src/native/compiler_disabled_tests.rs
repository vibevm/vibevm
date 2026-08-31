use tempfile::tempdir;
use vibe_core::lifecycle::CompilePoint;
use vibe_core::manifest::{Manifest, MechanismRoutes};
use vibe_core::{ContentHash, Group, PackageKind, PackageName};
use vibe_spec::{
    CompilerNativeCall, CompilerNativeInvoker, CompilerNativeInvokerErrorKind,
    compiler_native_implementation_digest,
};
use vibe_wire::generated::shared::{Ir, Project, World};

use crate::execution::effective_config;
use crate::{
    DependencyExtensionSource, DependencyProvider, DependencyProviderId, ExtensionWorld,
    HostExtensionSource, HostIdentity, HostProvider, collect_extensions, collect_mechanisms,
};

use super::*;

#[test]
fn disabled_native_row_refuses_before_scratch() {
    let root = tempdir().unwrap();
    let dependency = Manifest::parse_str(
        r#"
[package]
group = "org.example"
name = "native"
kind = "tool"
version = "1.0.0"

[[extension]]
id = "disabled"
point = "compile:pass"
handler = { kind = "native", crate_dir = "native" }
compiler_internals = true
"#,
    )
    .unwrap();
    let host = Manifest::parse_str(
        r#"
[project]
name = "compiler-host"
version = "0.1.0"

[extensions]
disable = ["org.example/native#disabled"]
"#,
    )
    .unwrap();
    let world_model = ExtensionWorld {
        installed: vec![DependencyExtensionSource {
            provider: DependencyProvider {
                id: DependencyProviderId::new(
                    Group::parse("org.example").unwrap(),
                    PackageName::parse("native").unwrap(),
                ),
                root: root.path().join("slot"),
                version: "1.0.0".to_owned(),
                kind: PackageKind::Tool,
                content_hash: ContentHash::parse("sha256:aa").unwrap(),
            },
            declarations: dependency.extensions,
            controls: dependency.extension_controls,
            mechanisms: dependency.mechanism_decls,
        }],
        host: HostExtensionSource {
            provider: HostProvider {
                identity: HostIdentity::ungrouped_project("compiler-host"),
                root: root.path().to_path_buf(),
                version: "0.1.0".to_owned(),
                kind: None,
                content_hash: None,
            },
            declarations: host.extensions,
            controls: host.extension_controls,
            mechanisms: host.mechanism_decls,
        },
        effective_stack: None,
    };
    let registry = collect_extensions(world_model.clone()).unwrap();
    let mechanisms = collect_mechanisms(&world_model).unwrap();
    let row = registry
        .rows()
        .iter()
        .find(|row| row.declaration().id == "disabled")
        .unwrap();
    assert!(!row.is_enabled());
    let all = vec![row];
    let routes = MechanismRoutes::default();
    let project = Project {
        root: root.path().display().to_string().replace('\\', "/"),
        name: "compiler-host".to_owned(),
        version: "0.1.0".to_owned(),
        kind: "flow".to_owned(),
        manifest: "vibe.toml".to_owned(),
        spec_roots: Vec::new(),
    };
    let world = World {
        deps_root: "vibevm/vibedeps".to_owned(),
        lockfile: "vibe.lock".to_owned(),
        packages: Vec::new(),
    };
    let execution = NativeBuildExecution {
        candidates: &all,
        selected_project_root: root.path(),
        registry: &mechanisms,
        routes: &routes,
        platform: NativePlatform::from_pair(std::env::consts::OS, std::env::consts::ARCH).unwrap(),
        offline: true,
        created_at: "2026-08-31T00:00:00Z",
    };
    let invoker = ArtifactCompilerNativeInvoker::new(
        &all,
        execution,
        &project,
        &world,
        "0123456789abcdef0123456789abcdef",
    );
    let config = effective_config(row).unwrap();
    let payload: Ir = serde_json::from_value(serde_json::json!({
        "shape": "source-document", "ir_schema": 1, "level": "source",
        "cardinality": "document", "doc": {
            "address": {"kind": "static-entry", "origin": "fixture", "path": "fixture.md"},
            "format": "markdown", "subject": {
                "declared_path": "fixture.md", "provider": {"kind": "unclaimed"}
            }, "text": "fixture"
        }
    }))
    .unwrap();
    let call = CompilerNativeCall::new_for_test(
        row.key(),
        CompilePoint::Pass,
        0,
        &config,
        compiler_native_implementation_digest(row).unwrap(),
        payload,
    );
    let error = invoker.invoke(call).unwrap_err();
    assert_eq!(
        error.kind(),
        CompilerNativeInvokerErrorKind::InvocationFailed
    );
    assert!(error.to_string().contains("disabled"));
    assert!(!root.path().join(".vibe").exists());
}
