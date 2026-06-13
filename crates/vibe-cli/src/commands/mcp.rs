//! `vibe mcp` — Model Context Protocol surface.
//!
//! Spec: PROP-004 §5.1 + ROADMAP §M1.7. Subcommands today (slice 5):
//!
//! - `vibe mcp serve` — run the JSON-RPC server over stdio.
//! - `vibe mcp install` — detect coding agents and write per-agent
//!   MCP config + optional SKILL.md. Wizard-driven when invoked
//!   without flags; fully scriptable with `--auto` / `--scope` /
//!   `--what` / `--agent`.
//! - `vibe mcp status` — show what `install` would write, no writes.
//!
//! Library implementation lives in `vibe-mcp`; this module is the CLI
//! dispatch + per-agent config writers.
//!
//! ## Scope axis (slice 5)
//!
//! Every install touches one or two physical files per agent:
//! - **Project scope** writes to `<project>/<agent-config-rel>` —
//!   committed to git, every clone gets the same setup.
//! - **User scope** writes to `<home>/<agent-config-rel>` — global,
//!   works in every directory (the MCP server entry omits `--path`
//!   so the server resolves CWD per invocation).
//! - **Both** writes to project AND user simultaneously, falling
//!   into a single user-level entry for the two agents that have no
//!   project surface (Claude Desktop, Codex).
//!
//! ## Agent matrix (slice 5)
//!
//! | Agent          | section       | format | project file              | user file                                              |
//! |----------------|---------------|--------|---------------------------|--------------------------------------------------------|
//! | Claude Code    | `mcpServers`  | JSON   | `.claude/settings.json`   | `~/.claude/settings.json`                              |
//! | Claude Desktop | `mcpServers`  | JSON   | (n/a — user-only)         | `<config-dir>/Claude/claude_desktop_config.json`       |
//! | Cursor         | `mcpServers`  | JSON   | `.cursor/mcp.json`        | `~/.cursor/mcp.json`                                   |
//! | OpenCode       | `mcp`         | JSON   | `opencode.json`           | `<config-dir>/opencode/opencode.json`                  |
//! | Codex          | `mcp_servers` | TOML   | (n/a — user-only)         | `~/.codex/config.toml`                                 |
//!
//! `<config-dir>` resolves through `dirs::config_dir()` — `%APPDATA%`
//! on Windows, `~/Library/Application Support` on macOS, `~/.config`
//! on Linux. `<home>` is `dirs::home_dir()`.

use std::fs;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use dialoguer::Confirm;
use serde::Serialize;
use vibe_core::manifest::Manifest;
use vibe_mcp::agent_config::{
    merge_json, merge_toml, read_json, read_toml, strip_json_entry, strip_toml_entry,
};
use vibe_mcp::agents::{Agent, ConfigFormat, ConfigPayload, Scope, What, detect_agents};
use vibe_mcp::install::{AgentInstallReport, SkillInstallReport, install_skill};
use vibe_mcp::{Server, ServerContext};

use crate::cli::{
    McpArgs, McpInstallArgs, McpServeArgs, McpStatusArgs, McpSubcommand, McpUninstallArgs,
    McpUpgradeArgs,
};
use crate::exit_code::InstallError;
use crate::output;

/// The config-entry key vibevm writes under each agent's MCP section.
const SERVER_NAME: &str = "vibevm";

/// Centralised TTY probe for the install UX gates. Pulled out so the
/// interactive helpers don't each grow their own `IsTerminal` import.
fn stdin_is_tty() -> bool {
    std::io::stdin().is_terminal()
}

pub fn run(ctx: &output::Context, args: McpArgs) -> Result<()> {
    match args.command {
        McpSubcommand::Serve(sub) => run_serve(sub),
        McpSubcommand::Install(sub) => run_install(ctx, sub),
        McpSubcommand::Status(sub) => run_status(ctx, sub),
        McpSubcommand::Upgrade(sub) => run_upgrade(ctx, sub),
        McpSubcommand::Uninstall(sub) => run_uninstall(ctx, sub),
    }
}

fn run_serve(args: McpServeArgs) -> Result<()> {
    // `vibe mcp serve` is the one place where path is *required*: the
    // server needs a project root to load the lockfile from. When
    // launched by a user-scope MCP entry that omits `--path`, the
    // server uses CWD (default value `.`).
    let project_root = resolve_project_root_required(&args.path)?;
    let server_ctx = ServerContext::new(project_root);
    let mut server = Server::stdio(server_ctx);
    server.run().context("MCP server I/O error")?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Reporting
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
struct InstallReport {
    ok: bool,
    command: &'static str,
    project: Option<String>,
    detected: Vec<String>,
    targeted: Vec<String>,
    scope: &'static str,
    what: &'static str,
    results: Vec<AgentInstallReport>,
    skill_results: Vec<SkillInstallReport>,
    mode: &'static str,
    dry_run: bool,
}

// ---------------------------------------------------------------------------
// install
// ---------------------------------------------------------------------------

/// Determines which UX path drove the install.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InstallMode {
    Auto,
    Flags,
    Interactive,
}

impl InstallMode {
    fn as_str(self) -> &'static str {
        match self {
            InstallMode::Auto => "auto",
            InstallMode::Flags => "flags",
            InstallMode::Interactive => "interactive",
        }
    }
}

fn run_install(ctx: &output::Context, args: McpInstallArgs) -> Result<()> {
    // The mode is `auto` if --auto was passed; `flags` if any of
    // (--scope/--what/--agent) was passed without --auto and we don't
    // need to ask anything; `interactive` otherwise (asks via wizard).
    let any_explicit_target = args.agent.is_some() || args.scope.is_some() || args.what.is_some();
    let mode = if args.auto {
        InstallMode::Auto
    } else if any_explicit_target {
        // Mixed: some flags given, others may need wizard prompts.
        // We classify as `flags` only when EVERY needed dimension is
        // explicit — see the resolution logic below.
        InstallMode::Flags
    } else {
        InstallMode::Interactive
    };

    // Under `--unattended` (or `VIBE_UNATTENDED`), no wizard may
    // open. The operator is in a script and a hung dialoguer prompt
    // would deadlock CI. Detect missing dimensions early and bail
    // with a concrete, actionable hint rather than letting the
    // interactive branches below try to prompt.
    if ctx.is_unattended() && !args.auto {
        let mut missing: Vec<&'static str> = Vec::new();
        if args.scope.is_none() {
            missing.push("--scope");
        }
        if args.what.is_none() {
            missing.push("--what");
        }
        if args.agent.is_none() {
            missing.push("--agent");
        }
        if !missing.is_empty() {
            bail!(
                "unattended mode requires every wizard dimension to be explicit; missing: {}. \
                 Either supply the missing flag(s), or use `--auto` to detect every \
                 dimension automatically.",
                missing.join(", ")
            );
        }
    }

    // 1. Resolve scope.
    let scope = if let Some(s) = &args.scope {
        Scope::parse(s)?
    } else if args.auto {
        // Auto mode: project if vibe.toml in --path, else user.
        if has_vibe_toml(&args.path) {
            Scope::Project
        } else {
            Scope::User
        }
    } else {
        interactive_ask_scope(&args.path)?
    };

    // 2. Resolve project_root. Two policies, mirroring the model in
    //    `vibe mcp upgrade` / `vibe mcp uninstall`:
    //
    //    - `Scope::requires_vibe_toml()` (only `Project`) → bail when
    //      `vibe.toml` is missing. The operator was explicit and
    //      there is nothing to write into.
    //    - Otherwise (`User` or `Both`) → best-effort: read the
    //      project_root only if `vibe.toml` exists; leave it as
    //      `None` if not. The walker below skips the project-leg
    //      when project_root is None, so the user-leg of `Both`
    //      runs unattended even on a fresh machine. This is what
    //      makes `--scope both` usable from first-time-user
    //      provisioning scripts.
    let project_root: Option<PathBuf> = if scope.requires_vibe_toml() {
        Some(resolve_project_root_required(&args.path)?)
    } else {
        args.path
            .canonicalize()
            .ok()
            .map(super::init::strip_unc_public)
            .filter(|p| p.join(Manifest::FILENAME).exists())
    };

    // 3. Resolve what.
    let what = if let Some(w) = &args.what {
        What::parse(w)?
    } else if args.auto {
        What::Both
    } else {
        interactive_ask_what()?
    };

    // 4. Resolve agents.
    let detected = detect_agents(project_root.as_deref());
    let targeted: Vec<Agent> = if args.auto {
        detected.clone()
    } else if let Some(filter) = &args.agent {
        let parsed = Agent::parse_filter(filter)?;
        parsed
            .into_iter()
            .filter(|a| args.force || detected.contains(a))
            .collect()
    } else {
        interactive_select_agents(&detected, args.force)?
    };

    if targeted.is_empty() && !ctx.is_json() {
        ctx.summary(
            "no supported agents detected; pass `--agent <name>` or `--force` to install anyway",
        );
        return Ok(());
    }

    // 5. Walk: for each agent × each concrete scope under `scope`, do
    //    the install (or skip when the agent has no surface for that
    //    scope, or when `Both` was selected without a `vibe.toml`
    //    making the project-leg unreachable).
    let project_leg_skipped_no_manifest = scope == Scope::Both && project_root.is_none();

    // Two-pass walk so the operator's `--yes` / `--unattended` /
    // `--auto` / `--json` / `--dry-run` flags actually gate a
    // confirmation prompt (PROP-002 §2.3.1 hint about destructive
    // operations). First pass is always dry-run — gathers the
    // would-do/won't-do list without touching any config files.
    // Second pass writes only when (a) the operator approved AND
    // (b) the original invocation wasn't `--dry-run`.
    let (preview_results, preview_skill) = walk_install(
        &targeted,
        scope,
        project_root.as_deref(),
        what,
        args.force,
        true,
    )?;

    let needs_change = preview_results
        .iter()
        .any(|r| matches!(r.status, "would-create" | "would-update"))
        || preview_skill
            .iter()
            .any(|r| matches!(r.status, "would-create" | "would-update"));

    if !args.dry_run && needs_change {
        // Confirmation gating: skip the prompt when the operator
        // already signalled "go" via flag / env, OR when we are
        // not attached to a TTY (CI / opencode harness — the
        // pre-this-commit behaviour with no confirm at all is the
        // baseline; we never break those scripts). Show the
        // interactive prompt only on a real TTY without an
        // explicit skip-flag.
        let approved = if args.yes
            || ctx.is_unattended()
            || args.auto
            || ctx.is_json()
            || !console::user_attended()
        {
            true
        } else {
            print_install_results(ctx, true, &preview_results, &preview_skill);
            let mcp_count = preview_results
                .iter()
                .filter(|r| matches!(r.status, "would-create" | "would-update"))
                .count();
            let skill_count = preview_skill
                .iter()
                .filter(|r| matches!(r.status, "would-create" | "would-update"))
                .count();
            Confirm::new()
                .with_prompt(format!(
                    "Apply this plan? ({mcp_count} MCP entr{}, {skill_count} SKILL.md file{})",
                    if mcp_count == 1 { "y" } else { "ies" },
                    if skill_count == 1 { "" } else { "s" },
                ))
                .default(false)
                .interact()
                .context("reading user confirmation")?
        };
        if !approved {
            return Err(InstallError::UserDeclined.into());
        }
    }

    let (results, skill_results) = if args.dry_run || !needs_change {
        (preview_results, preview_skill)
    } else {
        walk_install(
            &targeted,
            scope,
            project_root.as_deref(),
            what,
            args.force,
            false,
        )?
    };

    let report = InstallReport {
        ok: true,
        command: "mcp:install",
        project: project_root.as_ref().map(|p| p.display().to_string()),
        detected: detected.iter().map(|a| a.as_str().to_string()).collect(),
        targeted: targeted.iter().map(|a| a.as_str().to_string()).collect(),
        scope: scope.as_str(),
        what: what.as_str(),
        results: results.clone(),
        skill_results: skill_results.clone(),
        mode: mode.as_str(),
        dry_run: args.dry_run,
    };

    if ctx.is_json() {
        ctx.emit_json(&report)?;
        return Ok(());
    }
    if ctx.is_quiet() {
        let mcp_written = results
            .iter()
            .filter(|r| matches!(r.status, "created" | "updated"))
            .count();
        let skill_written = skill_results
            .iter()
            .filter(|r| matches!(r.status, "created" | "updated"))
            .count();
        let verb = if args.dry_run { "previewed" } else { "written" };
        ctx.summary(&format!(
            "vibe mcp install: scope={} what={} — {mcp_written} MCP + {skill_written} skill {verb}",
            scope.as_str(),
            what.as_str()
        ));
        return Ok(());
    }
    print_install_results(ctx, args.dry_run, &results, &skill_results);
    if project_leg_skipped_no_manifest {
        ctx.step(&format!(
            "note: --scope both was requested but `{}` carries no `vibe.toml`; \
             project-scope leg skipped, only user-level installs were written. \
             Run `vibe init` here first if you want both legs.",
            args.path.display()
        ));
    }
    Ok(())
}

fn print_install_results(
    ctx: &output::Context,
    dry_run: bool,
    results: &[AgentInstallReport],
    skill_results: &[SkillInstallReport],
) {
    for r in results {
        let prefix = if dry_run { "would" } else { r.status };
        let note = r
            .note
            .as_deref()
            .map(|n| format!(" ({n})"))
            .unwrap_or_default();
        let target = if r.config_path.is_empty() {
            "(no surface)".to_string()
        } else {
            r.config_path.clone()
        };
        ctx.step(&format!(
            "{} mcp     {} ({}) → {}{note}",
            prefix, r.agent, r.scope, target
        ));
    }
    for r in skill_results {
        let prefix = if dry_run { "would" } else { r.status };
        let note = r
            .note
            .as_deref()
            .map(|n| format!(" ({n})"))
            .unwrap_or_default();
        let path_str = r.path.as_deref().unwrap_or("(no skill loader)");
        ctx.step(&format!(
            "{} skill   {} ({}) → {}{note}",
            prefix, r.agent, r.scope, path_str
        ));
    }
}

/// Per-(agent × scope) install walker. Extracted from `run_install`
/// so the two-pass `confirm-then-apply` flow can call it twice —
/// once with `dry_run = true` to gather the would-do plan, then
/// (after the operator approves) once with `dry_run = false` to
/// actually write. The semantics inside the loop are unchanged
/// from the prior single-pass implementation; only the surrounding
/// state lives in `run_install` now.
fn walk_install(
    targeted: &[Agent],
    scope: Scope,
    project_root: Option<&Path>,
    what: What,
    _force: bool,
    dry_run: bool,
) -> Result<(Vec<AgentInstallReport>, Vec<SkillInstallReport>)> {
    let mut results: Vec<AgentInstallReport> = Vec::new();
    let mut skill_results: Vec<SkillInstallReport> = Vec::new();
    for agent in targeted {
        for concrete_scope in scope.expand() {
            // `Both` without `vibe.toml`: the user-leg runs as
            // normal, the project-leg is silently skipped.
            if concrete_scope == Scope::Project && project_root.is_none() {
                continue;
            }
            // ---- MCP entry ----
            if what.includes_mcp() {
                let path = agent.config_path(concrete_scope, project_root)?;
                if let Some(path) = path {
                    let payload = agent.build_mcp_entry(concrete_scope, project_root);
                    let outcome = if dry_run {
                        preview_install_mcp(*agent, concrete_scope, &path, &payload)?
                    } else {
                        apply_install_mcp(*agent, concrete_scope, &path, &payload)?
                    };
                    results.push(outcome);
                } else if scope == Scope::Both {
                    results.push(AgentInstallReport {
                        agent: agent.as_str().to_string(),
                        scope: concrete_scope.as_str(),
                        config_path: String::new(),
                        status: "skipped",
                        note: Some(format!(
                            "agent `{}` has no {}-scope MCP config",
                            agent.as_str(),
                            concrete_scope.as_str()
                        )),
                    });
                }
            }
            // ---- SKILL.md ----
            if what.includes_skill() {
                let outcome = install_skill(*agent, concrete_scope, project_root, dry_run)?;
                skill_results.push(outcome);
            }
        }
    }
    Ok((results, skill_results))
}

// ---------------------------------------------------------------------------
// status
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
struct StatusReport {
    ok: bool,
    command: &'static str,
    project: Option<String>,
    detected: Vec<String>,
    /// MCP-config preview entries, one per (agent × concrete-scope)
    /// combination that has a surface.
    results: Vec<AgentInstallReport>,
    /// SKILL.md drift preview entries — same shape as install /
    /// upgrade. Empty for agents without filesystem skill loaders
    /// (Cursor, Claude Desktop). Status is `would-create` /
    /// `would-update` / `unchanged`.
    skill_results: Vec<SkillInstallReport>,
}

fn run_status(ctx: &output::Context, args: McpStatusArgs) -> Result<()> {
    // Status is read-only and scope-agnostic: report on every agent ×
    // every scope that has a surface. Project entries require
    // resolved project_root; user entries don't.
    let project_root: Option<PathBuf> = args
        .path
        .canonicalize()
        .ok()
        .map(super::init::strip_unc_public)
        .filter(|p| p.join(Manifest::FILENAME).exists());
    let detected = detect_agents(project_root.as_deref());
    let mut results: Vec<AgentInstallReport> = Vec::new();
    let mut skill_results: Vec<SkillInstallReport> = Vec::new();
    for agent in Agent::ALL.iter().copied() {
        for scope in [Scope::Project, Scope::User] {
            if scope == Scope::Project && project_root.is_none() {
                continue;
            }
            // MCP-config preview.
            if let Some(path) = agent.config_path(scope, project_root.as_deref())? {
                let payload = agent.build_mcp_entry(scope, project_root.as_deref());
                results.push(preview_install_mcp(agent, scope, &path, &payload)?);
            }
            // Skill preview — only for agents that load skills + have
            // a path for this scope. install_skill with dry_run=true
            // reuses the decide-then-(don't-)apply logic and emits
            // would-create / would-update / unchanged.
            if agent.supports_skill() && agent.skill_path(scope, project_root.as_deref())?.is_some()
            {
                let outcome = install_skill(agent, scope, project_root.as_deref(), true)?;
                skill_results.push(outcome);
            }
        }
    }
    let report = StatusReport {
        ok: true,
        command: "mcp:status",
        project: project_root.as_ref().map(|p| p.display().to_string()),
        detected: detected.iter().map(|a| a.as_str().to_string()).collect(),
        results: results.clone(),
        skill_results: skill_results.clone(),
    };
    if ctx.is_json() {
        ctx.emit_json(&report)?;
        return Ok(());
    }
    ctx.summary(&format!(
        "Detected agents: {}",
        if detected.is_empty() {
            "(none)".to_string()
        } else {
            detected
                .iter()
                .map(|a| a.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        }
    ));
    for r in &results {
        let note = r
            .note
            .as_deref()
            .map(|n| format!(" ({n})"))
            .unwrap_or_default();
        ctx.step(&format!(
            "{} mcp     {} ({}) → {}{note}",
            r.status, r.agent, r.scope, r.config_path
        ));
    }
    for r in &skill_results {
        let note = r
            .note
            .as_deref()
            .map(|n| format!(" ({n})"))
            .unwrap_or_default();
        let path_str = r.path.as_deref().unwrap_or("(no skill loader)");
        ctx.step(&format!(
            "{} skill   {} ({}) → {}{note}",
            r.status, r.agent, r.scope, path_str
        ));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// upgrade — scan known places + refresh stale to current template
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
struct UpgradeReport {
    ok: bool,
    command: &'static str,
    project: Option<String>,
    scope: &'static str,
    what: &'static str,
    /// Per-(agent, scope) MCP-config check + refresh outcomes.
    /// Includes `not-installed` rows for places we scanned but found
    /// no vibevm-entry (upgrade does not create new installations).
    results: Vec<AgentInstallReport>,
    /// Per-(agent, scope) skill check + refresh outcomes.
    skill_results: Vec<SkillInstallReport>,
    dry_run: bool,
}

fn run_upgrade(ctx: &output::Context, args: McpUpgradeArgs) -> Result<()> {
    let scope = if let Some(s) = &args.scope {
        Scope::parse(s)?
    } else {
        Scope::Both
    };
    // What to scan — same axes as install. config-only / skill-only
    // map onto Mcp / Skill; default is Both.
    let what = if args.config_only {
        What::Mcp
    } else if args.skill_only {
        What::Skill
    } else {
        What::Both
    };

    let project_root: Option<PathBuf> = args
        .path
        .canonicalize()
        .ok()
        .map(super::init::strip_unc_public)
        .filter(|p| p.join(Manifest::FILENAME).exists());

    // If scope == Project but project_root is None — error fast.
    if scope == Scope::Project && project_root.is_none() {
        bail!(
            "no `vibe.toml` in `{}`; upgrade with --scope project requires a project. \
             Pass `--scope user` to refresh user-level installs only.",
            args.path.display()
        );
    }

    let agents: Vec<Agent> = if let Some(filter) = &args.agent {
        Agent::parse_filter(filter)?
    } else {
        Agent::ALL.to_vec()
    };

    // Two-pass walk + apply-confirm. Same shape as run_install
    // (above) — preview first, ask confirm, then real apply only
    // when the operator approves AND the original invocation is
    // not `--dry-run`.
    let (preview_results, preview_skill) =
        walk_upgrade(&agents, scope, project_root.as_deref(), what, true)?;
    let needs_change = preview_results.iter().any(|r| r.status == "would-update")
        || preview_skill.iter().any(|r| r.status == "would-update");

    if !args.dry_run && needs_change {
        // Confirm only on real TTY without a skip-flag (see the
        // matching block in `run_install` for the rationale —
        // non-TTY scripts pre-date the confirm prompt and must
        // continue to work without `--yes`).
        let approved =
            if args.yes || ctx.is_unattended() || ctx.is_json() || !console::user_attended() {
                true
            } else {
                print_upgrade_results(ctx, true, &preview_results, &preview_skill);
                let stale = preview_results
                    .iter()
                    .filter(|r| r.status == "would-update")
                    .count()
                    + preview_skill
                        .iter()
                        .filter(|r| r.status == "would-update")
                        .count();
                Confirm::new()
                    .with_prompt(format!(
                        "Refresh {stale} stale entr{}?",
                        if stale == 1 { "y" } else { "ies" }
                    ))
                    .default(false)
                    .interact()
                    .context("reading user confirmation")?
            };
        if !approved {
            return Err(InstallError::UserDeclined.into());
        }
    }

    let (results, skill_results) = if args.dry_run || !needs_change {
        (preview_results, preview_skill)
    } else {
        walk_upgrade(&agents, scope, project_root.as_deref(), what, false)?
    };

    let report = UpgradeReport {
        ok: true,
        command: "mcp:upgrade",
        project: project_root.as_ref().map(|p| p.display().to_string()),
        scope: scope.as_str(),
        what: what.as_str(),
        results: results.clone(),
        skill_results: skill_results.clone(),
        dry_run: args.dry_run,
    };

    if ctx.is_json() {
        ctx.emit_json(&report)?;
        return Ok(());
    }
    if ctx.is_quiet() {
        let stale = results
            .iter()
            .filter(|r| matches!(r.status, "would-update" | "updated"))
            .count()
            + skill_results
                .iter()
                .filter(|r| matches!(r.status, "would-update" | "updated"))
                .count();
        let verb = if args.dry_run {
            "previewed"
        } else {
            "refreshed"
        };
        ctx.summary(&format!(
            "vibe mcp upgrade: {stale} stale entr{} {verb}",
            if stale == 1 { "y" } else { "ies" }
        ));
        return Ok(());
    }

    print_upgrade_results(ctx, args.dry_run, &results, &skill_results);
    Ok(())
}

/// Per-(agent × scope) upgrade walker. Same role as `walk_install`
/// for the install path: invoked twice from `run_upgrade` (once
/// dry-run for the plan, once real for the apply) so the operator's
/// `--yes` / `--unattended` / `--auto` / `--json` actually gate a
/// confirmation prompt.
fn walk_upgrade(
    agents: &[Agent],
    scope: Scope,
    project_root: Option<&Path>,
    what: What,
    dry_run: bool,
) -> Result<(Vec<AgentInstallReport>, Vec<SkillInstallReport>)> {
    let mut results: Vec<AgentInstallReport> = Vec::new();
    let mut skill_results: Vec<SkillInstallReport> = Vec::new();
    for agent in agents {
        for concrete_scope in scope.expand() {
            if concrete_scope == Scope::Project && project_root.is_none() {
                continue;
            }
            if what.includes_mcp() {
                let path = agent.config_path(concrete_scope, project_root)?;
                if let Some(path) = path {
                    let outcome =
                        upgrade_mcp_entry(*agent, concrete_scope, &path, project_root, dry_run)?;
                    results.push(outcome);
                }
            }
            if what.includes_skill() {
                let outcome = upgrade_skill(*agent, concrete_scope, project_root, dry_run)?;
                if let Some(o) = outcome {
                    skill_results.push(o);
                }
            }
        }
    }
    Ok((results, skill_results))
}

/// One-place upgrade probe + apply for an MCP-config block.
fn upgrade_mcp_entry(
    agent: Agent,
    scope: Scope,
    config_path: &Path,
    project_root: Option<&Path>,
    dry_run: bool,
) -> Result<AgentInstallReport> {
    // If the config file does not exist OR the vibevm block is absent,
    // upgrade is a no-op. We report `not-installed` rather than
    // creating it (that would be `install`'s job).
    let payload = agent.build_mcp_entry(scope, project_root);
    if !config_path.exists() {
        return Ok(AgentInstallReport {
            agent: agent.as_str().to_string(),
            scope: scope.as_str(),
            config_path: config_path.display().to_string().replace('\\', "/"),
            status: "not-installed",
            note: Some("config file does not exist; use `vibe mcp install` to create".into()),
        });
    }
    let section = agent.mcp_section_key();
    let has_vibevm_block = match (&payload, agent.config_format()) {
        (ConfigPayload::Json(_), ConfigFormat::Json) => read_json(config_path)?
            .get(section)
            .and_then(|v| v.get(SERVER_NAME))
            .is_some(),
        (ConfigPayload::Toml(_), ConfigFormat::Toml) => read_toml(config_path)?
            .get(section)
            .and_then(|v| v.as_table())
            .and_then(|t| t.get(SERVER_NAME))
            .is_some(),
        _ => bail!(
            "internal: agent `{}` config_format/payload mismatch",
            agent.as_str()
        ),
    };
    if !has_vibevm_block {
        return Ok(AgentInstallReport {
            agent: agent.as_str().to_string(),
            scope: scope.as_str(),
            config_path: config_path.display().to_string().replace('\\', "/"),
            status: "not-installed",
            note: Some(format!(
                "no `{SERVER_NAME}` entry in {section}; use `vibe mcp install` to create"
            )),
        });
    }
    // vibevm block present — fall through to the install-time
    // diff/apply. `decide_action` returns `unchanged` / `updated`,
    // never `created` here (we just confirmed the file exists, but
    // the block-level absent case maps to `updated` from
    // decide_action which we explicitly handle above).
    if dry_run {
        preview_install_mcp(agent, scope, config_path, &payload)
    } else {
        apply_install_mcp(agent, scope, config_path, &payload)
    }
}

/// One-place upgrade probe + apply for a SKILL.md file.
fn upgrade_skill(
    agent: Agent,
    scope: Scope,
    project_root: Option<&Path>,
    dry_run: bool,
) -> Result<Option<SkillInstallReport>> {
    let Some(path) = agent.skill_path(scope, project_root)? else {
        // Agent does not load skills, or scope has no surface — skip
        // the row entirely (don't pollute the upgrade plan with rows
        // for agents that have no skill loader).
        return Ok(None);
    };
    if !path.exists() {
        return Ok(Some(SkillInstallReport {
            agent: agent.as_str().to_string(),
            scope: scope.as_str(),
            path: Some(path.display().to_string().replace('\\', "/")),
            status: "not-installed",
            note: Some("SKILL.md does not exist; use `vibe mcp install` to create".into()),
        }));
    }
    // Reuse install_skill — it already has the decide-then-apply
    // logic and returns `unchanged` / `updated` for existing files.
    let outcome = install_skill(agent, scope, project_root, dry_run)?;
    Ok(Some(outcome))
}

fn print_upgrade_results(
    ctx: &output::Context,
    dry_run: bool,
    results: &[AgentInstallReport],
    skill_results: &[SkillInstallReport],
) {
    for r in results {
        let prefix = match r.status {
            "unchanged" => "✓",
            "would-update" | "updated" => {
                if dry_run {
                    "would"
                } else {
                    "updated"
                }
            }
            "not-installed" => "·",
            other => other,
        };
        let note = r
            .note
            .as_deref()
            .map(|n| format!(" ({n})"))
            .unwrap_or_default();
        ctx.step(&format!(
            "{} mcp     {} ({}) → {}{note}",
            prefix, r.agent, r.scope, r.config_path
        ));
    }
    for r in skill_results {
        let prefix = match r.status {
            "unchanged" => "✓",
            "would-update" | "updated" => {
                if dry_run {
                    "would"
                } else {
                    "updated"
                }
            }
            "not-installed" => "·",
            other => other,
        };
        let note = r
            .note
            .as_deref()
            .map(|n| format!(" ({n})"))
            .unwrap_or_default();
        let path_str = r.path.as_deref().unwrap_or("(no skill loader)");
        ctx.step(&format!(
            "{} skill   {} ({}) → {}{note}",
            prefix, r.agent, r.scope, path_str
        ));
    }
}

// ---------------------------------------------------------------------------
// uninstall — scan + remove vibevm entries / SKILL.md
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
struct UninstallReport {
    ok: bool,
    command: &'static str,
    project: Option<String>,
    scope: &'static str,
    what: &'static str,
    /// Per-(agent, scope) MCP-config removal outcomes. Status:
    /// `removed`, `would-remove`, `not-installed`, `skipped`.
    results: Vec<AgentInstallReport>,
    /// Per-(agent, scope) skill removal outcomes. Same status set.
    skill_results: Vec<SkillInstallReport>,
    dry_run: bool,
}

fn run_uninstall(ctx: &output::Context, args: McpUninstallArgs) -> Result<()> {
    let scope = if let Some(s) = &args.scope {
        Scope::parse(s)?
    } else {
        Scope::Both
    };
    let what = if args.config_only {
        What::Mcp
    } else if args.skill_only {
        What::Skill
    } else {
        What::Both
    };

    let project_root: Option<PathBuf> = args
        .path
        .canonicalize()
        .ok()
        .map(super::init::strip_unc_public)
        .filter(|p| p.join(Manifest::FILENAME).exists());

    if scope == Scope::Project && project_root.is_none() {
        bail!(
            "no `vibe.toml` in `{}`; uninstall with --scope project requires a project. \
             Pass `--scope user` to remove user-level installs only.",
            args.path.display()
        );
    }

    let agents: Vec<Agent> = if let Some(filter) = &args.agent {
        Agent::parse_filter(filter)?
    } else {
        Agent::ALL.to_vec()
    };

    // Two-pass walk + apply-confirm. Uninstall is the most
    // destructive of the three MCP commands (it deletes SKILL.md
    // files and drops the vibevm block from MCP configs), so the
    // confirm step here matters more than for install / upgrade.
    let (preview_results, preview_skill) =
        walk_uninstall(&agents, scope, project_root.as_deref(), what, true)?;
    let needs_change = preview_results.iter().any(|r| r.status == "would-remove")
        || preview_skill.iter().any(|r| r.status == "would-remove");

    if !args.dry_run && needs_change {
        // Same TTY-only confirm policy as install / upgrade.
        let approved =
            if args.yes || ctx.is_unattended() || ctx.is_json() || !console::user_attended() {
                true
            } else {
                print_uninstall_results(ctx, true, &preview_results, &preview_skill);
                let to_remove = preview_results
                    .iter()
                    .filter(|r| r.status == "would-remove")
                    .count()
                    + preview_skill
                        .iter()
                        .filter(|r| r.status == "would-remove")
                        .count();
                Confirm::new()
                    .with_prompt(format!(
                        "Remove {to_remove} entr{}?",
                        if to_remove == 1 { "y" } else { "ies" }
                    ))
                    .default(false)
                    .interact()
                    .context("reading user confirmation")?
            };
        if !approved {
            return Err(InstallError::UserDeclined.into());
        }
    }

    let (results, skill_results) = if args.dry_run || !needs_change {
        (preview_results, preview_skill)
    } else {
        walk_uninstall(&agents, scope, project_root.as_deref(), what, false)?
    };

    let report = UninstallReport {
        ok: true,
        command: "mcp:uninstall",
        project: project_root.as_ref().map(|p| p.display().to_string()),
        scope: scope.as_str(),
        what: what.as_str(),
        results: results.clone(),
        skill_results: skill_results.clone(),
        dry_run: args.dry_run,
    };

    if ctx.is_json() {
        ctx.emit_json(&report)?;
        return Ok(());
    }
    if ctx.is_quiet() {
        let removed = results
            .iter()
            .filter(|r| matches!(r.status, "would-remove" | "removed"))
            .count()
            + skill_results
                .iter()
                .filter(|r| matches!(r.status, "would-remove" | "removed"))
                .count();
        let verb = if args.dry_run { "previewed" } else { "removed" };
        ctx.summary(&format!(
            "vibe mcp uninstall: {removed} entr{} {verb}",
            if removed == 1 { "y" } else { "ies" }
        ));
        return Ok(());
    }

    print_uninstall_results(ctx, args.dry_run, &results, &skill_results);
    Ok(())
}

/// Per-(agent × scope) uninstall walker. Mirrors `walk_install` /
/// `walk_upgrade`: invoked twice from `run_uninstall`, once
/// dry-run, once apply.
fn walk_uninstall(
    agents: &[Agent],
    scope: Scope,
    project_root: Option<&Path>,
    what: What,
    dry_run: bool,
) -> Result<(Vec<AgentInstallReport>, Vec<SkillInstallReport>)> {
    let mut results: Vec<AgentInstallReport> = Vec::new();
    let mut skill_results: Vec<SkillInstallReport> = Vec::new();
    for agent in agents {
        for concrete_scope in scope.expand() {
            if concrete_scope == Scope::Project && project_root.is_none() {
                continue;
            }
            if what.includes_mcp() {
                let path = agent.config_path(concrete_scope, project_root)?;
                if let Some(path) = path {
                    let outcome = uninstall_mcp_entry(*agent, concrete_scope, &path, dry_run)?;
                    results.push(outcome);
                }
            }
            if what.includes_skill() {
                let outcome = uninstall_skill(*agent, concrete_scope, project_root, dry_run)?;
                if let Some(o) = outcome {
                    skill_results.push(o);
                }
            }
        }
    }
    Ok((results, skill_results))
}

/// Remove the `vibevm` block from an MCP-config file. Foreign keys
/// preserved; if the section becomes empty after removal, it stays as
/// `{}` / `[section]` rather than being deleted (we don't trim other
/// people's containers).
fn uninstall_mcp_entry(
    agent: Agent,
    scope: Scope,
    config_path: &Path,
    dry_run: bool,
) -> Result<AgentInstallReport> {
    if !config_path.exists() {
        return Ok(AgentInstallReport {
            agent: agent.as_str().to_string(),
            scope: scope.as_str(),
            config_path: config_path.display().to_string().replace('\\', "/"),
            status: "not-installed",
            note: Some("config file does not exist".into()),
        });
    }
    let section = agent.mcp_section_key();
    let has_block = match agent.config_format() {
        ConfigFormat::Json => read_json(config_path)?
            .get(section)
            .and_then(|v| v.get(SERVER_NAME))
            .is_some(),
        ConfigFormat::Toml => read_toml(config_path)?
            .get(section)
            .and_then(|v| v.as_table())
            .and_then(|t| t.get(SERVER_NAME))
            .is_some(),
    };
    if !has_block {
        return Ok(AgentInstallReport {
            agent: agent.as_str().to_string(),
            scope: scope.as_str(),
            config_path: config_path.display().to_string().replace('\\', "/"),
            status: "not-installed",
            note: Some(format!("no `{SERVER_NAME}` entry in {section}")),
        });
    }

    if dry_run {
        return Ok(AgentInstallReport {
            agent: agent.as_str().to_string(),
            scope: scope.as_str(),
            config_path: config_path.display().to_string().replace('\\', "/"),
            status: "would-remove",
            note: Some(format!("drop `{SERVER_NAME}` from {section}")),
        });
    }

    match agent.config_format() {
        ConfigFormat::Json => {
            let stripped = strip_json_entry(config_path, section, SERVER_NAME)?;
            let serialized = serde_json::to_string_pretty(&stripped)
                .with_context(|| "serializing stripped JSON config")?;
            fs::write(config_path, serialized + "\n")
                .with_context(|| format!("writing `{}`", config_path.display()))?;
        }
        ConfigFormat::Toml => {
            let stripped = strip_toml_entry(config_path, section, SERVER_NAME)?;
            let serialized = toml::to_string_pretty(&stripped)
                .with_context(|| "serializing stripped TOML config")?;
            fs::write(config_path, serialized)
                .with_context(|| format!("writing `{}`", config_path.display()))?;
        }
    }
    Ok(AgentInstallReport {
        agent: agent.as_str().to_string(),
        scope: scope.as_str(),
        config_path: config_path.display().to_string().replace('\\', "/"),
        status: "removed",
        note: Some(format!("dropped `{SERVER_NAME}` from {section}")),
    })
}

fn uninstall_skill(
    agent: Agent,
    scope: Scope,
    project_root: Option<&Path>,
    dry_run: bool,
) -> Result<Option<SkillInstallReport>> {
    let Some(path) = agent.skill_path(scope, project_root)? else {
        return Ok(None);
    };
    if !path.exists() {
        return Ok(Some(SkillInstallReport {
            agent: agent.as_str().to_string(),
            scope: scope.as_str(),
            path: Some(path.display().to_string().replace('\\', "/")),
            status: "not-installed",
            note: Some("SKILL.md does not exist".into()),
        }));
    }
    if dry_run {
        return Ok(Some(SkillInstallReport {
            agent: agent.as_str().to_string(),
            scope: scope.as_str(),
            path: Some(path.display().to_string().replace('\\', "/")),
            status: "would-remove",
            note: Some("delete SKILL.md (and parent vibevm/ dir if empty)".into()),
        }));
    }
    fs::remove_file(&path).with_context(|| format!("removing SKILL.md `{}`", path.display()))?;
    // Try to remove the parent `vibevm/` skill dir if it became empty.
    // Best-effort — don't fail uninstall if the dir has stragglers.
    if let Some(parent) = path.parent() {
        let _ = fs::remove_dir(parent);
    }
    Ok(Some(SkillInstallReport {
        agent: agent.as_str().to_string(),
        scope: scope.as_str(),
        path: Some(path.display().to_string().replace('\\', "/")),
        status: "removed",
        note: None,
    }))
}

fn print_uninstall_results(
    ctx: &output::Context,
    dry_run: bool,
    results: &[AgentInstallReport],
    skill_results: &[SkillInstallReport],
) {
    for r in results {
        let prefix = match r.status {
            "removed" | "would-remove" => {
                if dry_run {
                    "would"
                } else {
                    "removed"
                }
            }
            "not-installed" => "·",
            other => other,
        };
        let note = r
            .note
            .as_deref()
            .map(|n| format!(" ({n})"))
            .unwrap_or_default();
        ctx.step(&format!(
            "{} mcp     {} ({}) → {}{note}",
            prefix, r.agent, r.scope, r.config_path
        ));
    }
    for r in skill_results {
        let prefix = match r.status {
            "removed" | "would-remove" => {
                if dry_run {
                    "would"
                } else {
                    "removed"
                }
            }
            "not-installed" => "·",
            other => other,
        };
        let note = r
            .note
            .as_deref()
            .map(|n| format!(" ({n})"))
            .unwrap_or_default();
        let path_str = r.path.as_deref().unwrap_or("(no skill loader)");
        ctx.step(&format!(
            "{} skill   {} ({}) → {}{note}",
            prefix, r.agent, r.scope, path_str
        ));
    }
}

// ---------------------------------------------------------------------------
// Interactive helpers — TTY-only paths
// ---------------------------------------------------------------------------

fn interactive_ask_scope(path: &Path) -> Result<Scope> {
    if !stdin_is_tty() {
        bail!(
            "no --scope and stdin is not a TTY — pass `--scope project|user|both` or \
             `--auto` (auto-resolves scope from vibe.toml presence)"
        );
    }
    let has_toml = has_vibe_toml(path);
    let default_idx = if has_toml { 0 } else { 1 };
    let prompt = if has_toml {
        "Where to install? (vibe.toml found — defaulting to project-level)"
    } else {
        "Where to install? (vibe.toml not found — defaulting to user-level)"
    };
    let chosen = dialoguer::Select::new()
        .with_prompt(prompt)
        .items([
            "Project-level — per-project files committed to git",
            "User-level    — global home/config dirs, works everywhere",
            "Both          — project AND user simultaneously",
        ])
        .default(default_idx)
        .interact()?;
    Ok(match chosen {
        0 => Scope::Project,
        1 => Scope::User,
        2 => Scope::Both,
        _ => unreachable!(),
    })
}

fn interactive_ask_what() -> Result<What> {
    if !stdin_is_tty() {
        return Ok(What::Both);
    }
    let chosen = dialoguer::Select::new()
        .with_prompt("What to install?")
        .items([
            "Both MCP server config and SKILL.md (recommended)",
            "MCP server only",
            "SKILL.md only",
        ])
        .default(0)
        .interact()?;
    Ok(match chosen {
        0 => What::Both,
        1 => What::Mcp,
        2 => What::Skill,
        _ => unreachable!(),
    })
}

fn interactive_select_agents(detected: &[Agent], force: bool) -> Result<Vec<Agent>> {
    if !stdin_is_tty() {
        bail!(
            "no --agent and stdin is not a TTY — pass `--agent <name>` (one of \
             `all`, `claude`, `claude-desktop`, `cursor`, `opencode`, `codex`) or \
             `--auto` to detect every supported agent"
        );
    }
    // Slice 5: always show ALL agents, with checkbox preselected for
    // detected ones and a `(not detected)` badge on the rest. `--force`
    // toggles whether unchecked-not-detected agents will install
    // anyway when chosen — but visually they're always pickable.
    let _ = force; // currently informational only at the wizard layer
    let pool: Vec<Agent> = Agent::ALL.to_vec();
    let labels: Vec<String> = pool
        .iter()
        .map(|a| {
            let badge = if detected.contains(a) {
                ""
            } else {
                "  (not detected)"
            };
            format!("{}{}", a.as_str(), badge)
        })
        .collect();
    let defaults: Vec<bool> = pool.iter().map(|a| detected.contains(a)).collect();
    let chosen = dialoguer::MultiSelect::new()
        .with_prompt("Which agents? (space to toggle, enter to confirm)")
        .items(&labels)
        .defaults(&defaults)
        .interact()?;
    Ok(chosen.into_iter().map(|i| pool[i]).collect())
}

// ---------------------------------------------------------------------------
// MCP-entry decide / preview / apply / merge — JSON + TOML
// ---------------------------------------------------------------------------

fn decide_action(
    agent: Agent,
    config_path: &Path,
    payload: &ConfigPayload,
) -> Result<(&'static str, Option<String>)> {
    if !config_path.exists() {
        return Ok(("created", Some("file does not exist yet".into())));
    }
    let section = agent.mcp_section_key();
    match (payload, agent.config_format()) {
        (ConfigPayload::Json(entry), ConfigFormat::Json) => {
            let existing = read_json(config_path)?;
            let existing_entry = existing.get(section).and_then(|v| v.get(SERVER_NAME));
            match existing_entry {
                Some(e) if e == entry => Ok(("unchanged", None)),
                Some(_) => Ok(("updated", Some(format!("{section}/{SERVER_NAME} differs")))),
                None => Ok(("updated", Some(format!("{section}/{SERVER_NAME} absent")))),
            }
        }
        (ConfigPayload::Toml(entry), ConfigFormat::Toml) => {
            let existing = read_toml(config_path)?;
            let existing_entry = existing
                .get(section)
                .and_then(|v| v.as_table())
                .and_then(|t| t.get(SERVER_NAME));
            match existing_entry {
                Some(e) if e == entry => Ok(("unchanged", None)),
                Some(_) => Ok((
                    "updated",
                    Some(format!("[{section}.{SERVER_NAME}] differs")),
                )),
                None => Ok(("updated", Some(format!("[{section}.{SERVER_NAME}] absent")))),
            }
        }
        _ => bail!(
            "internal: agent `{}` config_format/payload mismatch",
            agent.as_str()
        ),
    }
}

fn preview_install_mcp(
    agent: Agent,
    scope: Scope,
    config_path: &Path,
    payload: &ConfigPayload,
) -> Result<AgentInstallReport> {
    let (status, note) = decide_action(agent, config_path, payload)?;
    let dry = match status {
        "unchanged" => "unchanged",
        "created" => "would-create",
        "updated" => "would-update",
        other => other,
    };
    Ok(AgentInstallReport {
        agent: agent.as_str().to_string(),
        scope: scope.as_str(),
        config_path: config_path.display().to_string().replace('\\', "/"),
        status: dry,
        note,
    })
}

fn apply_install_mcp(
    agent: Agent,
    scope: Scope,
    config_path: &Path,
    payload: &ConfigPayload,
) -> Result<AgentInstallReport> {
    let (status, note) = decide_action(agent, config_path, payload)?;
    if status == "unchanged" {
        return Ok(AgentInstallReport {
            agent: agent.as_str().to_string(),
            scope: scope.as_str(),
            config_path: config_path.display().to_string().replace('\\', "/"),
            status: "unchanged",
            note,
        });
    }
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("creating dir `{}`", parent.display()))?;
    }
    match (payload, agent.config_format()) {
        (ConfigPayload::Json(entry), ConfigFormat::Json) => {
            let merged = merge_json(config_path, agent.mcp_section_key(), SERVER_NAME, entry)?;
            let serialized = serde_json::to_string_pretty(&merged)
                .with_context(|| "serializing merged JSON config")?;
            fs::write(config_path, serialized + "\n")
                .with_context(|| format!("writing `{}`", config_path.display()))?;
        }
        (ConfigPayload::Toml(entry), ConfigFormat::Toml) => {
            let merged = merge_toml(config_path, agent.mcp_section_key(), SERVER_NAME, entry)?;
            let serialized = toml::to_string_pretty(&merged)
                .with_context(|| "serializing merged TOML config")?;
            fs::write(config_path, serialized)
                .with_context(|| format!("writing `{}`", config_path.display()))?;
        }
        _ => bail!(
            "internal: agent `{}` config_format/payload mismatch",
            agent.as_str()
        ),
    }
    Ok(AgentInstallReport {
        agent: agent.as_str().to_string(),
        scope: scope.as_str(),
        config_path: config_path.display().to_string().replace('\\', "/"),
        status,
        note,
    })
}

// ---------------------------------------------------------------------------
// project-root resolution
// ---------------------------------------------------------------------------

fn has_vibe_toml(path: &Path) -> bool {
    path.canonicalize()
        .ok()
        .map(super::init::strip_unc_public)
        .map(|p| p.join(Manifest::FILENAME).exists())
        .unwrap_or(false)
}

fn resolve_project_root_required(path: &Path) -> Result<PathBuf> {
    let canonical = path
        .canonicalize()
        .with_context(|| format!("canonicalizing `{}`", path.display()))?;
    let stripped = super::init::strip_unc_public(canonical);
    if !stripped.join(Manifest::FILENAME).exists() {
        bail!(
            "no `vibe.toml` in `{}`; run `vibe init` first, pass `--path <dir>`, \
             or use `--scope user` to install without a project",
            stripped.display()
        );
    }
    Ok(stripped)
}
