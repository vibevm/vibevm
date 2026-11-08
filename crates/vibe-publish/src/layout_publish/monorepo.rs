//! `monorepo` publish: fetch the registry branch, write the version
//! directories and the refreshed registry index, commit once, push with
//! plain `git` (no host API, no organisation). Git serialises concurrent
//! publishers: a non-fast-forward rejection is absorbed by one refetch —
//! which re-decides every version and re-projects the index from the
//! winner's tree — and retry, then refused.
use std::path::Path;

use chrono::Utc;
use vibe_core::cancellation::CancellationToken;
use vibe_index::layout::LayoutIdentity;

use super::{
    BatchConfig, BatchOutcome, IndexPlan, LayoutAction, Staged, checkpoint, commit_message,
    decide_all, io, outcomes, plan_index, write_index,
};
use crate::PublishError;
use crate::git_publish::{git_command, redact_credentials, run_git_in, run_remote_git_in};

const FETCHED: &str = "refs/remotes/vibe-publish/registry";

pub(super) fn publish(
    config: &BatchConfig,
    url: &str,
    branch: &str,
    staged: &[Staged],
    cancellation: &CancellationToken,
) -> Result<BatchOutcome, PublishError> {
    let work = tempfile::TempDir::new().map_err(|e| PublishError::Io {
        path: std::env::temp_dir(),
        message: format!("creating publish working tree: {e}"),
    })?;
    let tree = work.path();
    run_git_in(tree, &["init", "-q", &format!("--initial-branch={branch}")])?;
    ensure_identity(tree)?;
    let identity = LayoutIdentity {
        registry: config.registry.name.clone(),
        registry_url: config.registry.url.clone(),
        branch: Some(branch.to_string()),
    };
    let mut retried = false;
    loop {
        checkpoint(cancellation)?;
        fetch(tree, url, branch)?;
        let actions = decide_all(tree, staged, |path| {
            format!("{}#{path}", redact_credentials(url))
        })?;
        let at = Utc::now();
        let plan = plan_index(tree, config, &identity, staged, &actions, at)?;
        let unchanged = actions.iter().all(|a| *a == LayoutAction::Unchanged) && !plan.refresh;
        if config.dry_run || unchanged {
            return Ok(BatchOutcome {
                items: outcomes(staged, &actions),
                facts_recorded: if config.dry_run { plan.facts.len() } else { 0 },
                index_refreshed: config.dry_run && plan.refresh,
                commit: None,
                retried,
                dry_run: config.dry_run,
            });
        }
        let facts_recorded = plan.facts.len();
        let batch = Batch {
            config,
            identity: &identity,
            staged,
        };
        let commit = commit_batch(tree, &batch, &actions, plan, at, cancellation)?;
        checkpoint(cancellation)?;
        match push(tree, url, branch)? {
            Pushed::Accepted => {
                return Ok(BatchOutcome {
                    items: outcomes(staged, &actions),
                    facts_recorded,
                    index_refreshed: true,
                    commit: Some(commit),
                    retried,
                    dry_run: false,
                });
            }
            Pushed::Behind if !retried => {
                // Someone published first: start again from their commit.
                retried = true;
                continue;
            }
            Pushed::Behind => {
                return Err(PublishError::MonorepoRaceLost {
                    repo: redact_credentials(url),
                    branch: branch.to_string(),
                });
            }
        }
    }
}

/// One batch as a Git working tree sees it.
pub(super) struct Batch<'a> {
    pub config: &'a BatchConfig,
    pub identity: &'a LayoutIdentity,
    pub staged: &'a [Staged],
}

/// Write the changed version directories and the refreshed index into the
/// Git working tree `tree`, stage exactly those paths and commit once.
/// Returns the commit.
pub(super) fn commit_batch(
    tree: &Path,
    batch: &Batch<'_>,
    actions: &[LayoutAction],
    plan: IndexPlan,
    at: chrono::DateTime<Utc>,
    cancellation: &CancellationToken,
) -> Result<String, PublishError> {
    let mut changed = Vec::new();
    for (index, (stage, action)) in batch.staged.iter().zip(actions).enumerate() {
        if *action == LayoutAction::Unchanged {
            continue;
        }
        checkpoint(cancellation)?;
        if let Some(git) = &stage.item.git {
            // By tree object: staged in the index, verified, never re-added.
            super::git_tree::insert(tree, stage, git, index)?;
            continue;
        }
        let target = tree.join(stage.path());
        if target.exists() {
            std::fs::remove_dir_all(&target).map_err(|e| io(&target, "remove_dir_all", e))?;
        }
        crate::git_publish::copy_dir_controlled(&stage.payload, &target, cancellation)?;
        changed.push(stage.path());
    }
    write_index(tree, batch.identity, plan, at)?;
    // `-f`: a package's own `.gitignore` (or the repository's) must not
    // drop any of its files — the bytes committed are the bytes hashed.
    for chunk in changed.chunks(64) {
        let mut args = vec!["add", "-f", "-A", "--"];
        args.extend(chunk.iter().map(String::as_str));
        run_git_in(tree, &args)?;
    }
    run_git_in(
        tree,
        &["add", "-A", "--", vibe_core::layout_registry::INDEX_DIR],
    )?;
    let message = commit_message(batch.config, batch.staged, actions);
    run_git_in(tree, &["commit", "-q", "--allow-empty", "-m", &message])?;
    Ok(
        String::from_utf8_lossy(&run_git_in(tree, &["rev-parse", "HEAD"])?.stdout)
            .trim()
            .to_string(),
    )
}

/// Fetch `branch` and check it out. A branch that does not exist yet (an
/// empty repository) leaves the unborn local branch to commit onto.
pub(super) fn fetch(tree: &Path, url: &str, branch: &str) -> Result<(), PublishError> {
    let heads = run_remote_git_in(
        tree,
        &[
            "ls-remote",
            "--heads",
            "--",
            url,
            &format!("refs/heads/{branch}"),
        ],
        url,
    )?;
    if heads.stdout.iter().all(u8::is_ascii_whitespace) {
        return Ok(());
    }
    run_remote_git_in(
        tree,
        &[
            "fetch",
            "-q",
            "--depth=1",
            "--no-tags",
            "--no-write-fetch-head",
            "--",
            url,
            &format!("+refs/heads/{branch}:{FETCHED}"),
        ],
        url,
    )?;
    run_git_in(tree, &["checkout", "-q", "-f", "-B", branch, FETCHED])?;
    // A retry starts from the winner's tree: nothing of the lost attempt
    // (written versions, the re-projected index) may survive.
    run_git_in(tree, &["clean", "-q", "-f", "-d", "-x"])?;
    Ok(())
}

enum Pushed {
    Accepted,
    /// Rejected as non-fast-forward: the branch moved since the fetch.
    Behind,
}

fn push(tree: &Path, url: &str, branch: &str) -> Result<Pushed, PublishError> {
    let refspec = format!("HEAD:refs/heads/{branch}");
    let args = ["push", "-q", "--", url, refspec.as_str()];
    let output = git_command(tree, &args)
        .output()
        .map_err(|e| PublishError::Git(format!("spawning git push: {e}")))?;
    if output.status.success() {
        return Ok(Pushed::Accepted);
    }
    let stderr = String::from_utf8_lossy(&output.stderr).to_lowercase();
    if stderr.contains("non-fast-forward") || stderr.contains("fetch first") {
        return Ok(Pushed::Behind);
    }
    let safe_repo = redact_credentials(url);
    if stderr.contains("permission denied")
        || stderr.contains("publickey")
        || stderr.contains("authentication failed")
    {
        return Err(PublishError::PushDenied { repo: safe_repo });
    }
    Err(PublishError::Git(format!(
        "git push {safe_repo} {refspec} failed: {}",
        redact_credentials(String::from_utf8_lossy(&output.stderr).trim())
    )))
}

/// Commit as the user's configured Git identity; without one, as the
/// publisher identity every other vibe publish path uses.
pub(super) fn ensure_identity(tree: &Path) -> Result<(), PublishError> {
    let configured = git_command(tree, &["config", "--get", "user.email"])
        .output()
        .map_err(|e| PublishError::Git(format!("spawning git config: {e}")))?;
    if !configured.status.success() {
        run_git_in(tree, &["config", "user.email", "publish@vibevm.local"])?;
        run_git_in(tree, &["config", "user.name", "vibevm publisher"])?;
    }
    Ok(())
}
