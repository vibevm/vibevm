//! The prune of a scoped `vibe update`: superseded versioned slots moved
//! aside under the run's rollback, each one recorded as it happens.
//!
//! Split from `scoped.rs` for the file-length budget. It is the first step
//! of region 3 there, called by the apply once a lifecycle exists.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#command-summary");

use anyhow::{Context, Result};
use vibe_core::manifest::Lockfile;
use vibe_workspace::Workspace;
use vibe_workspace::install::InstallRollback;
use vibe_workspace::vibedeps;

use super::Resolved;
use super::measured::Measured;

/// Remove any superseded *versioned* slot so a bump leaves no stale slot (an
/// in-place slot is unversioned — nothing to prune), and record the bumps.
///
/// Both facts are written into `measured` AS THEY HAPPEN, and that is the whole
/// contract: a removal that fails at package N must leave the N−1 slots this
/// run really deleted in the accumulator, and therefore in the failed report.
/// An all-or-nothing helper returning `Result<Vec<String>>` would drop its
/// partial vector on the way out and report a tree it had already changed as
/// untouched.
///
/// A bump is recorded before its removal is attempted: it is a fact about the
/// resolution, and stays true whether or not the stale slot could be deleted.
/// `pruned` conversely names only removals that REALLY removed something — a
/// slot already absent, or an unversioned in-place one, deleted nothing.
///
/// Each removal moves the slot aside into `rollback` (same volume, a rename)
/// rather than deleting it, so a later failure or acknowledged stop before the
/// lock is published can put the superseded slot back.
pub(super) fn prune_superseded(
    workspace: &Workspace,
    lockfile: &Lockfile,
    updated: &[Resolved],
    measured: &mut Measured,
    rollback: &mut InstallRollback,
) -> Result<()> {
    for (cached, _, _, _, _) in updated {
        let name = &cached.resolved.name;
        let Some(old_v) = lockfile
            .find(&cached.resolved.group, name)
            .map(|o| o.version.clone())
            .filter(|v| *v != cached.resolved.version)
        else {
            continue;
        };
        measured.record_bump(format!(
            "{}/{} {} -> {}",
            cached.resolved.group, name, old_v, cached.resolved.version
        ));
        if !cached.package_meta().materialization.is_in_place() {
            let group = &cached.package_meta().group;
            let slot = vibedeps::slot_abs_path(&workspace.root, group, name, &old_v);
            if slot.exists() {
                // A slot is a directory. Moving aside whatever else sits at
                // its path would succeed where the delete it replaces refused.
                if !slot.is_dir() {
                    return Err(vibe_workspace::WorkspaceError::Io {
                        path: slot,
                        reason: "the superseded slot path is not a directory".into(),
                    })
                    .context("removing the superseded vibedeps/ slot");
                }
                rollback
                    .remove(&slot)
                    .context("removing the superseded vibedeps/ slot")?;
                measured.record_pruned(vibedeps::slot_rel_path(group, name, &old_v));
            }
        }
    }
    measured.sort_pruned();
    Ok(())
}
