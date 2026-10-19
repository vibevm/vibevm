//! Focused placement and manifest regressions.

use super::*;
use crate::commands::vvm::model::Kind;
use crate::commands::vvm::store::BINARY_NAME;
use specmark::verifies;

fn write_test_binary(path: &Path, bytes: &[u8]) {
    fs::write(path, bytes).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(path).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions).unwrap();
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-019#instances", r = 2)]
fn manifest_round_trips_and_detects_change() {
    let tmp = tempfile::tempdir().unwrap();
    let f = tmp.path().join("vibe");
    fs::write(&f, b"hello").unwrap();
    let m = manifest_for(&[(f.clone(), "vibe".into())]).unwrap();
    let text = toml::to_string(&m).unwrap();
    let back: Manifest = toml::from_str(&text).unwrap();
    assert_eq!(m, back);
    assert!(matches(&m, &back), "identical manifest matches");

    // A content change (small file → hashed) is detected.
    fs::write(&f, b"hello world").unwrap();
    let m2 = manifest_for(&[(f, "vibe".into())]).unwrap();
    assert!(!matches(&m2, &m), "changed content does not match");
    assert!(should_hash(
        &format!("bin/{BINARY_NAME}"),
        SMALL_FILE_MAX + 1
    ));
    assert!(should_hash(
        &format!("bin/{INDEX_BINARY_NAME}"),
        SMALL_FILE_MAX + 1
    ));
    assert!(should_hash(
        &format!("bin/{APP_PROXY_BINARY_NAME}"),
        SMALL_FILE_MAX + 1
    ));
    assert!(!should_hash("assets/large.bin", SMALL_FILE_MAX + 1));
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-019#instances", r = 2)]
fn place_creates_an_instance_with_a_manifest() {
    let tmp = tempfile::tempdir().unwrap();
    let store = VersionStore::new(tmp.path());
    let id = VersionId::new(Kind::Branch, "main");
    let built = tmp.path().join("built-vibe");
    write_test_binary(&built, b"BIN");
    let dist = vec![(built.clone(), BINARY_NAME.to_string())];
    let manifest = manifest_for(&dist).unwrap();

    place(&store, &id, 1, &dist, &manifest, None).unwrap();
    let inst = store.instance_dir(&id, 1);
    assert_eq!(fs::read(inst.join(BINARY_NAME)).unwrap(), b"BIN");
    assert!(inst.join(MANIFEST_NAME).is_file());
    assert!(read_manifest(&inst).is_some());

    // Corrupting the previous bytes must not propagate through a hardlink.
    fs::write(inst.join(BINARY_NAME), b"CORRUPT").unwrap();
    let fresh = tmp.path().join("fresh-vibe");
    write_test_binary(&fresh, b"BIN");
    let dist2 = vec![(fresh, BINARY_NAME.to_string())];
    let m2 = manifest_for(&dist2).unwrap();
    place(&store, &id, 2, &dist2, &m2, Some((&inst, &manifest))).unwrap();
    assert_eq!(
        fs::read(store.instance_dir(&id, 2).join(BINARY_NAME)).unwrap(),
        b"BIN"
    );
    assert_eq!(fs::read(inst.join(BINARY_NAME)).unwrap(), b"CORRUPT");
    assert!(!matches_on_disk(&store, &m2, &manifest, &inst));

    // A reused terminal #N is immutable even if state bookkeeping was
    // damaged; placement never deletes or rewrites the old payload.
    write_test_binary(&built, b"REPLACEMENT");
    let replacement = vec![(built, BINARY_NAME.to_string())];
    let replacement_manifest = manifest_for(&replacement).unwrap();
    let error = place(&store, &id, 1, &replacement, &replacement_manifest, None)
        .unwrap_err()
        .to_string();
    assert!(error.contains("immutable local instance"));
    assert_eq!(fs::read(inst.join(BINARY_NAME)).unwrap(), b"CORRUPT");
}

/// `is_transient_lock` recognises the Windows `ERROR_ACCESS_DENIED` (5)
/// and the kind-level `PermissionDenied` — the scanner-held-handle races
/// the publish rename / staging cleanup retry on — and not other errors.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-019#instances", r = 2)]
fn is_transient_lock_classifies_access_denied() {
    assert!(is_transient_lock(&io::Error::from_raw_os_error(5)));
    assert!(is_transient_lock(&io::Error::new(
        io::ErrorKind::PermissionDenied,
        "denied",
    )));
    assert!(!is_transient_lock(&io::Error::new(
        io::ErrorKind::NotFound,
        "missing",
    )));
}

/// `rename_into_place` / `remove_tree` succeed on an unlocked directory
/// (the common path, no retry needed) and report the lock-class errors
/// they would retry on rather than masking them.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-019#instances", r = 2)]
fn rename_and_remove_succeed_when_unlocked() {
    let tmp = tempfile::tempdir().unwrap();
    let from = tmp.path().join("staging");
    let to = tmp.path().join("final");
    fs::create_dir_all(&from).unwrap();
    fs::write(from.join("vibe"), b"x").unwrap();
    rename_into_place(&from, &to).unwrap();
    assert!(to.join("vibe").is_file());
    assert!(!from.exists(), "rename moved the dir out of staging");
    remove_tree(&to).unwrap();
    assert!(!to.exists());
}

#[test]
fn installed_manifest_shape_rejects_traversal_duplicates_and_extras() {
    let entry = |rel: &str| FileEntry {
        rel: rel.into(),
        size: 1,
        mtime_nanos: 0,
        hash: Some("a".repeat(64)),
    };
    let valid = Manifest {
        files: vec![entry(BINARY_NAME)],
        ..Manifest::default()
    };
    assert!(manifest_shape_valid(
        super::super::model::Origin::Binary,
        &valid
    ));
    for files in [
        vec![entry("../vibe")],
        vec![entry(BINARY_NAME), entry("extra")],
        vec![entry(BINARY_NAME), entry(BINARY_NAME)],
    ] {
        assert!(!manifest_shape_valid(
            super::super::model::Origin::Binary,
            &Manifest {
                files,
                ..Manifest::default()
            }
        ));
    }
}
#[test]
fn source_change_between_manifest_and_copy_is_rejected() {
    let temp = tempfile::tempdir().unwrap();
    let store = VersionStore::new(temp.path().join("opt"));
    let source = temp.path().join("vibe");
    write_test_binary(&source, b"first");
    let dist = vec![(source.clone(), BINARY_NAME.to_string())];
    let manifest = manifest_for(&dist).unwrap();
    write_test_binary(&source, b"second");
    let id = VersionId::new(Kind::Tag, "1.0.0");
    let error = place(&store, &id, 1, &dist, &manifest, None)
        .unwrap_err()
        .to_string();
    assert!(error.contains("placed bytes do not match"), "{error}");
    assert!(!store.instance_dir(&id, 1).exists());
}
