//! `vibe skill` — project package-declared skills into coding agents
//! (PROP-018 §2.6). Standalone mode's only v1 functionality: no LLM, so it
//! works whether or not an agent is driving vibevm.
//!
//! The `vibe-mcp` library owns inventory, selection, target filtering and
//! projection. This module is only the CLI parse/confirm/render adapter.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-018#vibe-skill");

use std::io::IsTerminal;
use std::path::Path;

use anyhow::{Result, bail};
use dialoguer::Confirm;
use vibe_agent_projection::agents::Scope;
use vibe_agent_projection::pkgskill::{
    DeclaredSkillFilter, PackageSkillReport, collect_declared_skills,
    prepare_declared_skill_projection,
};
use vibe_core::machine_json_path;

use crate::cli::{SkillArgs, SkillInstallArgs, SkillListArgs, SkillSubcommand, SkillUninstallArgs};
use crate::output;

pub fn run(ctx: &output::Context, args: SkillArgs) -> Result<()> {
    match args.command {
        SkillSubcommand::List(sub) => run_list(ctx, sub),
        SkillSubcommand::Install(sub) => run_install(ctx, sub),
        SkillSubcommand::Uninstall(sub) => run_uninstall(ctx, sub),
    }
}

fn resolve_scope(scope: &Option<String>) -> Result<Scope> {
    match scope {
        Some(s) => Scope::parse(s),
        None => Ok(Scope::Project),
    }
}

fn render(ctx: &output::Context, r: &PackageSkillReport) {
    let note = r
        .note
        .as_deref()
        .map(|n| format!(" ({n})"))
        .unwrap_or_default();
    let path = r.path.as_deref().unwrap_or("(no skill loader)");
    ctx.step(&format!(
        "{} {} → {} ({}) {}{note}",
        r.status, r.skill, r.agent, r.scope, path
    ));
}

fn confirm_apply(ctx: &output::Context, yes: bool) -> Result<bool> {
    if yes || ctx.is_unattended() {
        return Ok(true);
    }
    if !std::io::stdin().is_terminal() {
        bail!(
            "no TTY available for confirmation; re-run with `--assume-yes` \
             to apply this plan non-interactively"
        );
    }
    Ok(Confirm::new()
        .with_prompt("Apply this skill plan?")
        .default(true)
        .interact()
        .unwrap_or(false))
}

fn emit_reports(
    ctx: &output::Context,
    command: &str,
    project_root: &Path,
    reports: &[PackageSkillReport],
) -> Result<()> {
    if ctx.is_json() {
        ctx.emit_json(&serde_json::json!({
            "ok": true,
            "command": command,
            "project": project_root.display().to_string(),
            "count": reports.len(),
            "results": reports,
        }))?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// list
// ---------------------------------------------------------------------------

fn run_list(ctx: &output::Context, args: SkillListArgs) -> Result<()> {
    let project_root = super::resolve_project_root(&args.path)?;
    let skills = collect_declared_skills(&project_root)?;

    if ctx.is_json() {
        let entries: Vec<serde_json::Value> = skills
            .iter()
            .map(|s| {
                serde_json::json!({
                    "name": s.decl.name,
                    "origin": s.origin,
                    "source": machine_json_path(&s.source),
                    "description": s.decl.description,
                    "agents": s.decl.agents,
                })
            })
            .collect();
        ctx.emit_json(&serde_json::json!({
            "ok": true,
            "command": "skill:list",
            "project": project_root.display().to_string(),
            "count": entries.len(),
            "skills": entries,
        }))?;
        return Ok(());
    }

    if skills.is_empty() {
        ctx.summary("(no skills declared by the project or installed packages)");
        return Ok(());
    }
    for s in &skills {
        let agents = if s.decl.agents.is_empty() {
            "all".to_string()
        } else {
            s.decl.agents.join(", ")
        };
        let desc = s
            .decl
            .description
            .as_deref()
            .map(|d| format!(" — {d}"))
            .unwrap_or_default();
        ctx.step(&format!(
            "{} [{}] → agents: {}{desc}",
            s.decl.name, s.origin, agents
        ));
    }
    ctx.summary(&format!("{} skill(s) declared.", skills.len()));
    Ok(())
}

// ---------------------------------------------------------------------------
// install
// ---------------------------------------------------------------------------

fn run_install(ctx: &output::Context, args: SkillInstallArgs) -> Result<()> {
    let project_root = super::resolve_project_root(&args.path)?;
    let scope = resolve_scope(&args.scope)?;
    let filter = DeclaredSkillFilter::new(&args.skills, args.agent.as_deref());
    let plan = prepare_declared_skill_projection(&project_root, &filter, scope)?;

    ctx.heading("Skill install plan:");
    let previews = plan.install(true)?;
    for r in &previews {
        render(ctx, r);
    }

    if args.dry_run {
        return emit_reports(ctx, "skill:install", &project_root, &previews);
    }
    if !confirm_apply(ctx, args.yes)? {
        ctx.summary("aborted.");
        return Ok(());
    }

    let results = plan.install(false)?;
    for r in &results {
        render(ctx, r);
    }
    ctx.summary(&format!("{} projection(s) processed.", results.len()));
    emit_reports(ctx, "skill:install", &project_root, &results)
}

// ---------------------------------------------------------------------------
// uninstall
// ---------------------------------------------------------------------------

fn run_uninstall(ctx: &output::Context, args: SkillUninstallArgs) -> Result<()> {
    let project_root = super::resolve_project_root(&args.path)?;
    let scope = resolve_scope(&args.scope)?;
    let filter = DeclaredSkillFilter::new(&args.skills, args.agent.as_deref());
    let plan = prepare_declared_skill_projection(&project_root, &filter, scope)?;

    ctx.heading("Skill uninstall plan:");
    let previews = plan.uninstall(true)?;
    for r in &previews {
        render(ctx, r);
    }

    if args.dry_run {
        return emit_reports(ctx, "skill:uninstall", &project_root, &previews);
    }
    if !confirm_apply(ctx, args.yes)? {
        ctx.summary("aborted.");
        return Ok(());
    }

    let results = plan.uninstall(false)?;
    for r in &results {
        render(ctx, r);
    }
    ctx.summary(&format!("{} removal(s) processed.", results.len()));
    emit_reports(ctx, "skill:uninstall", &project_root, &results)
}
