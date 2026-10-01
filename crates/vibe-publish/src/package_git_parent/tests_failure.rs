use super::*;

#[test]
fn missing_retained_blob_refuses_cold_closure() {
    let Some(binary) = binary() else {
        eprintln!("Git absent; parent fixture skipped");
        return;
    };
    let world = world(&binary, false);
    assert!(matches!(
        stage_local_parent_pending(
            &world.lease,
            &world.token,
            &world.scope,
            "parent",
            &world.package,
            &world.origin.project,
            &binary,
            limits()
        ),
        ParentInspection::ClosureCompleteButNotReady { .. }
    ));
    let oid = &world.origin.blob;
    let object_root = world.root.path().join(format!(
        ".vibe/package-git-operations/{}.objects",
        world.run_id
    ));
    let loose = object_root.join(format!("objects/{}/{}", &oid[..2], &oid[2..]));
    if loose.exists() {
        fs::remove_file(loose).unwrap();
    } else {
        let pack = object_root.join("objects/pack");
        let file = fs::read_dir(&pack)
            .unwrap()
            .filter_map(|item| item.ok())
            .map(|item| item.path())
            .find(|path| path.extension().is_some_and(|e| e == "pack"))
            .expect("fetched object pack absent");
        fs::remove_file(file).unwrap();
    }
    assert!(matches!(
        inspect_parent_cold(
            &world.lease,
            &world.token,
            &world.scope,
            "parent",
            None,
            None,
            &binary,
            limits()
        ),
        ParentInspection::Uncertain { .. }
    ));
}

#[test]
fn replaced_local_bare_source_after_fetch_refuses_landing() {
    let Some(binary) = binary() else {
        eprintln!("Git absent; parent fixture skipped");
        return;
    };
    let world = world(&binary, false);
    let source_path = world.origin.project.root_path().to_path_buf();
    let retired = world.origin._root.path().join("retired.git");
    let status = stage_inner(
        &world.lease,
        &world.token,
        &world.scope,
        "parent",
        &world.package,
        &world.origin.project,
        &binary,
        limits(),
        || {
            fs::rename(&source_path, &retired).unwrap();
            fs::create_dir(&source_path).unwrap();
            git(&binary, &source_path, &["init", "--bare", "-q"], b"");
        },
    );
    assert!(matches!(status, ParentInspection::Uncertain { .. }));
    assert!(
        journal::read_bytes(&world.lease, &world.run_id, "parent")
            .unwrap()
            .is_none()
    );
}

#[test]
fn shallow_source_refuses_before_parent_pending_publication() {
    let Some(binary) = binary() else {
        eprintln!("Git absent; parent fixture skipped");
        return;
    };
    let world = world(&binary, false);
    fs::write(
        world.origin.project.root_path().join("shallow"),
        format!("{}\n", world.origin.main),
    )
    .unwrap();
    assert!(matches!(
        stage_local_parent_pending(
            &world.lease,
            &world.token,
            &world.scope,
            "parent",
            &world.package,
            &world.origin.project,
            &binary,
            limits()
        ),
        ParentInspection::Unavailable { .. }
    ));
    assert!(
        journal::read_bytes(&world.lease, &world.run_id, "parent-pending")
            .unwrap()
            .is_none()
    );
}

#[test]
fn reachable_object_cap_refuses_landed_and_keeps_pending() {
    let Some(binary) = binary() else {
        eprintln!("Git absent; parent fixture skipped");
        return;
    };
    let world = world(&binary, false);
    let mut selected = limits();
    selected.max_objects = 2;
    assert!(matches!(
        stage_local_parent_pending(
            &world.lease,
            &world.token,
            &world.scope,
            "parent",
            &world.package,
            &world.origin.project,
            &binary,
            selected
        ),
        ParentInspection::Uncertain { .. }
    ));
    assert!(
        journal::read_bytes(&world.lease, &world.run_id, "parent-pending")
            .unwrap()
            .is_some()
    );
    assert!(
        journal::read_bytes(&world.lease, &world.run_id, "parent")
            .unwrap()
            .is_none()
    );
    assert!(matches!(
        inspect_parent_cold(
            &world.lease,
            &world.token,
            &world.scope,
            "parent",
            None,
            None,
            &binary,
            limits()
        ),
        ParentInspection::Uncertain { .. }
    ));
}
