use std::fs;

use crate::Project;

#[test]
fn absolute_file_containment_is_decided_by_pinned_ancestry() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir_all(root.path().join("vibevm/scrape")).unwrap();
    let contract = root.path().join("vibevm/scrape/contract.toml");
    fs::write(&contract, b"schema = 1").unwrap();
    let project = Project::open(root.path()).unwrap();

    let pinned = Project::pin_absolute_file(&contract).unwrap();
    assert_eq!(
        pinned.relative_to(&project).unwrap().as_deref(),
        Some("vibevm/scrape/contract.toml")
    );
    assert_eq!(
        pinned.read_snapshot_bounded(&project, 64).unwrap().bytes,
        b"schema = 1"
    );
}

#[test]
fn absent_destination_holds_parent_and_detects_project_ancestry() {
    let root = tempfile::tempdir().unwrap();
    let project = Project::open(root.path()).unwrap();
    let nested = root.path().join("exports/release");
    let pinned = Project::pin_absent_path(&nested).unwrap();
    assert!(pinned.descends_from(&project).unwrap());
    assert_eq!(pinned.identity_token().len(), "sha256:".len() + 64);
    assert_eq!(pinned.existing_parent().path(), root.path());

    fs::write(root.path().join("occupied"), b"file").unwrap();
    assert!(Project::pin_absent_path(&root.path().join("occupied/child")).is_err());
    fs::create_dir(root.path().join("exists")).unwrap();
    assert!(Project::pin_absent_path(&root.path().join("exists")).is_err());
}
