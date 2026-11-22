//! What a publish from inside a Git worktree carries, and what it leaves.
//!
//! The one case that needs a real worktree around the source — rather than a
//! bare origin and a scratch directory — lives apart from the rest of the
//! push tests, which only ever look at the origin's refs.

use std::fs;

use tempfile::tempdir;

use super::push_release;
use super::tests::{git_available, init_bare, write_package};

/// A source inside a Git worktree publishes exactly what Git versions there:
/// ignored debris left by running the package's tests stays behind, and the
/// published tree hashes as the source does — the hash the index receives
/// (PROP-024 `#SHIPPABLE-TREE-DEF`).
#[test]
fn publish_from_a_worktree_leaves_ignored_files_behind() {
    if !git_available() {
        eprintln!("skipping: git not on PATH");
        return;
    }
    let git = vibe_core::git_source::fixture::FixtureGit::new();
    let outer = tempdir().unwrap();
    let bare = init_bare(outer.path());
    let repo = outer.path().join("author");
    git.init(&repo);
    fs::write(repo.join(".gitignore"), "__pycache__/\n**/.vibe-boot-*\n").unwrap();
    let source = repo.join("registry/org.vibevm/wal/v0.1.0");
    fs::create_dir_all(&source).unwrap();
    write_package(&source, "protocol");
    git.commit_all(&repo, "fixture");
    fs::create_dir_all(source.join("spec/__pycache__")).unwrap();
    fs::write(
        source.join("spec/__pycache__/x.cpython-314.pyc"),
        "BYTECODE",
    )
    .unwrap();
    fs::write(source.join("spec/.vibe-boot-artifacts.lock"), "").unwrap();

    let url = bare.to_string_lossy().into_owned();
    let v = semver::Version::parse("0.1.0").unwrap();
    push_release(&source, &url, "v0.1.0", "wal", &v).expect("push ok");

    let published = outer.path().join("published");
    git.clone(&bare, &published);
    git.run(&published, &["checkout", "-q", "main"]);
    assert!(published.join("spec/PROTOCOL.md").is_file());
    assert!(!published.join("spec/__pycache__").exists());
    assert!(!published.join("spec/.vibe-boot-artifacts.lock").exists());
    assert_eq!(
        vibe_registry::compute_content_hash(&published).unwrap(),
        vibe_registry::compute_content_hash(&source).unwrap()
    );
}
