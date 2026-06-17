//! `vibe skill` — project package-declared skills into coding agents
//! (PROP-018 §2.6). Standalone mode's only v1 functionality: no LLM, so it
//! works whether or not an agent is driving vibevm.
//!
//! Skills are enumerated from two sources: the project's own workspace
//! nodes and every installed package's `vibedeps/` slot manifest. Each
//! declared `[[skill]]` is projected into the target agents' skill
//! directories via the `vibe-mcp` writer, reusing the PROP-015 agent
//! machinery. The library does the per-(agent, scope) work; this module is
//! the CLI dispatch, enumeration, plan rendering, and confirm UX.

specmark::scope!("spec://vibevm/common/PROP-018#vibe-skill");

use std::io::IsTerminal;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use dialoguer::Confirm;
use vibe_core::manifest::{Lockfile, Manifest, SkillDecl};
use vibe_mcp::agents::{Agent, Scope};
use vibe_mcp::pkgskill::{PackageSkillReport, install_package_skill, uninstall_package_skill};
use vibe_workspace::Workspace;

use crate::cli::{SkillArgs, SkillInstallArgs, SkillListArgs, SkillSubcommand, SkillUninstallArgs};
use crate::output;

pub fn run(ctx: &output::Context, args: SkillArgs) -> Result<()> {
    match args.command {
        SkillSubcommand::List(sub) => run_list(ctx, sub),
        SkillSubcommand::Install(sub) => run_install(ctx, sub),
        SkillSubcommand::Uninstall(sub) => run_uninstall(ctx, sub),
    }
}

/// A `[[skill]]` declaration paired with the absolute body path it
/// resolves to and a human origin label.
struct DeclaredSkill {
    decl: SkillDecl,
    /// Absolute path to the skill body (`base.join(decl.path)`).
    source: PathBuf,
    /// `"project"` / a member rel-path for a workspace node, or
    /// `"<kind>:<name>"` for an installed package.
    origin: String,
}

/// Collect every declared skill reachable from `project_root`: the
/// project's own nodes (root + workspace members) plus every installed
/// package's `vibedeps/` slot manifest (PROP-018 §2.6).
fn collect_skills(project_root: &Path) -> Result<Vec<DeclaredSkill>> {
    let ws = Workspace::discover(project_root)
        .with_context(|| format!("loading workspace at `{}`", project_root.display()))?;
    let mut out: Vec<DeclaredSkill> = Vec::new();

    // (a) the project's own nodes — root + workspace members.
    for (rel, manifest) in ws.iter_nodes() {
        let base = ws.node_abs_path(rel);
        let origin = if rel == "." {
            "project".to_string()
        } else {
            rel.to_string()
        };
        for decl in &manifest.skills {
            out.push(DeclaredSkill {
                source: base.join(&decl.path),
                decl: decl.clone(),
                origin: origin.clone(),
            });
        }
    }

    // (b) installed packages — read each lockfile entry's slot manifest.
    let lock_path = ws.lockfile_path();
    if lock_path.exists() {
        let lockfile = Lockfile::read(&lock_path)
            .with_context(|| format!("reading lockfile `{}`", lock_path.display()))?;
        for pkg in &lockfile.packages {
            let slot = ws.vibedeps_slot(pkg.kind, &pkg.name, &pkg.version);
            let manifest_path = slot.join(Manifest::FILENAME);
            if !manifest_path.exists() {
                continue;
            }
            // A malformed dependency manifest never blocks skill listing —
            // skip it rather than aborting the whole command.
            let Ok(manifest) = Manifest::read(&manifest_path) else {
                continue;
            };
            let origin = format!("{}:{}", pkg.kind.as_str(), pkg.name);
            for decl in &manifest.skills {
                out.push(DeclaredSkill {
                    source: slot.join(&decl.path),
                    decl: decl.clone(),
                    origin: origin.clone(),
                });
            }
        }
    }
    Ok(out)
}

/// Resolve the project root, requiring a `vibe.toml` (skills are always
/// enumerated from the project, even for a user-scope projection).
fn resolve_project_root(path: &Path) -> Result<PathBuf> {
    let canonical = path
        .canonicalize()
        .with_context(|| format!("canonicalizing `{}`", path.display()))?;
    let stripped = super::init::strip_unc_public(canonical);
    if !stripped.join(Manifest::FILENAME).exists() {
        bail!(
            "no `vibe.toml` in `{}`; run `vibe init` first",
            stripped.display()
        );
    }
    Ok(stripped)
}

/// The skill-supporting agents named by `--agent` (default: all). A
/// skill-unsupported agent passed explicitly stays in the list and is
/// reported `skipped` by the writer, so the operator sees why.
fn target_agents(filter: &Option<String>) -> Result<Vec<Agent>> {
    let base = match filter {
        Some(f) => Agent::parse_filter(f)?,
        None => Agent::ALL.to_vec(),
    };
    Ok(base)
}

fn resolve_scope(scope: &Option<String>) -> Result<Scope> {
    match scope {
        Some(s) => Scope::parse(s),
        None => Ok(Scope::Project),
    }
}

/// The agents a single skill projects into: the CLI target set, narrowed
/// by the skill's own `agents` filter when it declares one.
fn skill_agents(decl: &SkillDecl, cli_targets: &[Agent]) -> Vec<Agent> {
    if decl.agents.is_empty() {
        return cli_targets.to_vec();
    }
    cli_targets
        .iter()
        .copied()
        .filter(|a| {
            decl.agents.iter().any(|name| {
                Agent::parse_filter(name)
                    .map(|v| v.contains(a))
                    .unwrap_or(false)
            })
        })
        .collect()
}

/// Filter the collected skills by the `--skill <name>` selection (empty =
/// all), returning an error when the selection matches nothing.
fn select<'a>(all: &'a [DeclaredSkill], names: &[String]) -> Result<Vec<&'a DeclaredSkill>> {
    let selected: Vec<&DeclaredSkill> = all
        .iter()
        .filter(|s| names.is_empty() || names.iter().any(|n| n == &s.decl.name))
        .collect();
    if selected.is_empty() {
        bail!("no matching skills (run `vibe skill list` to see what is declared)");
    }
    Ok(selected)
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
    let project_root = resolve_project_root(&args.path)?;
    let skills = collect_skills(&project_root)?;

    if ctx.is_json() {
        let entries: Vec<serde_json::Value> = skills
            .iter()
            .map(|s| {
                serde_json::json!({
                    "name": s.decl.name,
                    "origin": s.origin,
                    "source": s.source.display().to_string().replace('\\', "/"),
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
    let project_root = resolve_project_root(&args.path)?;
    let scope = resolve_scope(&args.scope)?;
    let cli_targets = target_agents(&args.agent)?;
    let all = collect_skills(&project_root)?;
    let selected = select(&all, &args.skills)?;
    let scopes = scope.expand();

    // (agent, scope, skill name, body source) — each independent.
    let mut tasks: Vec<(Agent, Scope, String, PathBuf)> = Vec::new();
    for s in &selected {
        for a in skill_agents(&s.decl, &cli_targets) {
            for sc in &scopes {
                tasks.push((a, *sc, s.decl.name.clone(), s.source.clone()));
            }
        }
    }

    ctx.heading("Skill install plan:");
    let mut previews = Vec::with_capacity(tasks.len());
    for (a, sc, name, src) in &tasks {
        let r = install_package_skill(*a, *sc, Some(&project_root), name, src, true)?;
        render(ctx, &r);
        previews.push(r);
    }

    if args.dry_run {
        return emit_reports(ctx, "skill:install", &project_root, &previews);
    }
    if !confirm_apply(ctx, args.yes)? {
        ctx.summary("aborted.");
        return Ok(());
    }

    let mut results = Vec::with_capacity(tasks.len());
    for (a, sc, name, src) in &tasks {
        results.push(install_package_skill(
            *a,
            *sc,
            Some(&project_root),
            name,
            src,
            false,
        )?);
    }
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
    let project_root = resolve_project_root(&args.path)?;
    let scope = resolve_scope(&args.scope)?;
    let cli_targets = target_agents(&args.agent)?;
    let all = collect_skills(&project_root)?;
    let selected = select(&all, &args.skills)?;
    let scopes = scope.expand();

    let mut tasks: Vec<(Agent, Scope, String)> = Vec::new();
    for s in &selected {
        for a in skill_agents(&s.decl, &cli_targets) {
            for sc in &scopes {
                tasks.push((a, *sc, s.decl.name.clone()));
            }
        }
    }

    ctx.heading("Skill uninstall plan:");
    let mut previews = Vec::with_capacity(tasks.len());
    for (a, sc, name) in &tasks {
        let r = uninstall_package_skill(*a, *sc, Some(&project_root), name, true)?;
        render(ctx, &r);
        previews.push(r);
    }

    if args.dry_run {
        return emit_reports(ctx, "skill:uninstall", &project_root, &previews);
    }
    if !confirm_apply(ctx, args.yes)? {
        ctx.summary("aborted.");
        return Ok(());
    }

    let mut results = Vec::with_capacity(tasks.len());
    for (a, sc, name) in &tasks {
        results.push(uninstall_package_skill(
            *a,
            *sc,
            Some(&project_root),
            name,
            false,
        )?);
    }
    for r in &results {
        render(ctx, r);
    }
    ctx.summary(&format!("{} removal(s) processed.", results.len()));
    emit_reports(ctx, "skill:uninstall", &project_root, &results)
}
