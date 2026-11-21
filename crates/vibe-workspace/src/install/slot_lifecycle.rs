//! Neutral dependency-slot lifecycle seam shared by install and lifecycle.

use std::fmt;
use std::path::Path;

use specmark::spec;
use vibe_core::manifest::Manifest;
use vibe_core::{Group, PackageKind};

use super::{PostInstallPlan, ResolvedDep, is_in_place};
use crate::{WorkspaceError, vibedeps};

/// Exact provider/slot context at the pre- and post-install timing points.
#[derive(Debug, Clone, Copy)]
#[spec(
    documents = "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-020#MODERN-SLOT-EXECUTION"
)]
pub struct SlotLifecycleContext<'a> {
    pub group: &'a Group,
    pub name: &'a str,
    pub version: &'a semver::Version,
    pub kind: &'a PackageKind,
    pub slot: &'a Path,
    pub manifest: &'a Manifest,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM")]
pub struct SlotLifecycleTarget {
    pub group: Group,
    pub name: String,
    pub version: semver::Version,
}

/// Install-neutral callbacks for the two dependency-slot timing points.
///
/// A pre-install error is fatal and the materialisation layer rolls the slot
/// back. Post-install is invoked only from the one-shot deferred plan, after
/// the caller has made install state durable.
///
/// ```
/// use vibe_workspace::install::{SlotLifecycle, SlotLifecycleContext};
/// struct Recorder;
/// impl SlotLifecycle for Recorder {
///     fn pre_install(&self, _: SlotLifecycleContext<'_>) -> Result<(), String> { Ok(()) }
///     fn post_install(&self, _: SlotLifecycleContext<'_>) -> Result<(), String> { Ok(()) }
/// }
/// ```
#[spec(
    implements = "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-020#MODERN-SLOT-EXECUTION"
)]
pub trait SlotLifecycle {
    fn targets_ready(&self, _targets: &[SlotLifecycleTarget]) -> Result<(), String> {
        Ok(())
    }

    /// The materialisation boundary, reported BEFORE any deferred pre-install
    /// callback runs. A callback that then stops the install — a hosted park,
    /// or a failure — can still name exactly what this pass changed, instead
    /// of the caller inferring "nothing happened" from an error. Slot paths,
    /// never file counts: a slot is a directory, and this layer never measured
    /// its contents.
    fn materialised(&self, _materialised: &[String], _skipped: &[String]) {}

    /// A slot this pass materialised has just been removed again by rollback.
    /// The observer drops it, so a park never claims a change the tree no
    /// longer has.
    fn rolled_back(&self, _slot: &str) {}

    fn pre_install(&self, context: SlotLifecycleContext<'_>) -> Result<(), String>;
    fn post_install(&self, context: SlotLifecycleContext<'_>) -> Result<(), String>;
}

/// The slot-lifecycle execution mode for one install pass: no slot events at
/// all, or exactly one callback that owns both timing points.
#[derive(Clone, Copy)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#H-SCRIPT")]
pub enum SlotLifecycleMode<'a> {
    None,
    Callback(&'a dyn SlotLifecycle),
}

impl fmt::Debug for SlotLifecycleMode<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::None => formatter.write_str("None"),
            Self::Callback(_) => formatter.write_str("Callback(..)"),
        }
    }
}

impl<'a> SlotLifecycleMode<'a> {
    /// The callback observer, which must hear about a slot an undo removes.
    pub(super) fn callback(&self) -> Option<&'a dyn SlotLifecycle> {
        match self {
            Self::Callback(callback) => Some(*callback),
            Self::None => None,
        }
    }
}

/// The slot a lifecycle callback runs against — the unversioned in-place
/// working tree, or the versioned copy slot. A versioned slot is detached
/// from the shared store first: a callback may rewrite any recorded file,
/// and a retained hardlink would carry that write into another slot.
fn callback_slot(
    dep: &ResolvedDep,
    workspace_root: &Path,
) -> Result<std::path::PathBuf, WorkspaceError> {
    if is_in_place(dep) {
        return Ok(vibedeps::in_place_slot_abs_path(
            workspace_root,
            &dep.group,
            &dep.name,
        ));
    }
    let slot = vibedeps::slot_abs_path(workspace_root, &dep.group, &dep.name, &dep.version);
    vibedeps::detach_recorded_hardlinks(&slot)?;
    Ok(slot)
}

fn run_pre_slot_lifecycle(
    callback: &dyn SlotLifecycle,
    dep: &ResolvedDep,
    workspace_root: &Path,
) -> Result<(), WorkspaceError> {
    let slot = callback_slot(dep, workspace_root)?;
    callback
        .pre_install(context_with_slot(dep, &slot))
        .map_err(|reason| WorkspaceError::SlotLifecycle {
            phase: "pre-install",
            package: format!("{}/{}", dep.group, dep.name),
            reason,
        })
}

/// Consume one install-produced post plan under the pass's slot lifecycle
/// mode, after the caller has made the install durable. The opaque plan owns
/// the exact eligible dependency subset and its workspace; this seam accepts
/// neither separately.
pub fn run_post_install_slot_lifecycle(
    plan: PostInstallPlan,
    lifecycle: SlotLifecycleMode<'_>,
) -> Result<(), WorkspaceError> {
    let SlotLifecycleMode::Callback(callback) = lifecycle else {
        return Ok(());
    };
    let (workspace_root, eligible_deps) = plan.into_parts();
    for dep in &eligible_deps {
        let slot = callback_slot(dep, &workspace_root)?;
        callback
            .post_install(context_with_slot(dep, &slot))
            .map_err(|reason| WorkspaceError::SlotLifecycle {
                phase: "post-install",
                package: format!("{}/{}", dep.group, dep.name),
                reason,
            })?;
    }
    Ok(())
}

/// Ordered rollback-capable pre plan: callback events wait until every
/// provider slot of the pass is in place.
pub(super) struct PreInstallPlan<'a, 'l> {
    lifecycle: &'a SlotLifecycleMode<'l>,
    workspace_root: &'a Path,
    deferred: Vec<ResolvedDep>,
    checked: Vec<std::path::PathBuf>,
}

impl<'a, 'l> PreInstallPlan<'a, 'l> {
    pub(super) fn new_checked(
        lifecycle: &'a SlotLifecycleMode<'l>,
        workspace_root: &'a Path,
        checked: Vec<std::path::PathBuf>,
    ) -> Self {
        Self {
            lifecycle,
            workspace_root,
            deferred: Vec::new(),
            checked,
        }
    }

    /// Queue the pre-install event of a slot whose payload this pass changed.
    pub(super) fn defer(&mut self, dep: &ResolvedDep) {
        if self.lifecycle.callback().is_some() {
            self.deferred.push(dep.clone());
        }
    }

    pub(super) fn dispatch(
        self,
        materialised: &[String],
        skipped: &[String],
    ) -> Result<(), WorkspaceError> {
        let Some(callback) = self.lifecycle.callback() else {
            return Ok(());
        };
        // The observer learns what really changed BEFORE any deferred row
        // can stop the pass.
        callback.materialised(materialised, skipped);
        let targets = self
            .deferred
            .iter()
            .map(|dep| SlotLifecycleTarget {
                group: dep.group.clone(),
                name: dep.name.clone(),
                version: dep.version.clone(),
            })
            .collect::<Vec<_>>();
        callback
            .targets_ready(&targets)
            .map_err(|reason| WorkspaceError::SlotLifecycle {
                phase: "pre-install-plan",
                package: "<exact-payload-event-set>".into(),
                reason,
            })?;
        for dep in &self.deferred {
            if let Err(error) = run_pre_slot_lifecycle(callback, dep, self.workspace_root) {
                self.rollback(dep);
                return Err(error);
            }
        }
        Ok(())
    }

    fn rollback(&self, dep: &ResolvedDep) {
        if self.checked.contains(&vibedeps::in_place_slot_abs_path(
            self.workspace_root,
            &dep.group,
            &dep.name,
        )) {
            return; // The enclosing checked owner preserves unknown replacements.
        }
        let slot = if is_in_place(dep) {
            let _ = vibedeps::remove_in_place_slot(self.workspace_root, &dep.group, &dep.name);
            vibedeps::in_place_slot_rel_path(&dep.group, &dep.name)
        } else {
            let _ = vibedeps::remove_slot(self.workspace_root, &dep.group, &dep.name, &dep.version);
            vibedeps::slot_rel_path(&dep.group, &dep.name, &dep.version)
        };
        prune_empty_slot_parents(self.workspace_root, &slot);
        // The slot is gone again: an observer that already recorded it as
        // materialised must not keep reporting a change the tree no longer has.
        if let Some(callback) = self.lifecycle.callback() {
            callback.rolled_back(&slot);
        }
    }
}

/// After a rolled-back slot is gone, the directories that only held it go
/// too: `vibedeps/<group>.<name>/`, then the dependency root itself, each
/// only while empty, so a slot another package or writer left keeps them.
fn prune_empty_slot_parents(workspace_root: &std::path::Path, slot_rel: &str) {
    let deps = workspace_root.join(vibe_core::layout::current_vibedeps_root());
    let slot = workspace_root.join(slot_rel);
    for dir in slot.ancestors().skip(1) {
        if !dir.starts_with(&deps) || std::fs::remove_dir(dir).is_err() {
            return;
        }
    }
}

fn context_with_slot<'a>(dep: &'a ResolvedDep, slot: &'a Path) -> SlotLifecycleContext<'a> {
    SlotLifecycleContext {
        group: &dep.group,
        name: &dep.name,
        version: &dep.version,
        kind: &dep.kind,
        slot,
        manifest: &dep.manifest,
    }
}
