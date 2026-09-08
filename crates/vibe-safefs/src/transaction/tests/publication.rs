macro_rules! publication_tests {
    () => {
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
    };
}
