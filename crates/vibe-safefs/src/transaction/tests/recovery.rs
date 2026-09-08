macro_rules! recovery_tests {
    () => {
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
fn owned_directory_reopens_after_restart_from_validated_opaque_evidence() {
    let (root, project, identity, manifest) = sealed_owned_tree();
    let persisted_identity = crate::OwnedDirectoryIdentity::from_token(identity.as_str()).unwrap();
    let persisted_manifest =
        crate::TreeManifest::from_persisted(manifest.digest.clone(), manifest.entries.clone())
            .unwrap();
    let reopened = project
        .root_dir()
        .unwrap()
        .reopen_owned_child(
            "candidate",
            "durable-owner-token",
            &persisted_identity,
            &persisted_manifest,
        )
        .unwrap();
    assert_eq!(reopened.identity(), &persisted_identity);
    assert_eq!(reopened.manifest(), &persisted_manifest);
    assert_eq!(
        reopened
            .directory()
            .unwrap()
            .inspect_child_state("a.txt")
            .unwrap()
            .unwrap()
            .bytes,
        Some(10)
    );
    assert!(root.path().join("candidate").is_dir());
}

#[test]
#[cfg(windows)]
fn recovery_rebind_refuses_a_foreign_root_replacement_with_same_bytes() {
    let (root, project, identity, manifest) = sealed_owned_tree();
    let candidate = root.path().join("candidate");
    let original = root.path().join("original-candidate");
    fs::rename(&candidate, &original).unwrap();
    fs::create_dir_all(candidate.join("nested")).unwrap();
    fs::write(candidate.join("a.txt"), b"same bytes").unwrap();
    fs::write(candidate.join("nested/b.txt"), b"payload").unwrap();
    let error = project
        .root_dir()
        .unwrap()
        .reopen_owned_child("candidate", "durable-owner-token", &identity, &manifest)
        .unwrap_err();
    assert!(matches!(
        error,
        crate::ReopenOwnedDirectoryError::Third { .. }
    ));
    assert!(candidate.join("a.txt").exists());
    assert!(original.join("a.txt").exists());
}

#[test]
#[cfg(windows)]
fn recovery_rebind_refuses_an_added_descendant_without_removing_it() {
    let (root, project, identity, manifest) = sealed_owned_tree();
    fs::write(root.path().join("candidate/foreign"), b"foreign").unwrap();
    let error = project
        .root_dir()
        .unwrap()
        .reopen_owned_child("candidate", "durable-owner-token", &identity, &manifest)
        .unwrap_err();
    assert!(matches!(
        error,
        crate::ReopenOwnedDirectoryError::Third { .. }
    ));
    assert_eq!(
        fs::read(root.path().join("candidate/foreign")).unwrap(),
        b"foreign"
    );
}

#[test]
#[cfg(windows)]
fn recovery_rebind_refuses_a_junction_at_the_owned_root() {
    let (root, project, identity, manifest) = sealed_owned_tree();
    let candidate = root.path().join("candidate");
    let original = root.path().join("original-candidate");
    fs::rename(&candidate, &original).unwrap();
    if link_directory(&original, &candidate) {
        let error = project
            .root_dir()
            .unwrap()
            .reopen_owned_child("candidate", "durable-owner-token", &identity, &manifest)
            .unwrap_err();
        assert!(matches!(
            error,
            crate::ReopenOwnedDirectoryError::Third { .. }
        ));
        assert!(original.join("a.txt").exists());
    }
}

#[test]
#[cfg(not(windows))]
fn recovery_rebind_is_explicitly_unsupported_off_windows() {
    let (_root, project) = project();
    let identity =
        crate::OwnedDirectoryIdentity::from_token(&format!("sha256:{}", "0".repeat(64))).unwrap();
    let manifest = crate::TreeManifest {
        digest: format!("sha256:{}", "0".repeat(64)),
        entries: Vec::new(),
    };
    assert!(matches!(
        project
            .root_dir()
            .unwrap()
            .reopen_owned_child("candidate", "owner", &identity, &manifest,),
        Err(crate::ReopenOwnedDirectoryError::Unsupported)
    ));
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
    };
}
