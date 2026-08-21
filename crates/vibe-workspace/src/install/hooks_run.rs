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

/// Run the `post-install` hooks for the packages materialised this install
/// (PROP-020 §2.1), after the lockfile is written and boot regenerated — the
/// install layer calls this from its apply phase, once each package is
/// durable. A `post-install` non-zero exit is reported, not fatal (the
/// package is already installed); a missing interpreter is still a hard error
/// (PROP-020 §2.2). `materialised_slots` are the `vibedeps/` slot rel paths
/// [`apply_resolution`] reported as freshly written — only those run, so a
/// trusted-and-skipped slot (PROP-011 §2.3) does not re-run its hook.
pub fn run_post_install_hooks(
    workspace_root: &Path,
    resolution: &[ResolvedDep],
    materialised_slots: &[String],
    policy: &HookPolicy,
) -> Result<Vec<HookReport>, WorkspaceError> {
    run_post_install_with(
        workspace_root,
        resolution,
        materialised_slots,
        policy,
        &SystemProbe,
        &SystemHookRunner,
    )
}

/// What [`materialise_subtree`] placed — the freshly-written and skipped slot
/// labels, any `verify`-mode divergence warns, and the `pre-install` hook
/// reports, for the scoped-update caller.
#[derive(Debug)]
pub struct SubtreeOutcome {
    pub materialised: Vec<String>,
    pub skipped: Vec<String>,
    pub integrity_warnings: Vec<String>,
    pub hook_reports: Vec<HookReport>,
}

/// The seam-injectable body of [`run_post_install_hooks`]: run each
/// materialised dep's `post-install` hook against the given probe + runner.
/// A `post-install` non-zero exit is carried back as a flagged report by
/// [`run_package_hook`] (not an error); a missing interpreter still errors.
pub(super) fn run_post_install_with(
    workspace_root: &Path,
    resolution: &[ResolvedDep],
    materialised_slots: &[String],
    policy: &HookPolicy,
    probe: &dyn InterpreterProbe,
    runner: &dyn HookRunner,
) -> Result<Vec<HookReport>, WorkspaceError> {
    let fresh: HashSet<&str> = materialised_slots.iter().map(String::as_str).collect();
    let mut reports = Vec::new();
    for dep in resolution {
        // Match the slot label `apply_resolution` reported — the unversioned
        // in-place path (PROP-022 §2.4) or the versioned `copy` path.
        let slot = if is_in_place(dep) {
            vibedeps::in_place_slot_rel_path(&dep.group, &dep.name)
        } else {
            vibedeps::slot_rel_path(&dep.group, &dep.name, &dep.version)
        };
        if !fresh.contains(slot.as_str()) {
            continue;
        }
        if let Some(report) = run_dep_hook(
            HookPhase::PostInstall,
            dep,
            workspace_root,
            policy,
            probe,
            runner,
        )? {
            reports.push(report);
        }
    }
    Ok(reports)
}
