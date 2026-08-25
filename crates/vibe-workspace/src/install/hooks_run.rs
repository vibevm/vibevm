//! The post-install hook runners — split from `install.rs` at the XML S3
//! landing (the file crossed the 600-line budget): the dep-hook walk and
//! both post-install entry points; orchestration stays in the parent.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-020#phases");

use super::*;

/// Run one `phase` hook for `dep` against its materialised slot under the
/// resolved [`HookPolicy`] (PROP-020). Returns `None` when the package
/// declares no `[hooks]` at all — the common case: no work, no report.
pub(super) fn run_dep_hook(
    phase: HookPhase,
    dep: &ResolvedDep,
    workspace_root: &Path,
    policy: &HookPolicy,
    probe: &dyn InterpreterProbe,
    runner: &dyn HookRunner,
) -> std::result::Result<Option<HookReport>, HookError> {
    if dep.manifest.hooks.is_empty() {
        return Ok(None);
    }
    // The hook runs in the package's materialised slot — the unversioned
    // in-place working tree (PROP-022 §2.4), or the versioned `copy` slot.
    let slot = if is_in_place(dep) {
        vibedeps::in_place_slot_abs_path(workspace_root, &dep.group, &dep.name)
    } else {
        vibedeps::slot_abs_path(workspace_root, &dep.group, &dep.name, &dep.version)
    };
    let version = dep.version.to_string();
    let kind = dep.kind.to_string();
    let ctx = HookContext {
        group: &dep.group,
        name: &dep.name,
        version: &version,
        kind: &kind,
        slot: &slot,
    };
    run_package_hook(
        phase,
        &dep.manifest.hooks,
        &ctx,
        policy.trust_for(&dep.group),
        Platform::current(),
        probe,
        runner,
    )
    .map(Some)
}

/// Establish copy-on-write isolation before a declared hook can observe a
/// versioned slot. Reconciliation may retain hardlinks after a hardlink→copy
/// mode change because unchanged payload keeps its inode; a hook may rewrite
/// any recorded file, so the preflight scans copy and hardlink slots and
/// atomically detaches only rows whose link count is still greater than one.
pub(super) fn prepare_hook_payload(
    phase: HookPhase,
    dep: &ResolvedDep,
    workspace_root: &Path,
) -> Result<(), WorkspaceError> {
    let declared = match phase {
        HookPhase::PreInstall => dep.manifest.hooks.pre_install.is_some(),
        HookPhase::PostInstall => dep.manifest.hooks.post_install.is_some(),
    };
    if declared && !is_in_place(dep) {
        let slot = vibedeps::slot_abs_path(workspace_root, &dep.group, &dep.name, &dep.version);
        vibedeps::detach_recorded_hardlinks(&slot)?;
    }
    Ok(())
}

/// Prepare and invoke `pre-install` as one fallible operation so the caller's
/// slot rollback covers both copy-on-write isolation and hook execution.
pub(super) fn run_pre_install_hook(
    dep: &ResolvedDep,
    workspace_root: &Path,
    policy: &HookPolicy,
    probe: &dyn InterpreterProbe,
    runner: &dyn HookRunner,
) -> Result<Option<HookReport>, WorkspaceError> {
    prepare_hook_payload(HookPhase::PreInstall, dep, workspace_root)?;
    run_dep_hook(
        HookPhase::PreInstall,
        dep,
        workspace_root,
        policy,
        probe,
        runner,
    )
    .map_err(WorkspaceError::from)
}

/// Consume one install-produced plan and run its `post-install` hooks
/// (PROP-020 §2.1), after the lockfile is written and boot regenerated — the
/// install layer calls this from its apply phase, once each package is
/// durable. A `post-install` non-zero exit is reported, not fatal (the
/// package is already installed); a missing interpreter is still a hard error
/// (PROP-020 §2.2). The opaque plan already owns the exact eligible dependency
/// subset and workspace provenance; this seam accepts neither separately.
pub fn run_post_install_hooks(
    plan: PostInstallPlan,
    policy: &HookPolicy,
) -> Result<Vec<HookReport>, WorkspaceError> {
    run_post_install_with(plan, policy, &SystemProbe, &SystemHookRunner)
}

/// What [`materialise_subtree`] placed — reporting plus one-shot post-install
/// authority.
#[derive(Debug)]
pub struct SubtreeOutcome {
    pub materialised: Vec<String>,
    pub skipped: Vec<String>,
    pub integrity_warnings: Vec<String>,
    pub(super) post_install_plan: Option<PostInstallPlan>,
    pub hook_reports: Vec<HookReport>,
}

impl SubtreeOutcome {
    /// Take the post-install plan produced by this subtree update, at most once.
    pub fn take_post_install_plan(&mut self) -> Option<PostInstallPlan> {
        self.post_install_plan.take()
    }
}

/// The seam-injectable body of [`run_post_install_hooks`]: run each
/// payload-changing dep's `post-install` hook against the given probe + runner.
/// A `post-install` non-zero exit is carried back as a flagged report by
/// [`run_package_hook`] (not an error); a missing interpreter still errors.
pub(super) fn run_post_install_with(
    plan: PostInstallPlan,
    policy: &HookPolicy,
    probe: &dyn InterpreterProbe,
    runner: &dyn HookRunner,
) -> Result<Vec<HookReport>, WorkspaceError> {
    let (workspace_root, eligible_deps) = plan.into_parts();
    let mut reports = Vec::new();
    for dep in &eligible_deps {
        prepare_hook_payload(HookPhase::PostInstall, dep, &workspace_root)?;
        if let Some(report) = run_dep_hook(
            HookPhase::PostInstall,
            dep,
            &workspace_root,
            policy,
            probe,
            runner,
        )? {
            reports.push(report);
        }
    }
    Ok(reports)
}

#[cfg(test)]
#[path = "hooks_run/tests_hook_policy.rs"]
mod tests_hook_policy;

#[cfg(test)]
#[path = "hooks_run/tests_in_place_changes.rs"]
mod tests_in_place_changes;
