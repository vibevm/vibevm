//! Thin shell-out helpers around the `git` binary. Same posture
//! `vibe-registry::git_backend::shell` takes — no native library, no
//! C-toolchain dependency, works everywhere `git` works.
//!
//! The functions exposed here are just enough for the
//! `--from-clones` scanner: list tags, materialise a tagged tree to
//! a clean directory (no `.git/` left behind), resolve a tag to a
//! commit SHA. Richer git operations land when the scanner needs
//! them.

use std::path::Path;
use std::process::Command;

use crate::error::{Error, Result};

pub fn binary() -> String {
    std::env::var("VIBE_INDEX_GIT").unwrap_or_else(|_| "git".to_string())
}

/// Return `true` iff `path` looks like a git working tree (has a
/// `.git` directory or `.git` file). Does NOT shell out — it's a
/// directory test.
pub fn is_git_dir(path: &Path) -> bool {
    let dot_git = path.join(".git");
    dot_git.is_dir() || dot_git.is_file()
}

pub fn list_tags(repo: &Path) -> Result<Vec<String>> {
    let out = run(&[
        "-C",
        repo.to_str().ok_or_else(|| {
            Error::InvalidInput(format!("repo path `{}` is not UTF-8", repo.display()))
        })?,
        "tag",
        "-l",
    ])?;
    Ok(out
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect())
}

pub fn resolve_commit(repo: &Path, tag: &str) -> Result<String> {
    let out = run(&[
        "-C",
        repo.to_str().ok_or_else(|| {
            Error::InvalidInput(format!("repo path `{}` is not UTF-8", repo.display()))
        })?,
        "rev-list",
        "-n",
        "1",
        tag,
    ])?;
    Ok(out.trim().to_string())
}

/// Best-effort HEAD commit of the repo's working tree. Returns
/// `None` for empty repositories or weird states (no commits at all).
pub fn head_commit(repo: &Path) -> Option<String> {
    let out = std::process::Command::new(binary())
        .args(["-C", repo.to_str()?, "rev-parse", "HEAD"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if s.is_empty() { None } else { Some(s) }
}

/// Materialise the tag's working tree at `dest`. Uses
/// `git clone --depth 1 --branch <tag>` against the source repo path,
/// then removes the embedded `.git/` so the result is clean for
/// content-hashing (matches `vibe-registry`'s
/// `copy_dir_excluding_git` invariant).
pub fn materialise_at_ref(src_repo: &Path, tag: &str, dest: &Path) -> Result<()> {
    let src_str = src_repo.to_str().ok_or_else(|| {
        Error::InvalidInput(format!("repo path `{}` is not UTF-8", src_repo.display()))
    })?;
    let dest_str = dest.to_str().ok_or_else(|| {
        Error::InvalidInput(format!("dest path `{}` is not UTF-8", dest.display()))
    })?;
    let status = Command::new(binary())
        .args([
            "clone", "--quiet", "--depth", "1", "--branch", tag, src_str, dest_str,
        ])
        .status()
        .map_err(|e| Error::Io {
            path: src_repo.to_path_buf(),
            message: format!("git clone: {e}"),
        })?;
    if !status.success() {
        return Err(Error::Malformed(format!(
            "git clone --branch {tag} of `{}` failed",
            src_repo.display()
        )));
    }
    let dot_git = dest.join(".git");
    if dot_git.exists() {
        // `.git` may be a file (worktree marker) or a directory; remove either.
        if dot_git.is_dir() {
            std::fs::remove_dir_all(&dot_git).map_err(|e| Error::Io {
                path: dot_git.clone(),
                message: e.to_string(),
            })?;
        } else {
            std::fs::remove_file(&dot_git).map_err(|e| Error::Io {
                path: dot_git.clone(),
                message: e.to_string(),
            })?;
        }
    }
    Ok(())
}

fn run(args: &[&str]) -> Result<String> {
    let out = Command::new(binary())
        .args(args)
        .output()
        .map_err(|e| Error::Io {
            path: std::path::PathBuf::from("git"),
            message: format!("invoking git: {e}"),
        })?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        return Err(Error::Malformed(format!(
            "git {:?} failed: {}",
            args,
            stderr.trim()
        )));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn git_available() -> bool {
        Command::new("git").arg("--version").output().is_ok()
    }

    fn make_repo(parent: &Path, name: &str, versions: &[(&str, &str)]) -> std::path::PathBuf {
        let repo = parent.join(name);
        fs::create_dir_all(&repo).unwrap();
        Command::new("git")
            .args([
                "-C",
                repo.to_str().unwrap(),
                "init",
                "--quiet",
                "-b",
                "main",
            ])
            .status()
            .unwrap();
        Command::new("git")
            .args([
                "-C",
                repo.to_str().unwrap(),
                "config",
                "user.email",
                "test@test.invalid",
            ])
            .status()
            .unwrap();
        Command::new("git")
            .args(["-C", repo.to_str().unwrap(), "config", "user.name", "Test"])
            .status()
            .unwrap();
        for (tag, manifest_body) in versions {
            fs::write(repo.join("vibe.toml"), manifest_body).unwrap();
            fs::write(repo.join("README.md"), format!("# {tag}\n")).unwrap();
            Command::new("git")
                .args(["-C", repo.to_str().unwrap(), "add", "."])
                .status()
                .unwrap();
            Command::new("git")
                .args(["-C", repo.to_str().unwrap(), "commit", "--quiet", "-m", tag])
                .status()
                .unwrap();
            Command::new("git")
                .args(["-C", repo.to_str().unwrap(), "tag", tag])
                .status()
                .unwrap();
        }
        repo
    }

    fn manifest_for(version: &str) -> String {
        format!(
            "[package]\nname = \"wal\"\nkind = \"flow\"\nversion = \"{version}\"\nlicense = \"EULA\"\n"
        )
    }

    #[test]
    fn is_git_dir_distinguishes_clean_from_repo() {
        let dir = tempdir().unwrap();
        assert!(!is_git_dir(dir.path()));
        if !git_available() {
            return;
        }
        let repo = make_repo(
            dir.path(),
            "flow-wal",
            &[("v0.1.0", &manifest_for("0.1.0"))],
        );
        assert!(is_git_dir(&repo));
    }

    #[test]
    fn list_tags_returns_sorted_v_tags() {
        if !git_available() {
            return;
        }
        let dir = tempdir().unwrap();
        let repo = make_repo(
            dir.path(),
            "flow-wal",
            &[
                ("v0.1.0", &manifest_for("0.1.0")),
                ("v0.2.0", &manifest_for("0.2.0")),
            ],
        );
        let mut tags = list_tags(&repo).unwrap();
        tags.sort();
        assert_eq!(tags, vec!["v0.1.0".to_string(), "v0.2.0".to_string()]);
    }

    #[test]
    fn resolve_commit_returns_sha() {
        if !git_available() {
            return;
        }
        let dir = tempdir().unwrap();
        let repo = make_repo(
            dir.path(),
            "flow-wal",
            &[("v0.1.0", &manifest_for("0.1.0"))],
        );
        let sha = resolve_commit(&repo, "v0.1.0").unwrap();
        assert_eq!(sha.len(), 40, "expected a 40-char SHA, got `{sha}`");
    }

    #[test]
    fn materialise_at_ref_strips_git_dir() {
        if !git_available() {
            return;
        }
        let parent = tempdir().unwrap();
        let repo = make_repo(
            parent.path(),
            "flow-wal",
            &[("v0.1.0", &manifest_for("0.1.0"))],
        );
        let dest_holder = tempdir().unwrap();
        let dest = dest_holder.path().join("snapshot");
        materialise_at_ref(&repo, "v0.1.0", &dest).unwrap();
        assert!(dest.join("vibe.toml").is_file());
        assert!(dest.join("README.md").is_file());
        assert!(!dest.join(".git").exists());
    }
}
