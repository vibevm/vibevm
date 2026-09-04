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
