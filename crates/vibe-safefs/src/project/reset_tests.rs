use crate::Project;

#[test]
fn reset_dir_clears_only_the_named_tree_and_returns_a_live_directory() {
    let root = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(root.path().join("target/out/nested")).expect("tree creates");
    std::fs::write(root.path().join("target/out/nested/stale"), b"stale")
        .expect("stale file writes");
    std::fs::write(root.path().join("target/neighbour"), b"keep").expect("neighbour writes");
    let project = Project::open(root.path()).expect("capability opens");

    let output = project.reset_dir("target/out").expect("the output resets");

    assert!(output.path().is_dir());
    assert_eq!(std::fs::read_dir(output.path()).unwrap().count(), 0);
    assert_eq!(
        std::fs::read(root.path().join("target/neighbour")).unwrap(),
        b"keep",
    );
}

#[test]
fn remove_dir_all_if_present_removes_only_existing_trees_and_never_creates() {
    let root = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(root.path().join("state/staging/nested")).expect("tree creates");
    std::fs::write(root.path().join("state/staging/nested/stale"), b"stale")
        .expect("stale file writes");
    std::fs::write(root.path().join("state/neighbour"), b"keep").expect("neighbour writes");
    let project = Project::open(root.path()).expect("capability opens");

    assert!(
        project
            .remove_dir_all_if_present("state/staging")
            .expect("present tree removes")
    );
    assert!(!root.path().join("state/staging").exists());
    assert_eq!(
        std::fs::read(root.path().join("state/neighbour")).unwrap(),
        b"keep"
    );
    assert!(
        !project
            .remove_dir_all_if_present("absent/parents/staging")
            .expect("absence is a value")
    );
    assert!(!root.path().join("absent").exists());
}

#[test]
fn remove_dir_all_if_present_refuses_a_non_directory() {
    let root = tempfile::tempdir().expect("tempdir");
    std::fs::write(root.path().join("staging"), b"keep").expect("file writes");
    let project = Project::open(root.path()).expect("capability opens");

    assert!(project.remove_dir_all_if_present("staging").is_err());
    assert_eq!(std::fs::read(root.path().join("staging")).unwrap(), b"keep");
}

#[cfg(unix)]
#[test]
fn remove_dir_all_if_present_refuses_a_link_without_touching_its_target() {
    use std::os::unix::fs::symlink;

    let root = tempfile::tempdir().expect("tempdir");
    let outside = tempfile::tempdir().expect("outside tempdir");
    std::fs::write(outside.path().join("sentinel"), b"keep").expect("sentinel writes");
    symlink(outside.path(), root.path().join("staging")).expect("symlink creates");
    let project = Project::open(root.path()).expect("capability opens");

    assert!(project.remove_dir_all_if_present("staging").is_err());
    assert_eq!(
        std::fs::read(outside.path().join("sentinel")).unwrap(),
        b"keep"
    );
}

#[cfg(unix)]
#[test]
fn reset_dir_refuses_a_symlink_ancestor_without_touching_its_target() {
    use std::os::unix::fs::symlink;

    let root = tempfile::tempdir().expect("tempdir");
    let outside = tempfile::tempdir().expect("outside tempdir");
    std::fs::write(outside.path().join("sentinel"), b"keep").expect("sentinel writes");
    symlink(outside.path(), root.path().join("linked")).expect("symlink creates");
    let project = Project::open(root.path()).expect("capability opens");

    let error = project
        .reset_dir("linked/output")
        .expect_err("the symlink ancestor refuses");

    assert!(format!("{error:#}").contains("no-follow"));
    assert_eq!(
        std::fs::read(outside.path().join("sentinel")).unwrap(),
        b"keep",
    );
    assert!(!outside.path().join("output").exists());
}
