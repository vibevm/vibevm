use sha2::{Digest, Sha256};

use crate::Project;

#[test]
fn a_stable_source_streams_to_an_atomically_verified_destination() {
    let source_root = tempfile::tempdir().expect("source tempdir");
    let destination_root = tempfile::tempdir().expect("destination tempdir");
    let bytes = vec![0x5a; 2 * 64 * 1024 + 17];
    std::fs::write(source_root.path().join("source.bin"), &bytes).expect("source writes");
    let source = Project::open(source_root.path()).expect("source capability opens");
    let destination = Project::open(destination_root.path()).expect("destination opens");

    let (state, _) = source
        .copy_stable_file_to("source.bin", &destination, "nested/output.bin", None)
        .expect("the held source copies");

    assert_eq!(state.bytes, bytes.len() as u64);
    assert_eq!(state.sha256, format!("{:x}", Sha256::digest(&bytes)));
    assert_eq!(
        std::fs::read(destination_root.path().join("nested/output.bin")).unwrap(),
        bytes,
    );
}

#[test]
fn bounded_stable_observation_accepts_exact_limit_and_refuses_one_byte_less() {
    let root = tempfile::tempdir().expect("tempdir");
    let bytes = vec![0x5a; 2 * 64 * 1024 + 17];
    std::fs::write(root.path().join("bundle.bin"), &bytes).expect("fixture writes");
    let project = Project::open(root.path()).expect("capability opens");
    let exact = project
        .stable_file_state_bounded("bundle.bin", bytes.len() as u64)
        .expect("exact bound observes")
        .expect("file is present");
    assert_eq!(exact.bytes, bytes.len() as u64);
    assert_eq!(exact.sha256, format!("{:x}", Sha256::digest(&bytes)));
    assert_eq!(
        project.stable_file_state("bundle.bin").unwrap(),
        Some(exact)
    );
    let below = (bytes.len() - 1) as u64;
    let refusal = project
        .stable_file_state_bounded("bundle.bin", below)
        .expect_err("over-cap file refuses");
    assert!(format!("{refusal:#}").contains(&format!("over the {below}-byte cap")));
    assert!(project.stable_file_state_bounded("bundle.bin", 0).is_err());
}

#[cfg(unix)]
#[test]
fn bounded_stable_observation_refuses_symlink_and_hardlink_aliases() {
    let root = tempfile::tempdir().expect("tempdir");
    std::fs::write(root.path().join("actual.bin"), b"payload").unwrap();
    std::os::unix::fs::symlink("actual.bin", root.path().join("linked.bin")).unwrap();
    std::fs::hard_link(
        root.path().join("actual.bin"),
        root.path().join("hardlinked.bin"),
    )
    .unwrap();
    let project = Project::open(root.path()).unwrap();
    assert!(
        project
            .stable_file_state_bounded("linked.bin", 1024)
            .is_err()
    );
    assert!(
        project
            .stable_file_state_bounded("hardlinked.bin", 1024)
            .is_err()
    );
}

#[cfg(unix)]
#[test]
fn opt_in_linked_regular_snapshot_keeps_cap_and_default_refusal() {
    let root = tempfile::tempdir().unwrap();
    let bytes = b"cargo output";
    std::fs::write(root.path().join("program"), bytes).unwrap();
    std::fs::hard_link(
        root.path().join("program"),
        root.path().join("program-deps"),
    )
    .unwrap();
    let project = Project::open(root.path()).unwrap();
    assert!(
        project
            .read_file_snapshot_bounded("program", bytes.len())
            .is_err()
    );
    let linked = project
        .read_linked_regular_snapshot_bounded("program", bytes.len())
        .unwrap()
        .unwrap();
    assert_eq!(linked.bytes, bytes);
    assert_eq!(linked.sha256, format!("{:x}", Sha256::digest(bytes)));
    assert!(
        project
            .read_linked_regular_snapshot_bounded("program", bytes.len() - 1)
            .is_err()
    );
    assert_eq!(
        project
            .read_linked_regular_snapshot_bounded("missing", 16)
            .unwrap(),
        None
    );
}

#[cfg(unix)]
#[test]
fn opt_in_linked_regular_snapshot_still_refuses_symlink_and_directory() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("actual"), b"payload").unwrap();
    std::os::unix::fs::symlink("actual", root.path().join("link")).unwrap();
    std::fs::create_dir(root.path().join("directory")).unwrap();
    let project = Project::open(root.path()).unwrap();
    assert!(
        project
            .read_linked_regular_snapshot_bounded("link", 16)
            .is_err()
    );
    assert!(
        project
            .read_linked_regular_snapshot_bounded("directory", 16)
            .is_err()
    );
}

#[cfg(unix)]
#[test]
fn exact_unix_mode_is_published_before_the_destination_becomes_visible() {
    use std::os::unix::fs::PermissionsExt;

    let root = tempfile::tempdir().expect("tempdir");
    let project = Project::open(root.path()).expect("capability opens");
    project
        .write_atomic_with_mode("launcher", b"#!/bin/sh\n", Some(0o755))
        .expect("the mode-aware publication succeeds");
    let state = project
        .stable_file_state("launcher")
        .expect("the launcher observes")
        .expect("the launcher exists");
    assert_eq!(state.unix_mode, Some(0o755));
    assert_eq!(
        std::fs::metadata(root.path().join("launcher"))
            .unwrap()
            .permissions()
            .mode()
            & 0o7777,
        0o755,
    );
}
