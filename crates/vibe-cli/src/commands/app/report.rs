//! Rendering of application command results: the install / update /
//! uninstall / modify report, state changes (expose, hide, mark, hold,
//! rollback), tree removals and the outdated table (PROP-059 `#commands`,
//! PROP-065 `#modify`, `#outdated`, `#uninstall-tree`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#commands");

use anyhow::Result;
use vibe_app::{Change, ChangeVerb, Command as AppCommand, Outcome, Removal};
use vibe_wire::generated::application_outdated::{ApplicationOutdated, Status};
use vibe_wire::generated::application_report::{
    ApplicationReport, ApplicationReportCommand, ApplicationReportSelectedMode,
};

use crate::output;

const REPORT_PROTOCOL: &str = "vibe-application-command-report/2";

/// The report of an install, update, uninstall or modify.
pub(crate) fn render(ctx: &output::Context, outcome: &Outcome) -> Result<()> {
    let (command, verb) = match outcome.command {
        AppCommand::Install => (ApplicationReportCommand::Install, "install -g"),
        AppCommand::Update => (ApplicationReportCommand::Update, "update -g"),
        AppCommand::Uninstall => (ApplicationReportCommand::Uninstall, "uninstall -g"),
        AppCommand::Modify => (ApplicationReportCommand::Modify, "app modify"),
    };
    if ctx.is_json() {
        return ctx.emit_json(&ApplicationReport {
            protocol: REPORT_PROTOCOL.into(),
            ok: true,
            command,
            application_id: outcome.slot.clone(),
            installation: outcome.key.clone(),
            message: outcome.message.clone(),
            slot_default: outcome.slot_default.clone(),
            app_root: outcome
                .app_root
                .as_deref()
                .map(vibe_core::machine_json_path),
            work: outcome.work.as_deref().map(vibe_core::machine_json_path),
            selected_mode: Some(match outcome.method {
                vibe_app::Method::Shell => ApplicationReportSelectedMode::Shell,
                vibe_app::Method::Zip => ApplicationReportSelectedMode::Zip,
                vibe_app::Method::TarGz => ApplicationReportSelectedMode::TarGz,
                vibe_app::Method::Exe => ApplicationReportSelectedMode::Exe,
                vibe_app::Method::Msi => ApplicationReportSelectedMode::Msi,
                vibe_app::Method::SourceBuild => ApplicationReportSelectedMode::SourceBuild,
                vibe_app::Method::Steps => ApplicationReportSelectedMode::Steps,
            }),
        });
    }
    ctx.summary(&format!(
        "vibe {verb}: application `{}` — {}",
        outcome.slot, outcome.message
    ));
    if let Some(work) = &outcome.work {
        ctx.summary(&format!("work directory kept: {}", work.display()));
    }
    Ok(())
}

/// The report of `expose`, `hide`, `mark`, `hold`, `unhold`, `rollback`.
pub(crate) fn render_change(ctx: &output::Context, change: &Change) -> Result<()> {
    let (command, verb) = match change.verb {
        ChangeVerb::Expose => (ApplicationReportCommand::Expose, "expose"),
        ChangeVerb::Hide => (ApplicationReportCommand::Hide, "hide"),
        ChangeVerb::Mark => (ApplicationReportCommand::Mark, "mark"),
        ChangeVerb::Hold => (ApplicationReportCommand::Hold, "hold"),
        ChangeVerb::Unhold => (ApplicationReportCommand::Unhold, "unhold"),
        ChangeVerb::Rollback => (ApplicationReportCommand::Rollback, "rollback"),
    };
    if ctx.is_json() {
        return ctx.emit_json(&ApplicationReport {
            protocol: REPORT_PROTOCOL.into(),
            ok: true,
            command,
            application_id: change.slot.clone(),
            installation: change.key.clone(),
            message: change.message.clone(),
            slot_default: change.slot_default.clone(),
            app_root: None,
            work: None,
            selected_mode: None,
        });
    }
    ctx.summary(&format!(
        "vibe app {verb}: application `{}` — {}",
        change.slot, change.message
    ));
    Ok(())
}

/// A removal: the plan of `--plan-only`, the removed installations of
/// `autoremove`, or the uninstall report of the named installation.
pub(crate) fn render_removal(ctx: &output::Context, removal: &Removal) -> Result<()> {
    let report = &removal.report;
    if let (Some(outcome), false) = (&removal.outcome, report.plan_only) {
        return render(ctx, outcome);
    }
    if ctx.is_json() {
        return ctx.emit_json(report);
    }
    let heading = if report.plan_only {
        "removal plan"
    } else {
        "removed"
    };
    println!("{heading}: {} installation(s)", report.installations.len());
    for (index, installation) in report.installations.iter().enumerate() {
        let reason = serde_json::to_value(&installation.reason)
            .ok()
            .and_then(|value| value.as_str().map(str::to_owned))
            .unwrap_or_default();
        println!("  {}. {} ({reason})", index + 1, installation.key);
        if ctx.is_quiet() {
            continue;
        }
        for action in &installation.actions {
            let kind = serde_json::to_value(&action.kind)
                .ok()
                .and_then(|value| value.as_str().map(str::to_owned))
                .unwrap_or_default();
            match &action.step {
                Some(step) => println!("     {kind} `{step}`: {}", action.detail),
                None => println!("     {kind}: {}", action.detail),
            }
        }
    }
    if !report.orphans.is_empty() {
        println!(
            "orphans nothing requires any more: {} (remove them with --autoremove or `vibe app autoremove`)",
            report.orphans.join(", ")
        );
    }
    Ok(())
}

/// `vibe app outdated`: a table, or the `vibe-app-outdated/1` document.
pub(crate) fn render_outdated(ctx: &output::Context, report: &ApplicationOutdated) -> Result<()> {
    if ctx.is_json() {
        return ctx.emit_json(report);
    }
    let outdated = report
        .installations
        .iter()
        .filter(|row| row.status == Status::Outdated)
        .count();
    println!(
        "{outdated} of {} installed application(s) outdated",
        report.installations.len()
    );
    for row in &report.installations {
        let status = match row.status {
            Status::Current => "current",
            Status::Outdated => "outdated",
            Status::Held => "held",
            Status::Unknown => "unknown",
        };
        let latest = row.latest.as_deref().unwrap_or("?");
        let hidden = if row.hidden { " [hidden]" } else { "" };
        match &row.reason {
            Some(reason) => println!(
                "  {}: {} → {latest} ({status}: {reason}){hidden}",
                row.key, row.installed
            ),
            None => println!(
                "  {}: {} → {latest} ({status}){hidden}",
                row.key, row.installed
            ),
        }
    }
    Ok(())
}
