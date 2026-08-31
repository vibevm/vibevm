use sha2::{Digest, Sha256};
use vibe_wire::generated::native::e1::compile_reply::CompileReply;

use super::*;

#[test]
fn real_fixture_returns_raw_statuses_and_survives_panic() {
    let root = tempdir().unwrap();
    let relative = fixture(root.path());
    let ids = [
        "compiler-ok",
        "compiler-skip",
        "compiler-fail",
        "compiler-panic",
        "compiler-after",
    ];
    let declarations = ids
        .iter()
        .map(|id| native(id, Some(&relative), None))
        .collect();
    let (registry, mechanisms) = registries(root.path(), declarations);
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
    for (order, expected) in ["ok", "skip", "fail"].into_iter().enumerate() {
        let row = all[order];
        let config = effective_config(row).unwrap();
        let raw = invoker
            .invoke(call(row, order as u32, &config, CompilePoint::Pass, row))
            .unwrap();
        let reply: CompileReply = serde_json::from_slice(&raw).unwrap();
        assert_eq!(
            match reply {
                CompileReply::Ok(_) => "ok",
                CompileReply::Skip(_) => "skip",
                CompileReply::Fail(_) => "fail",
            },
            expected
        );
    }
    let panic_row = all[3];
    let panic_config = effective_config(panic_row).unwrap();
    assert_eq!(
        invoker
            .invoke(call(
                panic_row,
                3,
                &panic_config,
                CompilePoint::Pass,
                panic_row
            ))
            .unwrap_err()
            .kind(),
        CompilerNativeInvokerErrorKind::InvocationFailed
    );
    let after = all[4];
    let after_config = effective_config(after).unwrap();
    let raw = invoker
        .invoke(call(after, 4, &after_config, CompilePoint::Pass, after))
        .unwrap();
    assert!(matches!(
        serde_json::from_slice(&raw).unwrap(),
        CompileReply::Ok(_)
    ));
}

#[test]
fn lifecycle_and_compiler_share_loader_and_images_are_immutable() {
    let root = tempdir().unwrap();
    let (registry, mechanisms) = registries(root.path(), Vec::new());
    let all = registry.rows().iter().collect::<Vec<_>>();
    let routes = MechanismRoutes::default();
    let project = project(root.path());
    let world = world();
    let execution = execution(&all, root.path(), &mechanisms, &routes);
    let backend = ArtifactNativeBackend::new(execution);
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
    assert_eq!(
        backend.loader as *const NativeLoader,
        invoker.loader() as *const NativeLoader
    );

    let source = root
        .path()
        .join(format!("mutable{}", current_platform().suffix()));
    fs::write(&source, b"first image").unwrap();
    let first_digest = format!("{:x}", Sha256::digest(b"first image"));
    let first =
        super::super::path::publish_load_image(root.path(), &source, &first_digest, 11).unwrap();
    fs::write(&source, b"second image").unwrap();
    let second_digest = format!("{:x}", Sha256::digest(b"second image"));
    let second =
        super::super::path::publish_load_image(root.path(), &source, &second_digest, 12).unwrap();
    assert_ne!(first, second);
    assert_eq!(fs::read(first).unwrap(), b"first image");
    assert_eq!(fs::read(second).unwrap(), b"second image");
}

#[test]
fn compiler_adapter_source_fence_excludes_forbidden_planes() {
    let source = include_str!("compiler.rs");
    for forbidden in [
        "build_native_sources",
        "std::process",
        "Command",
        "cargo::",
        "target/",
        "glob",
        "selector",
        "Context",
        "CompileReply",
        "validate_reply",
        "from_slice",
        "Arc<",
        "NativeLoader::new()",
    ] {
        assert!(
            !source.contains(forbidden),
            "forbidden adapter source `{forbidden}`"
        );
    }
    assert!(source.contains("call.into_payload()"));
    assert!(source.contains("let manager_config = call.config().clone()"));
    assert!(source.contains("config: manager_config"));
    assert!(!source.contains("config: projected"));
    assert!(source.contains("publish_load_image"));
    assert!(source.contains("self.loader\n            .invoke_compile"));
    assert!(source.contains("library: &image"));
}
