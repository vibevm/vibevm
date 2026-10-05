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
use std::sync::Arc;

use anyhow::{Context, Result, bail};
use dialoguer::Confirm;
use serde::Serialize;
use vibe_core::machine_json_path;
use vibe_core::manifest::{Manifest, TargetOs};
use vibe_core::user_config::UserConfig;
use vibe_mcp::agent_config::{
    merge_json, merge_toml, read_json, read_toml, strip_json_entry, strip_toml_entry,
};
use vibe_mcp::agents::{Agent, ConfigFormat, ConfigPayload, Scope, What, detect_agents};
use vibe_mcp::install::{AgentInstallReport, SkillInstallReport, install_skill};
use vibe_mcp::{Server, ServerContext};
use vibe_orchestrator::machine_operation::fixed_cargo::FixedCargoRoute;
use vibe_orchestrator::machine_operation::fixed_package::FixedPackageRoute;
use vibe_orchestrator::machine_submission::HostPolicyFacts;
use vibe_orchestrator::machine_submission::cargo_grant::CargoGrantService;
use vibe_orchestrator::machine_submission::package_grant::PackageGrantService;

use crate::cli::{
    McpArgs, McpFixedCargoMode, McpFixedDeployMode, McpFixedInstallMode, McpFixedPackageMode,
    McpFixedValidateMode, McpInstallArgs, McpServeArgs, McpStatusArgs, McpSubcommand,
    McpUninstallArgs, McpUpgradeArgs,
};
use crate::commands::machine_session::{
    ExactLocalInstallHostMode, FixedDeployHostMode, exact_local_install_context,
    fixed_deploy_context,
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

pub(crate) fn restore_global_package(removal: &GlobalMcpRemoval) -> Result<()> {
    package_registration::restore_global_removal(&removal.plan)
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

#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-SURFACES",
    r = 2
)]
fn run_serve(args: McpServeArgs, runtime: McpRuntime) -> Result<()> {
    // `vibe mcp serve` is the one place where path is *required*: the
    // server needs a project root to load the lockfile from. When
    // launched by a user-scope MCP entry that omits `--path`, the
    // server uses CWD (default value `.`).
    if (args.fixed_validate.is_some()
        || args.fixed_package.is_some()
        || args.fixed_cargo.is_some()
        || args.fixed_deploy.is_some()
        || args.fixed_install.is_some())
        && !args.path.is_absolute()
    {
        bail!(
            "violates REQ spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-TRUSTED-CONTEXT: fixed effectful MCP host requires an explicit absolute --path; fix surface: --path <absolute-project-root>"
        );
    }
    match (
        args.fixed_validate,
        args.fixed_package,
        args.fixed_cargo,
        args.fixed_deploy,
        args.fixed_install,
        args.ttl_ms,
    ) {
        (Some(McpFixedValidateMode::Issue), None, None, None, None, Some(_))
        | (Some(McpFixedValidateMode::Reattach), None, None, None, None, None)
        | (None, Some(McpFixedPackageMode::Issue), None, None, None, Some(_))
        | (None, Some(McpFixedPackageMode::Reattach), None, None, None, None)
        | (None, None, Some(McpFixedCargoMode::Issue), None, None, Some(_))
        | (None, None, Some(McpFixedCargoMode::Reattach), None, None, None)
        | (None, None, None, Some(McpFixedDeployMode::Issue), None, Some(_))
        | (None, None, None, Some(McpFixedDeployMode::Reattach), None, None)
        | (None, None, None, None, Some(McpFixedInstallMode::Issue), Some(_))
        | (None, None, None, None, Some(McpFixedInstallMode::Reattach), None)
        | (None, None, None, None, None, None) => {}
        _ => bail!(
            "violates REQ spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-TRUSTED-CONTEXT: fixed host issue requires --ttl-ms and reattach forbids it; fix surface: select exactly one fixed host issue --ttl-ms <1..=86400000>, or reattach"
        ),
    }
    if args.fixed_deploy.is_some() != args.deploy_profile.is_some() {
        bail!("fixed Deploy requires exactly one --deploy-profile; other modes forbid it");
    }
    let install_source = args.registry_root.is_some()
        && args.package_pin.as_ref().is_some_and(|pin| !pin.is_empty())
        && (1..=8).contains(&args.source_files.len())
        && args.source_files.iter().any(|file| file == "vibe.toml");
    if args.fixed_install.is_some() != install_source
        || (args.fixed_install.is_none()
            && (args.registry_root.is_some()
                || args.package_pin.is_some()
                || !args.source_files.is_empty()
                || args.approve_exact_local_install))
    {
        bail!(
            "fixed Install requires --registry-root, --package-pin and --source-file vibe.toml; source and approval flags require fixed Install"
        );
    }
    let project_root = resolve_project_root_required(&args.path)?;
    let user = UserConfig::load().context("loading user config for MCP lifecycle execution")?;
    let policy = vibe_orchestrator::InstallPolicy {
        offline: output::resolve_offline(runtime.root_offline, user.net.offline),
        slot_integrity: user.install.slot_integrity,
        spec_format_default: user.install.spec_format,
    };
    let server_ctx = ServerContext::new(&project_root).with_lifecycle_execution(
        policy,
        runtime.embedded_registry_root,
        runtime.seed_default_registry,
    );
    // Pure migration authority validation must finish before any fixed host
    // `issue` can publish a durable pointer or C1 namespace.
    let server_ctx = if let Some(store) = args.migration_recovery_store {
        server_ctx
            .with_manifest_migration_authority(store, args.migration_file)
            .context("MCP manifest migration authority refused (governed by spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-SURFACES; fix: select an absolute disjoint recovery store and portable owned authored files within the trusted server project)")?
    } else {
        server_ctx
    };
    let server_ctx = match args.fixed_package {
        Some(mode) => fixed_package_server_context(server_ctx, &project_root, mode, args.ttl_ms)?,
        None => server_ctx,
    };
    let server_ctx = match args.fixed_cargo {
        Some(mode) => fixed_cargo_server_context(server_ctx, &project_root, mode, args.ttl_ms)?,
        None => server_ctx,
    };
    let server_ctx = match args.fixed_validate {
        Some(mode) => fixed_validate::server_context(
            server_ctx,
            &project_root,
            mode,
            args.ttl_ms,
            HostPolicyFacts {
                approve_captured_script: args.approve_captured_script,
                approve_exact_local_install: false,
                allow_detach: args.allow_detach,
                allow_cancel_pre_dispatch: args.allow_cancel_pre_dispatch,
                allow_cancel_deferred_boundary: args.allow_cancel_deferred_boundary,
            },
        )?,
        None => server_ctx,
    };
    let server_ctx = match args.fixed_deploy {
        Some(mode) => fixed_deploy_context(
            server_ctx,
            &project_root,
            args.deploy_profile
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("fixed Deploy requires --deploy-profile"))?,
            match mode {
                McpFixedDeployMode::Issue => FixedDeployHostMode::Issue(
                    args.ttl_ms
                        .ok_or_else(|| anyhow::anyhow!("fixed Deploy issue requires --ttl-ms"))?,
                ),
                McpFixedDeployMode::Reattach => FixedDeployHostMode::Reattach,
            },
        )?,
        None => server_ctx,
    };
    let server_ctx = match args.fixed_install {
        Some(mode) => exact_local_install_context(
            server_ctx,
            &project_root,
            args.registry_root
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("fixed Install requires --registry-root"))?,
            args.package_pin
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("fixed Install requires --package-pin"))?,
            &args.source_files,
            args.approve_exact_local_install,
            match mode {
                McpFixedInstallMode::Issue => ExactLocalInstallHostMode::Issue(
                    args.ttl_ms
                        .ok_or_else(|| anyhow::anyhow!("fixed Install issue requires --ttl-ms"))?,
                ),
                McpFixedInstallMode::Reattach => ExactLocalInstallHostMode::Reattach,
            },
        )?,
        None => server_ctx,
    };
    let mut server = Server::stdio(server_ctx);
    server.run().context("MCP server I/O error")?;
    Ok(())
}

#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-CLI-PACKAGE-GRANT-DECISION"
)]
fn fixed_package_server_context(
    context: ServerContext,
    project_root: &Path,
    mode: McpFixedPackageMode,
    ttl_ms: Option<u64>,
) -> Result<ServerContext> {
    let target_os = TargetOs::current().ok_or_else(|| {
        anyhow::anyhow!("violates REQ spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-CLI-PACKAGE-GRANT-DECISION: fixed package MCP host requires a supported target platform; fix surface: run on a supported platform")
    })?;
    let project = Arc::new(vibe_safefs::Project::open(project_root)?);
    let service = PackageGrantService::from_trusted_host(
        project,
        "vibe-cli-project-local/e1".into(),
        target_os,
    )?;
    let host = match mode {
        McpFixedPackageMode::Issue => service.issue(ttl_ms.ok_or_else(|| {
            anyhow::anyhow!("fixed package issue requires --ttl-ms <1..=86400000>")
        })?)?,
        McpFixedPackageMode::Reattach => service.reattach()?,
    };
    Ok(context.with_fixed_package_route(Arc::new(FixedPackageRoute::new(Arc::new(host))))?)
}

#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-FIXED-CARGO-BUILD-DECISION"
)]
fn fixed_cargo_server_context(
    context: ServerContext,
    project_root: &Path,
    mode: McpFixedCargoMode,
    ttl_ms: Option<u64>,
) -> Result<ServerContext> {
    let target_os = TargetOs::current().ok_or_else(|| {
        anyhow::anyhow!("violates REQ spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-FIXED-CARGO-BUILD-DECISION: fixed Cargo MCP host requires a supported target platform; fix surface: run on a supported platform")
    })?;
    let project = Arc::new(vibe_safefs::Project::open(project_root)?);
    let service = CargoGrantService::from_trusted_builtin_host(
        project,
        "vibe-cli-project-local/e1".into(),
        target_os,
    )?;
    let host = match mode {
        McpFixedCargoMode::Issue => service.issue(ttl_ms.ok_or_else(|| {
            anyhow::anyhow!("fixed Cargo issue requires --ttl-ms <1..=86400000>")
        })?)?,
        McpFixedCargoMode::Reattach => service.reattach()?,
    };
    Ok(context.with_fixed_cargo_route(Arc::new(FixedCargoRoute::new(Arc::new(host))))?)
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
        config_path: machine_json_path(config_path),
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
            config_path: machine_json_path(config_path),
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
        config_path: machine_json_path(config_path),
        status,
        note,
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

mod fixed_validate;
mod install;
mod install_prompts;
mod package_registration;
mod status;
mod uninstall;
mod upgrade;

#[cfg(test)]
mod exact_local_install_serve_tests {
    use super::*;
    use crate::cli::{Cli, Command};
    use clap::Parser;

    fn parsed(extra: &[&str]) -> std::result::Result<McpServeArgs, clap::Error> {
        let mut argv = vec![
            "vibe",
            "mcp",
            "serve",
            "--path",
            "/tmp/vibe-mcp-install-test",
        ];
        argv.extend_from_slice(extra);
        let cli = Cli::try_parse_from(argv)?;
        let Command::Mcp(mcp) = cli.command else {
            panic!("MCP command expected")
        };
        let McpSubcommand::Serve(args) = mcp.command else {
            panic!("serve expected")
        };
        Ok(args)
    }

    #[test]
    fn fixed_install_serve_requires_explicit_source_and_issue_lifetime() {
        let args = parsed(&[
            "--fixed-install",
            "issue",
            "--registry-root",
            "/tmp/vibe-registry-test",
            "--package-pin",
            "flow:org.example/fixture@=1.1.0",
            "--source-file",
            "vibe.toml",
            "--source-file",
            "README.md",
            "--ttl-ms",
            "60000",
        ])
        .unwrap();
        assert!(matches!(
            args.fixed_install,
            Some(McpFixedInstallMode::Issue)
        ));
        assert!(!args.approve_exact_local_install);
        assert!(
            parsed(&[
                "--fixed-install",
                "reattach",
                "--registry-root",
                "/tmp/vibe-registry-test",
                "--package-pin",
                "flow:org.example/fixture@=1.1.0",
                "--source-file",
                "vibe.toml",
                "--ttl-ms",
                "60000",
            ])
            .is_ok()
        );
        let invalid = parsed(&[
            "--fixed-install",
            "reattach",
            "--registry-root",
            "/tmp/vibe-registry-test",
            "--package-pin",
            "flow:org.example/fixture@=1.1.0",
            "--source-file",
            "vibe.toml",
            "--ttl-ms",
            "60000",
        ])
        .unwrap();
        assert!(
            run_serve(
                invalid,
                McpRuntime {
                    root_offline: false,
                    embedded_registry_root: None,
                    seed_default_registry: false,
                }
            )
            .is_err()
        );
        assert!(
            parsed(&[
                "--fixed-install",
                "issue",
                "--registry-root",
                "relative",
                "--package-pin",
                "flow:org.example/fixture@=1.1.0",
                "--source-file",
                "vibe.toml",
                "--ttl-ms",
                "60000",
            ])
            .is_err()
        );
        assert!(parsed(&["--approve-exact-local-install"]).is_err());
    }
}
