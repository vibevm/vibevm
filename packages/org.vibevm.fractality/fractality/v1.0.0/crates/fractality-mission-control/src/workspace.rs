//! Workspace provisioning (plan D8): where a worker works.
//!
//! `worktree` — `git worktree add <run-dir>/wt -b fractality/<run-id>
//! <base>` in the packet's repo; the branch is the deliverable, the boss
//! reviews and merges, collection removes the worktree (failure keeps it
//! for autopsy; richer GC is DEF-7). `dir` — a scratch directory under
//! the run dir; file artifacts are the deliverable. `none` — the run dir
//! itself (pure-analysis tasks).

use camino::{Utf8Path, Utf8PathBuf};
use fractality_core::ids::RunId;
use fractality_core::packet::{WorkspaceMode, WorkspaceSpec};

specmark::scope!("spec://fractality/PROP-001#architecture");

/// The deliverable branch of a worktree-mode run (D8).
pub fn branch_name(run_id: RunId) -> String {
    format!("fractality/{run_id}")
}

/// Provisions the workspace for one run and returns the worker's cwd.
pub fn provision(
    spec: &WorkspaceSpec,
    run_id: RunId,
    run_dir: &Utf8Path,
) -> Result<Utf8PathBuf, String> {
    match spec.mode {
        WorkspaceMode::None => Ok(run_dir.to_owned()),
        WorkspaceMode::Dir => {
            let dir = run_dir.join("work");
            std::fs::create_dir_all(dir.as_std_path())
                .map_err(|e| format!("creating scratch dir `{dir}`: {e}"))?;
            Ok(dir)
        }
        WorkspaceMode::Worktree => {
            let wt = run_dir.join("wt");
            let branch = branch_name(run_id);
            let output = std::process::Command::new("git")
                // F19: a worktree rooted in a run dir inherits the
                // consumer repo's full depth on top of the runs-root
                // prefix — deep trees overflow Windows MAX_PATH without
                // long-path support (`Filename too long`, found live by
                // MT-05 against the host repo). Harmless elsewhere.
                .arg("-c")
                .arg("core.longpaths=true")
                .arg("-C")
                .arg(spec.repo.as_str())
                .arg("worktree")
                .arg("add")
                .arg(wt.as_std_path())
                .arg("-b")
                .arg(&branch)
                .arg(&spec.base)
                .output()
                .map_err(|e| format!("running git worktree add: {e}"))?;
            if !output.status.success() {
                return Err(format!(
                    "git worktree add failed for repo `{}` (base `{}`): {}",
                    spec.repo,
                    spec.base,
                    String::from_utf8_lossy(&output.stderr).trim()
                ));
            }
            Ok(wt)
        }
    }
}

/// Removes a worktree after successful collection (Phase 3 calls this;
/// failures keep the worktree for autopsy — DEF-7 owns richer policy).
pub fn remove_worktree(repo: &str, worktree_dir: &Utf8Path) -> Result<(), String> {
    let output = std::process::Command::new("git")
        .arg("-c")
        .arg("core.longpaths=true")
        .arg("-C")
        .arg(repo)
        .arg("worktree")
        .arg("remove")
        .arg("--force")
        .arg(worktree_dir.as_std_path())
        .output()
        .map_err(|e| format!("running git worktree remove: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "git worktree remove failed for `{worktree_dir}`: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(())
}
