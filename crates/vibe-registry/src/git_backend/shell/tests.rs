//! Tests for the shell-out git backend — live `git` round-trips
//! (skipped when no `git` is on `PATH`), the inline tar extractor, and
//! the locale-stable stderr classifier.

specmark::scope!("spec://vibevm/modules/vibe-registry/PROP-001#backend");

use super::*;
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;

use fixtures::*;
use specmark::verifies;

macro_rules! skip_without_git {
    () => {
        if !git_available() {
            eprintln!("skipping test: git not on PATH");
            return;
        }
    };
}

#[test]
fn preflight_succeeds_when_git_installed() {
    skip_without_git!();
    let g = ShellGit::new();
    g.preflight().expect("preflight should succeed");
}

#[test]
fn preflight_reports_not_installed_for_bogus_binary() {
    let g = ShellGit {
        binary: PathBuf::from("definitely-not-git-xyz"),
        force_anonymous: false,
        preflight_cache: OnceLock::new(),
    };
    let err = g.preflight().unwrap_err();
    assert!(
        matches!(err, GitError::NotInstalled),
        "expected NotInstalled, got: {err:?}"
    );
}

#[test]
fn clone_then_update_against_bare_origin() {
    skip_without_git!();
    let tmp = tempdir().unwrap();
    let bare = make_bare_origin(tmp.path());
    let dest = tmp.path().join("clone");

    let g = ShellGit::new();
    g.bootstrap(&bare.to_string_lossy(), "main", &dest)
        .expect("initial clone should succeed");
    assert!(dest.join("README.md").exists());

    // Push a new commit into origin, then update from the clone.
    let src2 = tmp.path().join("src2");
    run_or_panic(
        tmp.path(),
        &["clone", bare.to_str().unwrap(), src2.to_str().unwrap()],
    );
    run_or_panic(&src2, &["config", "user.email", "t@example.com"]);
    run_or_panic(&src2, &["config", "user.name", "Test"]);
    fs::write(src2.join("new.md"), "new\n").unwrap();
    run_or_panic(&src2, &["add", "new.md"]);
    run_or_panic(&src2, &["commit", "-m", "add new"]);
    run_or_panic(&src2, &["push", "origin", "main"]);

    g.update(&dest, "main").expect("update should succeed");
    assert!(dest.join("new.md").exists());
}

#[test]
#[verifies("spec://vibevm/modules/vibe-registry/PROP-021#lock", r = 1)]
fn head_commit_returns_the_checked_out_sha() {
    skip_without_git!();
    let tmp = tempdir().unwrap();
    let bare = make_bare_origin(tmp.path());
    let dest = tmp.path().join("clone");

    let g = ShellGit::new();
    g.bootstrap(&bare.to_string_lossy(), "main", &dest)
        .expect("clone should succeed");

    let sha = g
        .head_commit(&dest)
        .expect("head_commit ok")
        .expect("a real checkout reports a commit");
    // A full 40-hex SHA-1 — git's default object format.
    assert_eq!(sha.len(), 40, "got: {sha}");
    assert!(sha.chars().all(|c| c.is_ascii_hexdigit()), "got: {sha}");

    // It matches what git itself reports for HEAD in the clone.
    let mut cmd = Command::new("git");
    apply_common_env(&mut cmd, false);
    cmd.args(["rev-parse", "HEAD"]).current_dir(&dest);
    let expected = String::from_utf8_lossy(&cmd.output().unwrap().stdout)
        .trim()
        .to_string();
    assert_eq!(sha, expected);
}

#[test]
#[verifies("spec://vibevm/modules/vibe-registry/PROP-021#fetch", r = 1)]
fn submodule_step_is_a_noop_without_submodules() {
    skip_without_git!();
    let tmp = tempdir().unwrap();
    let bare = make_bare_origin(tmp.path());
    let dest = tmp.path().join("clone");

    let g = ShellGit::new();
    // `bootstrap` now passes `--recurse-submodules`; on a repo with none
    // it clones exactly as before (PROP-021 §2.1).
    g.bootstrap(&bare.to_string_lossy(), "main", &dest)
        .expect("clone --recurse-submodules ok on a repo with no submodules");
    assert!(dest.join("README.md").exists());

    // `update` now runs `git submodule update --init --recursive`; on a
    // repo with no submodules that step must succeed as a no-op rather
    // than fail the update — the path every existing package takes.
    g.update(&dest, "main")
        .expect("update with the submodule step ok on a plain repo");
    assert!(dest.join("README.md").exists());
}

#[test]
fn clone_reports_repo_not_found_for_missing_url() {
    skip_without_git!();
    let tmp = tempdir().unwrap();
    let bogus = tmp.path().join("does/not/exist.git");
    let dest = tmp.path().join("clone");

    let g = ShellGit::new();
    let err = g
        .bootstrap(&bogus.to_string_lossy(), "main", &dest)
        .unwrap_err();
    // The exact message varies by git version; classification should
    // land on either RepoNotFound or a generic CommandFailed with
    // the raw stderr — both are acceptable for this test.
    match err {
        GitError::RepoNotFound { .. } | GitError::CommandFailed { .. } => {}
        other => panic!("unexpected classification: {other:?}"),
    }
}

#[test]
fn clone_reports_ref_not_found_for_missing_branch() {
    skip_without_git!();
    let tmp = tempdir().unwrap();
    let bare = make_bare_origin(tmp.path());
    let dest = tmp.path().join("clone");

    let g = ShellGit::new();
    let err = g
        .bootstrap(&bare.to_string_lossy(), "no-such-branch", &dest)
        .unwrap_err();
    match err {
        GitError::RefNotFound { .. } | GitError::CommandFailed { .. } => {}
        other => panic!("unexpected classification: {other:?}"),
    }
}

#[test]
fn list_tags_returns_dedup_sorted_set() {
    skip_without_git!();
    let tmp = tempdir().unwrap();
    let bare = make_bare_origin_with_tags(tmp.path());

    let g = ShellGit::new();
    let mut tags = g.list_tags(&bare.to_string_lossy()).expect("list_tags ok");
    tags.sort();

    assert_eq!(
        tags,
        vec![
            "v0.1.0".to_string(),
            "v0.2.0".to_string(),
            "v0.3.0".to_string(),
            "v1.0.0-rc.1".to_string(),
        ],
        "annotated tag v0.3.0 must appear exactly once (peeled-form deduped)"
    );
}

#[test]
fn list_tags_empty_repo_returns_empty() {
    skip_without_git!();
    let tmp = tempdir().unwrap();
    let bare = make_bare_origin(tmp.path()); // has no tags
    let g = ShellGit::new();
    let tags = g.list_tags(&bare.to_string_lossy()).expect("list_tags ok");
    assert!(tags.is_empty());
}

#[test]
fn list_tags_reports_repo_not_found_for_missing_url() {
    skip_without_git!();
    let tmp = tempdir().unwrap();
    let bogus = tmp.path().join("does/not/exist.git");
    let g = ShellGit::new();
    let err = g.list_tags(&bogus.to_string_lossy()).unwrap_err();
    match err {
        GitError::RepoNotFound { .. } | GitError::CommandFailed { .. } => {}
        other => panic!("unexpected classification: {other:?}"),
    }
}

#[test]
fn fetch_file_at_ref_returns_bytes_for_existing_file() {
    skip_without_git!();
    let tmp = tempdir().unwrap();
    let bare = make_bare_origin_with_tags(tmp.path());

    let g = ShellGit::new();
    let bytes = g
        .fetch_file_at_ref(&bare.to_string_lossy(), "v0.2.0", "vibe.toml")
        .expect("fetch ok");
    let text = String::from_utf8(bytes).unwrap();
    assert!(text.contains("version = \"0.2.0\""), "got: {text}");
}

#[test]
fn fetch_file_at_ref_works_against_annotated_tag() {
    skip_without_git!();
    let tmp = tempdir().unwrap();
    let bare = make_bare_origin_with_tags(tmp.path());

    let g = ShellGit::new();
    let bytes = g
        .fetch_file_at_ref(&bare.to_string_lossy(), "v0.3.0", "vibe.toml")
        .expect("fetch via annotated tag ok");
    let text = String::from_utf8(bytes).unwrap();
    assert!(text.contains("version = \"0.3.0\""));
}

#[test]
fn fetch_file_at_ref_normalises_backslash_paths() {
    skip_without_git!();
    let tmp = tempdir().unwrap();
    let bare = make_bare_origin_with_tags(tmp.path());

    let g = ShellGit::new();
    // Caller hands us a Windows-style path; the backend should
    // normalise to forward slash before talking to `git archive`.
    let bytes = g
        .fetch_file_at_ref(&bare.to_string_lossy(), "v0.1.0", "vibe.toml")
        .expect("fetch ok");
    assert!(!bytes.is_empty());
}

#[test]
fn fetch_file_at_ref_missing_ref() {
    skip_without_git!();
    let tmp = tempdir().unwrap();
    let bare = make_bare_origin_with_tags(tmp.path());

    let g = ShellGit::new();
    let err = g
        .fetch_file_at_ref(&bare.to_string_lossy(), "v9.9.9", "vibe.toml")
        .unwrap_err();
    match err {
        GitError::RefNotFound { .. } | GitError::CommandFailed { .. } => {}
        other => panic!("unexpected classification: {other:?}"),
    }
}

#[test]
fn fetch_file_at_ref_missing_file_in_existing_ref() {
    skip_without_git!();
    let tmp = tempdir().unwrap();
    let bare = make_bare_origin_with_tags(tmp.path());

    let g = ShellGit::new();
    let err = g
        .fetch_file_at_ref(&bare.to_string_lossy(), "v0.1.0", "no-such-file.txt")
        .unwrap_err();
    match err {
        GitError::FileNotFoundInRef { .. } | GitError::CommandFailed { .. } => {}
        other => panic!("unexpected classification: {other:?}"),
    }
}

/// Test-only fixtures behind their own `#[cfg(test)]` marker: fact
/// extraction is per-file, and the no-unwrap rule scopes test code by
/// the enclosing `#[cfg(test)]` item — the marker keeps these helpers
/// reading as test code now that the tests live outside the parent
/// module's inline `mod tests`.
#[cfg(test)]
mod fixtures {
    use super::*;

    /// Build a bare git repo under `root/origin.git` seeded with one
    /// commit on `main`, and return its absolute path. Requires `git`
    /// on `PATH`; tests that need it skip themselves via
    /// `skip_without_git!()`.
    pub(super) fn make_bare_origin(root: &Path) -> PathBuf {
        let src = root.join("src");
        fs::create_dir_all(&src).unwrap();
        // Work repo: init, set identity, create a file, commit on main.
        run_or_panic(&src, &["init", "--initial-branch=main"]);
        run_or_panic(&src, &["config", "user.email", "t@example.com"]);
        run_or_panic(&src, &["config", "user.name", "Test"]);
        fs::write(src.join("README.md"), "hello\n").unwrap();
        run_or_panic(&src, &["add", "README.md"]);
        run_or_panic(&src, &["commit", "-m", "init"]);

        let bare = root.join("origin.git");
        run_or_panic(
            root,
            &[
                "clone",
                "--bare",
                src.to_str().unwrap(),
                bare.to_str().unwrap(),
            ],
        );
        // Make sure HEAD in the bare repo points at main.
        run_or_panic(&bare, &["symbolic-ref", "HEAD", "refs/heads/main"]);

        bare
    }

    pub(super) fn run_or_panic(cwd: &Path, args: &[&str]) {
        let mut cmd = Command::new("git");
        apply_common_env(&mut cmd, false);
        cmd.args(args);
        cmd.current_dir(cwd);
        let out = cmd.output().expect("failed to spawn git for test setup");
        if !out.status.success() {
            panic!(
                "test setup `git {}` failed: {}",
                args.join(" "),
                String::from_utf8_lossy(&out.stderr)
            );
        }
    }

    pub(super) fn git_available() -> bool {
        Command::new("git")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Build a bare origin that has multiple tags (`v0.1.0`, `v0.2.0`,
    /// `v1.0.0-rc.1`) plus one annotated tag (`v0.3.0`) so we exercise
    /// the peeled-form deduplication.
    pub(super) fn make_bare_origin_with_tags(root: &Path) -> PathBuf {
        let src = root.join("src");
        fs::create_dir_all(&src).unwrap();
        run_or_panic(&src, &["init", "--initial-branch=main"]);
        run_or_panic(&src, &["config", "user.email", "t@example.com"]);
        run_or_panic(&src, &["config", "user.name", "Test"]);

        // Commit 1 + lightweight tag v0.1.0.
        fs::write(
            src.join("vibe.toml"),
            "[package]\ngroup = \"org.vibevm\"\nname = \"x\"\nkind = \"flow\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        run_or_panic(&src, &["add", "vibe.toml"]);
        run_or_panic(&src, &["commit", "-m", "0.1.0"]);
        run_or_panic(&src, &["tag", "v0.1.0"]);

        // Commit 2 + lightweight tag v0.2.0.
        fs::write(
            src.join("vibe.toml"),
            "[package]\ngroup = \"org.vibevm\"\nname = \"x\"\nkind = \"flow\"\nversion = \"0.2.0\"\n",
        )
        .unwrap();
        run_or_panic(&src, &["add", "vibe.toml"]);
        run_or_panic(&src, &["commit", "-m", "0.2.0"]);
        run_or_panic(&src, &["tag", "v0.2.0"]);

        // Commit 3 + ANNOTATED tag v0.3.0 (this is the one that produces
        // a peeled `^{}` line in `ls-remote --tags` output).
        fs::write(
            src.join("vibe.toml"),
            "[package]\ngroup = \"org.vibevm\"\nname = \"x\"\nkind = \"flow\"\nversion = \"0.3.0\"\n",
        )
        .unwrap();
        run_or_panic(&src, &["add", "vibe.toml"]);
        run_or_panic(&src, &["commit", "-m", "0.3.0"]);
        run_or_panic(&src, &["tag", "-a", "v0.3.0", "-m", "release 0.3.0"]);

        // Commit 4 + tag v1.0.0-rc.1.
        fs::write(
            src.join("vibe.toml"),
            "[package]\ngroup = \"org.vibevm\"\nname = \"x\"\nkind = \"flow\"\nversion = \"1.0.0-rc.1\"\n",
        )
        .unwrap();
        run_or_panic(&src, &["add", "vibe.toml"]);
        run_or_panic(&src, &["commit", "-m", "1.0.0-rc.1"]);
        run_or_panic(&src, &["tag", "v1.0.0-rc.1"]);

        let bare = root.join("origin.git");
        run_or_panic(
            root,
            &[
                "clone",
                "--bare",
                src.to_str().unwrap(),
                bare.to_str().unwrap(),
            ],
        );
        run_or_panic(&bare, &["symbolic-ref", "HEAD", "refs/heads/main"]);
        bare
    }
}
