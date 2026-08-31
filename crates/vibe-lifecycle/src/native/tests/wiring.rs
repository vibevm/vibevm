use super::*;

use vibe_core::manifest::Manifest;
use vibe_core::{ContentHash, Group, PackageKind, PackageName};

use crate::{
    DependencyExtensionSource, DependencyProvider, DependencyProviderId, HostExtensionSource,
    HostIdentity, HostProvider,
};

#[test]
fn enabled_native_candidates_keep_effective_order_and_ignore_selectors() {
    let root = tempdir().unwrap();
    let dependency = Manifest::parse_str(
        r#"
[package]
group = "org.example"
name = "native"
kind = "tool"
version = "1.0.0"

[[extension]]
id = "selector"
point = "compile:source"
handler = { kind = "native", crate_dir = "native" }
applies_to = { paths = ["never/**"] }

[[extension]]
id = "disabled"
point = "phase:build"
handler = { kind = "native", crate_dir = "native" }

[[extension]]
id = "inactive"
point = "compile:emitted"
handler = { kind = "native", crate_dir = "native" }
"#,
    )
    .unwrap();
    let host = Manifest::parse_str(
        r#"
[project]
name = "demo"
version = "0.1.0"

[extensions]
disable = ["org.example/native#disabled"]

[[extensions.use]]
ref = "org.example/native#selector"

[[extension]]
id = "host"
point = "slot:post-install"
handler = { kind = "native", crate_dir = "native" }
"#,
    )
    .unwrap();
    let registry = collect_extensions(ExtensionWorld {
        installed: vec![DependencyExtensionSource {
            provider: DependencyProvider {
                id: DependencyProviderId::new(
                    Group::parse("org.example").unwrap(),
                    PackageName::parse("native").unwrap(),
                ),
                root: root.path().join("slot"),
                version: "1.0.0".into(),
                kind: PackageKind::Tool,
                content_hash: ContentHash::parse("sha256:aa").unwrap(),
            },
            declarations: dependency.extensions,
            controls: dependency.extension_controls,
            mechanisms: dependency.mechanism_decls,
        }],
        host: HostExtensionSource {
            provider: HostProvider {
                identity: HostIdentity::ungrouped_project("demo"),
                root: root.path().to_path_buf(),
                version: "0.1.0".into(),
                kind: None,
                content_hash: None,
            },
            declarations: host.extensions,
            controls: host.extension_controls,
            mechanisms: host.mechanism_decls,
        },
        effective_stack: None,
    })
    .unwrap();

    let candidates = enabled_native_candidates(&registry);
    assert_eq!(
        candidates
            .iter()
            .map(|row| row.declaration().id.as_str())
            .collect::<Vec<_>>(),
        vec!["host", "selector"],
        "enabled native rows retain the one effective order; selector mismatch is irrelevant",
    );
    assert!(matches!(
        candidates[1].declaration().point,
        vibe_core::lifecycle::ExtensionPoint::Compile(_)
    ));
}

#[test]
fn every_production_backend_reuses_the_process_loader() {
    let root = tempdir().unwrap();
    let owned = world(root.path(), Vec::new());
    let mechanisms = collect_mechanisms(&owned).unwrap();
    let candidates = Vec::new();
    let routes = MechanismRoutes::default();
    let execution = NativeBuildExecution {
        candidates: &candidates,
        selected_project_root: root.path(),
        registry: &mechanisms,
        routes: &routes,
        platform: current_platform(),
        offline: true,
        created_at: "2026-08-31T00:00:00Z",
    };
    let first_backend = ArtifactNativeBackend::new(execution);
    let second_backend = ArtifactNativeBackend::new(execution);
    let first = first_backend.loader as *const NativeLoader;
    let second = second_backend.loader as *const NativeLoader;
    assert_eq!(first, second, "one process owns one loader cache");
}
