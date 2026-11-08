//! Direct production-guard coverage, not claimed as public apply execution.
//! The package disables its lib test harness; compile the actual private owner
//! here rather than reproduce its algorithm or expose a production test hook.
mod error {
    pub use vibe_install::Error;
    pub type Result<T> = std::result::Result<T, Error>;
}
#[path = "../src/apply/file_guard.rs"]
mod guards;

use std::fs;
use vibe_core::manifest::Manifest;

const MANIFEST: &str = "[project]\nname='guard-probe'\nversion='1.0.0'\n";

fn fixture() -> (tempfile::TempDir, guards::MetadataFiles) {
    let temp = tempfile::tempdir().unwrap();
    fs::write(temp.path().join("vibe.toml"), MANIFEST).unwrap();
    fs::write(
        temp.path().join("vibe.lock.provenance"),
        "# captured prior\n",
    )
    .unwrap();
    let files = guards::MetadataFiles::capture(
        temp.path(),
        &temp.path().join("vibe.lock"),
        &Manifest::parse_str(MANIFEST).unwrap(),
        false,
    )
    .unwrap();
    (temp, files)
}

fn primary() -> vibe_install::Error {
    vibe_install::Error::RootNotFetched {
        pkgref: "synthetic/primary-error".into(),
    }
}

#[cfg(unix)]
#[test]
fn removed_provenance_undo_uses_no_replace_creation_and_preserves_mode() {
    use std::os::unix::fs::PermissionsExt;
    let (temp, mut files) = fixture();
    files.disarm();
    let path = temp.path().join("vibe.lock.provenance");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap();
    let mut files = guards::MetadataFiles::capture(
        temp.path(),
        &temp.path().join("vibe.lock"),
        &Manifest::parse_str(MANIFEST).unwrap(),
        false,
    )
    .unwrap();
    files
        .check_lock_path(&temp.path().join("vibe.lock"))
        .unwrap();
    assert_eq!(
        files.provenance.previous_text().unwrap(),
        Some("# captured prior\n")
    );
    files.provenance.publish(None).unwrap();
    assert!(!path.exists());
    let error = files.undo_all(primary());
    assert!(matches!(error, vibe_install::Error::RootNotFetched { .. }));
    assert_eq!(fs::read_to_string(&path).unwrap(), "# captured prior\n");
    assert_eq!(
        fs::metadata(path).unwrap().permissions().mode() & 0o777,
        0o640
    );
    assert_eq!(
        fs::read_to_string(temp.path().join("vibe.toml")).unwrap(),
        MANIFEST
    );
}

#[test]
fn removed_provenance_undo_preserves_a_later_occupant_and_reports_both_failures() {
    let (temp, mut files) = fixture();
    let path = temp.path().join("vibe.lock.provenance");
    files.provenance.publish(None).unwrap();
    fs::write(&path, "another actor").unwrap();
    let error = files.undo_all(primary());
    assert!(matches!(error, vibe_install::Error::RollbackFailed { .. }));
    assert!(error.to_string().contains("synthetic/primary-error"));
    assert!(error.to_string().contains("vibe.lock.provenance"));
    assert_eq!(fs::read_to_string(path).unwrap(), "another actor");
}

#[test]
fn prior_absence_does_not_authorize_removing_a_third_state() {
    let (temp, mut files) = fixture();
    files.lock.publish(Some("ours")).unwrap();
    fs::write(temp.path().join("vibe.lock"), "other").unwrap();
    let error = files.undo_all(primary());
    assert!(matches!(error, vibe_install::Error::RollbackFailed { .. }));
    assert_eq!(
        fs::read_to_string(temp.path().join("vibe.lock")).unwrap(),
        "other"
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#PACKAGE-ADMISSION")]
fn admitted_migration_uses_owned_publication_and_preserves_changed_source() {
    let (temp, mut files) = fixture();
    files.disarm();
    let expected = Manifest::parse_str(MANIFEST).unwrap();
    let mut replacement = expected.clone();
    replacement
        .requires
        .packages
        .push(vibe_core::PackageRef::parse("org.example/tool@1").unwrap());
    guards::publish_admitted_manifest(temp.path(), &expected, &replacement).unwrap();
    assert_eq!(
        Manifest::read(temp.path().join("vibe.toml")).unwrap(),
        replacement
    );
    let changed = "[project]\nname='changed-by-other-actor'\nversion='1.0.0'\n";
    fs::write(temp.path().join("vibe.toml"), changed).unwrap();
    assert!(guards::publish_admitted_manifest(temp.path(), &expected, &replacement).is_err());
    assert_eq!(
        fs::read_to_string(temp.path().join("vibe.toml")).unwrap(),
        changed
    );
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#LF-CONFIGURED-STATE"
)]
fn known_legacy_empty_publication_is_supported_and_preserves_decoration() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("vibe.lock");
    std::fs::write(
        &path,
        "# operator decoration\n[meta]\ngenerated_by='old'\ngenerated_at='old'\nschema_version=6\n",
    )
    .unwrap();
    let replacement = vibe_core::manifest::Lockfile::empty("fixture", "fixed");
    guards::publish_empty_legacy_lock(root.path(), 6, &replacement).unwrap();
    assert_eq!(
        vibe_core::manifest::Lockfile::read(&path).unwrap(),
        replacement
    );
    assert!(
        std::fs::read_to_string(path)
            .unwrap()
            .contains("# operator decoration")
    );
}

#[test]
fn the_lock_path_check_accepts_only_the_captured_lock() {
    let (temp, mut files) = fixture();
    files
        .check_lock_path(&temp.path().join("vibe.lock"))
        .unwrap();
    assert!(
        files
            .check_lock_path(&temp.path().join("moved").join("vibe.lock"))
            .is_err(),
        "a lock under a different parent is refused"
    );
    files.disarm();
}
