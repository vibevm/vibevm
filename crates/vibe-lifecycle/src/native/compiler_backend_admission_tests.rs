use super::*;
use vibe_core::manifest::{ExtensionPass, ExtensionPassKind};

fn backend_native(id: &str, path: &Path, artifact: &str) -> ExtensionDecl {
    let mut declaration = native(id, Some(path), None);
    declaration.pass = Some(ExtensionPass {
        kind: ExtensionPassKind::Backend,
        level: None,
        from: None,
        to: None,
        after: None,
        before: None,
        replace: None,
        formats: None,
        artifact: Some(artifact.to_owned()),
    });
    declaration
}

#[test]
fn backend_invocation_requires_and_rechecks_its_exact_admitted_handle() {
    let root = tempdir().unwrap();
    let relative = fixture(root.path());
    let (registry, mechanisms) = registries(
        root.path(),
        vec![backend_native("compiler-ok", &relative, "opaque-bin")],
    );
    let rows = registry.rows().iter().collect::<Vec<_>>();
    let routes = MechanismRoutes::default();
    let project = project(root.path());
    let world = world();
    let invoker = ArtifactCompilerNativeInvoker::new(
        &rows,
        execution(&rows, root.path(), &mechanisms, &routes),
        &project,
        &world,
        RUN_ID,
    );
    let config = effective_config(rows[0]).unwrap();
    let digest = compiler_native_implementation_digest(rows[0]).unwrap();
    let marked = || {
        call(rows[0], 0, &config, CompilePoint::Pass, rows[0]).with_backend_for_test("opaque-bin")
    };

    let error = invoker.invoke(marked()).unwrap_err();
    assert!(error.to_string().contains("backend was not pre-admitted"));
    invoker
        .admit_backend(rows[0].key(), 0, &config, digest, "opaque-bin")
        .unwrap();
    let raw = invoker.invoke(marked()).unwrap();
    assert!(
        serde_json::from_slice::<vibe_wire::generated::native::e1::compile_reply::CompileReply>(
            &raw
        )
        .is_ok()
    );
}

#[test]
fn backend_admission_refuses_a_different_selected_artifact() {
    let root = tempdir().unwrap();
    let relative = fixture(root.path());
    let (registry, mechanisms) = registries(
        root.path(),
        vec![backend_native("compiler-ok", &relative, "opaque-bin")],
    );
    let rows = registry.rows().iter().collect::<Vec<_>>();
    let routes = MechanismRoutes::default();
    let project = project(root.path());
    let world = world();
    let invoker = ArtifactCompilerNativeInvoker::new(
        &rows,
        execution(&rows, root.path(), &mechanisms, &routes),
        &project,
        &world,
        RUN_ID,
    );
    let config = effective_config(rows[0]).unwrap();
    let error = invoker
        .admit_backend(
            rows[0].key(),
            0,
            &config,
            compiler_native_implementation_digest(rows[0]).unwrap(),
            "other-bin",
        )
        .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("differs from selected `other-bin`")
    );
}
