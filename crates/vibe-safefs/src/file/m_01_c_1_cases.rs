//! C1: one no-follow held observation owns returned bytes, digest and identity.

use std::fs;

use sha2::{Digest, Sha256};

use crate::{Project, arm_before_bounded_read};

#[test]
fn held_snapshot_bytes_digest_and_identity_share_one_observation() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir(root.path().join("docs")).unwrap();
    fs::write(
        root.path().join("docs/source.md"),
        b"# Source\nexact bytes\n",
    )
    .unwrap();
    let before = fs::read_dir(root.path().join("docs")).unwrap().count();
    let project = Project::open(root.path()).unwrap();
    let root_identity = project.root_identity().unwrap();
    let observed = project
        .read_file_snapshot_bounded("docs/source.md", 1024)
        .unwrap()
        .unwrap();
    assert_eq!(observed.bytes, b"# Source\nexact bytes\n");
    assert_eq!(observed.size, observed.bytes.len() as u64);
    assert_eq!(
        observed.sha256,
        format!("{:x}", Sha256::digest(&observed.bytes))
    );
    assert_eq!(project.root_identity().unwrap(), root_identity);
    assert_eq!(
        fs::read_dir(root.path().join("docs")).unwrap().count(),
        before
    );
}

#[test]
fn hard_linked_source_has_no_stable_snapshot() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("source.md"), b"shared").unwrap();
    fs::hard_link(root.path().join("source.md"), root.path().join("alias.md")).unwrap();
    assert!(
        Project::open(root.path())
            .unwrap()
            .read_file_snapshot_bounded("source.md", 64)
            .is_err()
    );
}

#[cfg(unix)]
#[test]
fn symlinked_source_has_no_stable_snapshot() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("source.md"), b"source").unwrap();
    std::os::unix::fs::symlink(root.path().join("source.md"), root.path().join("link.md")).unwrap();
    assert!(
        Project::open(root.path())
            .unwrap()
            .read_file_snapshot_bounded("link.md", 64)
            .is_err()
    );
}

#[cfg(unix)]
#[test]
fn same_byte_replacement_during_held_snapshot_refuses() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("source.md");
    let moved = root.path().join("moved.md");
    fs::write(&path, b"original").unwrap();
    let project = Project::open(root.path()).unwrap();
    let (from, to) = (path.clone(), moved.clone());
    arm_before_bounded_read(Some(Box::new(move |_, _| {
        fs::rename(&from, &to).unwrap();
        fs::write(&from, b"original").unwrap();
    })));
    let result = project.read_file_snapshot_bounded("source.md", 64);
    arm_before_bounded_read(None);
    let error = result.unwrap_err();
    assert!(format!("{error:#}").contains("final-name race"));
    assert_eq!(fs::read(&moved).unwrap(), fs::read(&path).unwrap());
    let old = project
        .read_file_snapshot_bounded("moved.md", 64)
        .unwrap()
        .unwrap();
    let new = project
        .read_file_snapshot_bounded("source.md", 64)
        .unwrap()
        .unwrap();
    assert_ne!(
        old.identity, new.identity,
        "equal bytes came from distinct files"
    );
}
