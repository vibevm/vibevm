//! Rendering of application command results: the install / update /
//! uninstall / modify report, state changes (expose, hide, mark, hold,
//! rollback), tree removals and the outdated table (PROP-059 `#commands`,
//! PROP-065 `#modify`, `#outdated`, `#uninstall-tree`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#commands");

use anyhow::Result;
use specmark::spec;
use vibe_app::{
    Change, ChangeVerb, Command as AppCommand, Outcome, RecommendationOutcome, Removal,
};
use vibe_wire::generated::application_outdated::{ApplicationOutdated, Status};
use vibe_wire::generated::application_report::{
    ApplicationReport, ApplicationReportCommand, ApplicationReportSelectedMode, Recommendation,
    RecommendationOutcome as WireOutcome,
};

use crate::output;
use crate::output::language::{Cell, List, Role, Row, paint, rule};

/// A library sentence as human output: identifiers lose their backticks,
/// which belong to the JSON and the logs (PROP-067 `#inline`).
pub(crate) fn prose(message: &str) -> String {
    message.replace('`', "")
}

const REPORT_PROTOCOL: &str = "vibe-application-command-report/2";

/// The report of an install, update, uninstall or modify, with the version
/// file `--write-version-file` wrote when one was asked for
/// (PROP-075 `#VERSION-FILE-WRITE`).
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-059#commands")]
pub(crate) fn render_written(
    ctx: &output::Context,
    outcome: &Outcome,
    written: Option<&vibe_app::WrittenVersionFile>,
) -> Result<()> {
    let command = match outcome.command {
        AppCommand::Install => ApplicationReportCommand::Install,
        AppCommand::Update => ApplicationReportCommand::Update,
        AppCommand::Uninstall => ApplicationReportCommand::Uninstall,
        AppCommand::Modify => ApplicationReportCommand::Modify,
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
            version_file: written.map(super::version_file::wire_report),
            recommended: outcome.recommended.iter().map(recommendation).collect(),
            suggested: outcome.suggested.clone(),
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
    ctx.trailer(&prose(&outcome.message));
    if let Some(work) = &outcome.work {
        ctx.summary(&format!(
            "  work directory kept {}",
            paint(Role::Accent, &work.display().to_string())
        ));
    }
    if let Some(written) = written {
        super::version_file::report(ctx, "vibe install -g", written);
    }
    // What the author recommended and what became of it — after it
    // happened, as the plan said it before (`PROP-077#APP-RECOMMENDS`) —
    // and the suggestions, which install nothing and are only ever seen
    // (`#SUGGESTS-UNCHANGED`).
    for line in &outcome.recommended {
        let name = if line.package.is_empty() {
            "recommendations".to_owned()
        } else {
            line.package.clone()
        };
        let state = match (&line.key, line.outcome) {
            (Some(key), RecommendationOutcome::Installed) => format!("installed {key}"),
            (Some(key), RecommendationOutcome::Satisfied) => format!("already installed {key}"),
            _ => "skipped".to_owned(),
        };
        let because = match &line.reason {
            Some(reason) => format!(" — {}", prose(reason)),
            None => String::new(),
        };
        ctx.summary(&format!(
            "  recommended {} · {state}{because}",
            paint(Role::Accent, &name)
        ));
    }
    if !outcome.suggested.is_empty() {
        ctx.summary(&format!(
            "  suggested (never installed) {}",
            outcome.suggested.join(", ")
        ));
    }
    Ok(())
}

/// The same report for the commands that write no version file.
pub(crate) fn render(ctx: &output::Context, outcome: &Outcome) -> Result<()> {
    render_written(ctx, outcome, None)
}

/// The report of `expose`, `hide`, `mark`, `hold`, `unhold`, `rollback`.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-059#commands")]
pub(crate) fn render_change(ctx: &output::Context, change: &Change) -> Result<()> {
    let command = match change.verb {
        ChangeVerb::Expose => ApplicationReportCommand::Expose,
        ChangeVerb::Hide => ApplicationReportCommand::Hide,
        ChangeVerb::Mark => ApplicationReportCommand::Mark,
        ChangeVerb::Hold => ApplicationReportCommand::Hold,
        ChangeVerb::Unhold => ApplicationReportCommand::Unhold,
        ChangeVerb::Rollback => ApplicationReportCommand::Rollback,
    };
    if ctx.is_json() {
        return ctx.emit_json(&ApplicationReport {
            // A state change writes no version file.
            version_file: None,
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
            // A state change installs nothing, so it recommends nothing.
            recommended: Vec::new(),
            suggested: Vec::new(),
        });
    }
    ctx.trailer(&format!("{} · {}", change.slot, prose(&change.message)));
    Ok(())
}

/// A removal: the plan of `--plan-only`, the removed installations of
/// `autoremove`, or the uninstall report of the named installation.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-059#commands")]
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
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-059#commands")]
pub(crate) fn render_outdated(ctx: &output::Context, report: &ApplicationOutdated) -> Result<()> {
    if ctx.is_json() {
        return ctx.emit_json(report);
    }
    println!("{}", outdated_text(report));
    Ok(())
}

fn outdated_text(report: &ApplicationOutdated) -> String {
    let outdated = report
        .installations
        .iter()
        .filter(|row| row.status == Status::Outdated)
        .count();
    let heading = rule(
        "outdated",
        Some(&format!("{outdated} of {}", report.installations.len())),
    );
    let rows = report.installations.iter().map(|row| {
        let status = match row.status {
            Status::Current => "current",
            Status::Outdated => "outdated",
            Status::Held => "held",
            Status::Unknown => "unknown",
        };
        let latest = row.latest.as_deref().unwrap_or("?");
        let mut cells = vec![
            Cell::new(&row.key, Role::Accent),
            Cell::text(format!("{} → {latest}", row.installed)),
            Cell::text(format!("[{status}]")),
        ];
        if row.hidden {
            cells.push(Cell::new("[hidden]", Role::Dim));
        }
        let mut rendered = Row::new(cells);
        if let Some(reason) = &row.reason {
            rendered = rendered.child(prose(reason));
        }
        rendered
    });
    format!("{heading}\n{}", List::new(rows.collect()).render())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outdated_rows_show_state() {
        let report: ApplicationOutdated = serde_json::from_value(serde_json::json!({
            "protocol": "vibe-app-outdated/1",
            "installations": [
                {
                    "key": "org.example/node",
                    "slot": "node",
                    "package": {
                        "group": "org.example",
                        "name": "node",
                        "version": "22.23.4"
                    },
                    "held": false,
                    "installed": "22.23.4",
                    "latest": "24.22.2",
                    "status": "outdated",
                    "hidden": false,
                    "reason": null
                },
                {
                    "key": "org.example/held",
                    "slot": "held",
                    "package": {
                        "group": "org.example",
                        "name": "held",
                        "version": "1.0.0"
                    },
                    "held": true,
                    "installed": "1.0.0",
                    "latest": null,
                    "status": "held",
                    "hidden": true,
                    "reason": "held by user"
                }
            ]
        }))
        .expect("synthetic outdated report");
        let rendered = outdated_text(&report);
        let text = console::strip_ansi_codes(&rendered);
        let mut lines = text.lines();
        let heading = lines.next().expect("outdated rule");
        assert!(heading.contains("outdated"), "{text}");
        assert!(heading.ends_with("1 of 2"), "{text}");
        assert!(heading.starts_with("── ") || heading.starts_with("-- "));
        let outdated = lines.next().expect("outdated row");
        assert!(outdated.contains("org.example/node"), "{text}");
        assert!(outdated.contains("22.23.4 → 24.22.2"), "{text}");
        assert!(outdated.contains("[outdated]"), "{text}");
        assert!(!outdated.contains("[hidden]"), "{text}");
        let held = lines.next().expect("held row");
        assert!(held.contains("org.example/held"), "{text}");
        assert!(held.contains("1.0.0 → ?"), "{text}");
        assert!(held.contains("[held]"), "{text}");
        assert!(held.contains("[hidden]"), "{text}");
        let reason = lines.next().expect("reason child");
        assert!(reason.contains("held by user"), "{text}");
        assert!(reason.contains('└') || reason.contains('\\'), "{text}");
        assert_eq!(text.matches("[outdated]").count(), 1);
        assert_eq!(text.matches("[held]").count(), 1);
        assert_eq!(text.matches("[hidden]").count(), 1);
        assert!(lines.next().is_none(), "unexpected action trailer: {text}");
    }
}

/// One recommendation as the report carries it (`PROP-077#APP-RECOMMENDS`).
fn recommendation(line: &vibe_app::Recommendation) -> Recommendation {
    Recommendation {
        package: line.package.clone(),
        by: line.by.clone(),
        key: line.key.clone(),
        outcome: match line.outcome {
            RecommendationOutcome::Installed => WireOutcome::Installed,
            RecommendationOutcome::Satisfied => WireOutcome::Satisfied,
            RecommendationOutcome::Skipped => WireOutcome::Skipped,
            RecommendationOutcome::Planned => WireOutcome::Planned,
        },
        reason: line.reason.clone(),
    }
}
