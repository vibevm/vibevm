//! Verified bundle placement and tampering regressions.
#![cfg(test)]

use super::*;

#[test]
fn verified_bundle_installs_all_tools_source_shims_and_force_refresh() {
    let temp = tempfile::tempdir().unwrap();
    let source = source_zip(None);
    let fixture = write_bundle(temp.path(), b"vibe-binary", b"vibe-binary", &source, false);
    let store = VersionStore::new(temp.path().join("opt"));

    let first = install_bundle(
        &store,
        &fixture.path,
        &fixture.manifest,
        &fixture.asset,
        false,
    )
    .unwrap();
    assert!(!first.reused);
    assert_eq!(first.record.selector().to_string(), "tag:1.0.0#1");
    assert_eq!(
        std::fs::read(store.binary_path(&first.record.version_id(), 1)).unwrap(),
        b"vibe-binary"
    );
    assert_eq!(
        std::fs::read(store.index_binary_path(&first.record.version_id(), 1)).unwrap(),
        b"index-binary"
    );
    assert_eq!(
        std::fs::read(store.app_proxy_binary_path(&first.record.version_id(), 1)).unwrap(),
        b"proxy-binary"
    );
    assert!(
        store
            .instance_source_dir(&first.record.version_id(), 1)
            .join("Cargo.toml")
            .is_file()
    );
    assert_eq!(first.record.commit, COMMIT);
    assert_eq!(
        PathBuf::from(first.record.source_path.as_deref().unwrap()),
        store
            .instance_source_dir(&first.record.version_id(), 1)
            .canonicalize()
            .unwrap()
    );
    let stored_manifest = BundleDistributionManifest::from_json_slice(
        &std::fs::read(first.home.join(DISTRIBUTION_MANIFEST_FILENAME)).unwrap(),
    )
    .unwrap();
    assert_eq!(stored_manifest.source_archive.tree_oid, TREE);
    assert!(super::super::archive::installed_bundle_intact(
        &store,
        &first.record
    ));
    let mut swapped_identity = first.record.clone();
    swapped_identity.commit = "f".repeat(40);
    assert!(!super::super::archive::installed_bundle_intact(
        &store,
        &swapped_identity
    ));
    let failing = FakePersister::new(true);
    let error = activate_install(&store, &first, &failing, false)
        .unwrap_err()
        .to_string();
    assert!(error.contains("injected PATH persistence failure"));
    assert!(store.active().unwrap().is_none(), "pointer flips last");
    assert!(
        failing.homes.borrow().is_empty(),
        "version-specific HOME is not written before generic PATH succeeds"
    );

    let persister = FakePersister::new(false);
    let activation = activate_install(&store, &first, &persister, false).unwrap();
    assert!(activation.durable_path_changed);
    assert!(!activation.path_on_current_process);
    assert!(store.shim_dir().join("vibe").is_file());
    assert!(store.shim_dir().join("vibe-index").is_file());
    assert_eq!(store.active().unwrap().unwrap().instance, 1);

    let reused = install_bundle(
        &store,
        &fixture.path,
        &fixture.manifest,
        &fixture.asset,
        false,
    )
    .unwrap();
    assert!(reused.reused);
    assert_eq!(reused.record.instance, 1);
    activate_install(&store, &reused, &persister, true).unwrap();

    let forced = install_bundle(
        &store,
        &fixture.path,
        &fixture.manifest,
        &fixture.asset,
        true,
    )
    .unwrap();
    assert!(!forced.reused);
    let advisory_failure = FakePersister::home_failure();
    let activation = activate_install(&store, &forced, &advisory_failure, true).unwrap();
    assert!(activation.advisory_home_warning.is_some());
    assert_eq!(store.active().unwrap().unwrap().instance, 2);
    assert_eq!(forced.record.selector().to_string(), "tag:1.0.0#2");
    assert_eq!(store.previous().unwrap().unwrap().instance, 1);
    assert!(
        store
            .instance_dir(&VersionId::new(Kind::Tag, "1.0.0"), 1)
            .is_dir()
    );
    store.write_current(&first.home).unwrap();
    assert_eq!(store.active().unwrap().unwrap().instance, 1);
    assert_eq!(store.previous().unwrap().unwrap().instance, 2);
}

#[test]
fn tampered_immutable_instance_is_never_reused_or_repaired() {
    for mutation in 0..6 {
        let temp = tempfile::tempdir().unwrap();
        let source = source_zip(None);
        let fixture = write_bundle(temp.path(), b"vibe-binary", b"vibe-binary", &source, false);
        let store = VersionStore::new(temp.path().join("opt"));
        let first = install_bundle(
            &store,
            &fixture.path,
            &fixture.manifest,
            &fixture.asset,
            false,
        )
        .unwrap();
        let home = &first.home;
        let tampered = match mutation {
            0 => store.binary_path(&first.record.version_id(), 1),
            1 => store.index_binary_path(&first.record.version_id(), 1),
            2 => store.app_proxy_binary_path(&first.record.version_id(), 1),
            3 => home.join(DISTRIBUTION_MANIFEST_FILENAME),
            4 => home.join("source/Cargo.toml"),
            5 => home.join(DISTRIBUTION_SOURCE_ARCHIVE_FILENAME),
            _ => unreachable!(),
        };
        std::fs::write(&tampered, b"tampered").unwrap();

        let replacement = install_bundle(
            &store,
            &fixture.path,
            &fixture.manifest,
            &fixture.asset,
            false,
        )
        .unwrap();
        assert!(!replacement.reused, "mutation {mutation} was reused");
        assert_eq!(replacement.record.instance, 2);
        assert_eq!(std::fs::read(&tampered).unwrap(), b"tampered");
    }
}

#[cfg(unix)]
#[test]
fn executable_mode_and_matching_byte_symlink_tamper_never_reuse() {
    use std::os::unix::fs::{PermissionsExt, symlink};

    let temp = tempfile::tempdir().unwrap();
    let source = source_zip(None);
    let fixture = write_bundle(temp.path(), b"vibe-binary", b"vibe-binary", &source, false);
    let store = VersionStore::new(temp.path().join("opt"));
    let first = install_bundle(
        &store,
        &fixture.path,
        &fixture.manifest,
        &fixture.asset,
        false,
    )
    .unwrap();
    let vibe = store.binary_path(&first.record.version_id(), first.record.instance);
    let mut permissions = std::fs::metadata(&vibe).unwrap().permissions();
    permissions.set_mode(0o644);
    std::fs::set_permissions(&vibe, permissions).unwrap();
    let second = install_bundle(
        &store,
        &fixture.path,
        &fixture.manifest,
        &fixture.asset,
        false,
    )
    .unwrap();
    assert_eq!(second.record.instance, 2);

    let cargo = second.home.join("source/Cargo.toml");
    let copy = second.home.join("same-bytes");
    std::fs::copy(&cargo, &copy).unwrap();
    std::fs::remove_file(&cargo).unwrap();
    symlink(&copy, &cargo).unwrap();
    let third = install_bundle(
        &store,
        &fixture.path,
        &fixture.manifest,
        &fixture.asset,
        false,
    )
    .unwrap();
    assert_eq!(third.record.instance, 3);
}

#[test]
fn component_hash_mismatch_never_publishes_or_activates() {
    let temp = tempfile::tempdir().unwrap();
    let source = source_zip(None);
    let fixture = write_bundle(temp.path(), b"tampered", b"expected", &source, false);
    let store = VersionStore::new(temp.path().join("opt"));

    let error = install_bundle(
        &store,
        &fixture.path,
        &fixture.manifest,
        &fixture.asset,
        false,
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("integrity mismatch"), "{error}");
    assert!(store.active().unwrap().is_none());
    assert!(store.load_state().unwrap().installs.is_empty());
}

#[test]
fn traversal_and_extra_outer_entries_are_rejected_before_activation() {
    let temp = tempfile::tempdir().unwrap();
    let malicious_source = source_zip(Some(("../escaped", b"escape")));
    let traversal = write_bundle(temp.path(), b"vibe", b"vibe", &malicious_source, false);
    let store = VersionStore::new(temp.path().join("opt"));
    let error = install_bundle(
        &store,
        &traversal.path,
        &traversal.manifest,
        &traversal.asset,
        false,
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("unsafe source ZIP entry"), "{error}");
    assert!(!temp.path().join("escaped").exists());
    assert!(store.active().unwrap().is_none());

    let safe_source = source_zip(None);
    let extra = write_bundle(temp.path(), b"vibe", b"vibe", &safe_source, true);
    let error = install_bundle(&store, &extra.path, &extra.manifest, &extra.asset, false)
        .unwrap_err()
        .to_string();
    assert!(error.contains("structure mismatch"), "{error}");
    assert!(store.active().unwrap().is_none());

    let symlink_source = symlink_source_zip();
    let symlink = write_bundle(temp.path(), b"vibe", b"vibe", &symlink_source, false);
    let error = install_bundle(
        &store,
        &symlink.path,
        &symlink.manifest,
        &symlink.asset,
        false,
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("symlink"), "{error}");
    assert!(store.active().unwrap().is_none());

    for unsafe_name in ["café.rs", "CONIN$", "CONOUT$.txt"] {
        let unsafe_source = source_zip(Some((unsafe_name, b"unsafe")));
        let fixture = write_bundle(temp.path(), b"vibe", b"vibe", &unsafe_source, false);
        let error = install_bundle(
            &store,
            &fixture.path,
            &fixture.manifest,
            &fixture.asset,
            false,
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("unsafe source ZIP entry"), "{error}");
        assert!(store.active().unwrap().is_none());
    }
}
