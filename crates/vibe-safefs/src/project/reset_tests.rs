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
