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

use anyhow::{Context, Result, anyhow, bail};
use serde::Serialize;
use serde_json::{Map, Value as JsonValue};
use vibe_core::manifest::ProjectManifest;
use vibe_mcp::{Server, ServerContext};

use crate::cli::{
    McpArgs, McpInstallArgs, McpServeArgs, McpStatusArgs, McpSubcommand, McpUninstallArgs,
    McpUpgradeArgs,
};
use crate::output;

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
// Scope + What — primary user-facing dimensions
// ---------------------------------------------------------------------------

/// Where a vibevm artefact (MCP-config block or SKILL.md) lives. The
/// install / upgrade / uninstall surface accept this through `--scope`;
/// the wizard asks via the first prompt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum Scope {
    /// Project-scope path — `<project>/<agent-rel>`. Committed to git.
    Project,
    /// User-scope path — `<home>/<agent-rel>`. Machine-local, global.
    User,
    /// Write to BOTH project and user scopes in one run. For agents
    /// with only one scope (Claude Desktop, Codex), Both collapses to
    /// the available scope.
    Both,
}

impl Scope {
    pub fn as_str(self) -> &'static str {
        match self {
            Scope::Project => "project",
            Scope::User => "user",
            Scope::Both => "both",
        }
    }

    pub fn parse(value: &str) -> Result<Scope> {
        match value {
            "project" => Ok(Scope::Project),
            "user" => Ok(Scope::User),
            "both" => Ok(Scope::Both),
            other => bail!(
                "unknown --scope value `{other}` (expected `project`, `user`, or `both`)"
            ),
        }
    }

    /// Expand a high-level Scope choice into the list of physical
    /// scopes to walk per agent. Both → [Project, User]; the singular
    /// variants → a one-element vector.
    pub fn expand(self) -> Vec<Scope> {
        match self {
            Scope::Both => vec![Scope::Project, Scope::User],
            other => vec![other],
        }
    }

    /// Whether installing under this scope requires a `vibe.toml` in
    /// the working directory. Project + Both — yes; User — no.
    pub fn requires_vibe_toml(self) -> bool {
        matches!(self, Scope::Project | Scope::Both)
    }
}

/// What to install / uninstall — MCP server entry, SKILL.md, or both.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum What {
    /// MCP server entry only.
    Mcp,
    /// SKILL.md only.
    Skill,
    /// Both (default).
    Both,
}

impl What {
    pub fn as_str(self) -> &'static str {
        match self {
            What::Mcp => "mcp",
            What::Skill => "skill",
            What::Both => "both",
        }
    }

    pub fn parse(value: &str) -> Result<What> {
        match value {
            "mcp" => Ok(What::Mcp),
            "skill" => Ok(What::Skill),
            "both" => Ok(What::Both),
            other => bail!(
                "unknown --what value `{other}` (expected `mcp`, `skill`, or `both`)"
            ),
        }
    }

    pub fn includes_mcp(self) -> bool {
        matches!(self, What::Mcp | What::Both)
    }

    pub fn includes_skill(self) -> bool {
        matches!(self, What::Skill | What::Both)
    }
}

// ---------------------------------------------------------------------------
// Agent profile
// ---------------------------------------------------------------------------

/// Coding agent supported by `vibe mcp install`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum Agent {
    ClaudeCode,
    ClaudeCodeDesktop,
    Cursor,
    OpenCode,
    Codex,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigFormat {
    Json,
    Toml,
}

#[derive(Debug, Clone)]
pub enum ConfigPayload {
    Json(JsonValue),
    Toml(toml::Value),
}

/// Bytes of the `vibevm` SKILL.md template, vendored at compile time.
pub const SKILL_TEMPLATE: &str = include_str!("skill_template.md");

/// Skill name. Matches the `name:` frontmatter in the template and the
/// directory name we write under each agent's skills root.
pub const SKILL_NAME: &str = "vibevm";

const SERVER_NAME: &str = "vibevm";

impl Agent {
    pub const ALL: &'static [Agent] = &[
        Agent::ClaudeCode,
        Agent::ClaudeCodeDesktop,
        Agent::Cursor,
        Agent::OpenCode,
        Agent::Codex,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Agent::ClaudeCode => "claude",
            Agent::ClaudeCodeDesktop => "claude-desktop",
            Agent::Cursor => "cursor",
            Agent::OpenCode => "opencode",
            Agent::Codex => "codex",
        }
    }

    pub fn parse_filter(filter: &str) -> Result<Vec<Agent>> {
        match filter {
            "all" => Ok(Agent::ALL.to_vec()),
            "claude" | "claude-code" => Ok(vec![Agent::ClaudeCode]),
            "claude-desktop" | "claude-code-desktop" => Ok(vec![Agent::ClaudeCodeDesktop]),
            "cursor" => Ok(vec![Agent::Cursor]),
            "opencode" => Ok(vec![Agent::OpenCode]),
            "codex" => Ok(vec![Agent::Codex]),
            other => bail!(
                "unknown --agent value `{other}` (expected one of `all`, \
                 `claude`, `claude-desktop`, `cursor`, `opencode`, `codex`)"
            ),
        }
    }

    pub fn config_format(self) -> ConfigFormat {
        match self {
            Agent::Codex => ConfigFormat::Toml,
            _ => ConfigFormat::Json,
        }
    }

    pub fn mcp_section_key(self) -> &'static str {
        match self {
            Agent::OpenCode => "mcp",
            Agent::Codex => "mcp_servers",
            _ => "mcpServers",
        }
    }

    /// Whether this agent has a meaningful project-scope config path.
    /// Claude Desktop and Codex are user-only — no project surface.
    #[allow(dead_code)] // wired by upgrade / uninstall in subsequent commits
    pub fn supports_project_scope(self) -> bool {
        match self {
            Agent::ClaudeCode | Agent::Cursor | Agent::OpenCode => true,
            Agent::ClaudeCodeDesktop | Agent::Codex => false,
        }
    }

    /// Whether this agent loads filesystem-backed skill files. Cursor
    /// and Claude Desktop are JSON-config-only.
    pub fn supports_skill(self) -> bool {
        match self {
            Agent::ClaudeCode | Agent::OpenCode | Agent::Codex => true,
            Agent::ClaudeCodeDesktop | Agent::Cursor => false,
        }
    }

    /// Resolve the per-agent config-file path for a single concrete
    /// scope (Project or User — never Both; expand Both first).
    /// Returns `Ok(None)` if the agent does not support this scope
    /// (e.g. Claude Desktop + Project). Returns `Err(...)` if the
    /// host cannot resolve required dirs (HOME / config-dir).
    pub fn config_path(
        self,
        scope: Scope,
        project_root: Option<&Path>,
    ) -> Result<Option<PathBuf>> {
        match (self, scope) {
            (_, Scope::Both) => bail!(
                "internal: Agent::config_path requires concrete scope; expand Both first"
            ),
            // ---- Project scope ----
            (Agent::ClaudeCode, Scope::Project) => Ok(project_root
                .map(|p| p.join(".claude").join("settings.json"))),
            (Agent::Cursor, Scope::Project) => Ok(project_root
                .map(|p| p.join(".cursor").join("mcp.json"))),
            (Agent::OpenCode, Scope::Project) => Ok(project_root
                .map(|p| p.join("opencode.json"))),
            (Agent::ClaudeCodeDesktop | Agent::Codex, Scope::Project) => Ok(None),
            // ---- User scope ----
            (Agent::ClaudeCode, Scope::User) => {
                let home = dirs::home_dir()
                    .ok_or_else(|| anyhow!("could not resolve home dir for Claude Code"))?;
                Ok(Some(home.join(".claude").join("settings.json")))
            }
            (Agent::Cursor, Scope::User) => {
                let home = dirs::home_dir()
                    .ok_or_else(|| anyhow!("could not resolve home dir for Cursor"))?;
                Ok(Some(home.join(".cursor").join("mcp.json")))
            }
            (Agent::OpenCode, Scope::User) => {
                // OpenCode's documented global-config location is
                // `~/.config/opencode/opencode.json` cross-platform —
                // they use a Unix-style XDG path on every OS, NOT
                // `%APPDATA%` on Windows. Verified empirically:
                // operator-set `~/.config/opencode/opencode.json` is
                // what `opencode` reads on Windows; `%APPDATA%\opencode\`
                // is silently ignored. So we resolve via `home_dir`,
                // not `config_dir`.
                let home = dirs::home_dir()
                    .ok_or_else(|| anyhow!("could not resolve home dir for OpenCode"))?;
                Ok(Some(
                    home.join(".config").join("opencode").join("opencode.json"),
                ))
            }
            (Agent::ClaudeCodeDesktop, Scope::User) => {
                // Claude Desktop is a native Anthropic GUI app and DOES
                // use platform-specific config dirs (`%APPDATA%\Claude\`
                // on Windows, `~/Library/Application Support/Claude/`
                // on macOS). dirs::config_dir() is the right resolver
                // here.
                let cfg = dirs::config_dir().ok_or_else(|| {
                    anyhow!("could not resolve user-config dir for Claude Desktop")
                })?;
                Ok(Some(cfg.join("Claude").join("claude_desktop_config.json")))
            }
            (Agent::Codex, Scope::User) => {
                let home = dirs::home_dir()
                    .ok_or_else(|| anyhow!("could not resolve home dir for Codex"))?;
                Ok(Some(home.join(".codex").join("config.toml")))
            }
        }
    }

    /// Resolve the per-agent SKILL.md path for a single concrete scope.
    /// Returns `Ok(None)` for agents that don't load filesystem skills
    /// (Cursor, Claude Desktop) regardless of scope. Returns `Ok(None)`
    /// for project scope when the agent has no project surface (Claude
    /// Desktop, Codex — though those are skill-unsupported anyway).
    pub fn skill_path(
        self,
        scope: Scope,
        project_root: Option<&Path>,
    ) -> Result<Option<PathBuf>> {
        if !self.supports_skill() {
            return Ok(None);
        }
        match (self, scope) {
            (_, Scope::Both) => bail!(
                "internal: Agent::skill_path requires concrete scope; expand Both first"
            ),
            // ---- Project scope ----
            (Agent::ClaudeCode, Scope::Project) => Ok(project_root.map(|p| {
                p.join(".claude").join("skills").join(SKILL_NAME).join("SKILL.md")
            })),
            (Agent::OpenCode, Scope::Project) => Ok(project_root.map(|p| {
                p.join(".opencode").join("skills").join(SKILL_NAME).join("SKILL.md")
            })),
            (Agent::Codex, Scope::Project) => Ok(project_root.map(|p| {
                p.join(".agents").join("skills").join(SKILL_NAME).join("SKILL.md")
            })),
            (Agent::Cursor | Agent::ClaudeCodeDesktop, _) => Ok(None),
            // ---- User scope ----
            (Agent::ClaudeCode, Scope::User) => {
                let home = dirs::home_dir().ok_or_else(|| {
                    anyhow!("could not resolve home dir for Claude Code skill")
                })?;
                Ok(Some(home.join(".claude").join("skills").join(SKILL_NAME).join("SKILL.md")))
            }
            (Agent::OpenCode, Scope::User) => {
                // Same XDG-on-every-OS contract as Agent::config_path
                // for OpenCode — see the comment there. Empirically
                // verified that opencode reads `~/.config/opencode/`
                // on Windows, NOT `%APPDATA%\opencode\`.
                let home = dirs::home_dir().ok_or_else(|| {
                    anyhow!("could not resolve home dir for OpenCode skill")
                })?;
                Ok(Some(
                    home.join(".config")
                        .join("opencode")
                        .join("skills")
                        .join(SKILL_NAME)
                        .join("SKILL.md"),
                ))
            }
            (Agent::Codex, Scope::User) => {
                let home = dirs::home_dir().ok_or_else(|| {
                    anyhow!("could not resolve home dir for Codex skill")
                })?;
                Ok(Some(home.join(".agents").join("skills").join(SKILL_NAME).join("SKILL.md")))
            }
        }
    }

    /// Wire shape of the per-server entry. Three flavours, scope-aware:
    /// - User scope omits `--path`, so the server resolves CWD per
    ///   invocation. Lets one global config serve every project.
    /// - Project scope hardcodes `--path <abs-project>` so the server
    ///   always serves the same project regardless of CWD.
    pub fn build_mcp_entry(self, scope: Scope, project_root: Option<&Path>) -> ConfigPayload {
        let args_array: Vec<String> = match scope {
            Scope::User => vec!["mcp".into(), "serve".into()],
            Scope::Project => {
                let proj = project_root
                    .map(|p| p.display().to_string().replace('\\', "/"))
                    .unwrap_or_else(|| ".".to_string());
                vec!["mcp".into(), "serve".into(), "--path".into(), proj]
            }
            Scope::Both => unreachable!("Both must be expanded before build_mcp_entry"),
        };
        match self {
            Agent::ClaudeCode | Agent::ClaudeCodeDesktop | Agent::Cursor => {
                ConfigPayload::Json(serde_json::json!({
                    "command": "vibe",
                    "args": args_array,
                }))
            }
            Agent::OpenCode => {
                let mut command = vec!["vibe".to_string()];
                command.extend(args_array);
                ConfigPayload::Json(serde_json::json!({
                    "type": "local",
                    "command": command,
                    "enabled": true,
                }))
            }
            Agent::Codex => {
                let mut tbl = toml::value::Table::new();
                tbl.insert("command".into(), toml::Value::String("vibe".into()));
                tbl.insert(
                    "args".into(),
                    toml::Value::Array(
                        args_array.into_iter().map(toml::Value::String).collect(),
                    ),
                );
                ConfigPayload::Toml(toml::Value::Table(tbl))
            }
        }
    }

    /// Project-tree presence markers — files / dirs whose existence in
    /// the working tree marks the agent as actively used.
    pub fn presence_markers(self) -> &'static [&'static str] {
        match self {
            Agent::ClaudeCode => &[".claude", "CLAUDE.md"],
            Agent::Cursor => &[".cursor", ".cursorrules"],
            Agent::OpenCode => &[".opencode", "opencode.json", "opencode.jsonc", "AGENTS.md"],
            Agent::ClaudeCodeDesktop | Agent::Codex => &[],
        }
    }

    /// Whether the agent's user-level config dir exists on this host.
    /// Lets `--auto` and the wizard mark host-installed agents even
    /// when the project tree has no markers.
    pub fn host_present(self) -> bool {
        let Ok(Some(cfg)) = self.config_path(Scope::User, None) else {
            return false;
        };
        cfg.parent().map(|p| p.exists()).unwrap_or(false)
    }

    /// Combined presence: project markers OR user-level dir exists.
    pub fn is_present(self, project_root: Option<&Path>) -> bool {
        if let Some(root) = project_root {
            for m in self.presence_markers() {
                if root.join(m).exists() {
                    return true;
                }
            }
        }
        self.host_present()
    }
}

/// Detect every supported agent that has any presence-marker in the
/// project tree or, for user-level agents, an existing config dir on
/// this host.
pub fn detect_agents(project_root: Option<&Path>) -> Vec<Agent> {
    Agent::ALL
        .iter()
        .copied()
        .filter(|a| a.is_present(project_root))
        .collect()
}

// ---------------------------------------------------------------------------
// Reporting
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct AgentInstallReport {
    pub agent: String,
    pub scope: &'static str,
    pub config_path: String,
    /// `created` / `updated` / `unchanged` / `would-create` /
    /// `would-update` / `skipped`.
    pub status: &'static str,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SkillInstallReport {
    pub agent: String,
    pub scope: &'static str,
    pub path: Option<String>,
    pub status: &'static str,
    pub note: Option<String>,
}

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
    let any_explicit_target = args.agent.is_some()
        || args.scope.is_some()
        || args.what.is_some();
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

    // 2. Resolve project_root if scope requires it.
    let project_root: Option<PathBuf> = if scope.requires_vibe_toml() {
        Some(resolve_project_root_required(&args.path)?)
    } else {
        // User-only — project is irrelevant. Try canonicalising for
        // logging clarity but don't fail.
        args.path
            .canonicalize()
            .ok()
            .map(super::init::strip_unc_public)
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
    //    scope).
    let mut results: Vec<AgentInstallReport> = Vec::new();
    let mut skill_results: Vec<SkillInstallReport> = Vec::new();
    for agent in &targeted {
        for concrete_scope in scope.expand() {
            // ---- MCP entry ----
            if what.includes_mcp() {
                let path = agent.config_path(concrete_scope, project_root.as_deref())?;
                if let Some(path) = path {
                    let payload = agent.build_mcp_entry(concrete_scope, project_root.as_deref());
                    let outcome = if args.dry_run {
                        preview_install_mcp(*agent, concrete_scope, &path, &payload)?
                    } else {
                        apply_install_mcp(*agent, concrete_scope, &path, &payload)?
                    };
                    results.push(outcome);
                } else if scope == Scope::Both {
                    // Both selected but this agent has no surface for
                    // this concrete scope — note as skipped, no error.
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
                let outcome = install_skill(
                    *agent,
                    concrete_scope,
                    project_root.as_deref(),
                    args.dry_run,
                )?;
                skill_results.push(outcome);
            }
        }
    }

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
        let mcp_written = results.iter().filter(|r| matches!(r.status, "created" | "updated")).count();
        let skill_written = skill_results.iter().filter(|r| matches!(r.status, "created" | "updated")).count();
        let verb = if args.dry_run { "previewed" } else { "written" };
        ctx.summary(&format!(
            "vibe mcp install: scope={} what={} — {mcp_written} MCP + {skill_written} skill {verb}",
            scope.as_str(), what.as_str()
        ));
        return Ok(());
    }
    print_install_results(ctx, args.dry_run, &results, &skill_results);
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
        let note = r.note.as_deref().map(|n| format!(" ({n})")).unwrap_or_default();
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
        let note = r.note.as_deref().map(|n| format!(" ({n})")).unwrap_or_default();
        let path_str = r.path.as_deref().unwrap_or("(no skill loader)");
        ctx.step(&format!(
            "{} skill   {} ({}) → {}{note}",
            prefix, r.agent, r.scope, path_str
        ));
    }
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
        .filter(|p| p.join(ProjectManifest::FILENAME).exists());
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
            if agent.supports_skill()
                && agent.skill_path(scope, project_root.as_deref())?.is_some()
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
            detected.iter().map(|a| a.as_str()).collect::<Vec<_>>().join(", ")
        }
    ));
    for r in &results {
        let note = r.note.as_deref().map(|n| format!(" ({n})")).unwrap_or_default();
        ctx.step(&format!(
            "{} mcp     {} ({}) → {}{note}",
            r.status, r.agent, r.scope, r.config_path
        ));
    }
    for r in &skill_results {
        let note = r.note.as_deref().map(|n| format!(" ({n})")).unwrap_or_default();
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
        .filter(|p| p.join(ProjectManifest::FILENAME).exists());

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

    let mut results: Vec<AgentInstallReport> = Vec::new();
    let mut skill_results: Vec<SkillInstallReport> = Vec::new();

    for agent in &agents {
        for concrete_scope in scope.expand() {
            // Skip project-scope walks when no vibe.toml resolved.
            if concrete_scope == Scope::Project && project_root.is_none() {
                continue;
            }

            // ---- MCP entry ----
            if what.includes_mcp() {
                let path = agent.config_path(concrete_scope, project_root.as_deref())?;
                if let Some(path) = path {
                    let outcome = upgrade_mcp_entry(
                        *agent,
                        concrete_scope,
                        &path,
                        project_root.as_deref(),
                        args.dry_run,
                    )?;
                    results.push(outcome);
                }
            }

            // ---- SKILL.md ----
            if what.includes_skill() {
                let outcome = upgrade_skill(
                    *agent,
                    concrete_scope,
                    project_root.as_deref(),
                    args.dry_run,
                )?;
                if let Some(o) = outcome {
                    skill_results.push(o);
                }
            }
        }
    }

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
        let verb = if args.dry_run { "previewed" } else { "refreshed" };
        ctx.summary(&format!("vibe mcp upgrade: {stale} stale entr{} {verb}",
            if stale == 1 { "y" } else { "ies" }));
        return Ok(());
    }

    print_upgrade_results(ctx, args.dry_run, &results, &skill_results);
    Ok(())
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
        (ConfigPayload::Json(_), ConfigFormat::Json) => {
            read_json(config_path)?
                .get(section)
                .and_then(|v| v.get(SERVER_NAME))
                .is_some()
        }
        (ConfigPayload::Toml(_), ConfigFormat::Toml) => {
            read_toml(config_path)?
                .get(section)
                .and_then(|v| v.as_table())
                .and_then(|t| t.get(SERVER_NAME))
                .is_some()
        }
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
            "would-update" | "updated" => if dry_run { "would" } else { "updated" },
            "not-installed" => "·",
            other => other,
        };
        let note = r.note.as_deref().map(|n| format!(" ({n})")).unwrap_or_default();
        ctx.step(&format!(
            "{} mcp     {} ({}) → {}{note}",
            prefix, r.agent, r.scope, r.config_path
        ));
    }
    for r in skill_results {
        let prefix = match r.status {
            "unchanged" => "✓",
            "would-update" | "updated" => if dry_run { "would" } else { "updated" },
            "not-installed" => "·",
            other => other,
        };
        let note = r.note.as_deref().map(|n| format!(" ({n})")).unwrap_or_default();
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
        .filter(|p| p.join(ProjectManifest::FILENAME).exists());

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

    let mut results: Vec<AgentInstallReport> = Vec::new();
    let mut skill_results: Vec<SkillInstallReport> = Vec::new();

    for agent in &agents {
        for concrete_scope in scope.expand() {
            if concrete_scope == Scope::Project && project_root.is_none() {
                continue;
            }

            // ---- MCP entry ----
            if what.includes_mcp() {
                let path = agent.config_path(concrete_scope, project_root.as_deref())?;
                if let Some(path) = path {
                    let outcome =
                        uninstall_mcp_entry(*agent, concrete_scope, &path, args.dry_run)?;
                    results.push(outcome);
                }
            }

            // ---- SKILL.md ----
            if what.includes_skill() {
                let outcome = uninstall_skill(
                    *agent,
                    concrete_scope,
                    project_root.as_deref(),
                    args.dry_run,
                )?;
                if let Some(o) = outcome {
                    skill_results.push(o);
                }
            }
        }
    }

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
        ctx.summary(&format!("vibe mcp uninstall: {removed} entr{} {verb}",
            if removed == 1 { "y" } else { "ies" }));
        return Ok(());
    }

    print_uninstall_results(ctx, args.dry_run, &results, &skill_results);
    Ok(())
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
        ConfigFormat::Json => {
            read_json(config_path)?
                .get(section)
                .and_then(|v| v.get(SERVER_NAME))
                .is_some()
        }
        ConfigFormat::Toml => {
            read_toml(config_path)?
                .get(section)
                .and_then(|v| v.as_table())
                .and_then(|t| t.get(SERVER_NAME))
                .is_some()
        }
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
    fs::remove_file(&path)
        .with_context(|| format!("removing SKILL.md `{}`", path.display()))?;
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

fn strip_json_entry(
    config_path: &Path,
    section_key: &str,
    server_name: &str,
) -> Result<JsonValue> {
    let mut existing = read_json(config_path)?;
    if let Some(obj) = existing.as_object_mut()
        && let Some(servers) = obj.get_mut(section_key).and_then(|v| v.as_object_mut())
    {
        servers.remove(server_name);
    }
    Ok(existing)
}

fn strip_toml_entry(
    config_path: &Path,
    section_key: &str,
    server_name: &str,
) -> Result<toml::Value> {
    let mut existing = read_toml(config_path)?;
    if let Some(root) = existing.as_table_mut()
        && let Some(servers) = root.get_mut(section_key).and_then(|v| v.as_table_mut())
    {
        servers.remove(server_name);
    }
    Ok(existing)
}

fn print_uninstall_results(
    ctx: &output::Context,
    dry_run: bool,
    results: &[AgentInstallReport],
    skill_results: &[SkillInstallReport],
) {
    for r in results {
        let prefix = match r.status {
            "removed" | "would-remove" => if dry_run { "would" } else { "removed" },
            "not-installed" => "·",
            other => other,
        };
        let note = r.note.as_deref().map(|n| format!(" ({n})")).unwrap_or_default();
        ctx.step(&format!(
            "{} mcp     {} ({}) → {}{note}",
            prefix, r.agent, r.scope, r.config_path
        ));
    }
    for r in skill_results {
        let prefix = match r.status {
            "removed" | "would-remove" => if dry_run { "would" } else { "removed" },
            "not-installed" => "·",
            other => other,
        };
        let note = r.note.as_deref().map(|n| format!(" ({n})")).unwrap_or_default();
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
                Some(_) => Ok(("updated", Some(format!("[{section}.{SERVER_NAME}] differs")))),
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

fn read_json(path: &Path) -> Result<JsonValue> {
    let text = fs::read_to_string(path).with_context(|| format!("reading `{}`", path.display()))?;
    if text.trim().is_empty() {
        return Ok(JsonValue::Object(Map::new()));
    }
    let v: JsonValue = serde_json::from_str(&text)
        .with_context(|| format!("parsing JSON `{}`", path.display()))?;
    Ok(v)
}

fn read_toml(path: &Path) -> Result<toml::Value> {
    let text = fs::read_to_string(path).with_context(|| format!("reading `{}`", path.display()))?;
    if text.trim().is_empty() {
        return Ok(toml::Value::Table(toml::value::Table::new()));
    }
    let v: toml::Value = toml::from_str(&text)
        .with_context(|| format!("parsing TOML `{}`", path.display()))?;
    Ok(v)
}

fn merge_json(
    config_path: &Path,
    section_key: &str,
    server_name: &str,
    new_entry: &JsonValue,
) -> Result<JsonValue> {
    let mut existing = if config_path.exists() {
        read_json(config_path)?
    } else {
        JsonValue::Object(Map::new())
    };
    let obj = existing
        .as_object_mut()
        .ok_or_else(|| anyhow!("`{}` is not a JSON object", config_path.display()))?;
    let servers = obj
        .entry(section_key.to_string())
        .or_insert_with(|| JsonValue::Object(Map::new()));
    let servers_obj = servers
        .as_object_mut()
        .ok_or_else(|| anyhow!("`{section_key}` is not a JSON object"))?;
    servers_obj.insert(server_name.to_string(), new_entry.clone());
    Ok(existing)
}

fn merge_toml(
    config_path: &Path,
    section_key: &str,
    server_name: &str,
    new_entry: &toml::Value,
) -> Result<toml::Value> {
    let mut existing = if config_path.exists() {
        read_toml(config_path)?
    } else {
        toml::Value::Table(toml::value::Table::new())
    };
    let root = existing
        .as_table_mut()
        .ok_or_else(|| anyhow!("`{}` root is not a TOML table", config_path.display()))?;
    let servers = root
        .entry(section_key.to_string())
        .or_insert_with(|| toml::Value::Table(toml::value::Table::new()));
    let servers_tbl = servers
        .as_table_mut()
        .ok_or_else(|| anyhow!("`[{section_key}]` is not a TOML table"))?;
    servers_tbl.insert(server_name.to_string(), new_entry.clone());
    Ok(existing)
}

// ---------------------------------------------------------------------------
// project-root resolution
// ---------------------------------------------------------------------------

fn has_vibe_toml(path: &Path) -> bool {
    path.canonicalize()
        .ok()
        .map(super::init::strip_unc_public)
        .map(|p| p.join(ProjectManifest::FILENAME).exists())
        .unwrap_or(false)
}

fn resolve_project_root_required(path: &Path) -> Result<PathBuf> {
    let canonical = path
        .canonicalize()
        .with_context(|| format!("canonicalizing `{}`", path.display()))?;
    let stripped = super::init::strip_unc_public(canonical);
    if !stripped.join(ProjectManifest::FILENAME).exists() {
        bail!(
            "no `vibe.toml` in `{}`; run `vibe init` first, pass `--path <dir>`, \
             or use `--scope user` to install without a project",
            stripped.display()
        );
    }
    Ok(stripped)
}

// ---------------------------------------------------------------------------
// Skill artefact — per-agent SKILL.md writer
// ---------------------------------------------------------------------------

pub fn install_skill(
    agent: Agent,
    scope: Scope,
    project_root: Option<&Path>,
    dry_run: bool,
) -> Result<SkillInstallReport> {
    let agent_str = agent.as_str().to_string();
    let scope_str = scope.as_str();

    let Some(path) = agent.skill_path(scope, project_root)? else {
        return Ok(SkillInstallReport {
            agent: agent_str,
            scope: scope_str,
            path: None,
            status: "skipped",
            note: Some(format!(
                "agent `{}` has no {}-scope skill loader",
                agent.as_str(),
                scope.as_str()
            )),
        });
    };

    let body = SKILL_TEMPLATE;
    let path_str = path.display().to_string().replace('\\', "/");
    let status = decide_skill_action(&path, body)?;

    let final_status: &'static str = match (status, dry_run) {
        ("unchanged", _) => "unchanged",
        ("created", true) => "would-create",
        ("updated", true) => "would-update",
        (s, _) => s,
    };

    if !dry_run && final_status != "unchanged" {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("creating skill dir `{}`", parent.display()))?;
        }
        fs::write(&path, body)
            .with_context(|| format!("writing skill `{}`", path.display()))?;
    }

    Ok(SkillInstallReport {
        agent: agent_str,
        scope: scope_str,
        path: Some(path_str),
        status: final_status,
        note: None,
    })
}

fn decide_skill_action(path: &Path, body: &str) -> Result<&'static str> {
    if !path.exists() {
        return Ok("created");
    }
    let existing = fs::read_to_string(path)
        .with_context(|| format!("reading skill `{}`", path.display()))?;
    if existing == body {
        Ok("unchanged")
    } else {
        Ok("updated")
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn json_payload(agent: Agent, scope: Scope, project: Option<&Path>) -> JsonValue {
        match agent.build_mcp_entry(scope, project) {
            ConfigPayload::Json(v) => v,
            ConfigPayload::Toml(_) => panic!("expected JSON for {}", agent.as_str()),
        }
    }

    fn toml_payload(agent: Agent, scope: Scope, project: Option<&Path>) -> toml::Value {
        match agent.build_mcp_entry(scope, project) {
            ConfigPayload::Toml(v) => v,
            ConfigPayload::Json(_) => panic!("expected TOML for {}", agent.as_str()),
        }
    }

    // ---- Scope / What ----

    #[test]
    fn scope_parse_known_values() {
        assert_eq!(Scope::parse("project").unwrap(), Scope::Project);
        assert_eq!(Scope::parse("user").unwrap(), Scope::User);
        assert_eq!(Scope::parse("both").unwrap(), Scope::Both);
        assert!(Scope::parse("global").is_err());
    }

    #[test]
    fn scope_expand_both_yields_two_concrete() {
        assert_eq!(Scope::Both.expand(), vec![Scope::Project, Scope::User]);
        assert_eq!(Scope::Project.expand(), vec![Scope::Project]);
        assert_eq!(Scope::User.expand(), vec![Scope::User]);
    }

    #[test]
    fn scope_requires_vibe_toml_only_for_project_or_both() {
        assert!(Scope::Project.requires_vibe_toml());
        assert!(Scope::Both.requires_vibe_toml());
        assert!(!Scope::User.requires_vibe_toml());
    }

    #[test]
    fn what_parse_known_values() {
        assert_eq!(What::parse("mcp").unwrap(), What::Mcp);
        assert_eq!(What::parse("skill").unwrap(), What::Skill);
        assert_eq!(What::parse("both").unwrap(), What::Both);
        assert!(What::parse("nope").is_err());
    }

    #[test]
    fn what_includes_axes() {
        assert!(What::Mcp.includes_mcp() && !What::Mcp.includes_skill());
        assert!(!What::Skill.includes_mcp() && What::Skill.includes_skill());
        assert!(What::Both.includes_mcp() && What::Both.includes_skill());
    }

    // ---- detection ----

    #[test]
    fn detect_finds_claude_via_marker_dir() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join(".claude")).unwrap();
        let agents = detect_agents(Some(dir.path()));
        assert!(agents.contains(&Agent::ClaudeCode));
    }

    #[test]
    fn detect_finds_opencode_via_agents_md() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("AGENTS.md"), "x").unwrap();
        let agents = detect_agents(Some(dir.path()));
        assert!(agents.contains(&Agent::OpenCode));
    }

    #[test]
    fn detect_with_no_project_root_falls_back_to_host_probe() {
        // Without a project root, only host-presence agents can show up.
        // The set is non-deterministic per machine, but the call must
        // not panic and must return at most all-five.
        let agents = detect_agents(None);
        for a in &agents {
            assert!(Agent::ALL.contains(a));
        }
    }

    // ---- parse_filter ----

    #[test]
    fn parse_filter_known_values() {
        assert_eq!(Agent::parse_filter("all").unwrap(), Agent::ALL.to_vec());
        assert_eq!(Agent::parse_filter("claude").unwrap(), vec![Agent::ClaudeCode]);
        assert_eq!(Agent::parse_filter("claude-desktop").unwrap(), vec![Agent::ClaudeCodeDesktop]);
        assert_eq!(Agent::parse_filter("cursor").unwrap(), vec![Agent::Cursor]);
        assert_eq!(Agent::parse_filter("opencode").unwrap(), vec![Agent::OpenCode]);
        assert_eq!(Agent::parse_filter("codex").unwrap(), vec![Agent::Codex]);
        assert!(Agent::parse_filter("nope").is_err());
    }

    // ---- per-agent profile ----

    #[test]
    fn supports_project_scope_only_for_three_agents() {
        assert!(Agent::ClaudeCode.supports_project_scope());
        assert!(Agent::Cursor.supports_project_scope());
        assert!(Agent::OpenCode.supports_project_scope());
        assert!(!Agent::ClaudeCodeDesktop.supports_project_scope());
        assert!(!Agent::Codex.supports_project_scope());
    }

    #[test]
    fn supports_skill_only_for_three_agents() {
        assert!(Agent::ClaudeCode.supports_skill());
        assert!(Agent::OpenCode.supports_skill());
        assert!(Agent::Codex.supports_skill());
        assert!(!Agent::ClaudeCodeDesktop.supports_skill());
        assert!(!Agent::Cursor.supports_skill());
    }

    // ---- config_path ----

    #[test]
    fn config_path_project_lands_under_project_root() {
        let dir = tempfile::tempdir().unwrap();
        let p = Agent::ClaudeCode
            .config_path(Scope::Project, Some(dir.path()))
            .unwrap()
            .unwrap();
        let s = p.display().to_string().replace('\\', "/");
        assert!(s.ends_with("/.claude/settings.json"), "got {s}");

        let p = Agent::OpenCode
            .config_path(Scope::Project, Some(dir.path()))
            .unwrap()
            .unwrap();
        let s = p.display().to_string().replace('\\', "/");
        assert!(s.ends_with("/opencode.json"), "got {s}");

        let p = Agent::Cursor
            .config_path(Scope::Project, Some(dir.path()))
            .unwrap()
            .unwrap();
        let s = p.display().to_string().replace('\\', "/");
        assert!(s.ends_with("/.cursor/mcp.json"), "got {s}");
    }

    #[test]
    fn config_path_user_only_agents_have_no_project_surface() {
        let dir = tempfile::tempdir().unwrap();
        assert!(
            Agent::ClaudeCodeDesktop
                .config_path(Scope::Project, Some(dir.path()))
                .unwrap()
                .is_none()
        );
        assert!(
            Agent::Codex
                .config_path(Scope::Project, Some(dir.path()))
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn config_path_user_resolves_for_all_agents() {
        for &a in Agent::ALL {
            let p = a.config_path(Scope::User, None).unwrap();
            assert!(p.is_some(), "user-scope path missing for {}", a.as_str());
        }
    }

    #[test]
    fn opencode_user_paths_use_xdg_style_on_every_os() {
        // OpenCode is documented to read `~/.config/opencode/` on
        // every platform — XDG-style, NOT %APPDATA% on Windows. We
        // check that both the config-file path and the skill path
        // contain the literal `.config/opencode` segment regardless
        // of the host's `dirs::config_dir()` resolution.
        let cfg = Agent::OpenCode
            .config_path(Scope::User, None)
            .unwrap()
            .unwrap();
        let cfg_s = cfg.display().to_string().replace('\\', "/");
        assert!(
            cfg_s.contains("/.config/opencode/opencode.json"),
            "expected XDG-style ~/.config/opencode/opencode.json; got `{cfg_s}`"
        );
        let skill = Agent::OpenCode
            .skill_path(Scope::User, None)
            .unwrap()
            .unwrap();
        let skill_s = skill.display().to_string().replace('\\', "/");
        assert!(
            skill_s.contains("/.config/opencode/skills/vibevm/SKILL.md"),
            "expected XDG-style ~/.config/opencode/skills/vibevm/SKILL.md; got `{skill_s}`"
        );
    }

    #[test]
    fn config_path_both_is_internal_error() {
        let dir = tempfile::tempdir().unwrap();
        assert!(Agent::ClaudeCode.config_path(Scope::Both, Some(dir.path())).is_err());
    }

    // ---- build_mcp_entry scope-awareness ----

    #[test]
    fn project_scope_mcp_entry_carries_path_arg() {
        let dir = tempfile::tempdir().unwrap();
        let v = json_payload(Agent::ClaudeCode, Scope::Project, Some(dir.path()));
        let args: Vec<&str> = v["args"].as_array().unwrap().iter().map(|a| a.as_str().unwrap()).collect();
        assert_eq!(args[0], "mcp");
        assert_eq!(args[1], "serve");
        assert_eq!(args[2], "--path");
        assert!(args.len() == 4, "expected 4 args, got {args:?}");
    }

    #[test]
    fn user_scope_mcp_entry_omits_path_arg() {
        let v = json_payload(Agent::ClaudeCode, Scope::User, None);
        let args: Vec<&str> = v["args"].as_array().unwrap().iter().map(|a| a.as_str().unwrap()).collect();
        assert_eq!(args, vec!["mcp", "serve"], "user-scope must omit --path");
    }

    #[test]
    fn opencode_user_scope_entry_uses_command_array_without_path() {
        let v = json_payload(Agent::OpenCode, Scope::User, None);
        let cmd: Vec<&str> = v["command"].as_array().unwrap().iter().map(|a| a.as_str().unwrap()).collect();
        assert_eq!(cmd, vec!["vibe", "mcp", "serve"]);
        assert_eq!(v["type"], "local");
        assert_eq!(v["enabled"], true);
    }

    #[test]
    fn codex_user_scope_entry_returns_toml_table_without_path() {
        let v = toml_payload(Agent::Codex, Scope::User, None);
        let tbl = v.as_table().unwrap();
        assert_eq!(tbl.get("command").and_then(|x| x.as_str()), Some("vibe"));
        let args = tbl.get("args").and_then(|x| x.as_array()).unwrap();
        let strs: Vec<&str> = args.iter().filter_map(|a| a.as_str()).collect();
        assert_eq!(strs, vec!["mcp", "serve"]);
    }

    // ---- skill_path ----

    #[test]
    fn skill_path_user_works_without_project_root() {
        let p = Agent::ClaudeCode
            .skill_path(Scope::User, None)
            .unwrap()
            .unwrap();
        let s = p.display().to_string().replace('\\', "/");
        assert!(s.ends_with("/.claude/skills/vibevm/SKILL.md"), "got {s}");
    }

    #[test]
    fn skill_path_project_lands_under_project_root() {
        let dir = tempfile::tempdir().unwrap();
        let p = Agent::OpenCode
            .skill_path(Scope::Project, Some(dir.path()))
            .unwrap()
            .unwrap();
        let s = p.display().to_string().replace('\\', "/");
        assert!(s.ends_with("/.opencode/skills/vibevm/SKILL.md"), "got {s}");
    }

    // ---- install_skill ----

    #[test]
    fn install_skill_creates_under_project() {
        let dir = tempfile::tempdir().unwrap();
        let r = install_skill(Agent::ClaudeCode, Scope::Project, Some(dir.path()), false).unwrap();
        assert_eq!(r.status, "created");
        let p = dir.path().join(".claude/skills/vibevm/SKILL.md");
        assert!(p.exists());
    }

    #[test]
    fn install_skill_dry_run_no_write() {
        let dir = tempfile::tempdir().unwrap();
        let r = install_skill(Agent::OpenCode, Scope::Project, Some(dir.path()), true).unwrap();
        assert_eq!(r.status, "would-create");
        assert!(!dir.path().join(".opencode/skills/vibevm/SKILL.md").exists());
    }

    #[test]
    fn install_skill_skipped_for_unsupported_agents() {
        let dir = tempfile::tempdir().unwrap();
        let r = install_skill(Agent::Cursor, Scope::Project, Some(dir.path()), false).unwrap();
        assert_eq!(r.status, "skipped");
        assert!(r.path.is_none());
    }

    // ---- JSON merger preserves foreign keys ----

    #[test]
    fn merge_json_preserves_foreign_keys() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        std::fs::write(
            &path,
            r#"{
              "preexisting": "value",
              "mcpServers": { "other-server": { "command": "x" } }
            }"#,
        )
        .unwrap();
        let entry = json_payload(Agent::ClaudeCode, Scope::Project, Some(dir.path()));
        let merged = merge_json(&path, "mcpServers", SERVER_NAME, &entry).unwrap();
        assert_eq!(merged["preexisting"], "value");
        assert_eq!(merged["mcpServers"]["other-server"]["command"], "x");
        assert_eq!(merged["mcpServers"]["vibevm"]["command"], "vibe");
    }

    // ---- TOML merger preserves foreign keys ----

    #[test]
    fn merge_toml_preserves_top_level() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, "model = \"gpt-5\"\n").unwrap();
        let entry = toml_payload(Agent::Codex, Scope::User, None);
        let merged = merge_toml(&path, "mcp_servers", SERVER_NAME, &entry).unwrap();
        assert_eq!(merged.get("model").and_then(|x| x.as_str()), Some("gpt-5"));
        assert!(
            merged
                .get("mcp_servers")
                .and_then(|x| x.as_table())
                .and_then(|t| t.get("vibevm"))
                .is_some()
        );
    }

    // ---- has_vibe_toml gate ----

    #[test]
    fn has_vibe_toml_returns_true_when_present() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(ProjectManifest::FILENAME), "").unwrap();
        assert!(has_vibe_toml(dir.path()));
    }

    #[test]
    fn has_vibe_toml_returns_false_when_absent() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!has_vibe_toml(dir.path()));
    }

    // ---- SKILL.md template content contract ----

    #[test]
    fn skill_template_has_required_frontmatter() {
        assert!(SKILL_TEMPLATE.starts_with("---"));
        assert!(SKILL_TEMPLATE.contains("name: vibevm"));
        assert!(SKILL_TEMPLATE.contains("description: "));
    }

    #[test]
    fn skill_template_documents_mcp_tools_and_invoked_by() {
        assert!(SKILL_TEMPLATE.contains("query_package"));
        assert!(SKILL_TEMPLATE.contains("read_subskill"));
        assert!(SKILL_TEMPLATE.contains("materialise_subskill"));
        assert!(SKILL_TEMPLATE.contains("--invoked-by"));
        assert!(SKILL_TEMPLATE.contains("VIBE_INVOKED_BY"));
    }

    #[test]
    fn skill_template_covers_both_bootstrap_and_inside_project_modes() {
        // Two-state contract: the skill must explain BOTH the
        // bootstrap path (no vibe.toml present, run `vibe init`) AND
        // the inside-project path (vibe.toml present, query MCP).
        // Without both sections, an agent in an empty directory has
        // no actionable guidance for "create a vibevm project".
        let body = SKILL_TEMPLATE.to_lowercase();
        assert!(body.contains("vibe init"), "expected mention of `vibe init` for bootstrap");
        assert!(body.contains("section a"), "expected explicit Section A header for bootstrap");
        assert!(body.contains("section b"), "expected explicit Section B header for inside-project");
        assert!(
            body.contains("vibe.toml"),
            "expected the detect-step to mention vibe.toml as the discriminator"
        );
        assert!(body.contains("vibe install"));
    }

    #[test]
    fn skill_template_mentions_new_mcp_subcommands() {
        // Slice 5 added upgrade / uninstall / status subcommands and
        // the --scope / --what axes — they must appear in the help
        // section so the agent knows to consider them.
        assert!(SKILL_TEMPLATE.contains("upgrade"));
        assert!(SKILL_TEMPLATE.contains("uninstall"));
        assert!(SKILL_TEMPLATE.contains("--scope"));
        assert!(SKILL_TEMPLATE.contains("--what"));
    }

    #[test]
    fn skill_template_does_not_impose_project_conventions() {
        // Regression guard. Past versions of the skill (slice 4 +
        // slice 5 first pass) treated "read CLAUDE.md → spec/boot/*
        // → spec/WAL.md → relevant PROP/FEAT" as a binding bootstrap
        // protocol for ALL vibevm projects. That conflated this
        // repo's conventions with the package manager's contract —
        // vibevm commands work identically whether the project
        // adopts WAL discipline, PROP-style design docs, or the four
        // commit rules. None of those are part of the package
        // manager.
        //
        // The replacement skill explicitly notes that conventions
        // are out of scope and live in the project's own
        // CLAUDE.md / additional skills / installed packages
        // (e.g. flow:wal as one possible WAL protocol — not
        // mandatory). This test locks the new posture so a future
        // edit doesn't regress it back to "you MUST read X".
        let body = SKILL_TEMPLATE.to_lowercase();
        // Reading WAL must not be presented as required.
        assert!(
            !body.contains("you must read spec/wal")
                && !body.contains("required to read spec/wal")
                && !body.contains("must read `spec/wal"),
            "skill must not mandate reading spec/WAL.md as universal requirement"
        );
        // PROP/FEAT must not be required reading either.
        assert!(
            !body.contains("must consult prop")
                && !body.contains("required prop")
                && !body.contains("you must read prop"),
            "skill must not mandate reading PROP-* / FEAT-* docs"
        );
        // No "non-negotiable rules" framing — those are this repo's,
        // not the package manager's.
        assert!(
            !body.contains("non-negotiable"),
            "skill must not import this repo's non-negotiable-rules framing"
        );
        // Positive: the skill must explicitly disclaim project-
        // convention scope.
        assert!(
            body.contains("project conventions") || body.contains("project-specific")
                || body.contains("conventions"),
            "skill must name 'conventions' explicitly to disclaim them"
        );
    }
}
