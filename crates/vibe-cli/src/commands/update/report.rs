//! The ONE document `vibe update` emits.
//!
//! Update runs on the install substrate but is its own command: the registered
//! root is `cli-update-report`, the run identity and chain are update's own,
//! and a hosted handoff resumes with `vibe update` — never `vibe install`.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#install-workflow-in-detail");

use std::path::Path;

use anyhow::Result;
use vibe_wire::generated::update_report::{
    UpdateContributionReport, UpdateDelegation, UpdateReport, UpdateReportScope,
};

use crate::cli::UpdateArgs;
use crate::commands::install::HookReportPresentation;
use crate::output;

/// The ONE `cli-update-report` document. Update runs on the install
/// substrate but is its own command: it reports `update`, its own scope and
/// package set, and a handoff whose resume line is the update command.
/// Everything the one update document reports. Grouped so the renderer reads
/// as one outcome rather than a positional list.
pub(super) struct UpdateOutcome<'a> {
    pub(super) project_root: &'a Path,
    pub(super) args: &'a UpdateArgs,
    pub(super) progress: &'a vibe_install::InstallProgress,
    pub(super) packages_resolved: usize,
    pub(super) bumps: &'a [String],
    pub(super) rows: &'a [vibe_install::SlotLifecycleReport],
    pub(super) delegation: Option<&'a vibe_lifecycle::Delegation>,
}

pub(super) fn emit_update_document(
    ctx: &output::Context,
    outcome: UpdateOutcome<'_>,
) -> Result<()> {
    let UpdateOutcome {
        project_root,
        args,
        progress,
        packages_resolved,
        bumps,
        rows,
        delegation,
    } = outcome;
    let report = UpdateReport {
        ok: true,
        command: "update".into(),
        project: vibe_core::machine_json_path(project_root),
        scope: if args.all || args.packages.is_empty() {
            UpdateReportScope::All
        } else {
            UpdateReportScope::Scoped
        },
        packages: args.packages.clone(),
        packages_resolved: packages_resolved as u32,
        version_bumps: bumps.to_vec(),
        // Command-level completion, decided once here: a parked update has a
        // finished materialisation and an unfinished command.
        complete: progress.complete && delegation.is_none(),
        unchanged: progress.fresh,
        materialised: progress.materialised.clone(),
        skipped: progress.skipped.clone(),
        pruned: progress.pruned.clone(),
        nodes_regenerated: progress.nodes_regenerated.clone(),
        // Emitted even when empty: the pre-R7.3 document carried these members
        // unconditionally, and the per-row echo that used to expose slot rows
        // is gone, so this is now their only home.
        hooks: Vec::new(),
        contributions: rows.iter().map(contribution_row).collect(),
        delegation: delegation.map(|delegation| UpdateDelegation {
            resume: delegation.resume.clone(),
            run_id: delegation.run_id.clone(),
            tasks: delegation.tasks.clone(),
        }),
        // R3.4: the shared trace member. Construction from a live recorder
        // lands with the command-owner atom; disabled omits it byte-for-byte.
        trace: None,
    };
    if ctx.is_json() {
        if delegation.is_some() {
            ctx.discard_json_plans();
        } else {
            ctx.flush_json_plans()?;
        }
        return ctx.emit_json(&report);
    }
    if let Some(delegation) = delegation {
        crate::commands::lifecycle::render_agent_task_fence(
            ctx,
            &delegation.run_id,
            &delegation.tasks,
            &delegation.resume,
        );
        ctx.summary(&format!(
            "vibe update: parked for the hosting agent — {} task(s) await it; resume with `{}`",
            delegation.tasks.len(),
            delegation.resume,
        ));
    }
    Ok(())
}

pub(super) fn emit_report(
    ctx: &output::Context,
    outcome: UpdateOutcome<'_>,
    hook_reports: &dyn HookReportPresentation,
) -> Result<()> {
    let (project_root, args, progress, count, bumps, rows) = (
        outcome.project_root,
        outcome.args,
        outcome.progress,
        outcome.packages_resolved,
        outcome.bumps,
        outcome.rows,
    );
    if ctx.is_json() {
        // The ordinary path uses the SAME generated root as the parked one —
        // an ad-hoc object here would mean the command's format depended on
        // its runtime status, which is exactly what the registered format
        // exists to prevent.
        return emit_update_document(
            ctx,
            UpdateOutcome {
                project_root,
                args,
                progress,
                packages_resolved: count,
                bumps,
                rows,
                delegation: None,
            },
        );
    }
    if ctx.is_quiet() {
        ctx.summary(&format!(
            "vibe update: {count} package{} re-resolved, {} bump{}{}",
            if count == 1 { "" } else { "s" },
            bumps.len(),
            if bumps.len() == 1 { "" } else { "s" },
            hook_reports.quiet_suffix(),
        ));
        return Ok(());
    }
    for b in bumps {
        ctx.created(b);
    }
    ctx.summary(&format!(
        "\nUpdated {count} package{} ({} version bump{}).",
        if count == 1 { "" } else { "s" },
        bumps.len(),
        if bumps.len() == 1 { "" } else { "s" },
    ));
    Ok(())
}

/// A scoped update whose slot is already materialised raises no payload event,
/// so its post-install pass never revisits a live park. The persisted
/// continuation is exactly the mechanism for that case: service it before
/// anything reports a completed update.
///
/// `Ok(None)` when nothing is owed, so the ordinary path proceeds untouched.
#[allow(clippy::too_many_arguments)]
pub(super) fn service_continuation(
    ctx: &output::Context,
    lifecycle: &vibe_install::InstallSlotLifecycle,
    project_root: &Path,
    workspace: &vibe_workspace::Workspace,
    manifest: &vibe_core::manifest::Manifest,
    metadata: &vibe_lifecycle::RunMetadata,
    spec_format: vibe_core::manifest::SpecFormat,
    args: &UpdateArgs,
    resolved: usize,
    bumps: &[String],
) -> Result<Option<Result<()>>> {
    if !lifecycle.owes_slot_work() {
        return Ok(None);
    }
    let Some(run) = crate::commands::install::resume_slot_continuation(
        ctx,
        crate::commands::install::ResumeRequest {
            project_root,
            workspace,
            manifest,
            metadata,
            spec_format,
            disposition: crate::commands::install::InstallDisposition::Fresh,
            progress: lifecycle.progress(),
            packages_resolved: resolved,
        },
    )?
    else {
        return Ok(None);
    };
    if let Some(delegation) = run.parked.as_ref() {
        crate::commands::lifecycle::check_delegation(delegation)?;
    }
    Ok(Some(emit_update_document(
        ctx,
        UpdateOutcome {
            project_root,
            args,
            progress: &run.progress,
            packages_resolved: resolved,
            bumps,
            rows: &run.slot_reports,
            delegation: run.parked.as_ref(),
        },
    )))
}

fn contribution_row(row: &vibe_install::SlotLifecycleReport) -> UpdateContributionReport {
    UpdateContributionReport {
        key: row.key.clone(),
        phase: "install".into(),
        point: row.point.clone(),
        handler: row.handler.clone(),
        provider: row.provider.clone(),
        tier: row.tier.clone(),
        status: row.status.clone(),
        message: row.message.clone(),
        version: row.version.clone(),
        reference: Some(row.reference.clone()),
        flagged: row.flagged.then_some(true),
        stdout: row.stdout.clone(),
        stderr: row.stderr.clone(),
    }
}
