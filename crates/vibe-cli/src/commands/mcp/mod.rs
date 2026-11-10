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
//!   into a single user-level entry for Claude Desktop, whose MCP
//!   configuration has no project surface.
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
//! | Codex          | `mcp_servers` | TOML   | `.codex/config.toml`| `~/.codex/config.toml`                           |
//! | Qwen Code      | `mcpServers`  | JSON   | `.qwen/settings.json`| `~/.qwen/settings.json`                         |
//!
//! Claude Code reads MCP servers from `.mcp.json` (project) and the
//! top-level `mcpServers` of `~/.claude.json` (user) — NOT from
//! `settings.json`, which only *gates* servers (`enabledMcpjsonServers`).
//! `<config-dir>` resolves through `dirs::config_dir()` — `%APPDATA%`
//! on Windows, `~/Library/Application Support` on macOS, `~/.config`
//! on Linux (used by Claude Desktop). OpenCode deliberately reads the
//! XDG-style `~/.config/opencode/` on every OS.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-015#lifecycle");

use std::fs;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::Serialize;
use vibe_core::machine_json_path;
use vibe_core::manifest::Manifest;
use vibe_core::user_config::UserConfig;
use vibe_mcp::agent_config::{
    merge_json, merge_toml, read_json, read_toml, strip_json_entry, strip_toml_entry,
};
use vibe_mcp::agents::{Agent, ConfigFormat, ConfigPayload, Scope, What, detect_agents};
use vibe_mcp::install::{AgentInstallReport, SkillInstallReport};
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

/// CLI-composed authority fixed for the lifetime of an MCP server. Environment
/// and user configuration stay above `vibe-mcp`; tools receive only these
/// decided values through `ServerContext`.
pub(crate) struct McpRuntime {
    pub(crate) root_offline: bool,
    pub(crate) embedded_registry_root: Option<PathBuf>,
    pub(crate) seed_default_registry: bool,
}

pub fn run(ctx: &output::Context, args: McpArgs, runtime: McpRuntime) -> Result<()> {
    match args.command {
        McpSubcommand::Enable(sub) => activation::run(ctx, sub, true),
        McpSubcommand::Disable(sub) => activation::run(ctx, sub, false),
        McpSubcommand::Serve(sub) => run_serve(sub, runtime),
        McpSubcommand::Install(sub) => install::run_install(ctx, sub),
        McpSubcommand::Status(sub) => status::run_status(ctx, sub),
        McpSubcommand::Upgrade(sub) => upgrade::run_upgrade(ctx, sub),
        McpSubcommand::Uninstall(sub) => uninstall::run_uninstall(ctx, sub),
    }
}

/// Register one already materialised global MCP package in the selected agents.
pub(crate) fn register_global_package(
    ctx: &output::Context,
    project_root: &Path,
    package: &str,
    agents: &[Agent],
    only_server: Option<&str>,
    consent: bool,
) -> Result<()> {
    let version = package_registration::package_version(project_root, package)?;
    let servers = package_registration::selected_servers(project_root, package, only_server)?;
    let selected = if only_server.is_none()
        && !consent
        && crate::commands::global_mcp_agents::interactive(ctx)
        && servers.len() > 1
    {
        let labels: Vec<_> = servers
            .iter()
            .map(|server| server.decl.name.clone())
            .collect();
        let defaults: Vec<_> = (0..labels.len()).collect();
        let indices = ctx.suspend_progress(|| {
            crate::commands::global_mcp_choices::choose(
                "Select MCP servers:",
                "All servers (default)",
                &labels,
                &defaults,
                "select a server with Space, or pass --server <name,name>",
            )
        })?;
        Some(package_registration::exact_server_filter(
            &indices
                .into_iter()
                .map(|index| labels[index].clone())
                .collect::<Vec<_>>(),
        )?)
    } else {
        only_server.map(str::to_owned)
    };
    let results = package_registration::register_with_dirs(
        project_root,
        package,
        selected.as_deref(),
        agents,
        Scope::User,
        package_registration::RegistrationPolicy {
            consent,
            dry_run: false,
        },
        &ctx.agent_user_dirs,
    )?;
    if ctx.is_json() {
        ctx.emit_json(&serde_json::json!({ "ok": true, "command": "install", "global": true, "scope": "user", "package": package, "version": version, "results": results }))?;
    } else {
        for row in results {
            ctx.step(&format!(
                "{} {} (user) → {} ({})",
                row.status,
                row.agent,
                row.config_path,
                row.note.as_deref().unwrap_or("")
            ));
        }
    }
    Ok(())
}

/// Refresh only the agents that already have an owned registration for this
/// package. A package update does not silently add new client integrations.
pub(crate) fn preflight_refresh_global_package(ctx: &output::Context, package: &str) -> Result<()> {
    package_registration::preflight_user_refresh(package, &ctx.agent_user_dirs)
}

pub(crate) fn refresh_global_package(
    ctx: &output::Context,
    project_root: &Path,
    package: &str,
    consent: bool,
) -> Result<()> {
    let version = package_registration::package_version(project_root, package)?;
    let agents: Vec<Agent> =
        package_registration::user_agent_configs_for_package(package, &ctx.agent_user_dirs)?
            .into_iter()
            .map(|(agent, _)| agent)
            .collect();
    let results = if agents.is_empty() {
        Vec::new()
    } else {
        package_registration::refresh_user_package(
            project_root,
            package,
            &agents,
            consent,
            &ctx.agent_user_dirs,
        )?
    };
    if ctx.is_json() {
        ctx.emit_json(&serde_json::json!({ "ok": true, "command": "update", "global": true, "scope": "user", "package": package, "version": version, "results": results }))?;
    } else if results.is_empty() {
        ctx.step(&format!(
            "no agent registrations for `{package}` to refresh"
        ));
    } else {
        for row in results {
            ctx.step(&format!(
                "{} {} (user) → {}",
                row.status, row.agent, row.config_path
            ));
        }
    }
    Ok(())
}

/// Opaque preparation for a global package uninstall. The exact bytes of
/// every affected agent config and receipt are retained until the package
/// uninstall succeeds, so a later failure can restore the prior state.
pub(crate) struct GlobalMcpRemoval {
    package: String,
    version: Option<String>,
    plan: package_registration::GlobalRemovalPlan,
}

pub(crate) fn preflight_unregister_global_package(
    ctx: &output::Context,
    package: &str,
    agents: &[Agent],
) -> Result<GlobalMcpRemoval> {
    let version = crate::commands::install::user_project_root()
        .ok()
        .and_then(|root| package_registration::package_version(&root, package).ok());
    Ok(GlobalMcpRemoval {
        package: package.to_owned(),
        version,
        plan: package_registration::plan_global_removal(package, agents, &ctx.agent_user_dirs)?,
    })
}

pub(crate) fn global_package_agents(ctx: &output::Context, package: &str) -> Result<Vec<Agent>> {
    Ok(
        package_registration::user_agent_configs_for_package(package, &ctx.agent_user_dirs)?
            .into_iter()
            .map(|(agent, _)| agent)
            .collect(),
    )
}

pub(crate) use package_registration::GlobalAgentRegistration;

pub(crate) fn global_package_registration_statuses(
    ctx: &output::Context,
    package: &str,
) -> Result<Vec<GlobalAgentRegistration>> {
    package_registration::user_registration_statuses(package, &ctx.agent_user_dirs)
}

pub(crate) fn removes_global_package(removal: &GlobalMcpRemoval) -> bool {
    removal.plan.package_removed
}

pub(crate) fn unregister_global_package(removal: &mut GlobalMcpRemoval) -> Result<()> {
    package_registration::apply_global_removal(&mut removal.plan)
}

pub(crate) fn restore_global_package(removal: &mut GlobalMcpRemoval) -> Result<()> {
    package_registration::restore_global_removal(&mut removal.plan)
}

pub(crate) fn report_global_uninstall(
    ctx: &output::Context,
    removal: &GlobalMcpRemoval,
) -> Result<()> {
    if ctx.is_json() {
        ctx.emit_json(&serde_json::json!({ "ok": true, "command": "uninstall", "global": true, "scope": "user", "package": removal.package, "version": removal.version, "results": removal.plan.results, "package_removed": removal.plan.package_removed }))?;
    } else {
        if !removal.plan.package_removed {
            ctx.summary("Global MCP package retained because other agent registrations remain.");
        }
        for row in &removal.plan.results {
            ctx.step(&format!(
                "{} {} (user) → {}",
                row.status, row.agent, row.config_path
            ));
        }
    }
    Ok(())
}

fn run_serve(args: McpServeArgs, runtime: McpRuntime) -> Result<()> {
    // `vibe mcp serve` is the one place where path is *required*: the
    // server needs a project root to load the lockfile from. When
    // launched by a user-scope MCP entry that omits `--path`, the
    // server uses CWD (default value `.`).
    let project_root = resolve_project_root_required(&args.path)?;
    let user = UserConfig::load().context("loading user config for MCP lifecycle execution")?;
    let policy = vibe_orchestrator::InstallPolicy {
        offline: output::resolve_offline(runtime.root_offline, user.net.offline),
        slot_integrity: user.install.slot_integrity,
        spec_format_default: user.install.spec_format,
        // A served lifecycle execution is not a surface with a flag
        // grammar: it takes the default posture, recommendations included
        // (`PROP-077#NO-RECOMMENDS-FLAG`).
        no_recommends: false,
    };
    let server_ctx = ServerContext::new(project_root).with_lifecycle_execution(
        policy,
        runtime.embedded_registry_root,
        runtime.seed_default_registry,
    );
    let server_ctx = match args.migration_recovery_store {
        Some(store) => server_ctx
            .with_manifest_migration_authority(store, args.migration_file)
            .context("admitting explicit MCP manifest migration authority")?,
        None => server_ctx,
    };
    let mut server = Server::stdio(server_ctx);
    server.run().context("MCP server I/O error")?;
    Ok(())
}

// ---------------------------------------------------------------------------
// MCP-entry decide / preview / apply / merge — JSON + TOML
// ---------------------------------------------------------------------------

/// Explicit builtin `vibevm` MCP-entry registration for `vibe mcp install`.
/// `force` lets the install adopt and replace a stale unreceipted entry an
/// older vibe wrote; without it such an entry stays a typed refusal.
pub(super) fn install_builtin_mcp(
    agent: Agent,
    scope: Scope,
    config_path: &Path,
    payload: &ConfigPayload,
    force: bool,
    dry_run: bool,
) -> Result<AgentInstallReport> {
    package_registration::register_builtin_explicit(
        agent,
        scope,
        config_path,
        payload,
        install_adoption(force),
        dry_run,
    )
}

/// The adoption policy `vibe mcp install [--force]` applies to unreceipted
/// builtin entries (MCP entry and SKILL.md alike).
pub(super) fn install_adoption(force: bool) -> vibe_agent_projection::activation::BuiltinAdoption {
    if force {
        vibe_agent_projection::activation::BuiltinAdoption::forced("vibe mcp install")
    } else {
        vibe_agent_projection::activation::BuiltinAdoption::explicit("vibe mcp install")
    }
}

/// The per-row report for a builtin SKILL.md that has no ownership receipt
/// and cannot be claimed by the running command.
pub(super) fn unreceipted_skill_row(
    agent: Agent,
    scope: Scope,
    project_root: Option<&Path>,
    status: &'static str,
) -> Result<SkillInstallReport> {
    Ok(SkillInstallReport {
        agent: agent.as_str().to_string(),
        scope: scope.as_str(),
        path: agent
            .skill_path(scope, project_root)?
            .map(|path| machine_json_path(&path)),
        status,
        note: Some(package_registration::UNRECEIPTED_HINT.into()),
    })
}

/// Whether `error` is a refusal to claim a builtin `vibevm` MCP entry or
/// SKILL.md that carries no ownership receipt (written by an older vibe).
/// Read-only and refresh commands report such an entry per row instead of
/// aborting.
pub(super) fn is_unreceipted_builtin_refusal(error: &anyhow::Error) -> bool {
    error
        .downcast_ref::<package_registration::BuiltinOwnershipError>()
        .is_some()
        || matches!(
            error.downcast_ref::<vibe_agent_projection::pkgskill::PackageSkillError>(),
            Some(vibe_agent_projection::pkgskill::PackageSkillError::Unowned { .. })
        )
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

mod activation;
mod install;
mod install_prompts;
mod package_registration;
mod status;
mod uninstall;
mod upgrade;
