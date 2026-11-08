//! Writing a [`GitTree`] item exactly: into a Git working tree by tree
//! object (`read-tree --prefix`), into a plain directory by checkout of
//! that tree; and the check that what was written hashes to that tree
//! (PROP-002 `#registry-mirror`).
use std::path::Path;

use super::{GitTree, Staged, io};
use crate::PublishError;
use crate::git_publish::{git_command, run_git_in};

/// Insert `git.tree` at `path` of the working tree `tree` (index and
/// files), replacing whatever was there, and verify the index's subtree.
pub(super) fn insert(
    tree: &Path,
    stage: &Staged,
    git: &GitTree,
    index: usize,
) -> Result<(), PublishError> {
    let path = stage.path();
    let local = format!("refs/vibe-mirror/item-{index}");
    let repo = git.repo.to_string_lossy();
    run_git_in(
        tree,
        &[
            "fetch",
            "-q",
            "--no-tags",
            "--no-write-fetch-head",
            "--",
            &repo,
            &format!("+{}:{local}", git.reference),
        ],
    )?;
    run_git_in(
        tree,
        &[
            "rm",
            "-r",
            "-q",
            "--cached",
            "--ignore-unmatch",
            "--",
            &path,
        ],
    )?;
    let target = tree.join(&path);
    if target.exists() {
        std::fs::remove_dir_all(&target).map_err(|e| io(&target, "remove_dir_all", e))?;
    }
    run_git_in(
        tree,
        &["read-tree", &format!("--prefix={path}/"), "-u", &git.tree],
    )?;
    let written = run_git_in(tree, &["write-tree", &format!("--prefix={path}/")])?;
    check(stage, git, String::from_utf8_lossy(&written.stdout).trim())
}

/// Check out `git.tree` into the (absent) directory `dest`.
pub(super) fn checkout(git: &GitTree, dest: &Path) -> Result<(), PublishError> {
    let scratch = tempfile::TempDir::new().map_err(|e| io(dest, "creating an index", e))?;
    std::fs::create_dir_all(dest).map_err(|e| io(dest, "create_dir_all", e))?;
    let index = scratch.path().join("index");
    let work = dest.to_string_lossy();
    for args in [
        vec!["read-tree", git.tree.as_str()],
        vec!["--work-tree", work.as_ref(), "checkout-index", "-a", "-f"],
    ] {
        let output = git_command(&git.repo, &args)
            .env("GIT_INDEX_FILE", &index)
            .env("GIT_DIR", &git.repo)
            .output()
            .map_err(|e| PublishError::Git(format!("spawning git: {e}")))?;
        if !output.status.success() {
            return Err(PublishError::Git(format!(
                "checking out tree {} from `{}` failed: {}",
                git.tree,
                git.repo.display(),
                String::from_utf8_lossy(&output.stderr).trim()
            )));
        }
    }
    Ok(())
}

/// The Git tree hash of a plain directory (every file, `-f`: no ignore
/// rule applies), computed in `git.repo`'s object store.
pub(super) fn tree_of_dir(git: &GitTree, dir: &Path) -> Result<String, PublishError> {
    let scratch = tempfile::TempDir::new().map_err(|e| io(dir, "creating an index", e))?;
    let index = scratch.path().join("index");
    let work = dir.to_string_lossy();
    let mut written = String::new();
    for args in [
        vec!["--work-tree", work.as_ref(), "add", "-A", "-f", "."],
        vec!["write-tree"],
    ] {
        let output = git_command(&git.repo, &args)
            .env("GIT_INDEX_FILE", &index)
            .env("GIT_DIR", &git.repo)
            .output()
            .map_err(|e| PublishError::Git(format!("spawning git: {e}")))?;
        if !output.status.success() {
            return Err(PublishError::Git(format!(
                "hashing `{}` as a Git tree failed: {}",
                dir.display(),
                String::from_utf8_lossy(&output.stderr).trim()
            )));
        }
        written = String::from_utf8_lossy(&output.stdout).trim().to_string();
    }
    Ok(written)
}

pub(super) fn check(stage: &Staged, git: &GitTree, written: &str) -> Result<(), PublishError> {
    if written == git.tree {
        return Ok(());
    }
    Err(PublishError::TreeMismatch {
        package: stage.path(),
        written: written.to_string(),
        expected: git.tree.clone(),
    })
}
