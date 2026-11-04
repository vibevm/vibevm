use super::*;
use crate::PublishStage;
use std::fs;

#[test]
fn retained_stage_identity_rejects_a_fresh_same_byte_object() {
    let dir = tempfile::tempdir().unwrap();
    let project = Project::open(dir.path()).unwrap();
    let parent = project.root_dir().unwrap();
    let published = project
        .write_atomic_owned_in_with_mode(&parent, "state", b"ours", None)
        .unwrap();
    let snapshot = project
        .read_file_snapshot_bounded("state", 4)
        .unwrap()
        .unwrap();
    assert!(published.receipt.matches_snapshot(&snapshot));
    fs::write(dir.path().join("replacement"), b"ours").unwrap();
    fs::rename(dir.path().join("replacement"), dir.path().join("state")).unwrap();
    let later = project
        .read_file_snapshot_bounded("state", 4)
        .unwrap()
        .unwrap();
    assert_eq!(snapshot.bytes, later.bytes);
    assert_ne!(snapshot.identity, later.identity);
    assert!(!published.receipt.matches_snapshot(&later));
}

#[test]
fn owned_error_keeps_prepublication_identity_instead_of_adopting_verification_replacement() {
    let dir = tempfile::tempdir().unwrap();
    let project = Project::open(dir.path()).unwrap();
    let parent = project.root_dir().unwrap();
    crate::arm_before_publish_verify(Some(Box::new(|parent, name| {
        let bytes = fs::read(parent.path().join(name)).unwrap();
        fs::write(parent.path().join("other"), bytes).unwrap();
        fs::rename(parent.path().join("other"), parent.path().join(name)).unwrap();
    })));
    let error = project
        .write_atomic_owned_in_with_mode(&parent, "state", b"ours", None)
        .unwrap_err();
    assert_eq!(error.publication.stage, PublishStage::PossiblyPublished);
    let current = project
        .read_file_snapshot_bounded("state", 4)
        .unwrap()
        .unwrap();
    assert_eq!(current.bytes, b"ours");
    assert!(!error.receipt.unwrap().matches_snapshot(&current));
}

#[test]
fn before_publication_failure_grants_no_undo_receipt_and_cleans_own_stage() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("state"), b"prior").unwrap();
    let project = Project::open(dir.path()).unwrap();
    let before = project.read_file_snapshot_bounded("state", 5).unwrap();
    fail_before_publish(Some("state"));
    let error = project
        .write_atomic_owned_in_with_mode(&project.root_dir().unwrap(), "state", b"ours", None)
        .unwrap_err();
    fail_before_publish(None);
    assert_eq!(error.publication.stage, PublishStage::BeforePublication);
    assert!(error.receipt.is_none());
    assert_eq!(
        project.read_file_snapshot_bounded("state", 5).unwrap(),
        before
    );
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
}

#[cfg(unix)]
#[test]
fn no_replace_restore_preserves_mode_and_never_overwrites_a_new_occupant() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let project = Project::open(dir.path()).unwrap();
    let parent = project.root_dir().unwrap();
    project
        .publish_new_in_with_mode(&parent, "prior", b"before", Some(0o640))
        .unwrap();
    assert_eq!(
        fs::metadata(dir.path().join("prior"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o640
    );
    let before = project.read_file_snapshot_bounded("prior", 6).unwrap();
    assert!(
        project
            .publish_new_in_with_mode(&parent, "prior", b"another", Some(0o600))
            .is_err()
    );
    assert_eq!(
        project.read_file_snapshot_bounded("prior", 6).unwrap(),
        before
    );
}

#[cfg(unix)]
#[test]
fn owned_publication_refuses_shared_target_without_mutating_its_sibling() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("original"), b"shared").unwrap();
    fs::hard_link(dir.path().join("original"), dir.path().join("state")).unwrap();
    let project = Project::open(dir.path()).unwrap();
    let error = project
        .write_atomic_owned_in_with_mode(&project.root_dir().unwrap(), "state", b"ours", None)
        .unwrap_err();
    assert_eq!(error.publication.stage, PublishStage::BeforePublication);
    assert!(error.receipt.is_none());
    assert_eq!(fs::read(dir.path().join("original")).unwrap(), b"shared");
    assert_eq!(fs::read(dir.path().join("state")).unwrap(), b"shared");
}

#[cfg(unix)]
#[test]
fn receipt_mint_refusal_retains_ambiguous_stage_and_reports_its_exact_path() {
    use std::io::Write;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("stage");
    let mut held = fs::OpenOptions::new()
        .create_new(true)
        .read(true)
        .write(true)
        .open(&path)
        .unwrap();
    held.write_all(b"stage bytes").unwrap();
    fs::hard_link(&path, dir.path().join("other-name")).unwrap();
    let parent = Project::open(dir.path()).unwrap().root_dir().unwrap();
    let error = owned::capture_stage(&parent, "stage", &held, b"stage bytes").unwrap_err();
    let text = format!("{error:#}");
    assert!(text.contains(&path.display().to_string()));
    assert!(text.contains("left untouched") && text.contains("hard link"));
    assert_eq!(fs::read(&path).unwrap(), b"stage bytes");
    assert_eq!(
        fs::read(dir.path().join("other-name")).unwrap(),
        b"stage bytes"
    );
}

#[cfg(unix)]
#[test]
fn new_owned_publication_retains_ordinary_creation_mode() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("ordinary"), b"ordinary").unwrap();
    let project = Project::open(dir.path()).unwrap();
    let _receipt = project
        .write_atomic_owned_in_with_mode(&project.root_dir().unwrap(), "owned", b"owned", None)
        .unwrap();
    assert_eq!(
        fs::metadata(dir.path().join("owned"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        fs::metadata(dir.path().join("ordinary"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777
    );
}
