use super::*;

const MANIFEST: &str =
    "[package]\ngroup = 'org.example'\nname = 'fixture'\nkind = 'flow'\nversion = '1.1.0'\n";

fn pin() -> PackageRef {
    PackageRef::parse("flow:org.example/fixture@=1.1.0").unwrap()
}

fn fixture() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("vibe.toml"), MANIFEST).unwrap();
    std::fs::write(dir.path().join("README.md"), b"source\n").unwrap();
    dir
}

#[test]
fn stable_tree_is_complete_sorted_and_rechecks() {
    let dir = fixture();
    let witness =
        capture_package_tree_witness(dir.path(), &pin(), &["vibe.toml", "README.md"]).unwrap();
    assert_eq!(witness.package_pin, pin());
    assert_eq!(witness.files.len(), 2);
    assert_eq!(witness.files[0].relative_path, "README.md");
    assert_eq!(witness.files[1].relative_path, "vibe.toml");
    assert_eq!(witness.files[0].bytes, 7);
    assert!(witness.tree_sha256.starts_with("sha256:"));
    recheck_package_tree_witness(&witness).unwrap();
    assert_eq!(
        capture_package_tree_witness(dir.path(), &pin(), &["README.md", "vibe.toml"])
            .unwrap()
            .tree_sha256,
        witness.tree_sha256
    );
}

#[test]
fn changed_byte_and_manifest_identity_are_refused() {
    let dir = fixture();
    let witness =
        capture_package_tree_witness(dir.path(), &pin(), &["vibe.toml", "README.md"]).unwrap();
    std::fs::write(dir.path().join("README.md"), b"Source\n").unwrap();
    assert!(matches!(
        recheck_package_tree_witness(&witness),
        Err(PackageTreeWitnessError::Drift(_))
    ));
    std::fs::write(dir.path().join("README.md"), b"source\n").unwrap();
    std::fs::write(
        dir.path().join("vibe.toml"),
        MANIFEST.replace("version = '1.1.0'", "version = '1.1.1'"),
    )
    .unwrap();
    assert!(capture_package_tree_witness(dir.path(), &pin(), &["vibe.toml", "README.md"]).is_err());
}

#[test]
fn replaced_root_with_identical_bytes_still_refuses() {
    let parent = tempfile::tempdir().unwrap();
    let root = parent.path().join("source");
    std::fs::create_dir(&root).unwrap();
    std::fs::write(root.join("vibe.toml"), MANIFEST).unwrap();
    std::fs::write(root.join("README.md"), b"source\n").unwrap();
    let witness = capture_package_tree_witness(&root, &pin(), &["vibe.toml", "README.md"]).unwrap();
    std::fs::rename(&root, parent.path().join("old-source")).unwrap();
    std::fs::create_dir(&root).unwrap();
    std::fs::write(root.join("vibe.toml"), MANIFEST).unwrap();
    std::fs::write(root.join("README.md"), b"source\n").unwrap();
    assert!(matches!(
        recheck_package_tree_witness(&witness),
        Err(PackageTreeWitnessError::Drift(_))
    ));
}

#[test]
fn added_unknown_and_deleted_child_refuse() {
    let dir = fixture();
    let witness =
        capture_package_tree_witness(dir.path(), &pin(), &["vibe.toml", "README.md"]).unwrap();
    std::fs::write(dir.path().join("other.txt"), b"foreign").unwrap();
    assert!(matches!(
        recheck_package_tree_witness(&witness),
        Err(PackageTreeWitnessError::Capture(_))
    ));
    std::fs::remove_file(dir.path().join("other.txt")).unwrap();
    std::fs::remove_file(dir.path().join("README.md")).unwrap();
    assert!(matches!(
        recheck_package_tree_witness(&witness),
        Err(PackageTreeWitnessError::Drift(_))
    ));
}

#[cfg(unix)]
#[test]
fn expected_symlink_is_never_followed() {
    use std::os::unix::fs::symlink;

    let dir = fixture();
    std::fs::remove_file(dir.path().join("README.md")).unwrap();
    symlink(dir.path().join("vibe.toml"), dir.path().join("README.md")).unwrap();
    assert!(capture_package_tree_witness(dir.path(), &pin(), &["vibe.toml", "README.md"]).is_err());
}

#[test]
fn file_width_depth_and_byte_caps_refuse_before_unbounded_capture() {
    let dir = fixture();
    let mut too_many = vec!["vibe.toml"; FILE_COUNT_CAP + 1];
    too_many[1] = "README.md";
    assert!(matches!(
        capture_package_tree_witness(dir.path(), &pin(), &too_many),
        Err(PackageTreeWitnessError::InvalidSelection(_))
    ));
    assert!(matches!(
        capture_package_tree_witness(dir.path(), &pin(), &["vibe.toml", "a/b/c/d/e.txt"]),
        Err(PackageTreeWitnessError::InvalidSelection(_))
    ));
    std::fs::write(
        dir.path().join("README.md"),
        vec![b'x'; FILE_CAP as usize + 1],
    )
    .unwrap();
    assert!(matches!(
        capture_package_tree_witness(dir.path(), &pin(), &["vibe.toml", "README.md"]),
        Err(PackageTreeWitnessError::Capture(_))
    ));
}
