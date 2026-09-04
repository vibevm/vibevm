use std::fs;
use std::path::Path;

use crate::{
    CleanupPreparation, DirectoryDurability, ExternalStore, OwnedTreeCleanupError,
    OwnedTreeCleanupProgress, OwnedTreeObservation, Project, RenameError,
    arm_after_owned_tree_publish_move, arm_after_rename_source_check, arm_before_owned_tree_check,
    arm_before_owned_tree_publish, arm_before_rename_noreplace, arm_between_manifest_passes,
    arm_during_manifest_lease, arm_during_native_mutation, arm_same_filesystem_check,
};

fn project() -> (tempfile::TempDir, Project) {
    let root = tempfile::tempdir().unwrap();
    let project = Project::open(root.path()).unwrap();
    (root, project)
}

#[test]
fn project_identity_is_stable_and_never_exposes_raw_os_numbers() {
    let (_root, project) = project();
    let first = project.identity_token().unwrap();
    let second = project.identity_token().unwrap();
    assert_eq!(first, second);
    assert_eq!(first.len(), 71);
    assert!(first.starts_with("sha256:"));
    assert!(first[7..].bytes().all(|byte| byte.is_ascii_hexdigit()));
}

#[test]
fn an_explicit_external_store_is_nofollow_and_proven_disjoint() {
    let scope = tempfile::tempdir().unwrap();
    let project_path = scope.path().join("project");
    let store_path = scope.path().join("state/scrape");
    fs::create_dir(&project_path).unwrap();
    let project = Project::open(&project_path).unwrap();
    let store = ExternalStore::open_or_create_disjoint(&store_path, &project).unwrap();
    store.prove_disjoint_from(&project).unwrap();
    assert_eq!(store.path(), store_path);
    assert_eq!(store.bootstrap_durability().len(), 2);
    assert_eq!(
        store.require_durable_bootstrap().is_ok(),
        store
            .bootstrap_durability()
            .iter()
            .all(|sync| sync.durability == DirectoryDurability::Synced)
    );

    let inside = ExternalStore::open_or_create(&project_path.join("private-state")).unwrap();
    assert!(inside.prove_disjoint_from(&project).is_err());
}

#[test]
fn disjoint_store_preflight_leaves_zero_mutation_for_an_inside_or_linked_path() {
    let scope = tempfile::tempdir().unwrap();
    let project_path = scope.path().join("project");
    fs::create_dir(&project_path).unwrap();
    let project = Project::open(&project_path).unwrap();

    let inside = project_path.join("must-not-exist/store");
    assert!(ExternalStore::open_or_create_disjoint(&inside, &project).is_err());
    assert!(!project_path.join("must-not-exist").exists());

    let alias = scope.path().join("project-alias");
    if link_directory(&project_path, &alias) {
        let linked_store = alias.join("also-must-not-exist/store");
        assert!(ExternalStore::open_or_create_disjoint(&linked_store, &project).is_err());
        assert!(!project_path.join("also-must-not-exist").exists());
    }
}

#[test]
#[cfg(unix)]
fn disjoint_proof_uses_retained_ancestry_after_a_namespace_alias_swap() {
    let scope = tempfile::tempdir().unwrap();
    let project_path = scope.path().join("project");
    let moved_project = scope.path().join("project-moved");
    let store_path = scope.path().join("store");
    fs::create_dir(&project_path).unwrap();
    fs::create_dir(&store_path).unwrap();
    let project = Project::open(&project_path).unwrap();
    let store = ExternalStore::open_or_create_disjoint(&store_path, &project).unwrap();
    fs::rename(&project_path, &moved_project).unwrap();
    if link_directory(&store_path, &project_path) {
        store.prove_disjoint_from(&project).unwrap();
        remove_directory_link(&project_path);
    }
    drop(store);
    drop(project);
    fs::rename(&moved_project, &project_path).unwrap();
}

#[test]
#[cfg(windows)]
fn pinned_project_namespace_prevents_the_alias_swap_on_windows() {
    let scope = tempfile::tempdir().unwrap();
    let project_path = scope.path().join("project");
    let moved_project = scope.path().join("project-moved");
    let store_path = scope.path().join("store");
    fs::create_dir(&project_path).unwrap();
    fs::create_dir(&store_path).unwrap();
    let project = Project::open(&project_path).unwrap();
    let store = ExternalStore::open_or_create_disjoint(&store_path, &project).unwrap();
    assert!(fs::rename(&project_path, &moved_project).is_err());
    store.prove_disjoint_from(&project).unwrap();
    assert!(project_path.is_dir());
}

#[test]
fn external_lock_rechecks_identity_and_durable_writes_report_parent_support() {
    let scope = tempfile::tempdir().unwrap();
    let store = ExternalStore::open_or_create(&scope.path().join("state")).unwrap();
    let _lock = store.open_and_lock_project("sha256:project-key").unwrap();
    let write = store
        .write_durable("transactions/TX0001/journal", b"sealed")
        .unwrap();
    assert!(write.file_synced);
    assert!(matches!(
        write.parent,
        DirectoryDurability::Synced
            | DirectoryDurability::Unsupported(_)
            | DirectoryDurability::Failed(_)
    ));
    assert_eq!(
        fs::read(scope.path().join("state/transactions/TX0001/journal")).unwrap(),
        b"sealed"
    );
}

#[test]
fn same_filesystem_comparison_has_a_deterministic_mismatch_gate() {
    let (_root, project) = project();
    let root = project.root_dir().unwrap();
    let child = root.ensure_child("candidate").unwrap();
    assert!(root.same_filesystem(&child).unwrap());
    arm_same_filesystem_check(Some(Box::new(|actual| {
        assert!(actual);
        false
    })));
    assert!(!root.same_filesystem(&child).unwrap());
    arm_same_filesystem_check(None);
}

#[test]
#[cfg(windows)]
fn post_create_failure_is_created_but_unsealed_never_not_created() {
    let (_root, project) = project();
    let parent = project.root_dir().unwrap();
    crate::arm_after_create_dir(Some(Box::new(|_, _| {
        Some(std::io::Error::other("injected seal failure"))
    })));
    let error = parent
        .create_owned_child_exclusive("candidate", "owner")
        .unwrap_err();
    crate::arm_after_create_dir(None);
    assert!(matches!(
        error,
        crate::OwnedDirectoryCreateError::CreatedButUnsealed { .. }
    ));
    assert!(parent.path().join("candidate").is_dir());
}

#[test]
#[cfg(windows)]
fn held_native_create_cannot_adopt_a_post_create_replacement() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    let (_root, project) = project();
    let parent = project.root_dir().unwrap();
    let replaced = Arc::new(AtomicBool::new(false));
    let observed = Arc::clone(&replaced);
    crate::arm_after_create_dir(Some(Box::new(move |parent, name| {
        let path = parent.join(name);
        if fs::remove_dir(&path).is_ok() {
            fs::create_dir(&path).unwrap();
            observed.store(true, Ordering::SeqCst);
        }
        None
    })));
    let owned = parent
        .create_owned_child_exclusive("candidate", "owner")
        .unwrap();
    crate::arm_after_create_dir(None);
    assert!(!replaced.load(Ordering::SeqCst));
    assert!(owned.path().is_dir());
}

#[test]
#[cfg(windows)]
fn a_raced_rename_occupant_survives_the_atomic_noreplace_attempt() {
    let (root, project) = project();
    let parent = project.root_dir().unwrap();
    fs::create_dir(root.path().join("candidate")).unwrap();
    let expected = parent.inspect_child_state("candidate").unwrap().unwrap();
    let planted = root.path().to_path_buf();
    arm_before_rename_noreplace(Some(Box::new(move |_, _, _, new| {
        fs::write(planted.join(new), b"somebody else's bytes").unwrap();
    })));
    let result = parent.rename_child_noreplace_to(&parent, "candidate", "output", &expected);
    arm_before_rename_noreplace(None);
    assert!(
        matches!(result, Err(RenameError::Occupied { .. })),
        "unexpected rename result: {result:?}"
    );
    assert_eq!(
        fs::read(root.path().join("output")).unwrap(),
        b"somebody else's bytes"
    );
    assert!(root.path().join("candidate").is_dir());
}

#[test]
#[cfg(windows)]
fn an_unoccupied_directory_rename_is_atomic_and_keeps_identity() {
    let (root, project) = project();
    let parent = project.root_dir().unwrap();
    fs::create_dir(root.path().join("candidate")).unwrap();
    fs::write(root.path().join("candidate/payload"), b"bytes").unwrap();
    let expected = parent.inspect_child_state("candidate").unwrap().unwrap();
    parent
        .rename_child_noreplace_to(&parent, "candidate", "output", &expected)
        .unwrap();
    assert!(!root.path().join("candidate").exists());
    assert_eq!(
        fs::read(root.path().join("output/payload")).unwrap(),
        b"bytes"
    );
    assert_eq!(
        parent.inspect_child_state("output").unwrap().unwrap(),
        expected
    );
}

#[test]
#[cfg(windows)]
fn an_expected_file_moves_by_handle_without_reopening_an_ambient_path() {
    let (root, project) = project();
    let parent = project.root_dir().unwrap();
    fs::write(root.path().join("before.bin"), b"expected bytes").unwrap();
    let expected = parent.inspect_child_state("before.bin").unwrap().unwrap();
    parent
        .rename_child_to(&parent, "before.bin", "after.bin", &expected)
        .unwrap();
    assert!(!root.path().join("before.bin").exists());
    assert_eq!(
        fs::read(root.path().join("after.bin")).unwrap(),
        b"expected bytes"
    );
    assert_eq!(
        parent.inspect_child_state("after.bin").unwrap().unwrap(),
        expected
    );
}

#[test]
#[cfg(windows)]
fn native_rename_handle_blocks_delete_races_until_the_move_finishes() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    let (root, project) = project();
    let parent = project.root_dir().unwrap();
    fs::write(root.path().join("before.bin"), b"expected").unwrap();
    let expected = parent.inspect_child_state("before.bin").unwrap().unwrap();
    let blocked = Arc::new(AtomicBool::new(false));
    let observed = Arc::clone(&blocked);
    let path = root.path().to_path_buf();
    arm_during_native_mutation(Some(Box::new(move |_, name| {
        observed.store(
            fs::rename(path.join(name), path.join("stolen.bin")).is_err(),
            Ordering::SeqCst,
        );
    })));
    parent
        .rename_child_to(&parent, "before.bin", "after.bin", &expected)
        .unwrap();
    arm_during_native_mutation(None);
    assert!(blocked.load(Ordering::SeqCst));
    assert!(!root.path().join("before.bin").exists());
    assert!(!root.path().join("stolen.bin").exists());
}

#[test]
#[cfg(windows)]
fn a_preexisting_delete_capable_handle_blocks_native_rename() {
    use std::os::windows::fs::OpenOptionsExt;
    let (root, project) = project();
    let parent = project.root_dir().unwrap();
    let path = root.path().join("before.bin");
    fs::write(&path, b"expected").unwrap();
    let expected = parent.inspect_child_state("before.bin").unwrap().unwrap();
    let held = fs::OpenOptions::new()
        .access_mode(0x0001_0000)
        .share_mode(0x0000_0007)
        .open(&path)
        .unwrap();
    assert!(matches!(
        parent.rename_child_to(&parent, "before.bin", "after.bin", &expected),
        Err(RenameError::Failed(_))
    ));
    drop(held);
    assert!(path.exists());
    assert!(!root.path().join("after.bin").exists());
}

#[cfg(windows)]
fn owned_publication_fixture() -> (
    tempfile::TempDir,
    Project,
    crate::OwnedDirectory,
    crate::ExistingTreeEntryLease,
    crate::TreeManifest,
) {
    let (root, project) = project();
    let parent = project.root_dir().unwrap();
    let owned = parent
        .create_owned_child_exclusive("candidate", "owner")
        .unwrap();
    fs::write(root.path().join("candidate/payload"), b"bytes").unwrap();
    let lease = owned.lease_existing_entries().unwrap();
    let manifest = lease.manifest().clone();
    (root, project, owned, lease, manifest)
}

#[test]
#[cfg(windows)]
fn dedicated_owned_publication_returns_pending_observation_and_entry_lease() {
    let (root, project, owned, lease, manifest) = owned_publication_fixture();
    let identity = owned.identity().clone();
    let published = owned
        .publish_noreplace_to(
            &project.root_dir().unwrap(),
            "output",
            "owner",
            &manifest,
            lease,
        )
        .unwrap();
    assert_eq!(published.entry_lease.manifest(), &manifest);
    assert!(matches!(
        published.reobserve_published(&identity, &manifest).unwrap(),
        OwnedTreeObservation::MatchesAtObservation(_)
    ));
    assert!(!root.path().join("candidate").exists());
    assert_eq!(
        fs::read(root.path().join("output/payload")).unwrap(),
        b"bytes"
    );
}

#[test]
#[cfg(windows)]
fn child_added_after_pending_observation_is_detected_by_final_reobservation() {
    let (root, project, owned, lease, manifest) = owned_publication_fixture();
    let identity = owned.identity().clone();
    let pending = owned
        .publish_noreplace_to(
            &project.root_dir().unwrap(),
            "output",
            "owner",
            &manifest,
            lease,
        )
        .unwrap();
    // Existing-entry handles do not pretend to lock directory membership.
    fs::write(root.path().join("output/after-observation"), b"foreign").unwrap();
    assert!(matches!(
        pending.reobserve_published(&identity, &manifest).unwrap(),
        OwnedTreeObservation::Third { .. }
    ));
    assert_eq!(
        fs::read(root.path().join("output/after-observation")).unwrap(),
        b"foreign"
    );
}

#[test]
#[cfg(windows)]
fn final_reobservation_rejects_a_manifest_other_than_the_pending_one() {
    let (_root, project, owned, lease, manifest) = owned_publication_fixture();
    let identity = owned.identity().clone();
    let pending = owned
        .publish_noreplace_to(
            &project.root_dir().unwrap(),
            "output",
            "owner",
            &manifest,
            lease,
        )
        .unwrap();
    let mut wrong = manifest.clone();
    wrong.digest = format!("sha256:{}", "0".repeat(64));
    assert!(matches!(
        pending.reobserve_published(&identity, &wrong).unwrap(),
        OwnedTreeObservation::Third { .. }
    ));
}

#[test]
#[cfg(windows)]
fn child_created_before_owned_publish_is_never_reported_as_success() {
    let (root, project, owned, lease, manifest) = owned_publication_fixture();
    arm_before_owned_tree_publish(Some(Box::new(|parent, name| {
        fs::write(parent.join(name).join("raced"), b"foreign").unwrap();
    })));
    let result = owned.publish_noreplace_to(
        &project.root_dir().unwrap(),
        "output",
        "owner",
        &manifest,
        lease,
    );
    arm_before_owned_tree_publish(None);
    assert!(matches!(
        result,
        Err(crate::OwnedTreePublishError::PossiblyMoved { .. })
    ));
    assert_eq!(
        fs::read(root.path().join("output/raced")).unwrap(),
        b"foreign"
    );
    assert_eq!(
        fs::read(root.path().join("output/payload")).unwrap(),
        b"bytes"
    );
}

#[test]
#[cfg(windows)]
fn child_created_after_final_root_observation_is_never_published_as_exact() {
    let (root, project, owned, lease, manifest) = owned_publication_fixture();
    arm_after_rename_source_check(Some(Box::new(|parent, _, name, _| {
        fs::write(parent.join(name).join("late"), b"foreign").unwrap();
    })));
    let result = owned.publish_noreplace_to(
        &project.root_dir().unwrap(),
        "output",
        "owner",
        &manifest,
        lease,
    );
    arm_after_rename_source_check(None);
    assert!(matches!(
        result,
        Err(crate::OwnedTreePublishError::PossiblyMoved { .. })
    ));
    assert_eq!(
        fs::read(root.path().join("output/late")).unwrap(),
        b"foreign"
    );
}

#[test]
#[cfg(windows)]
fn post_move_child_race_keeps_output_and_returns_non_success() {
    let (root, project, owned, lease, manifest) = owned_publication_fixture();
    arm_after_owned_tree_publish_move(Some(Box::new(|parent, name| {
        fs::write(parent.join(name).join("post-move"), b"foreign").unwrap();
    })));
    let result = owned.publish_noreplace_to(
        &project.root_dir().unwrap(),
        "output",
        "owner",
        &manifest,
        lease,
    );
    arm_after_owned_tree_publish_move(None);
    assert!(matches!(
        result,
        Err(crate::OwnedTreePublishError::PossiblyMoved { .. })
    ));
    assert_eq!(
        fs::read(root.path().join("output/post-move")).unwrap(),
        b"foreign"
    );
}

#[test]
#[cfg(windows)]
fn a_source_swap_is_refused_even_when_the_replacement_has_the_same_shape() {
    let (root, project) = project();
    let parent = project.root_dir().unwrap();
    fs::create_dir(root.path().join("candidate")).unwrap();
    let expected = parent.inspect_child_state("candidate").unwrap().unwrap();
    let planted = root.path().to_path_buf();
    arm_before_rename_noreplace(Some(Box::new(move |_, _, old, _| {
        fs::rename(planted.join(old), planted.join("old-candidate")).unwrap();
        fs::create_dir(planted.join(old)).unwrap();
    })));
    let result = parent.rename_child_to(&parent, "candidate", "output", &expected);
    arm_before_rename_noreplace(None);
    assert!(matches!(result, Err(RenameError::SourceChanged { .. })));
    assert!(root.path().join("candidate").is_dir());
    assert!(!root.path().join("output").exists());
}

#[test]
#[cfg(windows)]
fn a_swap_after_the_final_source_check_is_never_reported_as_success() {
    let (root, project) = project();
    let parent = project.root_dir().unwrap();
    fs::write(root.path().join("before.bin"), b"expected").unwrap();
    let expected = parent.inspect_child_state("before.bin").unwrap().unwrap();
    let planted = root.path().to_path_buf();
    arm_after_rename_source_check(Some(Box::new(move |_, _, old, _| {
        fs::rename(planted.join(old), planted.join("original.bin")).unwrap();
        fs::write(planted.join(old), b"replacement").unwrap();
    })));
    let result = parent.rename_child_to(&parent, "before.bin", "after.bin", &expected);
    arm_after_rename_source_check(None);
    assert!(matches!(result, Err(RenameError::SourceChanged { .. })));
    assert_eq!(
        fs::read(root.path().join("original.bin")).unwrap(),
        b"expected"
    );
}

#[test]
#[cfg(not(windows))]
fn epoch_one_rename_execution_is_explicitly_unsupported() {
    let (root, project) = project();
    let parent = project.root_dir().unwrap();
    fs::create_dir(root.path().join("candidate")).unwrap();
    let expected = parent.inspect_child_state("candidate").unwrap().unwrap();
    assert!(matches!(
        parent.rename_child_noreplace_to(&parent, "candidate", "output", &expected),
        Err(RenameError::Unsupported)
    ));
    assert!(root.path().join("candidate").is_dir());
}

#[test]
fn a_volume_mismatch_refuses_before_rename() {
    let (root, project) = project();
    let parent = project.root_dir().unwrap();
    fs::create_dir(root.path().join("candidate")).unwrap();
    let expected = parent.inspect_child_state("candidate").unwrap().unwrap();
    arm_same_filesystem_check(Some(Box::new(|_| false)));
    let result = parent.rename_child_to(&parent, "candidate", "output", &expected);
    arm_same_filesystem_check(None);
    assert!(matches!(result, Err(RenameError::CrossFilesystem)));
    assert!(root.path().join("candidate").exists());
    assert!(!root.path().join("output").exists());
}

#[cfg(windows)]
fn sealed_owned_tree() -> (
    tempfile::TempDir,
    Project,
    crate::OwnedDirectoryIdentity,
    crate::TreeManifest,
) {
    let (root, project) = project();
    let parent = project.root_dir().unwrap();
    let owned = parent
        .create_owned_child_exclusive("candidate", "durable-owner-token")
        .unwrap();
    let directory = owned.directory().unwrap();
    let view = Project::open(directory.path()).unwrap();
    view.write_atomic("a.txt", b"same bytes").unwrap();
    view.write_atomic("nested/b.txt", b"payload").unwrap();
    let identity = owned.identity().clone();
    let manifest = owned.manifest().unwrap();
    drop(owned);
    (root, project, identity, manifest)
}

#[test]
#[cfg(windows)]
fn exact_manifest_cleanup_removes_only_the_owned_tree() {
    let (root, project, identity, manifest) = sealed_owned_tree();
    fs::write(root.path().join("neighbour"), b"keep").unwrap();
    let parent = project.root_dir().unwrap();
    let mut progress = OwnedTreeCleanupProgress::new();
    loop {
        let prepared = parent
            .prepare_owned_tree_cleanup_next(
                "candidate",
                "durable-owner-token",
                &identity,
                &manifest,
                &progress,
            )
            .unwrap();
        let CleanupPreparation::Intent(intent) = prepared else {
            break;
        };
        // The intent is the value the transaction journals before the syscall.
        let completion = parent
            .execute_owned_tree_cleanup_intent(
                "candidate",
                "durable-owner-token",
                &identity,
                &manifest,
                &progress,
                &intent,
            )
            .unwrap();
        progress.record(&completion).unwrap();
        // Simulate a process restart after every durably recorded entry.
        progress = OwnedTreeCleanupProgress::from_completed(progress.completed().to_vec()).unwrap();
    }
    assert!(!root.path().join("candidate").exists());
    assert_eq!(fs::read(root.path().join("neighbour")).unwrap(), b"keep");
}

#[test]
#[cfg(windows)]
fn crash_after_syscall_before_completion_record_recovers_from_inflight_intent() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    let (root, project, identity, manifest) = sealed_owned_tree();
    let parent = project.root_dir().unwrap();
    let progress = OwnedTreeCleanupProgress::new();
    let CleanupPreparation::Intent(intent) = parent
        .prepare_owned_tree_cleanup_next(
            "candidate",
            "durable-owner-token",
            &identity,
            &manifest,
            &progress,
        )
        .unwrap()
    else {
        panic!("an entry intent is required")
    };
    let blocked = Arc::new(AtomicBool::new(false));
    let observed = Arc::clone(&blocked);
    arm_during_native_mutation(Some(Box::new(move |parent, name| {
        observed.store(
            fs::remove_file(parent.join(name)).is_err(),
            Ordering::SeqCst,
        );
    })));
    parent
        .execute_owned_tree_cleanup_intent(
            "candidate",
            "durable-owner-token",
            &identity,
            &manifest,
            &progress,
            &intent,
        )
        .unwrap();
    arm_during_native_mutation(None);
    assert!(blocked.load(Ordering::SeqCst));
    let retry = parent
        .execute_owned_tree_cleanup_intent(
            "candidate",
            "durable-owner-token",
            &identity,
            &manifest,
            &progress,
            &intent,
        )
        .unwrap();
    assert!(retry.recovered_after_syscall);
    assert!(root.path().join("candidate").exists());
}

#[test]
#[cfg(windows)]
fn an_added_descendant_is_a_third_state_and_the_whole_tree_survives() {
    let (root, project, identity, manifest) = sealed_owned_tree();
    let planted = root.path().to_path_buf();
    arm_before_owned_tree_check(Some(Box::new(move |_, name| {
        fs::write(planted.join(name).join("concurrent.txt"), b"foreign").unwrap();
    })));
    let result = project.root_dir().unwrap().prepare_owned_tree_cleanup_next(
        "candidate",
        "durable-owner-token",
        &identity,
        &manifest,
        &OwnedTreeCleanupProgress::new(),
    );
    arm_before_owned_tree_check(None);
    assert!(matches!(result, Err(OwnedTreeCleanupError::Third { .. })));
    assert_eq!(
        fs::read(root.path().join("candidate/concurrent.txt")).unwrap(),
        b"foreign"
    );
    assert!(root.path().join("candidate/a.txt").exists());
}

#[test]
#[cfg(windows)]
fn same_bytes_under_a_different_file_identity_are_a_third_state() {
    let (root, project, identity, manifest) = sealed_owned_tree();
    let file = root.path().join("candidate/a.txt");
    fs::remove_file(&file).unwrap();
    fs::write(&file, b"same bytes").unwrap();
    let result = project.root_dir().unwrap().prepare_owned_tree_cleanup_next(
        "candidate",
        "durable-owner-token",
        &identity,
        &manifest,
        &OwnedTreeCleanupProgress::new(),
    );
    assert!(matches!(result, Err(OwnedTreeCleanupError::Third { .. })));
    assert_eq!(fs::read(file).unwrap(), b"same bytes");
}

#[test]
#[cfg(windows)]
fn an_early_entry_replacement_between_complete_manifest_passes_refuses_sealing() {
    let (root, project) = project();
    let parent = project.root_dir().unwrap();
    let owned = parent
        .create_owned_child_exclusive("candidate", "owner")
        .unwrap();
    fs::write(root.path().join("candidate/a.txt"), b"same").unwrap();
    let planted = root.path().to_path_buf();
    arm_between_manifest_passes(Some(Box::new(move |_| {
        let path = planted.join("candidate/a.txt");
        fs::remove_file(&path).unwrap();
        fs::write(path, b"same").unwrap();
    })));
    let result = owned.manifest();
    arm_between_manifest_passes(None);
    assert!(result.is_err());
    assert_eq!(
        fs::read(root.path().join("candidate/a.txt")).unwrap(),
        b"same"
    );
}

#[test]
#[cfg(windows)]
fn manifest_lease_blocks_an_early_entry_mutation_through_the_later_scan() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    let (root, project) = project();
    let parent = project.root_dir().unwrap();
    let owned = parent
        .create_owned_child_exclusive("candidate", "owner")
        .unwrap();
    fs::write(root.path().join("candidate/a.txt"), b"same").unwrap();
    let blocked = Arc::new(AtomicBool::new(false));
    let observed = Arc::clone(&blocked);
    let planted = root.path().to_path_buf();
    arm_during_manifest_lease(Some(Box::new(move |_| {
        observed.store(
            fs::write(planted.join("candidate/a.txt"), b"changed").is_err(),
            Ordering::SeqCst,
        );
    })));
    let lease = owned.lease_existing_entries().unwrap();
    arm_during_manifest_lease(None);
    assert!(blocked.load(Ordering::SeqCst));
    assert!(matches!(
        parent
            .observe_owned_tree(
                "candidate",
                "owner",
                owned.identity(),
                lease.manifest(),
                &lease,
            )
            .unwrap(),
        OwnedTreeObservation::MatchesAtObservation(_)
    ));
}

#[test]
#[cfg(windows)]
fn a_missing_manifest_descendant_is_a_third_state_and_remaining_data_survives() {
    let (root, project, identity, manifest) = sealed_owned_tree();
    fs::remove_file(root.path().join("candidate/a.txt")).unwrap();
    let result = project.root_dir().unwrap().prepare_owned_tree_cleanup_next(
        "candidate",
        "durable-owner-token",
        &identity,
        &manifest,
        &OwnedTreeCleanupProgress::new(),
    );
    assert!(matches!(result, Err(OwnedTreeCleanupError::Third { .. })));
    assert_eq!(
        fs::read(root.path().join("candidate/nested/b.txt")).unwrap(),
        b"payload"
    );
}

#[test]
#[cfg(windows)]
fn a_root_swap_is_a_third_state_even_with_the_same_descendants() {
    let (root, project, identity, manifest) = sealed_owned_tree();
    let candidate = root.path().join("candidate");
    let old = root.path().join("old-candidate");
    fs::rename(&candidate, &old).unwrap();
    fs::create_dir(&candidate).unwrap();
    fs::create_dir(candidate.join("nested")).unwrap();
    fs::write(candidate.join("a.txt"), b"same bytes").unwrap();
    fs::write(candidate.join("nested/b.txt"), b"payload").unwrap();
    let result = project.root_dir().unwrap().prepare_owned_tree_cleanup_next(
        "candidate",
        "durable-owner-token",
        &identity,
        &manifest,
        &OwnedTreeCleanupProgress::new(),
    );
    assert!(matches!(result, Err(OwnedTreeCleanupError::Third { .. })));
    assert!(candidate.join("a.txt").exists());
    assert!(old.join("a.txt").exists());
}

#[test]
#[cfg(windows)]
fn links_and_hardlinks_refuse_complete_manifest_ownership() {
    let (root, project) = project();
    let parent = project.root_dir().unwrap();
    let owned = parent
        .create_owned_child_exclusive("candidate", "owner")
        .unwrap();
    fs::write(root.path().join("candidate/original"), b"shared").unwrap();
    fs::hard_link(
        root.path().join("candidate/original"),
        root.path().join("candidate/alias"),
    )
    .unwrap();
    assert!(owned.manifest().is_err());

    fs::remove_file(root.path().join("candidate/alias")).unwrap();
    fs::remove_file(root.path().join("candidate/original")).unwrap();
    fs::write(root.path().join("outside"), b"outside").unwrap();
    if link_file(
        &root.path().join("outside"),
        &root.path().join("candidate/link"),
    ) {
        assert!(owned.manifest().is_err());
        assert_eq!(fs::read(root.path().join("outside")).unwrap(), b"outside");
    }
}

#[cfg(unix)]
#[test]
fn strong_owned_directory_create_is_explicitly_unsupported() {
    let (_root, project) = project();
    let parent = project.root_dir().unwrap();
    assert!(matches!(
        parent.create_owned_child_exclusive("candidate", "owner"),
        Err(crate::OwnedDirectoryCreateError::Unsupported)
    ));
    assert!(!parent.path().join("candidate").exists());
}

#[cfg(unix)]
fn link_file(target: &Path, link: &Path) -> bool {
    std::os::unix::fs::symlink(target, link).is_ok()
}

#[cfg(unix)]
fn link_directory(target: &Path, link: &Path) -> bool {
    std::os::unix::fs::symlink(target, link).is_ok()
}

#[cfg(unix)]
fn remove_directory_link(link: &Path) {
    fs::remove_file(link).unwrap();
}

#[cfg(windows)]
fn link_directory(target: &Path, link: &Path) -> bool {
    let link = link.to_string_lossy().replace('/', "\\");
    std::process::Command::new("cmd")
        .args(["/c", "mklink", "/J"])
        .arg(link)
        .arg(target)
        .output()
        .is_ok_and(|output| output.status.success())
}

#[cfg(windows)]
fn link_file(target: &Path, link: &Path) -> bool {
    std::os::windows::fs::symlink_file(target, link).is_ok()
}
