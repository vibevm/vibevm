//! Stale-slot pruning: the dependency tree holds exactly the current
//! resolution and no empty husks.

use std::collections::HashSet;
use std::fs;
use std::path::Path;

use crate::{WorkspaceError, layout_paths};

use super::io_err;

/// Remove every dependency slot whose path is not in `kept`, returning
/// the removed slot paths (sorted). A `<kind>-<name>` directory left with
/// no surviving version is removed too, so the dependency tree holds exactly the
/// current resolution and no empty husks.
pub(super) fn prune_stale_slots(
    workspace_root: &Path,
    kept: &[String],
) -> Result<Vec<String>, WorkspaceError> {
    let vibedeps_dir = workspace_root.join(vibe_core::layout::current_vibedeps_root());
    if !vibedeps_dir.is_dir() {
        return Ok(Vec::new());
    }
    let keep: HashSet<&str> = kept.iter().map(String::as_str).collect();
    // The complete caller keep set authorizes desired state, not ownership of
    // arbitrary directories. Only the previous lock's slots can be pruned;
    // unknown/orphan state is outside this transaction's before-images.
    let lock_path = workspace_root.join("vibe.lock");
    let lock = vibe_core::manifest::Lockfile::read(&lock_path);
    let owned: HashSet<String> = match lock {
        Ok(lock) => lock
            .packages
            .iter()
            .filter(|p| !p.materialization.is_in_place())
            .map(|p| crate::vibedeps::slot_rel_path(&p.group, &p.name, &p.version))
            .collect(),
        Err(_) if !lock_path.exists() => HashSet::new(),
        Err(error) => {
            return Err(WorkspaceError::Io {
                path: lock_path,
                reason: error.to_string(),
            });
        }
    };
    let mut pruned = Vec::new();
    for kind_name in fs::read_dir(&vibedeps_dir).map_err(|e| io_err(&vibedeps_dir, e))? {
        let kind_name = kind_name.map_err(|e| io_err(&vibedeps_dir, e))?;
        let kind_name_dir = kind_name.path();
        if !kind_name_dir.is_dir() {
            continue;
        }
        // An in-place slot is the `<kind>-<name>` dir itself — a git working
        // tree (PROP-022 §2.4), not a container of versioned slots. Skip it:
        // its lifecycle is the move-into-slot / destructive-guard path, never
        // version pruning.
        if kind_name_dir.join(".git").exists() {
            continue;
        }
        let kn = kind_name.file_name().to_string_lossy().into_owned();
        let mut any_kept = false;
        for version in fs::read_dir(&kind_name_dir).map_err(|e| io_err(&kind_name_dir, e))? {
            let version = version.map_err(|e| io_err(&kind_name_dir, e))?;
            let version_dir = version.path();
            if !version_dir.is_dir() {
                continue;
            }
            let ver = version.file_name().to_string_lossy().into_owned();
            let rel = layout_paths::vibedeps(format!("{kn}/{ver}"));
            if keep.contains(rel.as_str()) {
                any_kept = true;
            } else if owned.contains(&rel)
                && crate::vibedeps::validate_owned_slot_removal(&version_dir).is_ok()
            {
                fs::remove_dir_all(&version_dir).map_err(|e| io_err(&version_dir, e))?;
                pruned.push(rel);
            } else {
                any_kept = true;
            }
        }
        if !any_kept {
            let _ = fs::remove_dir(&kind_name_dir);
        }
    }
    pruned.sort();
    Ok(pruned)
}
