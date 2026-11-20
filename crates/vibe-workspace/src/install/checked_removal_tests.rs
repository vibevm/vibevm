//! Disposable checked retirement custody tests.
use super::*;
struct Expected {
    row: LockedPackage,
    slot: PathBuf,
}
impl PreparedRetirementSlot for Expected {
    fn package(&self) -> &LockedPackage {
        &self.row
    }
    fn expected_slot(&self) -> &Path {
        &self.slot
    }
    fn recheck_source(&self) -> Result<(), String> {
        Ok(())
    }
}
fn fixture() -> (tempfile::TempDir, Expected, PathBuf) {
    let root = tempfile::tempdir().unwrap();
    let hash = format!("sha256:{}", "a".repeat(64));
    let row = super::super::tests_epoch_world::locked("leaf", &hash, &[]);
    let source = root.path().join("source");
    std::fs::create_dir_all(source.join("data")).unwrap();
    std::fs::write(
        source.join("vibe.toml"),
        "[package]\ngroup='org.lock'\nname='leaf'\nversion='1.0.0'\nkind='tool'\n",
    )
    .unwrap();
    std::fs::write(source.join("data/payload"), b"owned").unwrap();
    let expected_root = root.path().join("reference");
    let workspace = root.path().join("project");
    std::fs::create_dir_all(&expected_root).unwrap();
    std::fs::create_dir_all(&workspace).unwrap();
    vibedeps::materialise(
        &expected_root,
        &row.group,
        row.name.as_str(),
        &row.version,
        &source,
        &row.content_hash,
    )
    .unwrap();
    vibedeps::materialise(
        &workspace,
        &row.group,
        row.name.as_str(),
        &row.version,
        &source,
        &row.content_hash,
    )
    .unwrap();
    let expected = Expected {
        slot: vibedeps::slot_abs_path(&expected_root, &row.group, row.name.as_str(), &row.version),
        row,
    };
    (root, expected, workspace)
}
fn active(workspace: &Path, expected: &Expected) -> PathBuf {
    vibedeps::slot_abs_path(
        workspace,
        &expected.row.group,
        expected.row.name.as_str(),
        &expected.row.version,
    )
}
fn recovery(error: &str) -> PathBuf {
    PathBuf::from(
        error
            .rsplit("recovery backup retained at ")
            .next()
            .unwrap()
            .split(" (violates")
            .next()
            .unwrap(),
    )
}
#[test]
fn exact_owned_slot_clean_restore_retains_empty_container() {
    let (_root, expected, workspace) = fixture();
    let slot = active(&workspace, &expected);
    let plan = CheckedRemovalPlan::capture(&workspace, &expected.row, &expected).unwrap();
    let mut rollback = super::super::InstallRollback::new(&workspace);
    rollback.remove_checked(plan).unwrap();
    assert!(!slot.exists());
    rollback.restore(None, &[]).unwrap();
    assert_eq!(std::fs::read(slot.join("data/payload")).unwrap(), b"owned");
    let backups = rollback.retained_retirement_backups();
    assert_eq!(backups.len(), 1);
    assert_eq!(
        backups[0].state,
        RetirementBackupState::RestoredContainerRetained
    );
    assert!(backups[0].path.is_dir());
}
#[test]
fn extra_file_empty_directory_and_changed_file_refuse_without_move() {
    for mutation in ["file", "directory", "changed"] {
        let (_root, expected, workspace) = fixture();
        let slot = active(&workspace, &expected);
        match mutation {
            "file" => std::fs::write(slot.join("foreign"), b"foreign").unwrap(),
            "directory" => std::fs::create_dir(slot.join("empty-foreign")).unwrap(),
            _ => std::fs::write(slot.join("data/payload"), b"changed").unwrap(),
        }
        assert!(
            CheckedRemovalPlan::capture(&workspace, &expected.row, &expected).is_err(),
            "{mutation}"
        );
        assert!(slot.exists());
    }
}
#[test]
fn replacement_identity_after_capture_refuses_without_move() {
    let (root, expected, workspace) = fixture();
    let slot = active(&workspace, &expected);
    let plan = CheckedRemovalPlan::capture(&workspace, &expected.row, &expected).unwrap();
    std::fs::rename(&slot, root.path().join("retained")).unwrap();
    std::fs::create_dir(&slot).unwrap();
    let mut rollback = super::super::InstallRollback::new(&workspace);
    assert!(rollback.remove_checked(plan).is_err());
    assert!(slot.exists());
}
#[test]
fn foreign_occupant_after_move_is_preserved_with_backup() {
    let (_root, expected, workspace) = fixture();
    let slot = active(&workspace, &expected);
    let plan = CheckedRemovalPlan::capture(&workspace, &expected.row, &expected).unwrap();
    let mut rollback = super::super::InstallRollback::new(&workspace);
    rollback.remove_checked(plan).unwrap();
    std::fs::create_dir(&slot).unwrap();
    std::fs::write(slot.join("foreign"), b"foreign").unwrap();
    let error = rollback.restore(None, &[]).unwrap_err();
    assert_eq!(std::fs::read(slot.join("foreign")).unwrap(), b"foreign");
    assert_eq!(
        std::fs::read(recovery(&error).join("slot/data/payload")).unwrap(),
        b"owned"
    );
}
#[test]
fn foreign_stash_neighbor_survives_success_and_restore() {
    for restore in [true, false] {
        let (_root, expected, workspace) = fixture();
        let plan = CheckedRemovalPlan::capture(&workspace, &expected.row, &expected).unwrap();
        let step = CheckedRemoval::execute(plan, workspace.parent().unwrap()).unwrap();
        let container = step.stash_parent.path().to_path_buf();
        std::fs::write(container.join("foreign"), b"foreign").unwrap();
        let outcome = if restore {
            step.restore().unwrap()
        } else {
            step.discard().unwrap()
        };
        assert_eq!(outcome.path, container);
        assert_eq!(
            std::fs::read(container.join("foreign")).unwrap(),
            b"foreign"
        );
        if !restore {
            assert_eq!(
                std::fs::read(container.join("slot/data/payload")).unwrap(),
                b"owned"
            );
        }
    }
}
#[test]
fn changed_stash_refuses_restore_and_retains_every_byte() {
    let (_root, expected, workspace) = fixture();
    let slot = active(&workspace, &expected);
    let plan = CheckedRemovalPlan::capture(&workspace, &expected.row, &expected).unwrap();
    let step = CheckedRemoval::execute(plan, workspace.parent().unwrap()).unwrap();
    let container = step.stash_parent.path().to_path_buf();
    std::fs::write(container.join("slot/data/payload"), b"changed").unwrap();
    let error = step.restore().unwrap_err();
    assert_eq!(recovery(&error), container);
    assert!(!slot.exists());
    assert_eq!(
        std::fs::read(container.join("slot/data/payload")).unwrap(),
        b"changed"
    );
}
#[test]
fn possibly_moved_before_registration_retains_recovery_container() {
    let (root, expected, workspace) = fixture();
    let slot = active(&workspace, &expected);
    let plan = CheckedRemovalPlan::capture(&workspace, &expected.row, &expected).unwrap();
    let retained = root.path().join("retained-owned");
    let saved = retained.clone();
    vibe_safefs::arm_after_rename_source_check(Some(Box::new(move |parent, _, old, _| {
        std::fs::rename(parent.join(old), &saved).unwrap();
        std::fs::create_dir(parent.join(old)).unwrap();
        std::fs::write(parent.join(old).join("foreign"), b"foreign").unwrap();
    })));
    let mut rollback = super::super::InstallRollback::new(&workspace);
    let error = rollback.remove_checked(plan).unwrap_err();
    let WorkspaceError::SpecMaterialization { reason, .. } = &error else {
        panic!("wrong typed error: {error:?}")
    };
    let dependency_root = workspace.join(vibe_core::layout::current_vibedeps_root());
    let backup_parent = dependency_root.parent().unwrap();
    let containers = std::fs::read_dir(backup_parent)
        .unwrap()
        .filter_map(|entry| {
            let path = entry.unwrap().path();
            path.file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .starts_with(".vibe-retirement-")
                .then_some(path)
        })
        .collect::<Vec<_>>();
    assert_eq!(
        containers.len(),
        1,
        "typed error={error:?}; containers={containers:?}"
    );
    assert_eq!(
        recovery(reason),
        containers[0],
        "raw reason, not formatted error suffix"
    );
    assert_eq!(
        std::fs::read(containers[0].join("slot/foreign")).unwrap(),
        b"foreign"
    );
    assert_eq!(
        std::fs::read(retained.join("data/payload")).unwrap(),
        b"owned"
    );
    assert!(!slot.exists());
}
