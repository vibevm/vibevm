//! Presentation for `vibe install` — the plan listing and the outcome
//! / fresh-path envelopes. Pure rendering over the orchestrator's
//! types; nothing here mutates state.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#install-workflow-in-detail");

use vibe_install::SlotLifecycleReport;
use vibe_wire::generated::install_report::{
    InstallContributionReport, InstallDelegation, InstallSlotTarget,
};

use crate::output;

use super::WorldCallbackSummary;

/// A request-local handoff, populated only after successful report emission.
pub(crate) type OutputSlot = std::sync::Arc<std::sync::OnceLock<FinalOutput>>;

/// Owned final presentation, with no lease, command exit or wire document.
#[derive(Debug)]
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-067#trailer")]
pub(crate) struct FinalOutput {
    pub(super) progress: vibe_install::InstallProgress,
    pub(super) summary: WorldCallbackSummary,
    pub(super) human: super::observer::HumanTrailer,
    pub(super) suffix: String,
}

impl FinalOutput {
    /// Print only after the caller's remaining fallible work has succeeded.
    pub(crate) fn print(
        self,
        ctx: &output::Context,
        prompt: Option<&crate::commands::prompt::PromptSummary>,
    ) {
        render_human_progress(
            ctx,
            &self.progress,
            self.summary,
            &self.suffix,
            &self.human,
            prompt,
        );
    }
}

/// The quiet-summary account of slot rows that failed without stopping the
/// command, shared by the install-family commands that report them.
pub(crate) struct LifecycleHookView<'a>(&'a [SlotLifecycleReport]);

impl LifecycleHookView<'_> {
    pub(crate) const fn new(reports: &[SlotLifecycleReport]) -> LifecycleHookView<'_> {
        LifecycleHookView(reports)
    }
    pub(crate) fn quiet_suffix(&self) -> String {
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

#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-073#SOURCE-EVIDENCE-PROJECT-EVENTS"
)]
pub(super) fn present_resolution(ctx: &output::Context, planned: &vibe_install::PlannedInstall) {
    if ctx.is_json() {
        use vibe_wire::generated::install::e2::plan::{InstallPlan, PlanEntry};
        let packages = planned
            .package_sources()
            .map(|(group, name, version, evidence)| PlanEntry {
                package: format!("{group}/{name}"),
                version: version.to_string(),
                source_hosting: vibe_registry::source_hosting(Some(&evidence)),
                source_evidence: Some(evidence),
                source_candidates: planned.package_candidates(group, name).map(candidate_rows),
            })
            .collect();
        // The plan is a PREVIEW, deferred like every other one: a run that
        // parks emits its handoff document alone, and a run that completes
        // flushes this first, exactly as before.
        let _ = ctx.defer_json_plan(&hostings::with_hostings(InstallPlan {
            command: "install:plan".into(),
            packages,
            preview_complete: None,
            remaps: None,
            hostings: None,
        }));
        return;
    }
    if ctx.is_quiet() {
        return;
    }
    let rows: Vec<plan::PlanRow> = planned
        .package_sources()
        .map(|(group, name, version, evidence)| plan::PlanRow {
            package: format!("{group}/{name}"),
            version: version.to_string(),
            registry: evidence.registry.clone(),
        })
        .collect();
    ctx.rule(&plan::plan_label(rows.len()));
    ctx.block(&plan::plan_list(&rows, crate::output::language::viewport()));
}

pub(super) fn handoff_member(delegation: &vibe_lifecycle::Delegation) -> InstallDelegation {
    InstallDelegation {
        resume: delegation.resume.clone(),
        run_id: delegation.run_id.clone(),
        tasks: delegation.tasks.clone(),
    }
}

/// The typed slot rows this command itself ran, for its own document.
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/VIBEVM-SPEC#install-workflow-in-detail"
)]
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
    world_summary: WorldCallbackSummary,
    trace_suffix: &str,
    human: &super::observer::HumanTrailer,
    prompt: Option<&crate::commands::prompt::PromptSummary>,
) {
    let prompt_suffix = if ctx.is_quiet() {
        match prompt.map(|summary| &summary.state) {
            Some(crate::commands::prompt::PromptState::Rebuilt { .. }) => {
                " · system prompt rebuilt"
            }
            Some(crate::commands::prompt::PromptState::Unchanged) => " · system prompt unchanged",
            Some(crate::commands::prompt::PromptState::Removed) => " · system prompt removed",
            _ => "",
        }
    } else {
        if let Some(prompt) = prompt {
            ctx.block(&prompt.row_at(crate::output::language::viewport()));
        }
        ""
    };
    if progress.fresh {
        let nodes = progress.nodes_regenerated.len();
        let nodes = format!("{nodes} node{}", if nodes == 1 { "" } else { "s" });
        if ctx.is_quiet() {
            ctx.summary(&format!(
                "vibe install: vibe.lock unchanged — nothing to re-resolve ({nodes} up to date){}{prompt_suffix}{trace_suffix}",
                ritual_suffix(world_summary),
            ));
        } else {
            render_trailer(ctx, progress, world_summary, human);
        }
        return;
    }
    let count = progress.materialised.len();
    let plural = if count == 1 { "" } else { "s" };
    if ctx.is_quiet() {
        ctx.summary(&format!(
            "vibe install: {count} package{plural} materialised{}{prompt_suffix}{trace_suffix}",
            ritual_suffix(world_summary),
        ));
        return;
    }
    render_trailer(ctx, progress, world_summary, human);
}

/// Use the existing suffix-aware API to put actual evidence after elapsed time.
fn render_trailer(
    ctx: &output::Context,
    progress: &vibe_install::InstallProgress,
    summary: WorldCallbackSummary,
    human: &super::observer::HumanTrailer,
) {
    let mut what = trailer_what(progress, summary);
    if let Some(root) = &human.root {
        what.push_str(&format!(" · {root}"));
    }
    if let Some(directory) = &human.evidence_directory {
        ctx.trailer_at(&what, &format!("log {}", directory.display()));
    } else {
        ctx.trailer(&what);
    }
}

/// What landed and which contributions actually ran; quiet wording is separate.
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-067#trailer")]
pub(super) fn trailer_what(
    progress: &vibe_install::InstallProgress,
    summary: WorldCallbackSummary,
) -> String {
    let count =
        |n: usize, one: &str, many: &str| format!("{n} {}", if n == 1 { one } else { many });
    let nodes = progress.nodes_regenerated.len();
    let mut what = if progress.fresh {
        format!(
            "vibe.lock unchanged · nothing to re-resolve · {} up to date",
            count(nodes, "node", "nodes")
        )
    } else {
        let mut what = format!(
            "installed {} · {}",
            count(progress.materialised.len(), "package", "packages"),
            count(nodes, "boot node", "boot nodes")
        );
        if !progress.skipped.is_empty() {
            what.push_str(&format!(" · {} already present", progress.skipped.len()));
        }
        if !progress.pruned.is_empty() {
            what.push_str(&format!(" · {} stale removed", progress.pruned.len()));
        }
        what
    };
    if summary != WorldCallbackSummary::default() {
        let ran = summary.successful_contributions;
        let fresh = summary.fresh_contributions;
        let not_run = summary.selected_contributions.saturating_sub(ran + fresh);
        what.push_str(&format!(
            " · {} ran",
            count(ran, "contribution", "contributions")
        ));
        if fresh > 0 {
            what.push_str(&format!(" · {fresh} fresh"));
        }
        if not_run > 0 {
            what.push_str(&format!(" · {not_run} not run"));
        }
        if summary.notices > 0 {
            what.push_str(&format!(
                " · {}",
                count(summary.notices, "notice", "notices")
            ));
        }
    }
    what
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
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/VIBEVM-SPEC#install-workflow-in-detail"
)]
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

#[cfg(test)]
#[path = "report/tests.rs"]
mod tests;

mod candidates;
pub(crate) mod plan;
pub(crate) use candidates::candidate_rows;
pub(crate) mod hostings;
