//! Discriminators for the common owner's owned deterministic-stage combination.
//! No new production entrypoint is assumed by these baseline tests.
use super::*;
use std::fs;
const STAGE: &str = ".vibe-stage-tx-11111111111111111111111111111111";

#[test]
fn stage_presence_never_reads_or_adopts_occupants() {
    let temp = tempfile::tempdir().unwrap();
    let project = Project::open(&temp.path().canonicalize().unwrap()).unwrap();
    let parent = project.root_dir().unwrap();
    let path = temp.path().join(STAGE);
    assert!(!parent.transaction_stage_present(STAGE).unwrap());
    assert!(parent.transaction_stage_present("../escape").is_err());
    assert!(
        parent
            .transaction_stage_present(".vibe-stage-foreign")
            .is_err()
    );
    fs::write(&path, b"foreign").unwrap();
    assert!(parent.transaction_stage_present(STAGE).unwrap());
    assert_eq!(fs::read(&path).unwrap(), b"foreign");
    fs::remove_file(&path).unwrap();
    fs::create_dir(&path).unwrap();
    assert!(parent.transaction_stage_present(STAGE).unwrap());
    assert!(path.is_dir());
    fs::remove_dir(&path).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::{FileTypeExt as _, PermissionsExt as _};
        std::os::unix::fs::symlink(temp.path().join("absent"), &path).unwrap();
        assert!(parent.transaction_stage_present(STAGE).unwrap());
        assert!(
            fs::symlink_metadata(&path)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        fs::remove_file(&path).unwrap();
        assert!(
            std::process::Command::new("mkfifo")
                .arg(&path)
                .status()
                .unwrap()
                .success()
        );
        assert!(parent.transaction_stage_present(STAGE).unwrap());
        assert!(fs::symlink_metadata(&path).unwrap().file_type().is_fifo());
        let prior = fs::metadata(temp.path()).unwrap().permissions();
        fs::set_permissions(temp.path(), fs::Permissions::from_mode(0o0)).unwrap();
        let inaccessible = parent.transaction_stage_present(STAGE);
        fs::set_permissions(temp.path(), prior).unwrap();
        assert!(
            inaccessible.is_err(),
            "permission refusal must not become absence"
        );
    }
}

#[test]
#[cfg(unix)]
fn resumed_readonly_stage_preserves_mode_and_public_path_guard() {
    use std::os::unix::fs::PermissionsExt as _;
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("state"), b"prior").unwrap();
    fs::write(dir.path().join(STAGE), b"ours").unwrap();
    fs::set_permissions(dir.path().join(STAGE), fs::Permissions::from_mode(0o400)).unwrap();
    assert!(
        fs::OpenOptions::new()
            .write(true)
            .open(dir.path().join(STAGE))
            .is_err(),
        "readonly-stage discriminator needs an unprivileged writer-denied fixture"
    );
    let project = Project::open(dir.path()).unwrap();
    assert!(
        project.read_file_snapshot_bounded(STAGE, 4).is_err(),
        "public authored path reader must still refuse reserved stage names"
    );
    let published = project
        .write_atomic_transactional_owned_in_with_mode(
            &project.root_dir().unwrap(),
            "state",
            b"ours",
            Some(0o400),
            STAGE,
        )
        .unwrap();
    let current = project
        .read_file_snapshot_bounded("state", 4)
        .unwrap()
        .unwrap();
    assert_eq!(current.bytes, b"ours");
    assert_eq!(current.unix_mode, Some(0o400));
    assert!(published.receipt.matches_snapshot(&current));
    assert!(!dir.path().join(STAGE).exists());
}

#[test]
fn combined_owner_retains_fresh_and_reused_prepublication_objects() {
    for reuse in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("state"), b"prior").unwrap();
        if reuse {
            fs::write(dir.path().join(STAGE), b"ours").unwrap();
        }
        let project = Project::open(dir.path()).unwrap();
        let mut receipt = None;
        project
            .write_atomic_durable_in_with_mode_stage(
                &project.root_dir().unwrap(),
                "state",
                b"ours",
                None,
                Some(STAGE),
                Some(&mut receipt),
            )
            .unwrap();
        let current = project
            .read_file_snapshot_bounded("state", 4)
            .unwrap()
            .unwrap();
        assert_eq!(current.bytes, b"ours");
        assert!(
            receipt
                .as_ref()
                .is_some_and(|r| r.matches_snapshot(&current)),
            "reuse={reuse}: publication lost its actual staging receipt"
        );
    }
}

#[test]
fn foreign_deterministic_stage_is_retained_without_target_mutation() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("state"), b"prior").unwrap();
    fs::write(dir.path().join(STAGE), b"foreign").unwrap();
    let project = Project::open(dir.path()).unwrap();
    let before = project.read_file_snapshot_bounded("state", 5).unwrap();
    let mut receipt = None;
    let error = project
        .write_atomic_durable_in_with_mode_stage(
            &project.root_dir().unwrap(),
            "state",
            b"ours",
            None,
            Some(STAGE),
            Some(&mut receipt),
        )
        .unwrap_err();
    assert_eq!(error.stage, crate::PublishStage::BeforePublication);
    assert!(receipt.is_none());
    assert_eq!(
        project.read_file_snapshot_bounded("state", 5).unwrap(),
        before
    );
    assert_eq!(fs::read(dir.path().join(STAGE)).unwrap(), b"foreign");
}

#[test]
fn reused_stage_cannot_adopt_same_byte_postpublication_replacement() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join(STAGE), b"ours").unwrap();
    let project = Project::open(dir.path()).unwrap();
    crate::arm_before_publish_verify(Some(Box::new(|parent, name| {
        fs::write(parent.path().join("other"), b"ours").unwrap();
        fs::rename(parent.path().join("other"), parent.path().join(name)).unwrap();
    })));
    let mut receipt = None;
    let result = project.write_atomic_durable_in_with_mode_stage(
        &project.root_dir().unwrap(),
        "state",
        b"ours",
        None,
        Some(STAGE),
        Some(&mut receipt),
    );
    crate::arm_before_publish_verify(None);
    let error = result.unwrap_err();
    assert_eq!(error.stage, crate::PublishStage::PossiblyPublished);
    let current = project
        .read_file_snapshot_bounded("state", 4)
        .unwrap()
        .unwrap();
    assert_eq!(current.bytes, b"ours");
    assert!(
        receipt
            .as_ref()
            .is_some_and(|r| !r.matches_snapshot(&current))
    );
}

#[test]
fn reused_stage_actual_rename_refusal_retains_uncertain_owned_evidence() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("state"), b"prior").unwrap();
    fs::write(dir.path().join(STAGE), b"ours").unwrap();
    let project = Project::open(dir.path()).unwrap();
    let reached = std::rc::Rc::new(std::cell::Cell::new(false));
    let witness = reached.clone();
    crate::arm_before_bounded_read(Some(Box::new(move |parent, name| {
        if name == STAGE {
            witness.set(true);
            fs::rename(
                parent.path().join("state"),
                parent.path().join("prior-kept"),
            )
            .unwrap();
            fs::create_dir(parent.path().join("state")).unwrap();
        }
    })));
    let mut receipt = None;
    let result = project.write_atomic_durable_in_with_mode_stage(
        &project.root_dir().unwrap(),
        "state",
        b"ours",
        None,
        Some(STAGE),
        Some(&mut receipt),
    );
    crate::arm_before_bounded_read(None);
    assert!(
        reached.get(),
        "owned resumed-stage admission was not observed"
    );
    let error = result.unwrap_err();
    assert_eq!(error.stage, crate::PublishStage::PossiblyPublished);
    assert!(
        receipt.is_some(),
        "attempted rename must retain actual resumed stage evidence"
    );
    assert!(dir.path().join("state").is_dir());
    assert_eq!(fs::read(dir.path().join("prior-kept")).unwrap(), b"prior");
    assert_eq!(fs::read(dir.path().join(STAGE)).unwrap(), b"ours");
}
