macro_rules! native_tests {
    () => {
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
    };
}
