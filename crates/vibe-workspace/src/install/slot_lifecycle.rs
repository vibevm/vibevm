//! Neutral dependency-slot lifecycle seam shared by install and lifecycle.

use std::fmt;
use std::path::Path;

use specmark::spec;
use vibe_core::manifest::Manifest;
use vibe_core::{Group, PackageKind};

use super::hooks_run::run_pre_install_hook;
use super::{ResolvedDep, is_in_place};
use crate::hooks::{HookOutput, HookPolicy, HookReport, HookRunner, InterpreterProbe};
use crate::{WorkspaceError, vibedeps};

/// Exact provider/slot context at the pre- and post-install timing points.
#[derive(Debug, Clone, Copy)]
#[spec(documents = "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-020#phases")]
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
#[spec(implements = "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-020#phases")]
pub trait SlotLifecycle {
    fn targets_ready(&self, _targets: &[SlotLifecycleTarget]) -> Result<(), String> {
        Ok(())
    }

    fn pre_install(&self, context: SlotLifecycleContext<'_>) -> Result<(), String>;
    fn post_install(&self, context: SlotLifecycleContext<'_>) -> Result<(), String>;
}

/// The exclusive slot-lifecycle execution mode for one install pass.
///
/// This sum type deliberately has no representation for legacy hooks and a
/// lifecycle callback together. Existing callers use `LegacyHooks`; future
/// lifecycle orchestration uses `Callback` without depending on hook policy.
#[derive(Clone, Copy)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#H-SCRIPT")]
pub enum SlotLifecycleMode<'a> {
    None,
    LegacyHooks {
        policy: &'a HookPolicy,
        output: HookOutput,
    },
    Callback(&'a dyn SlotLifecycle),
}

impl fmt::Debug for SlotLifecycleMode<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::None => formatter.write_str("None"),
            Self::LegacyHooks { output, .. } => formatter
                .debug_struct("LegacyHooks")
                .field("output", output)
                .finish_non_exhaustive(),
            Self::Callback(_) => formatter.write_str("Callback(..)"),
        }
    }
}

pub(super) enum MaterialiseLifecycle<'a> {
    None,
    LegacyHooks {
        policy: &'a HookPolicy,
        probe: &'a dyn InterpreterProbe,
        runner: &'a dyn HookRunner,
    },
    Callback(&'a dyn SlotLifecycle),
}

pub(super) fn run_pre_slot_lifecycle(
    lifecycle: &MaterialiseLifecycle<'_>,
    dep: &ResolvedDep,
    workspace_root: &Path,
) -> Result<Option<HookReport>, WorkspaceError> {
    match lifecycle {
        MaterialiseLifecycle::None => Ok(None),
        MaterialiseLifecycle::LegacyHooks {
            policy,
            probe,
            runner,
        } => run_pre_install_hook(dep, workspace_root, policy, *probe, *runner),
        MaterialiseLifecycle::Callback(callback) => {
            let slot = if is_in_place(dep) {
                vibedeps::in_place_slot_abs_path(workspace_root, &dep.group, &dep.name)
            } else {
                vibedeps::slot_abs_path(workspace_root, &dep.group, &dep.name, &dep.version)
            };
            if !is_in_place(dep) {
                vibedeps::detach_recorded_hardlinks(&slot)?;
            }
            callback
                .pre_install(context_with_slot(dep, &slot))
                .map_err(|reason| WorkspaceError::SlotLifecycle {
                    phase: "pre-install",
                    package: format!("{}/{}", dep.group, dep.name),
                    reason,
                })?;
            Ok(None)
        }
    }
}

/// Ordered rollback-capable pre plan. Callback events wait for every provider
/// slot; legacy hooks retain their inline timing.
pub(super) struct PreInstallPlan<'a, 'l> {
    lifecycle: &'a MaterialiseLifecycle<'l>,
    workspace_root: &'a Path,
    deferred: Vec<ResolvedDep>,
}

impl<'a, 'l> PreInstallPlan<'a, 'l> {
    pub(super) fn new(lifecycle: &'a MaterialiseLifecycle<'l>, workspace_root: &'a Path) -> Self {
        Self {
            lifecycle,
            workspace_root,
            deferred: Vec::new(),
        }
    }

    pub(super) fn run_or_defer(
        &mut self,
        dep: &ResolvedDep,
        reports: &mut Vec<HookReport>,
    ) -> Result<(), WorkspaceError> {
        if matches!(self.lifecycle, MaterialiseLifecycle::Callback(_)) {
            self.deferred.push(dep.clone());
        } else {
            match run_pre_slot_lifecycle(self.lifecycle, dep, self.workspace_root) {
                Ok(Some(report)) => reports.push(report),
                Ok(None) => {}
                Err(error) => {
                    self.rollback(dep);
                    return Err(error);
                }
            }
        }
        Ok(())
    }

    pub(super) fn dispatch(self) -> Result<(), WorkspaceError> {
        if let MaterialiseLifecycle::Callback(callback) = self.lifecycle {
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
        }
        for dep in &self.deferred {
            if let Err(error) = run_pre_slot_lifecycle(self.lifecycle, dep, self.workspace_root) {
                self.rollback(dep);
                return Err(error);
            }
        }
        Ok(())
    }

    fn rollback(&self, dep: &ResolvedDep) {
        if is_in_place(dep) {
            let _ = vibedeps::remove_in_place_slot(self.workspace_root, &dep.group, &dep.name);
        } else {
            let _ = vibedeps::remove_slot(self.workspace_root, &dep.group, &dep.name, &dep.version);
        }
    }
}

pub(super) fn context_with_slot<'a>(
    dep: &'a ResolvedDep,
    slot: &'a Path,
) -> SlotLifecycleContext<'a> {
    SlotLifecycleContext {
        group: &dep.group,
        name: &dep.name,
        version: &dep.version,
        kind: &dep.kind,
        slot,
        manifest: &dep.manifest,
    }
}
