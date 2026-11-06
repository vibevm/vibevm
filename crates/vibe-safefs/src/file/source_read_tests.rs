//! Source-read observations and unchanged strict state-reader boundaries.
specmark::scope!(
    "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-035#URI-OBSERVED-SOURCE-READ"
);
use super::*;
use std::fs;

#[test]
fn observation_carries_shared_identity_bytes_and_permissions_without_ownership() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("source"), b"shared").unwrap();
    fs::hard_link(root.path().join("source"), root.path().join("alias")).unwrap();
    let project = Project::open(root.path()).unwrap();
    let source = project.read_source_observation("source").unwrap().unwrap();
    let alias = project.read_source_observation("alias").unwrap().unwrap();
    assert_eq!(source.identity(), alias.identity());
    assert_eq!(source.bytes(), b"shared");
    assert_eq!(
        source.permissions(),
        &fs::metadata(root.path().join("source"))
            .unwrap()
            .permissions()
    );
    assert!(project.read_file_snapshot_bounded("source", 64).is_err());
}

#[test]
fn readonly_change_during_source_observation_refuses_without_repair() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("source");
    fs::write(&path, b"stable").unwrap();
    let original = fs::metadata(&path).unwrap().permissions();
    assert!(!original.readonly());
    crate::arm_between_stream_passes(Some(Box::new(|parent, name| {
        let path = parent.join(name);
        let mut permissions = fs::metadata(&path).unwrap().permissions();
        permissions.set_readonly(true);
        fs::set_permissions(path, permissions).unwrap();
    })));
    let project = Project::open(root.path()).unwrap();
    assert!(project.read_source_observation("source").is_err());
    assert!(fs::metadata(&path).unwrap().permissions().readonly());
    fs::set_permissions(path, original).unwrap();
}

#[test]
fn shared_source_reads_preserve_both_names_and_strict_snapshot_refusal() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("source"), b"shared source").unwrap();
    fs::hard_link(root.path().join("source"), root.path().join("alias")).unwrap();
    let project = Project::open(root.path()).unwrap();
    assert_eq!(
        project.read_source_bytes("source").unwrap().unwrap(),
        b"shared source"
    );
    assert_eq!(
        project.read_source_bytes("alias").unwrap().unwrap(),
        b"shared source"
    );
    assert!(project.read_file_snapshot_bounded("source", 32).is_err());
    assert_eq!(
        fs::read(root.path().join("source")).unwrap(),
        fs::read(root.path().join("alias")).unwrap()
    );
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 2);
}

#[test]
fn empty_absent_nested_and_metadata_only_membership() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir(root.path().join("nested")).unwrap();
    fs::write(root.path().join("nested/empty"), b"").unwrap();
    let project = Project::open(root.path()).unwrap();
    assert_eq!(
        project.read_source_bytes("nested/empty").unwrap(),
        Some(vec![])
    );
    assert_eq!(project.read_source_bytes("missing/file").unwrap(), None);
    assert!(project.read_source_bytes("nested").is_err());
    let parent = project.root_dir().unwrap();
    assert!(!project.source_file_present_in(&parent, "nested").unwrap());
    assert!(!project.source_file_present_in(&parent, "absent").unwrap());
    let nested = parent.open_child("nested").unwrap();
    crate::arm_before_bounded_read(Some(Box::new(|_, _| {
        panic!("metadata lookup read content")
    })));
    assert!(project.source_file_present_in(&nested, "empty").unwrap());
    crate::arm_before_bounded_read(None);
}

#[test]
fn growth_and_truncation_refuse_instead_of_returning_a_prefix() {
    for replacement in [b"longer content".as_slice(), b"x".as_slice()] {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("source"), b"original").unwrap();
        let replacement = replacement.to_vec();
        let expected = replacement.clone();
        crate::arm_before_bounded_read(Some(Box::new(move |parent, name| {
            fs::write(parent.join(name), &replacement).unwrap()
        })));
        let project = Project::open(root.path()).unwrap();
        assert!(project.read_source_bytes("source").is_err());
        assert_eq!(fs::read(root.path().join("source")).unwrap(), expected);
    }
}

#[test]
fn same_length_shared_inode_change_between_passes_refuses() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("source"), b"before").unwrap();
    fs::hard_link(root.path().join("source"), root.path().join("alias")).unwrap();
    let alias = root.path().join("alias");
    crate::arm_between_stream_passes(Some(Box::new(move |_, _| {
        fs::write(&alias, b"after!").unwrap()
    })));
    let project = Project::open(root.path()).unwrap();
    assert!(project.read_source_bytes("source").is_err());
    assert_eq!(fs::read(root.path().join("source")).unwrap(), b"after!");
}

#[cfg(unix)]
#[test]
fn same_byte_native_replacement_refuses_and_preserves_injected_name() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("source"), b"same").unwrap();
    crate::arm_before_bounded_read(Some(Box::new(|parent, name| {
        fs::rename(parent.join(name), parent.join("old")).unwrap();
        fs::write(parent.join(name), b"same").unwrap();
    })));
    let project = Project::open(root.path()).unwrap();
    let error = project.read_source_bytes("source").unwrap_err();
    assert!(format!("{error:#}").contains("replaced"));
    assert_eq!(fs::read(root.path().join("source")).unwrap(), b"same");
    assert_eq!(fs::read(root.path().join("old")).unwrap(), b"same");
}

#[cfg(unix)]
#[test]
fn links_and_changed_mode_refuse_without_follow_or_repair() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    fs::write(outside.path().join("source"), b"outside").unwrap();
    symlink(outside.path(), root.path().join("directory")).unwrap();
    symlink(outside.path().join("source"), root.path().join("link")).unwrap();
    let project = Project::open(root.path()).unwrap();
    assert!(project.read_source_bytes("directory/source").is_err());
    assert!(project.read_source_bytes("link").is_err());
    assert!(
        project
            .source_file_present_in(&project.root_dir().unwrap(), "link")
            .is_err()
    );
    fs::write(root.path().join("source"), b"stable").unwrap();
    crate::arm_between_stream_passes(Some(Box::new(|parent, name| {
        let path = parent.join(name);
        let mode = fs::metadata(&path).unwrap().permissions().mode();
        fs::set_permissions(path, fs::Permissions::from_mode(mode ^ 0o100)).unwrap();
    })));
    assert!(project.read_source_bytes("source").is_err());
    assert_eq!(fs::read(outside.path().join("source")).unwrap(), b"outside");
}

#[test]
fn simulated_final_identity_mismatch_refuses_once_disarmed_it_reads() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("source"), b"same").unwrap();
    let project = Project::open(root.path()).unwrap();
    crate::arm_bounded_read_identity_check(Some(Box::new(|_| false)));
    let result = project.read_source_bytes("source");
    crate::arm_bounded_read_identity_check(None);
    assert!(result.is_err());
    assert_eq!(
        project.read_source_bytes("source").unwrap().unwrap(),
        b"same"
    );
}

#[test]
fn impossible_allocation_and_second_pass_errors_are_refusals() {
    assert!(reserve_source(u64::MAX).is_err());
    for actual in [
        b"short".as_slice(),
        b"expected plus".as_slice(),
        b"changed!".as_slice(),
    ] {
        assert!(compare_pass(&mut std::io::Cursor::new(actual), b"expected").is_err());
    }
    struct Broken;
    impl Read for Broken {
        fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
            Err(std::io::Error::other("injected read failure"))
        }
    }
    assert!(compare_pass(&mut Broken, b"expected").is_err());
}
