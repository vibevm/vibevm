//! Integration: workspace provisioning — worktree, dir, and none modes
//! exercised against a real scratch git repo, plus worktree removal.

use camino::{Utf8Path, Utf8PathBuf};
use fractality_core::ids::RunId;
use fractality_core::packet::{WorkspaceMode, WorkspaceSpec};
use fractality_mission_control::workspace::{branch_name, provision, remove_worktree};
use std::str::FromStr;

fn scratch_dir(tag: &str) -> Utf8PathBuf {
    let dir = std::env::temp_dir().join(format!("fractality-ws-{tag}-{}", ulid::Ulid::new()));
    Utf8PathBuf::from_path_buf(dir).expect("utf-8 temp dir")
}

fn sample_run_id() -> RunId {
    RunId::from_str("01ARZ3NDEKTSV4RRFFQ69G5FAV").expect("valid ulid")
}

fn git(repo: &Utf8Path, args: &[&str]) -> String {
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(repo.as_str())
        .args(args)
        .output()
        .expect("git spawns");
    assert!(
        output.status.success(),
        "git {:?} in {repo} failed: {}",
        args,
        String::from_utf8_lossy(&output.stderr).trim()
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn seed_repo(repo: &Utf8Path) {
    git(repo, &["init", "-b", "main", "."]);
    git(repo, &["config", "user.email", "test@example.com"]);
    git(repo, &["config", "user.name", "test"]);
    git(repo, &["config", "commit.gpgsign", "false"]);
    std::fs::write(repo.join("seed.txt").as_std_path(), "seed").expect("seed.txt writes");
    git(repo, &["add", "."]);
    git(repo, &["commit", "-m", "seed"]);
}

#[test]
fn worktree_mode_provisions_a_branch_checkout() {
    let repo = scratch_dir("prov-repo");
    std::fs::create_dir_all(repo.as_std_path()).expect("repo dir");
    seed_repo(&repo);

    let run_dir = scratch_dir("prov-run");
    std::fs::create_dir_all(run_dir.as_std_path()).expect("run dir");

    let run_id = sample_run_id();
    let expected_branch = branch_name(run_id);
    let spec = WorkspaceSpec {
        mode: WorkspaceMode::Worktree,
        repo: repo.as_str().to_string(),
        base: "main".to_string(),
    };
    let wt = provision(&spec, run_id, &run_dir).expect("worktree provisioned");

    assert_eq!(wt, run_dir.join("wt"));
    assert!(wt.is_dir(), "the worktree is a real directory");

    let head = git(&wt, &["rev-parse", "--abbrev-ref", "HEAD"]);
    assert_eq!(head.trim(), expected_branch, "HEAD sits on the run branch");

    assert!(
        wt.join("seed.txt").is_file(),
        "the base contents were checked out"
    );

    let branches = git(&repo, &["branch", "--list", &expected_branch]);
    assert!(
        !branches.trim().is_empty(),
        "the run branch exists in the repo"
    );

    remove_worktree(repo.as_str(), &wt).ok();
    std::fs::remove_dir_all(run_dir.as_std_path()).ok();
    std::fs::remove_dir_all(repo.as_std_path()).ok();
}

#[test]
fn remove_worktree_removes_dir_but_keeps_branch() {
    let repo = scratch_dir("rm-repo");
    std::fs::create_dir_all(repo.as_std_path()).expect("repo dir");
    seed_repo(&repo);

    let run_dir = scratch_dir("rm-run");
    std::fs::create_dir_all(run_dir.as_std_path()).expect("run dir");

    let run_id = sample_run_id();
    let expected_branch = branch_name(run_id);
    let spec = WorkspaceSpec {
        mode: WorkspaceMode::Worktree,
        repo: repo.as_str().to_string(),
        base: "main".to_string(),
    };
    let wt = provision(&spec, run_id, &run_dir).expect("worktree provisioned");

    remove_worktree(repo.as_str(), &wt).expect("worktree removed");

    assert!(!wt.exists(), "the worktree directory is gone");
    let list = git(&repo, &["worktree", "list"]);
    assert!(
        !list.contains(wt.as_str()),
        "worktree list no longer mentions the worktree: {list}"
    );
    let branches = git(&repo, &["branch", "--list", &expected_branch]);
    assert!(
        !branches.trim().is_empty(),
        "the branch survives worktree removal"
    );

    std::fs::remove_dir_all(run_dir.as_std_path()).ok();
    std::fs::remove_dir_all(repo.as_std_path()).ok();
}

#[test]
fn worktree_mode_with_unknown_base_fails_loudly() {
    let repo = scratch_dir("bad-repo");
    std::fs::create_dir_all(repo.as_std_path()).expect("repo dir");
    seed_repo(&repo);

    let run_dir = scratch_dir("bad-run");
    std::fs::create_dir_all(run_dir.as_std_path()).expect("run dir");

    let spec = WorkspaceSpec {
        mode: WorkspaceMode::Worktree,
        repo: repo.as_str().to_string(),
        base: "no-such-ref".to_string(),
    };
    let err =
        provision(&spec, sample_run_id(), &run_dir).expect_err("an unknown base must be rejected");
    assert!(
        err.contains("git worktree add failed"),
        "expected a loud git failure, got: {err}"
    );

    std::fs::remove_dir_all(run_dir.as_std_path()).ok();
    std::fs::remove_dir_all(repo.as_std_path()).ok();
}

#[test]
fn dir_mode_creates_the_work_dir() {
    let run_dir = scratch_dir("dir-run");
    std::fs::create_dir_all(run_dir.as_std_path()).expect("run dir");

    let spec = WorkspaceSpec {
        mode: WorkspaceMode::Dir,
        repo: String::new(),
        base: String::new(),
    };
    let work = provision(&spec, sample_run_id(), &run_dir).expect("dir provisioned");
    assert_eq!(work, run_dir.join("work"));
    assert!(work.is_dir(), "the work directory exists");

    std::fs::remove_dir_all(run_dir.as_std_path()).ok();
}

#[test]
fn none_mode_returns_run_dir_untouched() {
    let run_dir = scratch_dir("none-run");
    std::fs::create_dir_all(run_dir.as_std_path()).expect("run dir");

    let spec = WorkspaceSpec {
        mode: WorkspaceMode::None,
        repo: String::new(),
        base: String::new(),
    };
    let got = provision(&spec, sample_run_id(), &run_dir).expect("none provisioned");
    assert_eq!(got, run_dir);
    assert!(
        !run_dir.join("work").exists(),
        "none mode creates no work directory"
    );

    std::fs::remove_dir_all(run_dir.as_std_path()).ok();
}
