//! `localdir` publish: copy each version into
//! `<root>/<group>/<name>/v<version>/` through a temporary sibling
//! directory and a rename inside the target, then rewrite the registry
//! index (`_index/`, listing last). The rename is atomic on one filesystem
//! and nothing more: no lock, no lease, no synchronisation with other
//! writers of a shared directory (PROP-002 `#registry-type` — conflicts are
//! settled administratively). A reader that finds the index out of step
//! with the directories computes it in memory instead (`#registry-index`).
use std::path::Path;

use chrono::Utc;
use vibe_core::cancellation::CancellationToken;
use vibe_index::layout::LayoutIdentity;

use super::{
    BatchConfig, BatchOutcome, LayoutAction, Staged, checkpoint, decide_all, io, outcomes,
    plan_index, write_index,
};
use crate::PublishError;

pub(super) fn publish(
    config: &BatchConfig,
    root: &Path,
    staged: &[Staged],
    cancellation: &CancellationToken,
) -> Result<BatchOutcome, PublishError> {
    if !root.is_dir() {
        return Err(PublishError::Io {
            path: root.to_path_buf(),
            message: "the localdir registry root is not a directory (create it, or fix the \
                      registry url)"
                .into(),
        });
    }
    let identity = LayoutIdentity {
        registry: config.registry.name.clone(),
        registry_url: config.registry.url.clone(),
        branch: None,
    };
    let actions = decide_all(root, staged, |p| root.join(p).display().to_string())?;
    let at = Utc::now();
    let plan = plan_index(root, config, &identity, staged, &actions, at)?;
    let unchanged = actions.iter().all(|a| *a == LayoutAction::Unchanged) && !plan.refresh;
    if config.dry_run || unchanged {
        return Ok(BatchOutcome {
            items: outcomes(staged, &actions),
            facts_recorded: if config.dry_run { plan.facts.len() } else { 0 },
            index_refreshed: config.dry_run && plan.refresh,
            commit: None,
            retried: false,
            dry_run: config.dry_run,
        });
    }
    for (stage, action) in staged.iter().zip(&actions) {
        if *action == LayoutAction::Unchanged {
            continue;
        }
        checkpoint(cancellation)?;
        place_version(root, stage, *action, cancellation)?;
    }
    let facts_recorded = plan.facts.len();
    write_index(root, &identity, plan, at)?;
    Ok(BatchOutcome {
        items: outcomes(staged, &actions),
        facts_recorded,
        index_refreshed: true,
        commit: None,
        retried: false,
        dry_run: false,
    })
}

fn place_version(
    root: &Path,
    stage: &Staged,
    action: LayoutAction,
    cancellation: &CancellationToken,
) -> Result<(), PublishError> {
    let parent = root.join(&stage.item.group).join(&stage.item.name);
    let target = parent.join(format!("v{}", stage.item.version));
    std::fs::create_dir_all(&parent).map_err(|e| io(&parent, "create_dir_all", e))?;
    let unique = format!(
        "{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|elapsed| elapsed.as_nanos())
            .unwrap_or_default()
    );
    let incoming = parent.join(format!(".v{}.incoming-{unique}", stage.item.version));
    let written = match &stage.item.git {
        // Every file, mode and symlink of the tree, verified before placing.
        Some(git) => super::git_tree::checkout(git, &incoming).and_then(|()| {
            let tree = super::git_tree::tree_of_dir(git, &incoming)?;
            super::git_tree::check(stage, git, &tree)
        }),
        None => crate::git_publish::copy_dir_controlled(&stage.payload, &incoming, cancellation),
    };
    let result =
        written.and_then(|()| place(&incoming, &target, action, &unique, &stage.item.version));
    if result.is_err() && incoming.exists() {
        // Leave no half-published sibling behind; the target is untouched.
        let _ = std::fs::remove_dir_all(&incoming);
    }
    result
}

/// Move `incoming` into place. A replaced version is first renamed aside,
/// so the target name never points at a partially written tree.
fn place(
    incoming: &Path,
    target: &Path,
    action: LayoutAction,
    unique: &str,
    version: &semver::Version,
) -> Result<(), PublishError> {
    if action != LayoutAction::Replaced {
        return std::fs::rename(incoming, target).map_err(|e| io(target, "rename", e));
    }
    let parent = target.parent().unwrap_or(target);
    let retired = parent.join(format!(".v{version}.replaced-{unique}"));
    std::fs::rename(target, &retired).map_err(|e| io(target, "rename aside", e))?;
    if let Err(error) = std::fs::rename(incoming, target) {
        // Put the previous version back before reporting.
        let _ = std::fs::rename(&retired, target);
        return Err(io(target, "rename", error));
    }
    std::fs::remove_dir_all(&retired).map_err(|e| io(&retired, "remove replaced version", e))
}
