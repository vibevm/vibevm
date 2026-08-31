use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use tempfile::tempdir;
use vibe_core::lifecycle::CompilePoint;
use vibe_core::manifest::{
    ExtensionConfig, ExtensionDecl, ExtensionHandler, ExtensionsControl, MechanismRoutes,
};
use vibe_spec::{
    CompilerNativeCall, CompilerNativeInvoker, CompilerNativeInvokerErrorKind,
    compiler_native_implementation_digest,
};
use vibe_wire::generated::shared::{Ir, Project, World};

use crate::execution::effective_config;
use crate::{
    ExtensionRegistry, ExtensionRegistryRow, ExtensionWorld, HostExtensionSource, HostIdentity,
    HostProvider, MechanismRegistry, collect_extensions, collect_mechanisms,
};

use super::*;

const RUN_ID: &str = "0123456789abcdef0123456789abcdef";

fn declaration(
    id: &str,
    handler: ExtensionHandler,
    point: &str,
    config: Option<ExtensionConfig>,
) -> ExtensionDecl {
    ExtensionDecl {
        id: id.to_owned(),
        point: point.parse().unwrap(),
        handler,
        config,
        auto: None,
        inputs: None,
        applies_to: None,
        compiler_internals: Some(true),
        pass: None,
        when: None,
    }
}

fn native(id: &str, path: Option<&Path>, config: Option<ExtensionConfig>) -> ExtensionDecl {
    let prebuilt = path
        .map(|path| BTreeMap::from([(current_platform().key().to_owned(), path.to_path_buf())]));
    declaration(
        id,
        ExtensionHandler::Native {
            crate_dir: None,
            prebuilt,
        },
        "compile:pass",
        config,
    )
}

fn source_native(id: &str, config: Option<ExtensionConfig>) -> ExtensionDecl {
    declaration(
        id,
        ExtensionHandler::Native {
            crate_dir: Some(PathBuf::from("native")),
            prebuilt: None,
        },
        "compile:pass",
        config,
    )
}

fn registries(
    root: &Path,
    declarations: Vec<ExtensionDecl>,
) -> (ExtensionRegistry, MechanismRegistry) {
    let world = ExtensionWorld {
        installed: Vec::new(),
        host: HostExtensionSource {
            provider: HostProvider {
                identity: HostIdentity::ungrouped_project("compiler-host"),
                root: root.to_path_buf(),
                version: "0.1.0".to_owned(),
                kind: None,
                content_hash: None,
            },
            declarations,
            controls: ExtensionsControl::default(),
            mechanisms: Vec::new(),
        },
        effective_stack: None,
    };
    let extensions = collect_extensions(world.clone()).unwrap();
    let mechanisms = collect_mechanisms(&world).unwrap();
    (extensions, mechanisms)
}

fn project(root: &Path) -> Project {
    Project {
        root: root.display().to_string().replace('\\', "/"),
        name: "compiler-host".to_owned(),
        version: "0.1.0".to_owned(),
        kind: "flow".to_owned(),
        manifest: "vibe.toml".to_owned(),
        spec_roots: vec!["vibevm/vibespecs".to_owned()],
    }
}

fn world() -> World {
    World {
        lockfile: "vibe.lock".to_owned(),
        deps_root: "vibevm/vibedeps".to_owned(),
        packages: Vec::new(),
    }
}

fn payload() -> Ir {
    serde_json::from_value(serde_json::json!({
        "shape": "source-document", "ir_schema": 1, "level": "source",
        "cardinality": "document", "doc": {
            "address": {"kind": "static-entry", "origin": "fixture", "path": "fixture.md"},
            "format": "markdown", "subject": {
                "declared_path": "fixture.md", "provider": {"kind": "unclaimed"}
            }, "text": "fixture compiler input"
        }
    }))
    .unwrap()
}

fn call<'a>(
    row: &'a ExtensionRegistryRow,
    order: u32,
    config: &'a BTreeMap<String, Option<serde_json::Value>>,
    point: CompilePoint,
    implementation_row: &ExtensionRegistryRow,
) -> CompilerNativeCall<'a> {
    CompilerNativeCall::new_for_test(
        row.key(),
        point,
        order,
        config,
        compiler_native_implementation_digest(implementation_row).unwrap(),
        payload(),
    )
}

fn execution<'a>(
    rows: &'a [&'a ExtensionRegistryRow],
    root: &'a Path,
    mechanisms: &'a MechanismRegistry,
    routes: &'a MechanismRoutes,
) -> NativeBuildExecution<'a> {
    NativeBuildExecution {
        candidates: rows,
        selected_project_root: root,
        registry: mechanisms,
        routes,
        platform: current_platform(),
        offline: true,
        created_at: "2026-08-31T00:00:00Z",
    }
}

fn current_platform() -> NativePlatform {
    NativePlatform::from_pair(std::env::consts::OS, std::env::consts::ARCH).unwrap()
}

fn fixture(root: &Path) -> PathBuf {
    let source = fixture_library();
    let relative = PathBuf::from(format!("prebuilt/compiler{}", current_platform().suffix()));
    fs::create_dir_all(root.join("prebuilt")).unwrap();
    fs::copy(source, root.join(&relative)).unwrap();
    relative
}

fn fixture_library() -> PathBuf {
    assert_eq!(
        vibe_native_loader_compiler_fixture::fixture_marker(),
        "vibe-native-loader-compiler-fixture"
    );
    let executable = std::env::current_exe().unwrap();
    let deps = executable.parent().unwrap();
    let profile = deps.parent().unwrap();
    let exact = format!(
        "{}vibe_native_loader_compiler_fixture{}",
        std::env::consts::DLL_PREFIX,
        std::env::consts::DLL_SUFFIX
    );
    [profile.to_path_buf(), deps.to_path_buf()]
        .into_iter()
        .flat_map(|directory| fs::read_dir(directory).unwrap())
        .map(|entry| entry.unwrap().path())
        .find(|path| path.file_name().is_some_and(|name| name == exact.as_str()))
        .expect("compiler fixture cdylib")
}

#[expect(
    clippy::too_many_arguments,
    reason = "the fixture names every independently mutated captured authority"
)]
fn make_invoker<'a>(
    all: &'a [&'a ExtensionRegistryRow],
    candidates: &'a [&'a ExtensionRegistryRow],
    root: &'a Path,
    mechanisms: &'a MechanismRegistry,
    routes: &'a MechanismRoutes,
    project: &'a Project,
    world: &'a World,
    run_id: &'a str,
) -> ArtifactCompilerNativeInvoker<'a> {
    ArtifactCompilerNativeInvoker::new(
        all,
        execution(candidates, root, mechanisms, routes),
        project,
        world,
        run_id,
    )
}

#[test]
fn test_support_call_moves_payload_and_is_feature_gated() {
    let root = tempdir().unwrap();
    let (registry, _) = registries(root.path(), vec![source_native("move", None)]);
    let row = &registry.rows()[0];
    let config = effective_config(row).unwrap();
    let expected = payload();
    let call = CompilerNativeCall::new_for_test(
        row.key(),
        CompilePoint::Pass,
        0,
        &config,
        compiler_native_implementation_digest(row).unwrap(),
        expected.clone(),
    );
    assert_eq!(call.into_payload(), expected);
    let source = include_str!("../../../vibe-spec/src/compiler/transform/native_manager.rs");
    assert!(source.contains("#[cfg(any(test, feature = \"test-support\"))]"));
    assert!(source.contains("pub fn into_payload(self) -> Ir"));
}

#[test]
fn all_row_order_and_request_authorities_are_exact() {
    let root = tempdir().unwrap();
    let relative = fixture(root.path());
    let nested = ExtensionConfig::from_table(
        toml::from_str("name = 'exact'\n[nested]\ncount = 3\nflags = [true, false]\n").unwrap(),
    );
    let declarations = vec![
        declaration(
            "builtin",
            ExtensionHandler::Builtin {
                name: "noop".to_owned(),
            },
            "compile:pass",
            None,
        ),
        native("compiler-ok", Some(&relative), Some(nested)),
    ];
    let (registry, mechanisms) = registries(root.path(), declarations);
    let all = registry.rows().iter().collect::<Vec<_>>();
    assert_eq!(all[0].declaration().id, "builtin");
    assert_eq!(all[1].declaration().id, "compiler-ok");
    let candidates = vec![all[1]];
    let routes = MechanismRoutes::default();
    let project = project(root.path());
    let world = world();
    let invoker = make_invoker(
        &all,
        &candidates,
        root.path(),
        &mechanisms,
        &routes,
        &project,
        &world,
        RUN_ID,
    );
    let config = effective_config(all[1]).unwrap();
    let expected_payload = payload();
    let request = invoker
        .request_for_test(CompilerNativeCall::new_for_test(
            all[1].key(),
            CompilePoint::Pass,
            1,
            &config,
            compiler_native_implementation_digest(all[1]).unwrap(),
            expected_payload.clone(),
        ))
        .unwrap();
    assert_eq!(request.envelope, 1);
    assert_eq!(request.point, "compile:pass");
    assert_eq!(request.execution.id, "compiler-ok");
    assert_eq!(request.execution.package, all[1].provider().to_string());
    assert_eq!(request.execution.config, config);
    assert_eq!(request.project, project);
    assert_eq!(request.world, world);
    assert_eq!(request.payload, expected_payload);
    assert!(!request.io.scratch.contains('\\'));
    let canonical_root = root
        .path()
        .canonicalize()
        .unwrap()
        .display()
        .to_string()
        .replace('\\', "/");
    assert!(request.io.scratch.starts_with(&canonical_root));
    assert!(request.io.scratch.ends_with(&format!(
        "{:x}",
        Sha256::digest(all[1].key().to_string().as_bytes())
    )));
    assert_ne!(all[1].key().to_string(), request.execution.id);
}

#[test]
fn absent_empty_and_nested_configs_use_the_lifecycle_projector() {
    let root = tempdir().unwrap();
    let empty = ExtensionConfig::from_table(toml::Table::new());
    let nested = ExtensionConfig::from_table(toml::from_str("[nested]\nanswer = 42\n").unwrap());
    let (registry, mechanisms) = registries(
        root.path(),
        vec![
            source_native("absent", None),
            source_native("empty", Some(empty)),
            source_native("nested", Some(nested)),
        ],
    );
    let all = registry.rows().iter().collect::<Vec<_>>();
    let routes = MechanismRoutes::default();
    let project_value = project(root.path());
    let world_value = world();
    let invoker = make_invoker(
        &all,
        &all,
        root.path(),
        &mechanisms,
        &routes,
        &project_value,
        &world_value,
        RUN_ID,
    );
    for (order, row) in all.iter().enumerate() {
        let config = effective_config(row).unwrap();
        let request = invoker
            .request_for_test(call(row, order as u32, &config, CompilePoint::Pass, row))
            .unwrap();
        assert_eq!(request.execution.config, config);
    }
    assert!(effective_config(all[0]).unwrap().is_empty());
    assert!(effective_config(all[1]).unwrap().is_empty());
    assert_eq!(
        effective_config(all[2]).unwrap()["nested"],
        Some(serde_json::json!({"answer": 42}))
    );
}

#[test]
fn row_cross_checks_refuse_before_scratch() {
    let root = tempdir().unwrap();
    let other_path = PathBuf::from(format!("other{}", current_platform().suffix()));
    let (registry, mechanisms) = registries(
        root.path(),
        vec![
            declaration(
                "builtin",
                ExtensionHandler::Builtin {
                    name: "noop".to_owned(),
                },
                "compile:pass",
                None,
            ),
            source_native("selected", None),
            native("other", Some(&other_path), None),
        ],
    );
    let all = registry.rows().iter().collect::<Vec<_>>();
    let candidates = vec![all[1], all[2]];
    let routes = MechanismRoutes::default();
    let project = project(root.path());
    let world = world();
    let invoker = make_invoker(
        &all,
        &candidates,
        root.path(),
        &mechanisms,
        &routes,
        &project,
        &world,
        RUN_ID,
    );
    let exact = effective_config(all[1]).unwrap();
    let wrong_config = BTreeMap::from([("wrong".to_owned(), Some(serde_json::json!(true)))]);
    let cases = [
        call(all[1], 99, &exact, CompilePoint::Pass, all[1]),
        CompilerNativeCall::new_for_test(
            all[0].key(),
            CompilePoint::Pass,
            1,
            &exact,
            compiler_native_implementation_digest(all[1]).unwrap(),
            payload(),
        ),
        CompilerNativeCall::new_for_test(
            all[0].key(),
            CompilePoint::Pass,
            0,
            &exact,
            compiler_native_implementation_digest(all[1]).unwrap(),
            payload(),
        ),
        call(all[1], 1, &exact, CompilePoint::Source, all[1]),
        call(all[1], 1, &wrong_config, CompilePoint::Pass, all[1]),
        call(all[1], 1, &exact, CompilePoint::Pass, all[2]),
    ];
    for call in cases {
        let error = invoker.invoke(call).unwrap_err();
        assert_eq!(
            error.kind(),
            CompilerNativeInvokerErrorKind::InvocationFailed
        );
        assert!(
            !root.path().join(".vibe").exists(),
            "cross-check allocated scratch"
        );
    }
}

#[test]
fn invalid_run_id_refuses_before_artifact_resolution() {
    let root = tempdir().unwrap();
    let (registry, mechanisms) = registries(root.path(), vec![source_native("source", None)]);
    let all = registry.rows().iter().collect::<Vec<_>>();
    let routes = MechanismRoutes::default();
    let project = project(root.path());
    let world = world();
    let invoker = make_invoker(
        &all,
        &all,
        root.path(),
        &mechanisms,
        &routes,
        &project,
        &world,
        "invalid",
    );
    let config = effective_config(all[0]).unwrap();
    let error = invoker
        .invoke(call(all[0], 0, &config, CompilePoint::Pass, all[0]))
        .unwrap_err();
    assert_eq!(
        error.kind(),
        CompilerNativeInvokerErrorKind::InvocationFailed
    );
    assert!(!root.path().join(".vibe").exists());
}

#[test]
fn missing_source_record_is_the_only_buildable_class() {
    let root = tempdir().unwrap();
    fs::create_dir_all(root.path().join("native/src")).unwrap();
    fs::write(
        root.path().join("native/Cargo.toml"),
        "[package]\nname='source'\nversion='0.1.0'\nedition='2024'\n[lib]\ncrate-type=['cdylib']\n",
    )
    .unwrap();
    fs::write(
        root.path().join("native/src/lib.rs"),
        "pub fn marker() {}\n",
    )
    .unwrap();
    let (registry, mechanisms) = registries(root.path(), vec![source_native("source", None)]);
    let all = registry.rows().iter().collect::<Vec<_>>();
    let routes = MechanismRoutes::default();
    let project_value = project(root.path());
    let world_value = world();
    let invoker = make_invoker(
        &all,
        &all,
        root.path(),
        &mechanisms,
        &routes,
        &project_value,
        &world_value,
        RUN_ID,
    );
    let config = effective_config(all[0]).unwrap();
    let error = invoker
        .invoke(call(all[0], 0, &config, CompilePoint::Pass, all[0]))
        .unwrap_err();
    assert_eq!(
        error.kind(),
        CompilerNativeInvokerErrorKind::BuildableSourceUnavailable
    );
    assert!(!root.path().join("target").exists());

    let provider = super::provider::facts(all[0]);
    let id = super::witness::record_id(&provider.identity, "native", current_platform());
    let relative = super::record::record_path(&id);
    fs::create_dir_all(root.path().join(&relative).parent().unwrap()).unwrap();
    fs::write(root.path().join(relative), b"not-json").unwrap();
    let error = invoker
        .invoke(call(all[0], 0, &config, CompilePoint::Pass, all[0]))
        .unwrap_err();
    assert_eq!(
        error.kind(),
        CompilerNativeInvokerErrorKind::InvocationFailed
    );
    assert!(!root.path().join("target").exists());

    build_native_sources(&execution(&all, root.path(), &mechanisms, &routes)).unwrap();
    let changed = ExtensionConfig::from_table(toml::from_str("mode = 'changed'\n").unwrap());
    let (changed_registry, changed_mechanisms) =
        registries(root.path(), vec![source_native("source", Some(changed))]);
    let changed_rows = changed_registry.rows().iter().collect::<Vec<_>>();
    let changed_project = project(root.path());
    let changed_world = world();
    let changed_invoker = make_invoker(
        &changed_rows,
        &changed_rows,
        root.path(),
        &changed_mechanisms,
        &routes,
        &changed_project,
        &changed_world,
        RUN_ID,
    );
    let changed_config = effective_config(changed_rows[0]).unwrap();
    let error = changed_invoker
        .invoke(call(
            changed_rows[0],
            0,
            &changed_config,
            CompilePoint::Pass,
            changed_rows[0],
        ))
        .unwrap_err();
    assert_eq!(
        error.kind(),
        CompilerNativeInvokerErrorKind::InvocationFailed
    );
}

#[test]
fn prebuilt_image_and_loader_failures_are_hard() {
    for mode in ["missing", "corrupt", "image"] {
        let root = tempdir().unwrap();
        let relative = PathBuf::from(format!("prebuilt/compiler{}", current_platform().suffix()));
        if mode != "missing" {
            fs::create_dir_all(root.path().join("prebuilt")).unwrap();
            fs::write(root.path().join(&relative), b"not a dynamic library").unwrap();
        }
        if mode == "image" {
            fs::create_dir_all(root.path().join(".vibe")).unwrap();
            fs::write(root.path().join(".vibe/native-load"), b"blocks directory").unwrap();
        }
        let (registry, mechanisms) = registries(
            root.path(),
            vec![native("compiler-ok", Some(&relative), None)],
        );
        let all = registry.rows().iter().collect::<Vec<_>>();
        let routes = MechanismRoutes::default();
        let project = project(root.path());
        let world = world();
        let invoker = make_invoker(
            &all,
            &all,
            root.path(),
            &mechanisms,
            &routes,
            &project,
            &world,
            RUN_ID,
        );
        let config = effective_config(all[0]).unwrap();
        let error = invoker
            .invoke(call(all[0], 0, &config, CompilePoint::Pass, all[0]))
            .unwrap_err();
        assert_eq!(
            error.kind(),
            CompilerNativeInvokerErrorKind::InvocationFailed
        );
    }
}

#[path = "compiler_fixture_tests.rs"]
mod fixture_tests;

#[path = "compiler_coherence_tests.rs"]
mod coherence_tests;

#[path = "compiler_facts_tests.rs"]
mod facts_tests;

#[path = "compiler_facts_semantic_tests.rs"]
mod facts_semantic_tests;
