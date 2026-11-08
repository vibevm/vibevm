//! `monorepo` publish: fetch the registry branch, write the version
//! directory, commit, push with plain `git` (no host API, no organisation).
//! Git serialises concurrent publishers: a non-fast-forward rejection is
//! absorbed by one refetch and retry, then refused.

use std::path::Path;

use vibe_core::cancellation::CancellationToken;

use super::{LayoutAction, LayoutPublishConfig, LayoutPublishOutcome, Staged, checkpoint, io};
use crate::PublishError;
use crate::git_publish::{git_command, redact_credentials, run_git_in, run_remote_git_in};

const FETCHED: &str = "refs/remotes/vibe-publish/registry";

pub(super) fn publish(
    config: &LayoutPublishConfig,
    url: &str,
    branch: &str,
    staged: &Staged,
    cancellation: &CancellationToken,
) -> Result<LayoutPublishOutcome, PublishError> {
    let work = tempfile::TempDir::new().map_err(|e| PublishError::Io {
        path: std::env::temp_dir(),
        message: format!("creating publish working tree: {e}"),
    })?;
    let tree = work.path();
    run_git_in(tree, &["init", "-q", &format!("--initial-branch={branch}")])?;
    ensure_identity(tree)?;

    let mut retried = false;
    loop {
        checkpoint(cancellation)?;
        fetch(tree, url, branch)?;
        let target = tree.join(staged.package.path());
        let location = format!("{}#{}", redact_credentials(url), staged.package.path());
        let action = staged.decide(&target, config.replace, &location)?;
        if config.dry_run || action == LayoutAction::Unchanged {
            return Ok(staged.outcome(action, None, retried, config.dry_run));
        }
        if target.exists() {
            std::fs::remove_dir_all(&target).map_err(|e| io(&target, "remove_dir_all", e))?;
        }
        crate::git_publish::copy_dir_controlled(&staged.payload, &target, cancellation)?;
        let package = &staged.package;
        run_git_in(tree, &["add", "-A", "--", &package.path()])?;
        let verb = if action == LayoutAction::Replaced {
            "replace"
        } else {
            "publish"
        };
        run_git_in(
            tree,
            &[
                "commit",
                "-q",
                "-m",
                &format!(
                    "{verb}: {}/{}@{}",
                    package.group, package.name, package.version
                ),
            ],
        )?;
        let commit = String::from_utf8_lossy(&run_git_in(tree, &["rev-parse", "HEAD"])?.stdout)
            .trim()
            .to_string();
        checkpoint(cancellation)?;
        match push(tree, url, branch)? {
            Pushed::Accepted => {
                return Ok(staged.outcome(action, Some(commit), retried, false));
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

/// Fetch `branch` and check it out. A branch that does not exist yet (an
/// empty repository) leaves the unborn local branch to commit onto.
fn fetch(tree: &Path, url: &str, branch: &str) -> Result<(), PublishError> {
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
    run_git_in(tree, &["checkout", "-q", "-B", branch, FETCHED])?;
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
fn ensure_identity(tree: &Path) -> Result<(), PublishError> {
    let configured = git_command(tree, &["config", "--get", "user.email"])
        .output()
        .map_err(|e| PublishError::Git(format!("spawning git config: {e}")))?;
    if !configured.status.success() {
        run_git_in(tree, &["config", "user.email", "publish@vibevm.local"])?;
        run_git_in(tree, &["config", "user.name", "vibevm publisher"])?;
    }
    Ok(())
}
