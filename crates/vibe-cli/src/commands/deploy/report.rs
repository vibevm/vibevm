//! Human and machine projections for deploy commands.

use std::path::Path;

use anyhow::{Context, Result};
use vibe_core::manifest::TargetApplicability;
use vibe_lifecycle::{
    DeployPlanReport, DeploySelection, DeploymentRow, RemovalOutcome, list_deployments,
};

use crate::output;

use super::profile;

pub(super) fn deployments(ctx: &output::Context, state_home: &Path) -> Result<()> {
    let rows = list_deployments(state_home)?;
    if ctx.is_json() {
        return ctx.emit_json(&json_rows(&rows));
    }
    if rows.is_empty() {
        ctx.summary("no deployments recorded on this machine");
        return Ok(());
    }
    ctx.heading("Deployments");
    for row in &rows {
        ctx.step(&format!(
            "{} — profile={} generation={} status={} scope={} provider={} resources={} \
             reversible={} applied={}",
            row.target,
            row.profile,
            row.generation,
            row.status.as_str(),
            row.scope,
            row.provider,
            row.resources,
            row.reversible,
            row.applied_at,
        ));
    }
    ctx.summary(&format!("{} deployment(s)", rows.len()));
    Ok(())
}

/// The listing's machine form. Hand-rendered rather than a wire format:
/// §12 froze the two §7.2 RECORDS, and a command's own view of them is a
/// projection, not a fourth record epoch.
fn json_rows(rows: &[DeploymentRow]) -> serde_json::Value {
    serde_json::json!({
        "command": "deployments",
        "ok": true,
        "count": rows.len(),
        "deployments": rows
            .iter()
            .map(|row| serde_json::json!({
                "deployment": row.deployment,
                "project": row.project,
                "package": row.package,
                "profile": row.profile,
                "target": row.target,
                "generation": row.generation,
                "status": row.status.as_str(),
                "scope": row.scope,
                "provider": row.provider,
                "reversible": row.reversible,
                "resources": row.resources,
                "applied_at": row.applied_at,
                "finalized_at": row.finalized_at,
            }))
            .collect::<Vec<_>>(),
    })
}

pub(super) fn plan(
    ctx: &output::Context,
    resolution: &profile::ProfileResolution,
    reports: &[DeployPlanReport],
) -> Result<()> {
    if ctx.is_json() {
        return ctx.emit_json(&plan_json(resolution, reports)?);
    }
    ctx.heading(&format!(
        "Deploy plan — profile `{}` on `{}`",
        resolution.profile,
        resolution
            .observed_os()
            .context("forward deploy plan carries no observed OS")?,
    ));
    for decision in resolution.decisions() {
        if decision.is_active() {
            let report = report_for(reports, decision.target())?;
            ctx.step(&format!(
                "{} [{}] provider={} via={} — {}",
                report.target,
                if report.planned { "planned" } else { "fresh" },
                report.provider,
                report.via,
                report.reason,
            ));
            for resource in &report.resources {
                ctx.step(&format!("    {} {}", resource.change, resource.resource));
            }
        } else {
            ctx.step(&format!(
                "{} [skipped] when={} — {}",
                decision.target(),
                guard_text(decision),
                decision.reason(),
            ));
        }
    }
    let planned = reports.iter().filter(|report| report.planned).count();
    let skipped = resolution
        .decisions()
        .iter()
        .filter(|decision| !decision.is_active())
        .count();
    ctx.summary(&format!(
        "{planned} of {} applicable target(s) would be deployed; {skipped} skipped; nothing was read, built or changed",
        reports.len(),
    ));
    Ok(())
}

pub(super) fn plan_json(
    resolution: &profile::ProfileResolution,
    reports: &[DeployPlanReport],
) -> Result<serde_json::Value> {
    let observed = resolution
        .observed_os()
        .context("forward deploy plan carries no observed OS")?;
    let mut targets = Vec::with_capacity(resolution.decisions().len());
    for decision in resolution.decisions() {
        let mut row = if decision.is_active() {
            let report = report_for(reports, decision.target())?;
            serde_json::json!({
                "target": report.target,
                "mechanism": report.mechanism,
                "provider": report.provider,
                "via": report.via,
                "displaced_default": report.displaced_default,
                "planned": report.planned,
                "reason": report.reason,
                "summary": report.summary,
                "resources": report.resources.iter().map(|resource| serde_json::json!({
                    "resource": resource.resource,
                    "desired_digest": resource.desired_digest,
                    "recorded_digest": resource.recorded_digest,
                    "change": resource.change,
                })).collect::<Vec<_>>(),
            })
        } else {
            serde_json::json!({
                "target": decision.target(),
                "planned": false,
                "reason": decision.reason(),
                "resources": Vec::<serde_json::Value>::new(),
            })
        };
        let object = row
            .as_object_mut()
            .context("internal: deploy plan row is not an object")?;
        object.insert("status".into(), decision.status().into());
        object.insert("observed_os".into(), observed.as_str().into());
        if let Some(guard) = decision.when() {
            object.insert(
                "when".into(),
                serde_json::json!({
                    "os": guard.os().iter().map(|os| os.as_str()).collect::<Vec<_>>()
                }),
            );
        }
        targets.push(row);
    }
    Ok(serde_json::json!({
        "command": "deploy",
        "ok": true,
        "mode": "plan",
        "profile": resolution.profile,
        "observed_os": observed.as_str(),
        "targets": targets,
    }))
}

fn report_for<'a>(reports: &'a [DeployPlanReport], target: &str) -> Result<&'a DeployPlanReport> {
    reports
        .iter()
        .find(|report| report.target == target)
        .with_context(|| format!("internal: active deploy target `{target}` has no plan report"))
}

fn guard_text(decision: &TargetApplicability) -> String {
    decision.when().map_or_else(
        || "unconditional".to_owned(),
        |guard| {
            format!(
                "os=[{}]",
                guard
                    .os()
                    .iter()
                    .map(|os| os.as_str())
                    .collect::<Vec<_>>()
                    .join(",")
            )
        },
    )
}

pub(super) fn removals(
    ctx: &output::Context,
    selection: &DeploySelection,
    removals: &[RemovalOutcome],
) -> Result<()> {
    if ctx.is_json() {
        return ctx.emit_json(&serde_json::json!({
            "command": "undeploy",
            "ok": true,
            "profile": selection.profile,
            "targets": removals
                .iter()
                .map(|outcome| serde_json::json!({
                    "target": outcome.target,
                    "provider": outcome.provider,
                    "removed": outcome.removed,
                }))
                .collect::<Vec<_>>(),
        }));
    }
    ctx.heading(&format!("Undeploy — profile `{}`", selection.profile));
    for outcome in removals {
        ctx.step(&format!(
            "{} — removed {} resource(s)",
            outcome.target,
            outcome.removed.len(),
        ));
    }
    ctx.summary(&format!("{} target(s) reversed", removals.len()));
    Ok(())
}

pub(super) fn nothing(ctx: &output::Context, mode: &str) -> Result<()> {
    if ctx.is_json() {
        return ctx.emit_json(&serde_json::json!({
            "command": "deploy",
            "ok": true,
            "mode": mode,
            "profile": serde_json::Value::Null,
            "targets": Vec::<serde_json::Value>::new(),
        }));
    }
    ctx.summary("this project declares no deploy profiles; nothing to plan");
    Ok(())
}
