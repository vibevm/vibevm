//! The provisional lifecycle world a slot continuation runs its rows over.
//!
//! Pure projection: the lockfile plus the exact materialised slots it names,
//! with any already-resolved replacement substituted in place. It reads the
//! slot manifests the install just wrote, which is the point of a post-install
//! epoch, and decides nothing about identity, streams or presentation.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#REF-AGENT-RESUME");

use anyhow::{Context, Result};
use vibe_core::manifest::{Lockfile, Manifest};
use vibe_workspace::Workspace;
use vibe_workspace::install::ResolvedDep;
use vibe_workspace::vibedeps;

/// Project the provisional world of one continuation.
///
/// ```no_run
/// use vibe_orchestrator::provisional_world;
/// # fn call(w: &vibe_workspace::Workspace, l: &vibe_core::manifest::Lockfile)
/// #   -> anyhow::Result<()> {
/// let world = provisional_world(w, l, &[])?;
/// assert!(world.len() <= l.packages.len());
/// # Ok(())
/// # }
/// ```
pub fn provisional_world(
    workspace: &Workspace,
    lockfile: &Lockfile,
    updated: &[ResolvedDep],
) -> Result<Vec<ResolvedDep>> {
    let mut world = Vec::with_capacity(lockfile.packages.len().max(updated.len()));
    for locked in &lockfile.packages {
        if let Some(replacement) = updated
            .iter()
            .find(|dep| dep.group == locked.group && dep.name == locked.name.as_str())
        {
            world.push(replacement.clone());
            continue;
        }
        let slot = if locked.materialization.is_in_place() {
            vibedeps::in_place_slot_abs_path(&workspace.root, &locked.group, &locked.name)
        } else {
            vibedeps::slot_abs_path(
                &workspace.root,
                &locked.group,
                &locked.name,
                &locked.version,
            )
        };
        let manifest = Manifest::read(slot.join(Manifest::FILENAME)).with_context(|| {
            format!(
                "reading unchanged provisional lifecycle provider `{}/{}@{}`",
                locked.group, locked.name, locked.version
            )
        })?;
        world.push(ResolvedDep {
            kind: locked.kind,
            group: locked.group.clone(),
            name: locked.name.to_string(),
            version: locked.version.clone(),
            content_dir: slot,
            source_hash: Some(locked.content_hash.clone()),
            manifest,
            requires: locked
                .dependencies
                .iter()
                .filter_map(|dependency| {
                    dependency
                        .group
                        .clone()
                        .map(|group| (group, dependency.name.to_string()))
                })
                .collect(),
            admitted_by: locked.admitted_by.clone(),
            via_override: locked.via_override.clone(),
            source_mutable: false,
            in_place_changed: None,
        });
    }
    for dep in updated {
        if !world
            .iter()
            .any(|row| row.group == dep.group && row.name == dep.name)
        {
            world.push(dep.clone());
        }
    }
    Ok(world)
}
