use super::*;

#[test]
fn same_value_row_from_another_epoch_is_not_candidate_membership() {
    let root = tempdir().unwrap();
    let declaration = source_native("same", None);
    let (selected_registry, mechanisms) = registries(root.path(), vec![declaration.clone()]);
    let (candidate_registry, _) = registries(root.path(), vec![declaration]);
    let selected = &selected_registry.rows()[0];
    let foreign = &candidate_registry.rows()[0];
    assert_eq!(selected.key(), foreign.key());
    assert_eq!(
        selected.declaration().handler,
        foreign.declaration().handler
    );
    assert!(!std::ptr::eq(selected, foreign));

    let all = vec![selected];
    let candidates = vec![foreign];
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
    let config = effective_config(selected).unwrap();
    let error = invoker
        .request_for_test(call(selected, 0, &config, CompilePoint::Pass, selected))
        .unwrap_err();
    assert_eq!(
        error.kind(),
        CompilerNativeInvokerErrorKind::InvocationFailed
    );
    assert!(!root.path().join(".vibe").exists());
}

#[test]
fn injected_project_root_must_match_selected_root_before_scratch() {
    let selected_root = tempdir().unwrap();
    let injected_root = tempdir().unwrap();
    let (registry, mechanisms) =
        registries(selected_root.path(), vec![source_native("root", None)]);
    let all = registry.rows().iter().collect::<Vec<_>>();
    let routes = MechanismRoutes::default();
    let project = project(injected_root.path());
    let world = world();
    let invoker = make_invoker(
        &all,
        &all,
        selected_root.path(),
        &mechanisms,
        &routes,
        &project,
        &world,
        RUN_ID,
    );
    let config = effective_config(all[0]).unwrap();
    let error = invoker
        .request_for_test(call(all[0], 0, &config, CompilePoint::Pass, all[0]))
        .unwrap_err();
    assert_eq!(
        error.kind(),
        CompilerNativeInvokerErrorKind::InvocationFailed
    );
    assert!(!selected_root.path().join(".vibe").exists());
    assert!(!injected_root.path().join(".vibe").exists());
}
