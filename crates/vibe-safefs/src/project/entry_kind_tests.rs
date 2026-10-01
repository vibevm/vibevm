use std::fs;

use super::{ChildKind, Project};

#[test]
fn direct_regular_directory_and_missing_are_distinct() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir(root.path().join("nested")).unwrap();
    fs::write(root.path().join("plain.txt"), b"plain").unwrap();
    let project = Project::open(root.path()).unwrap();
    let held = project.root_dir().unwrap();
    assert_eq!(
        project.child_kind_nofollow(&held, "plain.txt").unwrap(),
        Some(ChildKind::RegularFile)
    );
    assert_eq!(
        project.child_kind_nofollow(&held, "nested").unwrap(),
        Some(ChildKind::Directory)
    );
    assert_eq!(project.child_kind_nofollow(&held, "missing").unwrap(), None);
    assert!(project.child_kind_nofollow(&held, "../outside").is_err());
}

#[test]
fn hard_link_is_a_regular_kind_but_not_a_later_byte_witness() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("source"), b"shared").unwrap();
    if fs::hard_link(root.path().join("source"), root.path().join("alias")).is_err() {
        return; // This filesystem does not support hard links.
    }
    let project = Project::open(root.path()).unwrap();
    let held = project.root_dir().unwrap();
    assert_eq!(
        project.child_kind_nofollow(&held, "alias").unwrap(),
        Some(ChildKind::RegularFile)
    );
    assert!(project.stable_file_state_bounded("alias", 1024).is_err());
}

#[cfg(unix)]
#[test]
fn file_and_directory_symlinks_refuse_without_following() {
    use std::os::unix::fs::symlink;
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    fs::write(outside.path().join("target.txt"), b"secret").unwrap();
    symlink(
        outside.path().join("target.txt"),
        root.path().join("file-link"),
    )
    .unwrap();
    symlink(outside.path(), root.path().join("dir-link")).unwrap();
    let project = Project::open(root.path()).unwrap();
    let held = project.root_dir().unwrap();
    for name in ["file-link", "dir-link"] {
        let reason = format!(
            "{:#}",
            project.child_kind_nofollow(&held, name).unwrap_err()
        );
        assert!(
            reason.contains("symlink") || reason.contains("reparse"),
            "{reason}"
        );
    }
}

#[cfg(unix)]
#[test]
fn a_special_child_is_not_classified_as_a_file() {
    use std::os::unix::net::UnixListener;
    let root = tempfile::tempdir().unwrap();
    let _socket = UnixListener::bind(root.path().join("socket")).unwrap();
    let project = Project::open(root.path()).unwrap();
    let held = project.root_dir().unwrap();
    assert!(project.child_kind_nofollow(&held, "socket").is_err());
}

#[cfg(windows)]
#[test]
fn windows_file_and_directory_reparse_links_refuse_when_supported() {
    use std::os::windows::fs::{symlink_dir, symlink_file};
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    fs::write(outside.path().join("target.txt"), b"secret").unwrap();
    let file = symlink_file(
        outside.path().join("target.txt"),
        root.path().join("file-link"),
    );
    let dir = symlink_dir(outside.path(), root.path().join("dir-link"));
    if file.is_err() || dir.is_err() {
        return;
    } // Host policy denies link creation.
    let project = Project::open(root.path()).unwrap();
    let held = project.root_dir().unwrap();
    assert!(project.child_kind_nofollow(&held, "file-link").is_err());
    assert!(project.child_kind_nofollow(&held, "dir-link").is_err());
}
