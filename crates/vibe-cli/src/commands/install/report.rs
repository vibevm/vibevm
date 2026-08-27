//! Presentation for `vibe install` — the plan listing and the outcome
//! / fresh-path envelopes. Pure rendering over the orchestrator's
//! types; nothing here mutates state.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#install-workflow-in-detail");

use serde::Serialize;
use vibe_install::SlotLifecycleReport;
use vibe_wire::generated::install_report::{
    InstallContributionReport, InstallDelegation, InstallHookReport, InstallSlotTarget,
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

pub(super) fn handoff_member(delegation: &vibe_lifecycle::Delegation) -> InstallDelegation {
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

/// The human/quiet half of a SUCCESSFUL install document.
///
/// `trace_suffix` is appended to whichever line is this command's single
/// summary — never printed on its own, because quiet mode's contract is one
/// line and a second one breaks every script that reads it.
pub(super) fn render_human_progress(
    ctx: &output::Context,
    progress: &vibe_install::InstallProgress,
    hooks: &HookReportView<'_>,
    world_summary: WorldCallbackSummary,
    trace_suffix: &str,
) {
    if progress.fresh {
        ctx.summary(&format!(
            "vibe install: vibe.lock unchanged — nothing to re-resolve ({} node{} up to date){}{trace_suffix}",
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
            "vibe install: {count} package{plural} materialised{}{}{trace_suffix}",
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
