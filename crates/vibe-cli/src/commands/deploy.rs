//! The three deploy command surfaces — §7's own list:
//!
//! ```text
//! vibe deploy [--profile X] [--plan]
//! vibe undeploy --profile X
//! vibe deployments [--json]
//! ```
//!
//! What this cell owns is exactly what a command layer owns: the flags,
//! the ONE profile resolution ([`profile`]), and the rendering. It owns no
//! transaction, no state layout and no provider — those are the engine's,
//! and this surface reaches them through the same public functions any
//! other surface would.
//!
//! Two of the three verbs are READ-ONLY and say so by construction: they
//! take no mutation lease, run no chain, and call only functions that
//! write nothing. `vibe deploy` without `--plan` is the exception, and it
//! is not a fourth path — it is the ordinary ninth phase verb, carrying
//! the resolved selection down as data.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS");

use anyhow::{Context, Result, bail};
use specmark::spec;
use vibe_core::manifest::Manifest;
use vibe_lifecycle::{
    DeployExecution, DeployPlanReport, DeploySelection, DeploymentRow, RemovalOutcome,
    deploy_state_home, list_deployments, plan_deploy_targets, undeploy_targets,
};

use crate::cli::{DeployArgs, UndeployArgs};
use crate::output;

pub(crate) mod profile;

#[cfg(test)]
#[path = "deploy/tests.rs"]
mod tests;

pub(crate) use profile::resolve_profile;

/// `vibe deploy [--profile X] [--plan]`.
///
/// Without `--plan` this is the ninth phase verb: it runs the inclusive
/// chain through `deploy`, carrying the resolved selection as data. With
/// `--plan` it is a read-only planner and NOT a chain run (§7.0.6).
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
pub fn run(
    ctx: &output::Context,
    args: DeployArgs,
    prepare_install: impl FnOnce() -> Option<std::path::PathBuf>,
    root_offline: bool,
) -> Result<()> {
    if args.plan {
        return plan(ctx, &args);
    }
    let profile = args.profile.clone();
    super::lifecycle::run(
        ctx,
        vibe_lifecycle::Phase::Deploy,
        args.lifecycle,
        prepare_install,
        root_offline,
        Some(super::lifecycle::DeployRequest { profile }),
    )
}

/// `vibe deploy --profile X --plan` — §7.0.6's read-only planner.
///
/// It takes NO mutation lease and enters no chain: a plan that leased the
/// workspace would be a plan that could block a build, and a plan that
/// entered the chain would be a plan that built.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn plan(ctx: &output::Context, args: &DeployArgs) -> Result<()> {
    let root = super::resolve_project_root(&args.lifecycle.path)?;
    let manifest = read_manifest(&root)?;
    let Some(selection) = resolve_profile(manifest.deploy.as_ref(), args.profile.as_deref())?
    else {
        return report_nothing(ctx, "plan");
    };
    let loaded = vibe_orchestrator::inspect(&root)?;
    let state_home = state_home()?;
    let targets = deploy_targets(&manifest);
    let reports = plan_deploy_targets(&DeployExecution {
        project_root: &root,
        targets: &targets,
        selection: &selection,
        registry: &loaded.mechanisms,
        routes: &manifest.mechanism_routes,
        state_home: &state_home,
        project: &identity(&manifest),
        package: None,
        created_at: &now(),
    })?;
    render_plan(ctx, &selection, &reports)
}

/// `vibe undeploy --profile X`.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
pub fn run_undeploy(ctx: &output::Context, args: UndeployArgs) -> Result<()> {
    let root = super::resolve_project_root(&args.path)?;
    let manifest = read_manifest(&root)?;
    let Some(selection) = resolve_profile(manifest.deploy.as_ref(), Some(&args.profile))? else {
        bail!(
            "`--profile {}` was requested, but this project declares no deploy profiles \
             (violates spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS; fix: \
             run `vibe deployments` to see what this machine has deployed)",
            args.profile,
        );
    };
    let loaded = vibe_orchestrator::inspect(&root)?;
    let state_home = state_home()?;
    let targets = deploy_targets(&manifest);
    let removals = undeploy_targets(&DeployExecution {
        project_root: &root,
        targets: &targets,
        selection: &selection,
        registry: &loaded.mechanisms,
        routes: &manifest.mechanism_routes,
        state_home: &state_home,
        project: &identity(&manifest),
        package: None,
        created_at: &now(),
    })?;
    render_removals(ctx, &selection, &removals)
}

/// `vibe deployments [--json]` — the machine's receipts, and nothing else.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
pub fn run_deployments(ctx: &output::Context) -> Result<()> {
    let rows = list_deployments(&state_home()?)?;
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

/// Render one read-only plan.
fn render_plan(
    ctx: &output::Context,
    selection: &DeploySelection,
    reports: &[DeployPlanReport],
) -> Result<()> {
    if ctx.is_json() {
        return ctx.emit_json(&serde_json::json!({
            "command": "deploy",
            "ok": true,
            "mode": "plan",
            "profile": selection.profile,
            "targets": reports
                .iter()
                .map(|report| serde_json::json!({
                    "target": report.target,
                    "mechanism": report.mechanism,
                    "provider": report.provider,
                    "via": report.via,
                    "displaced_default": report.displaced_default,
                    "planned": report.planned,
                    "reason": report.reason,
                    "summary": report.summary,
                    "resources": report
                        .resources
                        .iter()
                        .map(|resource| serde_json::json!({
                            "resource": resource.resource,
                            "desired_digest": resource.desired_digest,
                            "recorded_digest": resource.recorded_digest,
                            "change": resource.change,
                        }))
                        .collect::<Vec<_>>(),
                }))
                .collect::<Vec<_>>(),
        }));
    }
    ctx.heading(&format!("Deploy plan — profile `{}`", selection.profile));
    for report in reports {
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
    }
    let planned = reports.iter().filter(|report| report.planned).count();
    ctx.summary(&format!(
        "{planned} of {} target(s) would be deployed; nothing was read, built or changed",
        reports.len(),
    ));
    Ok(())
}

/// Render one inverse deployment.
fn render_removals(
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

/// A project that declares nothing deployable, said once.
fn report_nothing(ctx: &output::Context, mode: &str) -> Result<()> {
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

/// The selected node's manifest — ONE read, at the resolved root.
fn read_manifest(root: &std::path::Path) -> Result<Manifest> {
    Manifest::read(root.join(Manifest::FILENAME))
        .with_context(|| format!("reading `{}`", root.join(Manifest::FILENAME).display()))
}

/// The declared deploy targets, or none.
fn deploy_targets(manifest: &Manifest) -> Vec<vibe_core::manifest::DeployTarget> {
    manifest
        .deploy
        .as_ref()
        .map(|section| section.targets.clone())
        .unwrap_or_default()
}

/// The absolute deployment state home under this user's settings dir.
fn state_home() -> Result<std::path::PathBuf> {
    let settings = vibe_core::settings::settings_dir().ok_or_else(|| {
        anyhow::anyhow!(
            "the vibevm settings directory could not be resolved, so deployment receipts have \
             nowhere to live (violates \
             spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS; fix: set \
             `$VIBE_SETTINGS`, or make a home directory resolvable, then rerun)"
        )
    })?;
    Ok(deploy_state_home(&settings))
}

/// The selected node's identity — the same rendering the dispatch
/// assembles, so a receipt written by `vibe deploy` and one read by
/// `vibe undeploy` key under one name.
fn identity(manifest: &Manifest) -> String {
    if let Some(package) = &manifest.package {
        return format!("{}/{}", package.group, package.name);
    }
    if let Some(project) = &manifest.project {
        return match &project.group {
            Some(group) => format!("{group}/{}", project.name),
            None => project.name.clone(),
        };
    }
    "<workspace>".to_owned()
}

/// The invocation's RFC 3339 instant. The read-only surfaces stamp
/// nothing durable, but the executor's record vocabulary takes one, and a
/// surface is where a clock is read.
fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}
