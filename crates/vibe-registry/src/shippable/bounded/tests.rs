use super::*;

fn pin() -> PackageRef {
    PackageRef::parse("flow:org.example/fixture@=1.1.0").unwrap()
}

fn fixture() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("docs")).unwrap();
    std::fs::write(
        dir.path().join("vibe.toml"),
        "[package]\ngroup = 'org.example'\nname = 'fixture'\nkind = 'flow'\nversion = '1.1.0'\n",
    )
    .unwrap();
    std::fs::write(dir.path().join("docs/a.md"), b"nested\n").unwrap();
    std::fs::write(dir.path().join("docsX.md"), b"sibling\n").unwrap();
    dir
}

fn rows(root: &Path) -> Vec<BoundedContentFile> {
    ["docs/a.md", "docsX.md", "vibe.toml"]
        .into_iter()
        .map(|relative_path| {
            let bytes = std::fs::read(root.join(relative_path)).unwrap();
            BoundedContentFile {
                relative_path: relative_path.into(),
                bytes: bytes.len() as u64,
                sha256: format!("sha256:{:x}", Sha256::digest(&bytes)),
            }
        })
        .collect()
}

#[test]
fn bounded_nofollow_replay_matches_frozen_legacy_recipe_for_portable_nested_paths() {
    let dir = fixture();
    let expected = rows(dir.path());
    let replay = compute_content_hash_legacy0_bounded_exact(dir.path(), &pin(), &expected).unwrap();
    assert_eq!(
        replay.as_str(),
        super::super::compute_content_hash(dir.path()).unwrap()
    );
    assert_eq!(
        compute_content_hash_legacy0_bounded_exact(dir.path(), &pin(), &expected).unwrap(),
        replay
    );
}

#[test]
fn changed_extra_and_missing_bytes_refuse_without_partial_hash() {
    let dir = fixture();
    let expected = rows(dir.path());
    std::fs::write(dir.path().join("docs/a.md"), b"changed\n").unwrap();
    assert!(compute_content_hash_legacy0_bounded_exact(dir.path(), &pin(), &expected).is_err());
    std::fs::write(dir.path().join("docs/a.md"), b"nested\n").unwrap();
    std::fs::write(dir.path().join("extra.txt"), b"extra").unwrap();
    assert!(compute_content_hash_legacy0_bounded_exact(dir.path(), &pin(), &expected).is_err());
    std::fs::remove_file(dir.path().join("extra.txt")).unwrap();
    std::fs::remove_file(dir.path().join("docsX.md")).unwrap();
    assert!(compute_content_hash_legacy0_bounded_exact(dir.path(), &pin(), &expected).is_err());
}

#[cfg(unix)]
#[test]
fn expected_symlink_is_refused_by_held_nofollow_read() {
    use std::os::unix::fs::symlink;

    let dir = fixture();
    let expected = rows(dir.path());
    std::fs::remove_file(dir.path().join("docs/a.md")).unwrap();
    symlink(dir.path().join("vibe.toml"), dir.path().join("docs/a.md")).unwrap();
    assert!(compute_content_hash_legacy0_bounded_exact(dir.path(), &pin(), &expected).is_err());
}

#[test]
fn oversized_file_and_forged_row_refuse() {
    let dir = fixture();
    let expected = rows(dir.path());
    let mut forged = expected.clone();
    forged[0].sha256 = format!("sha256:{}", "0".repeat(64));
    assert!(compute_content_hash_legacy0_bounded_exact(dir.path(), &pin(), &forged).is_err());
    std::fs::write(dir.path().join("docs/a.md"), vec![b'x'; FILE_CAP + 1]).unwrap();
    assert!(compute_content_hash_legacy0_bounded_exact(dir.path(), &pin(), &expected).is_err());
}
