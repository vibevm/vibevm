//! Publishing into a Git working copy of a monorepo registry: the batch
//! is written onto the checked-out branch and committed there — never
//! pushed (PROP-002 `#registry-mirror`). The operator reviews and pushes.
//! A working copy with uncommitted changes is refused before anything is
//! written: the commit must hold exactly this batch.
use std::path::Path;

use chrono::Utc;
use vibe_core::cancellation::CancellationToken;
use vibe_index::layout::LayoutIdentity;

use super::monorepo::{Batch, commit_batch, ensure_identity};
use super::{
    BatchConfig, BatchOutcome, LayoutAction, Staged, checkpoint, decide_all, outcomes, plan_index,
};
use crate::PublishError;
use crate::git_publish::run_git_in;

/// The top of the working copy containing `path`.
pub(super) fn toplevel(path: &Path) -> Result<std::path::PathBuf, PublishError> {
    let top = run_git_in(path, &["rev-parse", "--show-toplevel"])?;
    Ok(std::path::PathBuf::from(
        String::from_utf8_lossy(&top.stdout).trim(),
    ))
}

/// [`toplevel`], refusing a working copy with uncommitted changes.
pub(super) fn clean_toplevel(path: &Path) -> Result<std::path::PathBuf, PublishError> {
    let tree = toplevel(path)?;
    let status = run_git_in(&tree, &["status", "--porcelain", "--untracked-files=all"])?;
    if !status.stdout.iter().all(u8::is_ascii_whitespace) {
        return Err(PublishError::DirtyWorkingCopy { path: tree });
    }
    Ok(tree)
}

pub(super) fn publish(
    config: &BatchConfig,
    path: &Path,
    staged: &[Staged],
    cancellation: &CancellationToken,
) -> Result<BatchOutcome, PublishError> {
    let tree = clean_toplevel(path)?;
    let branch = run_git_in(&tree, &["symbolic-ref", "--quiet", "--short", "HEAD"])
        .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_string())
        .unwrap_or_else(|_| "HEAD".to_string());
    let identity = LayoutIdentity {
        registry: config.registry.name.clone(),
        registry_url: config.registry.url.clone(),
        branch: Some(branch),
    };
    checkpoint(cancellation)?;
    let actions = decide_all(&tree, staged, |p| format!("{}#{p}", tree.display()))?;
    let at = Utc::now();
    let plan = plan_index(&tree, config, &identity, staged, &actions, at)?;
    let unchanged = actions.iter().all(|a| *a == LayoutAction::Unchanged) && !plan.refresh;
    if config.dry_run || unchanged {
        return Ok(BatchOutcome {
            items: outcomes(staged, &actions),
            facts_recorded: if config.dry_run { plan.facts.len() } else { 0 },
            removed: plan.removed.len(),
            index_refreshed: config.dry_run && plan.refresh,
            commit: None,
            retried: false,
            dry_run: config.dry_run,
        });
    }
    ensure_identity(&tree)?;
    let facts_recorded = plan.facts.len();
    let removed = plan.removed.len();
    let batch = Batch {
        config,
        identity: &identity,
        staged,
    };
    let commit = commit_batch(&tree, &batch, &actions, plan, at, cancellation)?;
    Ok(BatchOutcome {
        items: outcomes(staged, &actions),
        facts_recorded,
        removed,
        index_refreshed: true,
        commit: Some(commit),
        retried: false,
        dry_run: false,
    })
}
