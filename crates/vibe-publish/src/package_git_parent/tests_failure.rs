use super::*;
use crate::package_git_objects::{self, ObjectInspection};
use crate::package_git_ready::{self, ReadyInspection, ReadyLimits};
use crate::package_git_tree::git::LocalGit;

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
    assert!(matches!(
        package_git_objects::stage_existing_main_objects_pending(
            &world.lease,
            &world.token,
            &world.scope,
            "parent",
            &world.package,
            &world.origin.project,
            &binary,
            limits()
        ),
        ObjectInspection::Uncertain { .. }
    ));
    assert!(
        journal::read_bytes(&world.lease, &world.run_id, "objects")
            .unwrap()
            .is_none()
    );
}

#[test]
fn imported_parent_precedes_new_child_commit_and_tag_cold_proof() {
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
    assert!(matches!(
        package_git_objects::stage_existing_main_objects_pending(
            &world.lease,
            &world.token,
            &world.scope,
            "parent",
            &world.package,
            &world.origin.project,
            &binary,
            limits()
        ),
        ObjectInspection::ObjectsCompleteButNotReady {
            source_reobserved: true,
            ..
        }
    ));
    let child = journal::read::<journal::ObjectReceipt>(&world.lease, &world.run_id, "objects")
        .unwrap()
        .unwrap();
    let parent_bytes = journal::read_bytes(&world.lease, &world.run_id, "parent")
        .unwrap()
        .unwrap();
    assert_eq!(
        child.parent_oid.as_deref(),
        Some(world.origin.main.as_str())
    );
    assert_eq!(
        child.parent_receipt_sha256.as_deref(),
        Some(sha(&parent_bytes).as_str())
    );
    assert_eq!(child.commit_oid, world.plan.staged.tag_target_commit_oid);
    assert!(matches!(
        package_git_objects::inspect_existing_main_objects_cold(
            &world.lease,
            &world.token,
            &world.scope,
            "parent",
            None,
            None,
            &binary,
            limits()
        ),
        ObjectInspection::ObjectsCompleteButNotReady {
            source_reobserved: false,
            ..
        }
    ));
}

#[test]
fn exact_parented_new_commit_seals_ready_then_cold_rechecks_without_sources() {
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
    assert!(matches!(
        package_git_objects::stage_existing_main_objects_pending(
            &world.lease,
            &world.token,
            &world.scope,
            "parent",
            &world.package,
            &world.origin.project,
            &binary,
            limits()
        ),
        ObjectInspection::ObjectsCompleteButNotReady { .. }
    ));
    let selected = ReadyLimits {
        census: limits().census,
        parent: Some(limits()),
    };
    let sealed = package_git_ready::seal_local_ready(
        &world.lease,
        &world.token,
        &world.scope,
        "parent",
        &world.package,
        Some(&world.origin.project),
        &binary,
        selected,
    );
    assert!(
        matches!(sealed, ReadyInspection::ReadyForApproval { .. }),
        "{sealed:?}"
    );
    assert!(matches!(
        package_git_ready::inspect_ready_seal_cold(
            &world.lease,
            &world.token,
            &world.scope,
            "parent",
            None,
            None,
            &binary,
            selected
        ),
        ReadyInspection::ReadyForApproval { .. }
    ));
    fs::write(
        world.root.path().join(format!(
            ".vibe/package-git-operations/{}.package-git-parent.json",
            world.run_id
        )),
        b"not-json",
    )
    .unwrap();
    assert!(matches!(
        package_git_ready::inspect_ready_seal_cold(
            &world.lease,
            &world.token,
            &world.scope,
            "parent",
            None,
            None,
            &binary,
            selected
        ),
        ReadyInspection::RecoveryRequired { .. }
    ));
}

#[test]
fn reused_main_noop_cannot_manufacture_new_object_ready_seal() {
    let Some(binary) = binary() else {
        eprintln!("Git absent; parent fixture skipped");
        return;
    };
    let world = world(&binary, true);
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
    let selected = ReadyLimits {
        census: limits().census,
        parent: Some(limits()),
    };
    let status = package_git_ready::seal_local_ready(
        &world.lease,
        &world.token,
        &world.scope,
        "parent",
        &world.package,
        Some(&world.origin.project),
        &binary,
        selected,
    );
    assert!(
        matches!(status, ReadyInspection::Unavailable { .. }),
        "{status:?}"
    );
    assert!(
        journal::read_bytes(&world.lease, &world.run_id, "ready")
            .unwrap()
            .is_none()
    );
    assert!(matches!(
        crate::package_git_operation::inspect_pending_cold(
            &world.lease,
            &world.token,
            &world.scope,
            "parent",
            Some(&world.package)
        )
        .unwrap(),
        crate::package_git_operation::PendingInspection::VaultCompleteButNotReady { .. }
    ));
}

#[test]
fn missing_or_changed_parent_receipt_refuses_child_before_object_write() {
    let Some(binary) = binary() else {
        eprintln!("Git absent; parent fixture skipped");
        return;
    };
    for corrupt in [false, true] {
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
        let receipt = world.root.path().join(format!(
            ".vibe/package-git-operations/{}.package-git-parent.json",
            world.run_id
        ));
        if corrupt {
            fs::write(&receipt, b"not-json").unwrap();
        } else {
            fs::remove_file(&receipt).unwrap();
        }
        assert!(matches!(
            package_git_objects::stage_existing_main_objects_pending(
                &world.lease,
                &world.token,
                &world.scope,
                "parent",
                &world.package,
                &world.origin.project,
                &binary,
                limits()
            ),
            ObjectInspection::Uncertain { .. }
        ));
        let objects = journal::objects(&world.lease, &world.run_id, false)
            .unwrap()
            .unwrap();
        let git = LocalGit::new(&binary, &objects, world.plan.oid_hex_len).unwrap();
        assert!(
            !git.object_present(&world.plan.staged.tag_target_commit_oid)
                .unwrap()
        );
        assert!(
            !git.object_present(world.plan.staged.annotated_tag_oid.as_deref().unwrap())
                .unwrap()
        );
        assert!(
            journal::read_bytes(&world.lease, &world.run_id, "objects")
                .unwrap()
                .is_none()
        );
    }
}

#[test]
fn changed_local_bare_refs_after_parent_landed_refuse_child() {
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
    let source = world.origin.project.root_path().to_str().unwrap();
    git(
        &binary,
        world.origin.project.root_path(),
        &[
            "--git-dir",
            source,
            "update-ref",
            "refs/heads/main",
            &world.origin.previous,
        ],
        b"",
    );
    assert!(matches!(
        package_git_objects::stage_existing_main_objects_pending(
            &world.lease,
            &world.token,
            &world.scope,
            "parent",
            &world.package,
            &world.origin.project,
            &binary,
            limits()
        ),
        ObjectInspection::Uncertain { .. }
    ));
    assert!(
        journal::read_bytes(&world.lease, &world.run_id, "objects")
            .unwrap()
            .is_none()
    );
}
