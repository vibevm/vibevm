use sha2::{Digest, Sha256};

use crate::{Project, PublishStage};

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[test]
fn bounded_create_new_streams_exact_expected_bytes() {
    let source_root = tempfile::tempdir().unwrap();
    let destination_root = tempfile::tempdir().unwrap();
    let bytes = vec![0x5a; 2 * 64 * 1024 + 17];
    std::fs::write(source_root.path().join("bundle"), &bytes).unwrap();
    let source = Project::open(source_root.path()).unwrap();
    let destination = Project::open(destination_root.path()).unwrap();
    let (state, _) = source
        .copy_stable_file_to_expected_new_bounded(
            "bundle",
            &destination,
            "vault/bundle",
            &digest(&bytes),
            bytes.len() as u64,
            bytes.len() as u64,
            None,
        )
        .unwrap();
    assert_eq!(state.bytes, bytes.len() as u64);
    assert_eq!(state.sha256, digest(&bytes));
    assert_eq!(
        std::fs::read(destination_root.path().join("vault/bundle")).unwrap(),
        bytes
    );
}

#[test]
fn bounded_create_new_refuses_oversize_before_destination_mutation() {
    let source_root = tempfile::tempdir().unwrap();
    let destination_root = tempfile::tempdir().unwrap();
    std::fs::write(source_root.path().join("bundle"), b"eight!!!").unwrap();
    let source = Project::open(source_root.path()).unwrap();
    let destination = Project::open(destination_root.path()).unwrap();
    let error = source
        .copy_stable_file_to_expected_new_bounded(
            "bundle",
            &destination,
            "vault/bundle",
            &digest(b"eight!!!"),
            7,
            7,
            None,
        )
        .unwrap_err();
    assert_eq!(error.stage, PublishStage::BeforePublication);
    assert!(!destination_root.path().join("vault").exists());
}

#[test]
fn bounded_create_new_never_replaces_occupied_destination_or_race_winner() {
    let source_root = tempfile::tempdir().unwrap();
    let destination_root = tempfile::tempdir().unwrap();
    std::fs::write(source_root.path().join("bundle"), b"source").unwrap();
    std::fs::create_dir(destination_root.path().join("vault")).unwrap();
    let output = destination_root.path().join("vault/bundle");
    std::fs::write(&output, b"occupant").unwrap();
    let source = Project::open(source_root.path()).unwrap();
    let destination = Project::open(destination_root.path()).unwrap();
    let copy = || {
        source.copy_stable_file_to_expected_new_bounded(
            "bundle",
            &destination,
            "vault/bundle",
            &digest(b"source"),
            6,
            6,
            None,
        )
    };
    assert_eq!(copy().unwrap_err().stage, PublishStage::BeforePublication);
    assert_eq!(std::fs::read(&output).unwrap(), b"occupant");
    std::fs::remove_file(&output).unwrap();
    crate::race_hook::arm_before_link(Some(Box::new(|directory, name| {
        std::fs::write(directory.join(name), b"racer").unwrap();
    })));
    assert_eq!(copy().unwrap_err().stage, PublishStage::BeforePublication);
    assert_eq!(std::fs::read(&output).unwrap(), b"racer");
    assert!(
        std::fs::read_dir(destination_root.path().join("vault"))
            .unwrap()
            .all(|entry| !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".vibe-stage-"))
    );
}

#[cfg(unix)]
#[test]
fn bounded_create_new_refuses_source_symlink_and_hardlink() {
    let source_root = tempfile::tempdir().unwrap();
    let destination_root = tempfile::tempdir().unwrap();
    std::fs::write(source_root.path().join("actual"), b"source").unwrap();
    std::os::unix::fs::symlink("actual", source_root.path().join("link")).unwrap();
    std::fs::hard_link(
        source_root.path().join("actual"),
        source_root.path().join("alias"),
    )
    .unwrap();
    let source = Project::open(source_root.path()).unwrap();
    let destination = Project::open(destination_root.path()).unwrap();
    for name in ["link", "alias"] {
        let error = source
            .copy_stable_file_to_expected_new_bounded(
                name,
                &destination,
                "vault/bundle",
                &digest(b"source"),
                6,
                6,
                None,
            )
            .unwrap_err();
        assert_eq!(error.stage, PublishStage::BeforePublication);
    }
    assert!(!destination_root.path().join("vault").exists());
}

#[test]
fn bounded_create_new_refuses_source_drift_between_passes() {
    let source_root = tempfile::tempdir().unwrap();
    let destination_root = tempfile::tempdir().unwrap();
    let path = source_root.path().join("bundle");
    std::fs::write(&path, b"source").unwrap();
    let source = Project::open(source_root.path()).unwrap();
    let destination = Project::open(destination_root.path()).unwrap();
    crate::race_hook::arm_between_stream_passes(Some(Box::new(move |_, _| {
        std::fs::write(&path, b"change").unwrap();
    })));
    let error = source
        .copy_stable_file_to_expected_new_bounded(
            "bundle",
            &destination,
            "vault/bundle",
            &digest(b"source"),
            6,
            6,
            None,
        )
        .unwrap_err();
    assert_eq!(error.stage, PublishStage::BeforePublication);
    assert!(!destination_root.path().join("vault/bundle").exists());
}

#[cfg(unix)]
#[test]
fn bounded_create_new_same_bytes_destination_replacement_is_uncertain() {
    let source_root = tempfile::tempdir().unwrap();
    let destination_root = tempfile::tempdir().unwrap();
    std::fs::write(source_root.path().join("bundle"), b"source").unwrap();
    let source = Project::open(source_root.path()).unwrap();
    let destination = Project::open(destination_root.path()).unwrap();
    crate::race_hook::arm_before_publish_verify(Some(Box::new(|directory, name| {
        let path = directory.join(name);
        std::fs::remove_file(&path).unwrap();
        std::fs::write(path, b"source").unwrap();
    })));
    let error = source
        .copy_stable_file_to_expected_new_bounded(
            "bundle",
            &destination,
            "vault/bundle",
            &digest(b"source"),
            6,
            6,
            None,
        )
        .unwrap_err();
    assert_eq!(error.stage, PublishStage::PossiblyPublished);
}

#[test]
fn bounded_create_new_post_link_failure_retains_uncertain_visible_file() {
    let source_root = tempfile::tempdir().unwrap();
    let destination_root = tempfile::tempdir().unwrap();
    std::fs::write(source_root.path().join("bundle"), b"source").unwrap();
    let source = Project::open(source_root.path()).unwrap();
    let destination = Project::open(destination_root.path()).unwrap();
    crate::file::fail_after_publish(Some("vault/bundle"));
    let error = source
        .copy_stable_file_to_expected_new_bounded(
            "bundle",
            &destination,
            "vault/bundle",
            &digest(b"source"),
            6,
            6,
            None,
        )
        .unwrap_err();
    crate::file::fail_after_publish(None);
    assert_eq!(error.stage, PublishStage::PossiblyPublished);
    assert_eq!(
        std::fs::read(destination_root.path().join("vault/bundle")).unwrap(),
        b"source"
    );
}

#[test]
fn root_correlation_token_reopens_stably_and_detects_replacement() {
    let parent = tempfile::tempdir().unwrap();
    let root = parent.path().join("root");
    std::fs::create_dir(&root).unwrap();
    let first = Project::open(&root)
        .unwrap()
        .root_correlation_token()
        .unwrap();
    let second = Project::open(&root)
        .unwrap()
        .root_correlation_token()
        .unwrap();
    assert_eq!(first, second);
    assert!(first.starts_with("sha256:") && first.len() == 71);
    std::fs::rename(&root, parent.path().join("old-root")).unwrap();
    std::fs::create_dir(&root).unwrap();
    let replaced = Project::open(&root)
        .unwrap()
        .root_correlation_token()
        .unwrap();
    assert_ne!(first, replaced);
}

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
