use super::*;
use vibe_wire::generated::native::e1::compile_reply::CompileReply;

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

fn fixture_invoker<'a>(
    rows: &'a [&'a ExtensionRegistryRow],
    root: &'a Path,
    mechanisms: &'a MechanismRegistry,
    routes: &'a MechanismRoutes,
    project: &'a Project,
    world: &'a World,
) -> ArtifactCompilerNativeInvoker<'a> {
    ArtifactCompilerNativeInvoker::new(
        rows,
        execution(rows, root, mechanisms, routes),
        project,
        world,
        RUN_ID,
    )
}

#[test]
fn frontend_stem_projects_and_the_admitted_handle_executes_later() {
    let root = tempdir().unwrap();
    let relative = fixture(root.path());
    let (registry, mechanisms) = registries(
        root.path(),
        vec![native("compiler-ok", Some(&relative), None)],
    );
    let routes = MechanismRoutes::default();
    let project = project(root.path());
    let world = world();
    let rows = registry.rows().iter().collect::<Vec<_>>();
    assert_eq!(rows[0].declaration().id, "compiler-ok");
    let invoker = fixture_invoker(&rows, root.path(), &mechanisms, &routes, &project, &world);
    let config = effective_config(rows[0]).unwrap();
    let digest = compiler_native_implementation_digest(rows[0]).unwrap();
    invoker
        .admit_frontend(rows[0].key(), 0, &config, digest)
        .unwrap();
    let call = call(rows[0], 0, &config, CompilePoint::Pass, rows[0])
        .with_frontend_physical_stem_for_test("NOTE-descriptive");
    let raw = invoker.invoke(call).unwrap();
    let CompileReply::Ok(reply) = serde_json::from_slice(&raw).unwrap() else {
        panic!("frontend fixture returns ok")
    };
    let Ir::DocumentDocument(document) = reply.payload else {
        panic!("frontend fixture returns document IR")
    };
    assert_eq!(document.doc.tree.nodes[1].heading, "NOTE-descriptive");
}

#[test]
fn missing_compiler_descriptor_refuses_during_frontend_admission() {
    let root = tempdir().unwrap();
    let relative = fixture(root.path());
    let (registry, mechanisms) =
        registries(root.path(), vec![native("missing", Some(&relative), None)]);
    let routes = MechanismRoutes::default();
    let project = project(root.path());
    let world = world();
    let rows = registry.rows().iter().collect::<Vec<_>>();
    assert_eq!(rows[0].declaration().id, "missing");
    let invoker = fixture_invoker(&rows, root.path(), &mechanisms, &routes, &project, &world);
    let config = effective_config(rows[0]).unwrap();
    let error = invoker
        .admit_frontend(
            rows[0].key(),
            0,
            &config,
            compiler_native_implementation_digest(rows[0]).unwrap(),
        )
        .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("does not declare extension id `missing`")
    );
}
