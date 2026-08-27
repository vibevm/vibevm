//! Presentation for `vibe install` — the plan listing and the outcome
//! / fresh-path envelopes. Pure rendering over the orchestrator's
//! types; nothing here mutates state.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#install-workflow-in-detail");

use anyhow::Result;
use serde::Serialize;
use vibe_install::SlotLifecycleReport;
use vibe_wire::generated::install_report::{
    InstallContributionReport, InstallDelegation, InstallHookReport, InstallReport,
    InstallSlotTarget,
};
use vibe_workspace::hooks::HookReport;
use vibe_workspace::install::ResolvedDep;

use crate::output;

use super::WorldCallbackSummary;

/// One rendering policy for install-hook reports across every install-family
/// command. The view borrows the pre/post slices so callers retain the typed
/// reports without cloning or flattening away phase ordering.
pub(crate) struct HookReportView<'a> {
    reports: Vec<&'a HookReport>,
}

pub(crate) trait HookReportPresentation {
    fn quiet_suffix(&self) -> String;
}

impl<'a> HookReportView<'a> {
    pub(crate) fn new(pre_install: &'a [HookReport], post_install: &'a [HookReport]) -> Self {
        Self {
            reports: pre_install.iter().chain(post_install).collect(),
        }
    }

    pub(crate) fn empty() -> Self {
        Self {
            reports: Vec::new(),
        }
    }

    /// The typed rows the one install document carries. Generated wire, not
    /// an ad-hoc object: the hook record is part of the registered format.
    pub(crate) fn typed(&self) -> Vec<InstallHookReport> {
        self.reports
            .iter()
            .map(|report| InstallHookReport {
                phase: report.phase.to_string(),
                status: report.status.to_string(),
                note: report.note.clone(),
            })
            .collect()
    }

    pub(crate) fn flagged_count(&self) -> usize {
        self.reports
            .iter()
            .filter(|report| !matches!(report.status, "not-declared" | "ran"))
            .count()
    }

    pub(crate) fn quiet_suffix(&self) -> String {
        let flagged: Vec<String> = self
            .reports
            .iter()
            .filter(|report| !matches!(report.status, "not-declared" | "ran"))
            .map(|report| match report.note.as_deref() {
                Some(note) => format!("{}: {} — {note}", report.phase, report.status),
                None => format!("{}: {}", report.phase, report.status),
            })
            .collect();
        let count = self.flagged_count();
        if count == 0 {
            String::new()
        } else {
            format!(
                ", {count} hook report{} flagged ({})",
                if count == 1 { "" } else { "s" },
                flagged.join("; "),
            )
        }
    }

    pub(crate) fn render_human(&self, ctx: &output::Context) {
        for report in &self.reports {
            let note = report
                .note
                .as_deref()
                .map(|note| format!(" — {note}"))
                .unwrap_or_default();
            match report.status {
                "not-declared" => {}
                "ran" => ctx.step(&format!("{} hook ran", report.phase)),
                "skipped-needs-consent" => ctx.step(&format!(
                    "{} hook skipped (consent withheld){note}",
                    report.phase
                )),
                "post-install-failed" => ctx.step(&format!("{} hook failed{note}", report.phase)),
                status => ctx.step(&format!("{} hook reported `{status}`{note}", report.phase)),
            }
        }
    }
}

pub(crate) struct LifecycleHookView<'a>(&'a [SlotLifecycleReport]);

impl LifecycleHookView<'_> {
    pub(crate) const fn new(reports: &[SlotLifecycleReport]) -> LifecycleHookView<'_> {
        LifecycleHookView(reports)
    }
    fn quiet_suffix(&self) -> String {
        let flagged = self.0.iter().filter(|report| report.flagged).count();
        if flagged == 0 {
            String::new()
        } else {
            format!(
                ", {flagged} lifecycle hook{} flagged",
                if flagged == 1 { "" } else { "s" },
            )
        }
    }
}

impl HookReportPresentation for HookReportView<'_> {
    fn quiet_suffix(&self) -> String {
        HookReportView::quiet_suffix(self)
    }
}

impl HookReportPresentation for LifecycleHookView<'_> {
    fn quiet_suffix(&self) -> String {
        LifecycleHookView::quiet_suffix(self)
    }
}

pub(super) fn present_resolution(ctx: &output::Context, resolution: &[ResolvedDep]) {
    if ctx.is_json() {
        #[derive(Serialize)]
        struct PlanEntry {
            package: String,
            version: String,
        }
        let payload: Vec<PlanEntry> = resolution
            .iter()
            .map(|d| PlanEntry {
                package: format!("{}/{}", d.group, d.name),
                version: d.version.to_string(),
            })
            .collect();
        // The plan is a PREVIEW, deferred like every other one: a run that
        // parks emits its handoff document alone, and a run that completes
        // flushes this first, exactly as before.
        let _ = ctx.defer_json_plan(&serde_json::json!({
            "command": "install:plan",
            "packages": payload,
        }));
        return;
    }
    if ctx.is_quiet() {
        return;
    }
    ctx.heading(&format!(
        "\nMaterialising {} package{} into vibedeps/:",
        resolution.len(),
        if resolution.len() == 1 { "" } else { "s" },
    ));
    for d in resolution {
        println!("  {}/{}@{}", d.group, d.name, d.version);
    }
    println!();
}

/// Everything the one install document reports. Grouped so the renderer reads
/// as one outcome rather than a positional list.
pub(super) struct InstallOutcome<'a> {
    pub(super) project_root: &'a std::path::Path,
    pub(super) progress: &'a vibe_install::InstallProgress,
    pub(super) hooks: &'a HookReportView<'a>,
    pub(super) contributions: Vec<InstallContributionReport>,
    pub(super) notices: &'a [String],
    pub(super) delegation: Option<&'a vibe_lifecycle::Delegation>,
    pub(super) world_summary: WorldCallbackSummary,
}

/// The ONE `cli-install-report` document, whatever this invocation did.
///
/// Normal apply, the fresh fast path and a hosted park all render through
/// here: the command's registered root format never changes with runtime
/// status, and no path appends a second object. Progress is the slot-level
/// record the engine actually measured — directories, never a file census.
///
/// A parked run prints the one fenced handoff and NO completed summary: this
/// install did not finish, and saying otherwise would be the lie the whole
/// handoff exists to avoid.
pub(super) fn emit_install_document(
    ctx: &output::Context,
    outcome: InstallOutcome<'_>,
) -> Result<()> {
    let InstallOutcome {
        project_root,
        progress,
        hooks,
        contributions,
        notices,
        delegation,
        world_summary,
    } = outcome;
    let report = InstallReport {
        ok: true,
        command: "install".into(),
        project: vibe_core::machine_json_path(project_root),
        // `complete` is the COMMAND's completion, not the materialise pass's.
        // A park after the barrier has a finished materialisation and an
        // unfinished command: the run is waiting on the hosting agent and will
        // be resumed. Deciding it here, at the one document construction,
        // makes "a parked command is never complete" true by construction
        // rather than by every progress producer remembering to say so.
        complete: progress.complete && delegation.is_none(),
        unchanged: progress.fresh,
        materialised: progress.materialised.clone(),
        skipped: progress.skipped.clone(),
        pruned: progress.pruned.clone(),
        nodes_regenerated: progress.nodes_regenerated.clone(),
        contributions,
        notices: notices.to_vec(),
        hooks: hooks.typed(),
        delegation: delegation.map(handoff_member),
    };
    if ctx.is_json() {
        if delegation.is_some() {
            // A park emits exactly one document IN TOTAL: the plan preview
            // this run buffered is dropped rather than printed beside the
            // handoff. (A completed run keeps preview + one root.)
            ctx.discard_json_plans();
        } else {
            ctx.flush_json_plans()?;
        }
        return ctx.emit_json(&report);
    }
    if let Some(delegation) = delegation {
        super::super::lifecycle::render_agent_task_fence(
            ctx,
            &delegation.run_id,
            &delegation.tasks,
            &delegation.resume,
        );
        ctx.summary(&format!(
            "vibe install: parked for the hosting agent — {} task(s) await it; \
             {} slot(s) materialised so far, resume with `{}`",
            delegation.tasks.len(),
            progress.materialised.len(),
            delegation.resume,
        ));
        return Ok(());
    }
    render_human_progress(ctx, progress, hooks, world_summary);
    Ok(())
}

/// The ONE document a FAILED `vibe install` emits: same registered root, same
/// typed rows, `ok: false`. The exit code still comes from the error itself.
pub(super) fn emit_failed_document(
    ctx: &output::Context,
    project_root: &std::path::Path,
    progress: &vibe_install::InstallProgress,
    reports: &[SlotLifecycleReport],
) -> Result<()> {
    if !ctx.is_json() {
        return Ok(());
    }
    ctx.flush_json_plans()?;
    ctx.emit_json(&InstallReport {
        ok: false,
        command: "install".into(),
        project: vibe_core::machine_json_path(project_root),
        complete: progress.complete,
        unchanged: progress.fresh,
        materialised: progress.materialised.clone(),
        skipped: progress.skipped.clone(),
        pruned: progress.pruned.clone(),
        nodes_regenerated: progress.nodes_regenerated.clone(),
        contributions: contribution_rows(reports),
        notices: Vec::new(),
        hooks: Vec::new(),
        delegation: None,
    })
}

fn handoff_member(delegation: &vibe_lifecycle::Delegation) -> InstallDelegation {
    InstallDelegation {
        resume: delegation.resume.clone(),
        run_id: delegation.run_id.clone(),
        tasks: delegation.tasks.clone(),
    }
}

/// The typed slot rows this command itself ran, for its own document.
pub(crate) fn contribution_rows(reports: &[SlotLifecycleReport]) -> Vec<InstallContributionReport> {
    reports
        .iter()
        .map(|report| InstallContributionReport {
            key: report.key.clone(),
            phase: "install".into(),
            point: report.point.clone(),
            handler: report.handler.clone(),
            provider: report.provider.clone(),
            tier: report.tier.clone(),
            status: report.status.clone(),
            message: report.message.clone(),
            version: report.version.clone(),
            reference: Some(report.reference.clone()),
            // Streams, the soft-failure flag and the slot target travel with
            // the row: the per-row JSON echo is gone, so this document is the
            // only place they can be observed.
            flagged: report.flagged.then_some(true),
            stdout: report.stdout.clone(),
            stderr: report.stderr.clone(),
            stdout_truncated: report.stdout_truncated.then_some(true),
            stderr_truncated: report.stderr_truncated.then_some(true),
            slot_target: report.slot_target.as_ref().map(|target| InstallSlotTarget {
                group: target.group.clone(),
                kind: target.kind.clone(),
                name: target.name.clone(),
                root: target.root.clone(),
                version: target.version.clone(),
            }),
        })
        .collect()
}

fn render_human_progress(
    ctx: &output::Context,
    progress: &vibe_install::InstallProgress,
    hooks: &HookReportView<'_>,
    world_summary: WorldCallbackSummary,
) {
    if progress.fresh {
        ctx.summary(&format!(
            "vibe install: vibe.lock unchanged — nothing to re-resolve ({} node{} up to date){}",
            progress.nodes_regenerated.len(),
            if progress.nodes_regenerated.len() == 1 {
                ""
            } else {
                "s"
            },
            ritual_suffix(world_summary),
        ));
        return;
    }
    let count = progress.materialised.len();
    let plural = if count == 1 { "" } else { "s" };
    if ctx.is_quiet() {
        ctx.summary(&format!(
            "vibe install: {count} package{plural} materialised{}{}",
            hooks.quiet_suffix(),
            ritual_suffix(world_summary),
        ));
        return;
    }
    ctx.summary(&format!(
        "\nMaterialised {count} package{plural} into vibedeps/; \
         regenerated boot artifacts for {} node(s).",
        progress.nodes_regenerated.len(),
    ));
    if !progress.skipped.is_empty() {
        ctx.step(&format!(
            "{} slot(s) already present — re-copy skipped (PROP-011 §2.3)",
            progress.skipped.len(),
        ));
    }
    if !progress.pruned.is_empty() {
        ctx.step(&format!("{} stale slot(s) pruned", progress.pruned.len()));
    }
    hooks.render_human(ctx);
}

fn ritual_suffix(summary: WorldCallbackSummary) -> String {
    if summary == WorldCallbackSummary::default() {
        String::new()
    } else {
        format!(
            ", {} lifecycle contribution(s) selected, {} executed, {} ok, {} fresh, {} lifecycle notice(s)",
            summary.selected_contributions,
            summary.executed_contributions,
            summary.successful_contributions,
            summary.fresh_contributions,
            summary.notices,
        )
    }
}

/// The `phase:install` ritual rows this command ran, in the install report's
/// own row shape. They reach the document alongside the slot rows: `vibe
/// install` is the outermost command on this path, so its single report is the
/// only place either kind can be observed.
pub(crate) fn phase_rows(
    rows: &[vibe_wire::generated::lifecycle_report::LifecycleContributionReport],
) -> Vec<InstallContributionReport> {
    rows.iter()
        .map(|row| InstallContributionReport {
            key: row.key.clone(),
            phase: row.phase.clone(),
            point: row.point.clone(),
            handler: row.handler.clone(),
            provider: row.provider.clone(),
            tier: row.tier.clone(),
            status: row.status.clone(),
            message: row.message.clone(),
            version: row.version.clone(),
            reference: row.reference.clone(),
            flagged: row.flagged,
            stdout: row.stdout.clone(),
            stderr: row.stderr.clone(),
            stdout_truncated: row.stdout_truncated,
            stderr_truncated: row.stderr_truncated,
            slot_target: row.slot_target.as_ref().map(|target| InstallSlotTarget {
                group: target.group.clone(),
                kind: target.kind.clone(),
                name: target.name.clone(),
                root: target.root.clone(),
                version: target.version.clone(),
            }),
        })
        .collect()
}
