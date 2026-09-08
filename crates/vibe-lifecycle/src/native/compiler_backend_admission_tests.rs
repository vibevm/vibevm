use super::*;
use crate::native::path::publish_load_image;
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

fn backend_source(id: &str, artifact: &str) -> ExtensionDecl {
    let mut declaration = source_native(id, None);
    declaration.pass = backend_native(id, Path::new("unused"), artifact).pass;
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

#[test]
fn existing_backend_reuses_a_verified_image_without_touching_scratch() {
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
    let run = execution(&rows, root.path(), &mechanisms, &routes);
    let artifact = resolve_native_artifact(&run, rows[0]).unwrap();
    let image = publish_load_image(
        root.path(),
        Path::new(&artifact.path_absolute),
        &artifact.digest,
        artifact.bytes,
    )
    .unwrap();
    let scratch_before = fs::metadata(root.path()).unwrap().modified().unwrap();
    let image_before = fs::read(&image).unwrap();
    let invoker = ArtifactCompilerNativeInvoker::existing_for_test(
        &rows,
        run,
        &project,
        &world,
        RUN_ID,
        root.path(),
    );
    let config = effective_config(rows[0]).unwrap();
    invoker
        .admit_backend(
            rows[0].key(),
            0,
            &config,
            compiler_native_implementation_digest(rows[0]).unwrap(),
            "opaque-bin",
        )
        .unwrap();
    let call =
        call(rows[0], 0, &config, CompilePoint::Pass, rows[0]).with_backend_for_test("opaque-bin");
    invoker.invoke(call).unwrap();
    assert_eq!(fs::read(image).unwrap(), image_before);
    assert_eq!(
        fs::metadata(root.path()).unwrap().modified().unwrap(),
        scratch_before
    );
}

#[test]
fn existing_backend_missing_stale_or_source_built_never_creates_an_image() {
    let missing = tempdir().unwrap();
    let relative = fixture(missing.path());
    let (registry, mechanisms) = registries(
        missing.path(),
        vec![backend_native("compiler-ok", &relative, "opaque-bin")],
    );
    let rows = registry.rows().iter().collect::<Vec<_>>();
    let routes = MechanismRoutes::default();
    let project_value = project(missing.path());
    let world_value = world();
    let invoker = ArtifactCompilerNativeInvoker::existing_for_test(
        &rows,
        execution(&rows, missing.path(), &mechanisms, &routes),
        &project_value,
        &world_value,
        RUN_ID,
        missing.path(),
    );
    let config = effective_config(rows[0]).unwrap();
    assert!(
        invoker
            .admit_backend(
                rows[0].key(),
                0,
                &config,
                compiler_native_implementation_digest(rows[0]).unwrap(),
                "opaque-bin",
            )
            .is_err()
    );
    assert!(!missing.path().join(".vibe/native-load").exists());
    let run = execution(&rows, missing.path(), &mechanisms, &routes);
    let artifact = resolve_native_artifact(&run, rows[0]).unwrap();
    let image = publish_load_image(
        missing.path(),
        Path::new(&artifact.path_absolute),
        &artifact.digest,
        artifact.bytes,
    )
    .unwrap();
    fs::write(&image, b"stale image").unwrap();
    let stale = fs::read(&image).unwrap();
    assert!(
        invoker
            .admit_backend(
                rows[0].key(),
                0,
                &config,
                compiler_native_implementation_digest(rows[0]).unwrap(),
                "opaque-bin",
            )
            .is_err()
    );
    assert_eq!(fs::read(image).unwrap(), stale);

    let source = tempdir().unwrap();
    let (registry, mechanisms) = registries(
        source.path(),
        vec![backend_source("compiler-ok", "opaque-bin")],
    );
    let rows = registry.rows().iter().collect::<Vec<_>>();
    let source_project = project(source.path());
    let invoker = ArtifactCompilerNativeInvoker::existing_for_test(
        &rows,
        execution(&rows, source.path(), &mechanisms, &routes),
        &source_project,
        &world_value,
        RUN_ID,
        source.path(),
    );
    let config = effective_config(rows[0]).unwrap();
    assert!(
        invoker
            .admit_backend(
                rows[0].key(),
                0,
                &config,
                compiler_native_implementation_digest(rows[0]).unwrap(),
                "opaque-bin"
            )
            .is_err()
    );
    assert!(!source.path().join(".vibe").exists());
}
