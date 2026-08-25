//! Presentation for `vibe install` — the plan listing and the outcome
//! / fresh-path envelopes. Pure rendering over the orchestrator's
//! types; nothing here mutates state.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#install-workflow-in-detail");

use anyhow::Result;
use serde::Serialize;
use vibe_install::ApplyReport;
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

    pub(crate) fn json(&self) -> Vec<serde_json::Value> {
        self.reports
            .iter()
            .map(|report| {
                serde_json::json!({
                    "phase": report.phase,
                    "status": report.status,
                    "note": report.note,
                })
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
        let _ = ctx.emit_json(&serde_json::json!({
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

pub(super) fn emit_report(
    ctx: &output::Context,
    applied: &ApplyReport,
    world_summary: WorldCallbackSummary,
) -> Result<()> {
    let outcome = &applied.outcome;
    // Every install-hook report for this run — pre-install (gathered during
    // the materialise pass) followed by post-install (after the lockfile
    // write). Surfaced so a skipped or failed hook is never silent
    // (PROP-020 §2.3/§2.5).
    let hooks = HookReportView::new(&outcome.hook_reports, &applied.post_install_reports);

    if ctx.is_json() {
        ctx.emit_json(&serde_json::json!({
            "ok": true,
            "command": "install",
            "materialised": outcome.materialised,
            "skipped": outcome.skipped,
            "pruned": outcome.pruned,
            "nodes_regenerated": outcome.nodes_regenerated,
            "hooks": hooks.json(),
        }))?;
        return Ok(());
    }
    if ctx.is_quiet() {
        let suffix = format!("{}{}", hooks.quiet_suffix(), ritual_suffix(world_summary));
        ctx.summary(&format!(
            "vibe install: {} package{} materialised{}",
            outcome.materialised.len(),
            if outcome.materialised.len() == 1 {
                ""
            } else {
                "s"
            },
            suffix,
        ));
        return Ok(());
    }
    ctx.summary(&format!(
        "\nMaterialised {} package{} into vibedeps/; regenerated boot artifacts for {} node{}.",
        outcome.materialised.len(),
        if outcome.materialised.len() == 1 {
            ""
        } else {
            "s"
        },
        outcome.nodes_regenerated.len(),
        if outcome.nodes_regenerated.len() == 1 {
            ""
        } else {
            "s"
        },
    ));
    if !outcome.skipped.is_empty() {
        ctx.step(&format!(
            "{} slot{} already present — re-copy skipped (PROP-011 §2.3)",
            outcome.skipped.len(),
            if outcome.skipped.len() == 1 { "" } else { "s" },
        ));
    }
    if !outcome.pruned.is_empty() {
        ctx.step(&format!(
            "pruned {} stale vibedeps/ slot{}",
            outcome.pruned.len(),
            if outcome.pruned.len() == 1 { "" } else { "s" },
        ));
    }
    hooks.render_human(ctx);
    Ok(())
}

/// Report the PROP-011 §2.2 fast path — `vibe.lock` was fresh, so no
/// resolution ran. Kept distinct from [`emit_report`] so the operator can
/// tell a no-op `vibe install` from one that materialised packages.
pub(super) fn emit_fresh_report(
    ctx: &output::Context,
    nodes_regenerated: &[String],
    world_summary: WorldCallbackSummary,
) -> Result<()> {
    if ctx.is_json() {
        ctx.emit_json(&serde_json::json!({
            "ok": true,
            "command": "install",
            "unchanged": true,
            "nodes_regenerated": nodes_regenerated,
        }))?;
        return Ok(());
    }
    ctx.summary(&format!(
        "vibe install: vibe.lock unchanged — nothing to re-resolve ({} node{} up to date){}",
        nodes_regenerated.len(),
        if nodes_regenerated.len() == 1 {
            ""
        } else {
            "s"
        },
        ritual_suffix(world_summary),
    ));
    Ok(())
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
