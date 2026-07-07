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
//! ## Scope axis
//!
//! Every install touches one or two physical files per agent:
//! - **Project scope** writes the agent's committed project config
//!   (`<project>/.mcp.json` for Claude Code) — every clone gets the
//!   same setup.
//! - **User scope** writes the agent's home/global config
//!   (`~/.claude.json` for Claude Code) — works in every directory.
//! - **Both** writes to project AND user simultaneously, falling
//!   into a single user-level entry for the two agents that have no
//!   project surface (Claude Desktop, Codex).
//!
//! The MCP entry is identical for every scope — `vibe mcp serve` with
//! no `--path`, resolving its root from the launcher's CWD — and on
//! Windows it is wrapped as `cmd /c vibe …` so the `vibe.cmd` shim can
//! be spawned. See `vibe_mcp::agents::Agent::build_mcp_entry`.
//!
//! ## Agent matrix
//!
//! | Agent          | section       | format | project file        | user file                                        |
//! |----------------|---------------|--------|---------------------|--------------------------------------------------|
//! | Claude Code    | `mcpServers`  | JSON   | `.mcp.json`         | `~/.claude.json`                                 |
//! | Claude Desktop | `mcpServers`  | JSON   | (n/a — user-only)   | `<config-dir>/Claude/claude_desktop_config.json` |
//! | Cursor         | `mcpServers`  | JSON   | `.cursor/mcp.json`  | `~/.cursor/mcp.json`                             |
//! | OpenCode       | `mcp`         | JSON   | `opencode.json`     | `~/.config/opencode/opencode.json`               |
//! | Codex          | `mcp_servers` | TOML   | (n/a — user-only)   | `~/.codex/config.toml`                           |
//!
//! Claude Code reads MCP servers from `.mcp.json` (project) and the
//! top-level `mcpServers` of `~/.claude.json` (user) — NOT from
//! `settings.json`, which only *gates* servers (`enabledMcpjsonServers`).
//! `<config-dir>` resolves through `dirs::config_dir()` — `%APPDATA%`
//! on Windows, `~/Library/Application Support` on macOS, `~/.config`
//! on Linux (used by Claude Desktop). OpenCode deliberately reads the
//! XDG-style `~/.config/opencode/` on every OS.

specmark::scope!("spec://vibevm/modules/vibe-mcp/PROP-015#lifecycle");

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
        McpSubcommand::Install(sub) => install::run_install(ctx, sub),
        McpSubcommand::Status(sub) => run_status(ctx, sub),
        McpSubcommand::Upgrade(sub) => upgrade::run_upgrade(ctx, sub),
        McpSubcommand::Uninstall(sub) => uninstall::run_uninstall(ctx, sub),
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
    /// Package-declared servers (PROP-027 §2.4) and their lifecycle
    /// state: registered where, artifact built or the build recipe.
    pkg_servers: Vec<PkgServerStatus>,
}

#[derive(Debug, Clone, Serialize)]
struct PkgServerStatus {
    name: String,
    package: String,
    version: String,
    artifact: String,
    /// `built` / `unbuilt` — an unbuilt artifact registers fine and
    /// fails at agent-launch time, so status names the recipe.
    artifact_state: &'static str,
    /// The recipe when unbuilt.
    note: Option<String>,
}

/// Collect the PROP-027 lifecycle rows for `vibe mcp status`.
fn pkg_server_status(project_root: Option<&Path>) -> Vec<PkgServerStatus> {
    let Some(root) = project_root else {
        return Vec::new();
    };
    let Ok(servers) = vibe_workspace::bins::collect_mcp_servers(root) else {
        return Vec::new();
    };
    servers
        .into_iter()
        .map(|s| {
            let artifact = {
                let a = s.binary.artifact();
                if a.is_absolute() { a } else { root.join(a) }
            };
            let built = artifact.exists();
            PkgServerStatus {
                name: s.decl.name.clone(),
                package: s.binary.package.clone(),
                version: s.version.clone(),
                artifact: artifact.display().to_string().replace('\\', "/"),
                artifact_state: if built { "built" } else { "unbuilt" },
                note: (!built).then(|| {
                    format!(
                        "run `vibe bin build {}` before an agent launches it",
                        s.binary.decl.name
                    )
                }),
            }
        })
        .collect()
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
                let payload = agent.build_mcp_entry();
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
    let pkg_servers = pkg_server_status(project_root.as_deref());
    let report = StatusReport {
        ok: true,
        command: "mcp:status",
        project: project_root.as_ref().map(|p| p.display().to_string()),
        detected: detected.iter().map(|a| a.as_str().to_string()).collect(),
        results: results.clone(),
        skill_results: skill_results.clone(),
        pkg_servers: pkg_servers.clone(),
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
    for s in &pkg_servers {
        let note = s
            .note
            .as_deref()
            .map(|n| format!(" ({n})"))
            .unwrap_or_default();
        ctx.step(&format!(
            "{} server  {} ({}@{}) → {}{note}",
            s.artifact_state, s.name, s.package, s.version, s.artifact
        ));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// MCP-entry decide / preview / apply / merge — JSON + TOML
// ---------------------------------------------------------------------------

pub(super) fn decide_action(
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

pub(super) fn preview_install_mcp(
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

pub(super) fn apply_install_mcp(
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
// package-declared servers (PROP-027 §2.4) — JSON-only by construction:
// registration is project-scope (the {project_root} substitution demands
// a project), and every project-scope agent config is JSON.
// ---------------------------------------------------------------------------

fn decide_pkg_action(
    config_path: &Path,
    section: &str,
    name: &str,
    entry: &serde_json::Value,
) -> Result<(&'static str, Option<String>)> {
    if !config_path.exists() {
        return Ok(("created", Some("file does not exist yet".into())));
    }
    let existing = read_json(config_path)?;
    match existing.get(section).and_then(|v| v.get(name)) {
        Some(e) if e == entry => Ok(("unchanged", None)),
        Some(_) => Ok(("updated", Some(format!("`{section}.{name}` differs")))),
        None => Ok(("created", Some(format!("`{section}.{name}` absent")))),
    }
}

pub(super) fn preview_install_pkg_server(
    agent: Agent,
    config_path: &Path,
    name: &str,
    entry: &serde_json::Value,
) -> Result<AgentInstallReport> {
    let (status, note) = decide_pkg_action(config_path, agent.mcp_section_key(), name, entry)?;
    let dry = match status {
        "unchanged" => "unchanged",
        "created" => "would-create",
        "updated" => "would-update",
        other => other,
    };
    Ok(AgentInstallReport {
        agent: agent.as_str().to_string(),
        scope: "project",
        config_path: config_path.display().to_string().replace('\\', "/"),
        status: dry,
        note: Some(match note {
            Some(n) => format!("pkg server `{name}` — {n}"),
            None => format!("pkg server `{name}`"),
        }),
    })
}

pub(super) fn apply_install_pkg_server(
    agent: Agent,
    config_path: &Path,
    name: &str,
    entry: &serde_json::Value,
) -> Result<AgentInstallReport> {
    let section = agent.mcp_section_key();
    let (status, note) = decide_pkg_action(config_path, section, name, entry)?;
    if status != "unchanged" {
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("creating dir `{}`", parent.display()))?;
        }
        let mut merged = merge_json(config_path, section, name, entry)?;
        vibe_mcp::pkg_servers::mark_managed(&mut merged, name)?;
        let serialized = serde_json::to_string_pretty(&merged)
            .with_context(|| "serializing merged JSON config")?;
        fs::write(config_path, serialized + "\n")
            .with_context(|| format!("writing `{}`", config_path.display()))?;
    }
    Ok(AgentInstallReport {
        agent: agent.as_str().to_string(),
        scope: "project",
        config_path: config_path.display().to_string().replace('\\', "/"),
        status,
        note: Some(match note {
            Some(n) => format!("pkg server `{name}` — {n}"),
            None => format!("pkg server `{name}`"),
        }),
    })
}

// ---------------------------------------------------------------------------
// project-root resolution
// ---------------------------------------------------------------------------

pub(super) fn has_vibe_toml(path: &Path) -> bool {
    path.canonicalize()
        .ok()
        .map(super::init::strip_unc_public)
        .map(|p| p.join(Manifest::FILENAME).exists())
        .unwrap_or(false)
}

pub(super) fn resolve_project_root_required(path: &Path) -> Result<PathBuf> {
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

mod install;
mod uninstall;
mod upgrade;
