//! The ONE document `vibe reinstall` emits.
//!
//! `vibe reinstall --force` is a MATERIALISATION force — it re-fetches from
//! source so changed-slot callbacks run. It is NOT the lifecycle's repark
//! force: `RunMetadata.force` stays false here, precisely so a forced
//! reinstall can adopt and satisfy the run it parked instead of minting a new
//! identity on every resume. The command owns its own registered root, run
//! identity and `resume: vibe reinstall` rather than borrowing install's.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#install-workflow-in-detail");

use std::path::Path;

use anyhow::Result;
use vibe_wire::generated::reinstall_report::{
    ReinstallContributionReport, ReinstallDelegation, ReinstallReport,
};

use crate::output;

/// The ONE `cli-reinstall-report` document. `--force` here forces
/// MATERIALISATION only — the run can still adopt and satisfy its own park —
/// and the command owns its own command, run and resume line rather than
/// borrowing install's.
pub(super) fn emit_reinstall_document(
    ctx: &output::Context,
    project_root: &Path,
    progress: &vibe_install::InstallProgress,
    forced: bool,
    rows: &[vibe_install::SlotLifecycleReport],
    delegation: Option<&vibe_lifecycle::Delegation>,
) -> Result<()> {
    let report = ReinstallReport {
        ok: true,
        command: "reinstall".into(),
        project: vibe_core::machine_json_path(project_root),
        forced,
        // Command-level completion, decided once here: a parked reinstall has
        // a finished materialisation and an unfinished command.
        complete: progress.complete && delegation.is_none(),
        unchanged: progress.fresh,
        materialised: progress.materialised.clone(),
        skipped: progress.skipped.clone(),
        pruned: progress.pruned.clone(),
        nodes_regenerated: progress.nodes_regenerated.clone(),
        hooks: Vec::new(),
        contributions: rows.iter().map(contribution_row).collect(),
        delegation: delegation.map(|delegation| ReinstallDelegation {
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
            "vibe reinstall: parked for the hosting agent — {} task(s) await it; resume with `{}`",
            delegation.tasks.len(),
            delegation.resume,
        ));
    }
    Ok(())
}

/// Finish a slot run a FORCED reinstall parked.
///
/// `vibe reinstall --force` is the mode that re-fetches and therefore reaches
/// changed slot callbacks, so it is the mode that can park. The handoff names
/// the plain `vibe reinstall` base verb — a command that must actually be able
/// to service the continuation, which is what this does: rebuild the locked
/// world, select exactly the persisted targets, and run the post-install
/// continuation before any ordinary boot regeneration.
///
/// `Ok(None)` when nothing is owed, so the ordinary path proceeds untouched.
pub(super) fn resume_reinstall_continuation(
    ctx: &output::Context,
    workspace: &vibe_workspace::Workspace,
    args: &crate::cli::ReinstallArgs,
    spec_format: vibe_core::manifest::SpecFormat,
) -> Result<Option<Result<()>>> {
    let metadata = super::inputs::reinstall_metadata(ctx, &workspace.root, false, args)?;
    let manifest = vibe_core::manifest::Manifest::read(
        workspace.root.join(vibe_core::manifest::Manifest::FILENAME),
    )?;
    // Mechanical: the seam now returns the run PLUS the context a direct
    // install would continue with. Reinstall's behaviour is closed, so it takes
    // the run and ignores the context.
    let Some(resumed) = crate::commands::install::resume_slot_continuation(
        ctx,
        crate::commands::install::ResumeRequest {
            project_root: &workspace.root,
            workspace,
            manifest: &manifest,
            metadata: &metadata,
            spec_format,
            disposition: crate::commands::install::InstallDisposition::Fresh,
            progress: vibe_install::InstallProgress::fresh(Vec::new()),
            // A reinstall report carries no resolved count; this path only
            // finishes a run that already resolved.
            packages_resolved: 0,
        },
    )?
    else {
        return Ok(None);
    };
    let run = resumed.run;
    if let Some(delegation) = run.parked.as_ref() {
        crate::commands::lifecycle::check_delegation(delegation)?;
    }
    Ok(Some(emit_reinstall_document(
        ctx,
        &workspace.root,
        &run.progress,
        false,
        &run.slot_reports,
        run.parked.as_ref(),
    )))
}

fn contribution_row(row: &vibe_install::SlotLifecycleReport) -> ReinstallContributionReport {
    ReinstallContributionReport {
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
