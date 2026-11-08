//! `localdir` publish: copy into `<root>/<group>/<name>/v<version>/` through
//! a temporary sibling directory and a rename inside the target. The rename
//! is atomic on one filesystem and nothing more: no lock, no lease, no
//! synchronisation with other writers of a shared directory (PROP-002
//! `#registry-type` — conflicts are settled administratively).

use std::path::Path;

use vibe_core::cancellation::CancellationToken;

use super::{LayoutAction, LayoutPublishConfig, LayoutPublishOutcome, Staged, checkpoint, io};
use crate::PublishError;

pub(super) fn publish(
    config: &LayoutPublishConfig,
    root: &Path,
    staged: &Staged,
    cancellation: &CancellationToken,
) -> Result<LayoutPublishOutcome, PublishError> {
    if !root.is_dir() {
        return Err(PublishError::Io {
            path: root.to_path_buf(),
            message: "the localdir registry root is not a directory (create it, or fix the \
                      registry url)"
                .into(),
        });
    }
    let package = &staged.package;
    let parent = root.join(&package.group).join(&package.name);
    let target = parent.join(format!("v{}", package.version));
    let action = staged.decide(&target, config.replace, &target.display().to_string())?;
    if config.dry_run || action == LayoutAction::Unchanged {
        return Ok(staged.outcome(action, None, false, config.dry_run));
    }
    checkpoint(cancellation)?;
    std::fs::create_dir_all(&parent).map_err(|e| io(&parent, "create_dir_all", e))?;
    let unique = format!(
        "{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|elapsed| elapsed.as_nanos())
            .unwrap_or_default()
    );
    let incoming = parent.join(format!(".v{}.incoming-{unique}", package.version));
    crate::git_publish::copy_dir_controlled(&staged.payload, &incoming, cancellation)?;
    let result = place(&incoming, &target, action, &unique, &package.version);
    if result.is_err() && incoming.exists() {
        // Leave no half-published sibling behind; the target is untouched.
        let _ = std::fs::remove_dir_all(&incoming);
    }
    result?;
    Ok(staged.outcome(action, None, false, false))
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
