//! `vibe mcp` — Model Context Protocol surface.
//!
//! Spec: PROP-004 §5.1 + ROADMAP §M1.7. Three subcommands:
//!
//! - `vibe mcp serve` — run the JSON-RPC server over stdio.
//! - `vibe mcp install` — detect coding agents and write per-agent
//!   MCP config so they pick up vibevm on next start. Idempotent.
//! - `vibe mcp status` — show what `install` would write.
//!
//! Library implementation lives in `vibe-mcp`; this module is the CLI
//! dispatch + the per-agent config writers.
//!
//! ## Agent matrix
//!
//! Five agents land per slice 4. Each carries its own (a) project-tree
//! presence markers, (b) config-file path (project-level or
//! user-level), (c) wire format (JSON or TOML), (d) MCP section key,
//! (e) per-server JSON/TOML payload shape:
//!
//! | Agent             | section       | file                                                | format | shape             |
//! |-------------------|---------------|-----------------------------------------------------|--------|-------------------|
//! | Claude Code       | `mcpServers`  | `<proj>/.claude/settings.json`                      | JSON   | `{command, args}` |
//! | Claude Desktop    | `mcpServers`  | `<config>/Claude/claude_desktop_config.json`        | JSON   | `{command, args}` |
//! | Cursor            | `mcpServers`  | `<proj>/.cursor/mcp.json`                           | JSON   | `{command, args}` |
//! | OpenCode          | `mcp`         | `<proj>/opencode.json`                              | JSON   | `{type, command:[…], enabled}` |
//! | Codex             | `mcp_servers` | `<home>/.codex/config.toml`                         | TOML   | `{command, args}` |

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use serde::Serialize;
use serde_json::{Map, Value as JsonValue};
use vibe_core::manifest::ProjectManifest;
use vibe_mcp::{Server, ServerContext};

use crate::cli::{McpArgs, McpInstallArgs, McpServeArgs, McpStatusArgs, McpSubcommand};
use crate::output;

pub fn run(ctx: &output::Context, args: McpArgs) -> Result<()> {
    match args.command {
        McpSubcommand::Serve(sub) => run_serve(sub),
        McpSubcommand::Install(sub) => run_install(ctx, sub),
        McpSubcommand::Status(sub) => run_status(ctx, sub),
    }
}

fn run_serve(args: McpServeArgs) -> Result<()> {
    let project_root = resolve_project_root(&args.path)?;
    let server_ctx = ServerContext::new(project_root);
    let mut server = Server::stdio(server_ctx);
    server.run().context("MCP server I/O error")?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Agent profile
// ---------------------------------------------------------------------------

/// Coding agent supported by `vibe mcp install`. Variants below carry
/// the full per-agent profile (markers, config path, wire format, MCP
/// section key, payload shape) via inherent methods.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum Agent {
    /// Claude Code CLI — project `.claude/settings.json`.
    ClaudeCode,
    /// Claude Desktop GUI — user-level
    /// `<config-dir>/Claude/claude_desktop_config.json`.
    ClaudeCodeDesktop,
    /// Cursor IDE — project `.cursor/mcp.json`.
    Cursor,
    /// OpenCode TUI — project `opencode.json`.
    OpenCode,
    /// Codex CLI — user-level `~/.codex/config.toml`.
    Codex,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigFormat {
    Json,
    Toml,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigLocation {
    /// Path is `project_root/<rel>`.
    Project,
    /// Path is rooted in the operator's home or config dir, independent
    /// of the project tree.
    User,
}

/// Where a `vibevm` skill artefact lives — alongside the project (in
/// the agent's project-scoped skills dir, committed to git) or in the
/// operator's home / config dir (machine-local, not in git).
//
// `#[allow(dead_code)]` until the Phase-D install UX wires this through.
// The next slice consumes every variant + method on this type.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillScope {
    Project,
    User,
}

#[allow(dead_code)]
impl SkillScope {
    pub fn as_str(self) -> &'static str {
        match self {
            SkillScope::Project => "project",
            SkillScope::User => "user",
        }
    }

    pub fn parse(value: &str) -> Result<SkillScope> {
        match value {
            "project" => Ok(SkillScope::Project),
            "user" => Ok(SkillScope::User),
            other => bail!(
                "unknown --skill-scope value `{other}` (expected `project` or `user`)"
            ),
        }
    }
}

/// Bytes of the `vibevm` SKILL.md template, vendored at compile time.
/// Living right beside `mcp.rs` keeps the template version-locked to
/// the surrounding agent + tool surface — when the CLI grows a new
/// flag the skill text travels with it through `cargo build`.
#[allow(dead_code)] // wired in Phase D
pub const SKILL_TEMPLATE: &str = include_str!("skill_template.md");

/// Skill name. Matches the `name:` frontmatter field in the template
/// and the directory name we write under each agent's skills root.
#[allow(dead_code)] // wired in Phase D
pub const SKILL_NAME: &str = "vibevm";

#[derive(Debug, Clone)]
pub enum ConfigPayload {
    Json(JsonValue),
    Toml(toml::Value),
}

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

    /// Parse `--agent <filter>` into the explicit list of agents the
    /// operator targeted. `all` expands to [`Agent::ALL`].
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

    pub fn config_location(self) -> ConfigLocation {
        match self {
            Agent::ClaudeCodeDesktop | Agent::Codex => ConfigLocation::User,
            _ => ConfigLocation::Project,
        }
    }

    /// Absolute path to the agent's MCP-config file. Project-level
    /// agents resolve under `project_root`; user-level agents resolve
    /// against `dirs::config_dir()` / `dirs::home_dir()` independently
    /// of the project.
    pub fn config_path(self, project_root: &Path) -> Result<PathBuf> {
        match self {
            Agent::ClaudeCode => Ok(project_root.join(".claude/settings.json")),
            Agent::Cursor => Ok(project_root.join(".cursor/mcp.json")),
            Agent::OpenCode => Ok(project_root.join("opencode.json")),
            Agent::ClaudeCodeDesktop => {
                let dir = dirs::config_dir().ok_or_else(|| {
                    anyhow!("could not resolve user-config dir for Claude Desktop")
                })?;
                Ok(dir.join("Claude").join("claude_desktop_config.json"))
            }
            Agent::Codex => {
                let home = dirs::home_dir()
                    .ok_or_else(|| anyhow!("could not resolve home dir for Codex"))?;
                Ok(home.join(".codex").join("config.toml"))
            }
        }
    }

    pub fn mcp_section_key(self) -> &'static str {
        match self {
            Agent::OpenCode => "mcp",
            Agent::Codex => "mcp_servers",
            _ => "mcpServers",
        }
    }

    /// Wire shape of the per-server entry. Three flavours:
    /// - JSON `{command: "vibe", args: [...]}` for Claude Code, Claude
    ///   Desktop, Cursor.
    /// - JSON `{type: "local", command: [...], enabled: true}` for
    ///   OpenCode (single command-array, plus `type` discriminator).
    /// - TOML `command = "vibe"` + `args = [...]` for Codex.
    pub fn build_mcp_entry(self, project_root: &Path) -> ConfigPayload {
        let project_str = project_root.display().to_string().replace('\\', "/");
        match self {
            Agent::ClaudeCode | Agent::ClaudeCodeDesktop | Agent::Cursor => {
                ConfigPayload::Json(serde_json::json!({
                    "command": "vibe",
                    "args": ["mcp", "serve", "--path", project_str],
                }))
            }
            Agent::OpenCode => ConfigPayload::Json(serde_json::json!({
                "type": "local",
                "command": ["vibe", "mcp", "serve", "--path", project_str],
                "enabled": true,
            })),
            Agent::Codex => {
                let mut tbl = toml::value::Table::new();
                tbl.insert("command".into(), toml::Value::String("vibe".into()));
                tbl.insert(
                    "args".into(),
                    toml::Value::Array(vec![
                        toml::Value::String("mcp".into()),
                        toml::Value::String("serve".into()),
                        toml::Value::String("--path".into()),
                        toml::Value::String(project_str),
                    ]),
                );
                ConfigPayload::Toml(toml::Value::Table(tbl))
            }
        }
    }

    /// Project-relative paths whose presence in the working tree marks
    /// the agent as actively used. Empty for user-level-only agents
    /// (Claude Desktop, Codex), which rely on `host_present` instead.
    pub fn presence_markers(self) -> &'static [&'static str] {
        match self {
            Agent::ClaudeCode => &[".claude", "CLAUDE.md"],
            Agent::Cursor => &[".cursor", ".cursorrules"],
            Agent::OpenCode => &[".opencode", "opencode.json", "opencode.jsonc", "AGENTS.md"],
            Agent::ClaudeCodeDesktop | Agent::Codex => &[],
        }
    }

    /// Cheap presence probe for user-level agents: their config-file
    /// parent dir exists. The OS creates `%APPDATA%\Claude` /
    /// `~/.codex/` only after the agent has run on this machine, so
    /// the parent's existence is a reliable "installed and used"
    /// signal that does not require running the agent's own binary.
    pub fn host_present(self) -> bool {
        if self.config_location() != ConfigLocation::User {
            return false;
        }
        match self.config_path(Path::new(".")) {
            Ok(cfg) => cfg.parent().map(|p| p.exists()).unwrap_or(false),
            Err(_) => false,
        }
    }

    /// Combined detection: project markers OR (for user-level agents)
    /// the host-presence probe.
    pub fn is_present(self, project_root: &Path) -> bool {
        for m in self.presence_markers() {
            if project_root.join(m).exists() {
                return true;
            }
        }
        self.host_present()
    }

    /// Whether the agent loads filesystem-backed skill files
    /// (`<dir>/<name>/SKILL.md` with YAML frontmatter). Cursor and
    /// Claude Desktop are JSON-config-only — they have no on-disk skill
    /// loader, so [`Agent::skill_path`] returns `None` and
    /// `vibe mcp install --with-skill` reports them as `skipped`.
    #[allow(dead_code)] // wired in Phase D
    pub fn supports_skill(self) -> bool {
        match self {
            Agent::ClaudeCode | Agent::OpenCode | Agent::Codex => true,
            Agent::ClaudeCodeDesktop | Agent::Cursor => false,
        }
    }

    /// Filesystem path the skill artefact should land at, given a
    /// [`SkillScope`]. Returns `Ok(None)` for agents that do not
    /// support filesystem skills (Cursor, Claude Desktop). Returns
    /// `Err(...)` if the host-config dir cannot be resolved (HOME /
    /// XDG_CONFIG_HOME / APPDATA missing) — that is a hard failure
    /// because the operator explicitly asked for `--skill-scope user`.
    ///
    /// Per-agent layout (see PROP-004 §5.1 / WAL slice 4 notes):
    ///
    /// | Agent       | Project scope                                         | User scope                                            |
    /// |-------------|-------------------------------------------------------|-------------------------------------------------------|
    /// | Claude Code | `<project>/.claude/skills/vibevm/SKILL.md`            | `<home>/.claude/skills/vibevm/SKILL.md`               |
    /// | OpenCode    | `<project>/.opencode/skills/vibevm/SKILL.md`          | `<config-dir>/opencode/skills/vibevm/SKILL.md`        |
    /// | Codex       | `<project>/.agents/skills/vibevm/SKILL.md`            | `<home>/.agents/skills/vibevm/SKILL.md`               |
    #[allow(dead_code)] // wired in Phase D
    pub fn skill_path(
        self,
        scope: SkillScope,
        project_root: &Path,
    ) -> Result<Option<PathBuf>> {
        if !self.supports_skill() {
            return Ok(None);
        }
        let path = match (self, scope) {
            (Agent::ClaudeCode, SkillScope::Project) => project_root
                .join(".claude")
                .join("skills")
                .join(SKILL_NAME)
                .join("SKILL.md"),
            (Agent::ClaudeCode, SkillScope::User) => {
                let home = dirs::home_dir().ok_or_else(|| {
                    anyhow!("could not resolve home dir for Claude Code skill")
                })?;
                home.join(".claude")
                    .join("skills")
                    .join(SKILL_NAME)
                    .join("SKILL.md")
            }
            (Agent::OpenCode, SkillScope::Project) => project_root
                .join(".opencode")
                .join("skills")
                .join(SKILL_NAME)
                .join("SKILL.md"),
            (Agent::OpenCode, SkillScope::User) => {
                let cfg = dirs::config_dir().ok_or_else(|| {
                    anyhow!("could not resolve user-config dir for OpenCode skill")
                })?;
                cfg.join("opencode")
                    .join("skills")
                    .join(SKILL_NAME)
                    .join("SKILL.md")
            }
            (Agent::Codex, SkillScope::Project) => project_root
                .join(".agents")
                .join("skills")
                .join(SKILL_NAME)
                .join("SKILL.md"),
            (Agent::Codex, SkillScope::User) => {
                let home = dirs::home_dir()
                    .ok_or_else(|| anyhow!("could not resolve home dir for Codex skill"))?;
                home.join(".agents")
                    .join("skills")
                    .join(SKILL_NAME)
                    .join("SKILL.md")
            }
            (Agent::Cursor | Agent::ClaudeCodeDesktop, _) => {
                return Ok(None);
            }
        };
        Ok(Some(path))
    }
}

/// Detect every supported agent that has any presence-marker in the
/// project tree or, for user-level agents, an existing config dir on
/// this machine.
pub fn detect_agents(project_root: &Path) -> Vec<Agent> {
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
    pub config_path: String,
    /// `created` / `updated` / `unchanged` / `would-create` /
    /// `would-update` (dry-run / status) / `skipped`.
    pub status: &'static str,
    /// Human-readable explanation when status carries a note.
    pub note: Option<String>,
}

#[derive(Debug, Serialize)]
struct InstallReport {
    ok: bool,
    command: &'static str,
    project: String,
    detected: Vec<String>,
    targeted: Vec<String>,
    results: Vec<AgentInstallReport>,
    dry_run: bool,
}

// ---------------------------------------------------------------------------
// install / status
// ---------------------------------------------------------------------------

fn run_install(ctx: &output::Context, args: McpInstallArgs) -> Result<()> {
    let project_root = resolve_project_root(&args.path)?;
    let detected = detect_agents(&project_root);
    let filter = Agent::parse_filter(&args.agent)?;
    let targeted: Vec<Agent> = filter
        .iter()
        .copied()
        .filter(|a| args.force || detected.contains(a))
        .collect();

    let mut results: Vec<AgentInstallReport> = Vec::new();
    for agent in &targeted {
        let path = agent.config_path(&project_root)?;
        let payload = agent.build_mcp_entry(&project_root);
        let outcome = if args.dry_run {
            preview_install(*agent, &path, &payload)?
        } else {
            apply_install(*agent, &path, &payload)?
        };
        results.push(outcome);
    }

    let report = InstallReport {
        ok: true,
        command: "mcp:install",
        project: project_root.display().to_string(),
        detected: detected.iter().map(|a| a.as_str().to_string()).collect(),
        targeted: targeted.iter().map(|a| a.as_str().to_string()).collect(),
        results: results.clone(),
        dry_run: args.dry_run,
    };

    if ctx.is_json() {
        ctx.emit_json(&report)?;
        return Ok(());
    }
    if ctx.is_quiet() {
        let written = results
            .iter()
            .filter(|r| matches!(r.status, "created" | "updated"))
            .count();
        ctx.summary(&format!(
            "vibe mcp install: {written} agent config{} {}",
            if written == 1 { "" } else { "s" },
            if args.dry_run { "previewed" } else { "written" }
        ));
        return Ok(());
    }

    if results.is_empty() {
        ctx.summary(&format!(
            "no supported agents detected in `{}` (Claude Code, Claude Desktop, Cursor, OpenCode, Codex). Use `--force` to provision regardless.",
            project_root.display()
        ));
        return Ok(());
    }
    for r in &results {
        let prefix = if args.dry_run { "would" } else { r.status };
        let note = r
            .note
            .as_deref()
            .map(|n| format!(" ({n})"))
            .unwrap_or_default();
        ctx.step(&format!(
            "{} update  {}  → {}{note}",
            prefix, r.agent, r.config_path
        ));
    }
    Ok(())
}

fn run_status(ctx: &output::Context, args: McpStatusArgs) -> Result<()> {
    let project_root = resolve_project_root(&args.path)?;
    let detected = detect_agents(&project_root);
    let mut results: Vec<AgentInstallReport> = Vec::new();
    for agent in Agent::ALL.iter().copied() {
        let path = agent.config_path(&project_root)?;
        let payload = agent.build_mcp_entry(&project_root);
        let outcome = preview_install(agent, &path, &payload)?;
        results.push(outcome);
    }
    let report = InstallReport {
        ok: true,
        command: "mcp:status",
        project: project_root.display().to_string(),
        detected: detected.iter().map(|a| a.as_str().to_string()).collect(),
        targeted: vec![],
        results: results.clone(),
        dry_run: true,
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
            "{}  {}  → {}{note}",
            r.status, r.agent, r.config_path
        ));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// decide / preview / apply / merge — JSON + TOML
// ---------------------------------------------------------------------------

const SERVER_NAME: &str = "vibevm";

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
                None => Ok((
                    "updated",
                    Some(format!("[{section}.{SERVER_NAME}] absent")),
                )),
            }
        }
        _ => bail!(
            "internal: agent `{}` config_format/payload mismatch",
            agent.as_str()
        ),
    }
}

fn preview_install(
    agent: Agent,
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
        config_path: config_path.display().to_string().replace('\\', "/"),
        status: dry,
        note,
    })
}

fn apply_install(
    agent: Agent,
    config_path: &Path,
    payload: &ConfigPayload,
) -> Result<AgentInstallReport> {
    let (status, note) = decide_action(agent, config_path, payload)?;
    if status == "unchanged" {
        return Ok(AgentInstallReport {
            agent: agent.as_str().to_string(),
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
        config_path: config_path.display().to_string().replace('\\', "/"),
        status,
        note,
    })
}

fn read_json(path: &Path) -> Result<JsonValue> {
    let text =
        fs::read_to_string(path).with_context(|| format!("reading `{}`", path.display()))?;
    if text.trim().is_empty() {
        return Ok(JsonValue::Object(Map::new()));
    }
    let v: JsonValue = serde_json::from_str(&text)
        .with_context(|| format!("parsing JSON `{}`", path.display()))?;
    Ok(v)
}

fn read_toml(path: &Path) -> Result<toml::Value> {
    let text =
        fs::read_to_string(path).with_context(|| format!("reading `{}`", path.display()))?;
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

fn resolve_project_root(path: &Path) -> Result<PathBuf> {
    let canonical = path
        .canonicalize()
        .with_context(|| format!("canonicalizing `{}`", path.display()))?;
    let stripped = super::init::strip_unc_public(canonical);
    if !stripped.join(ProjectManifest::FILENAME).exists() {
        bail!(
            "no `vibe.toml` in `{}`; run `vibe init` first or pass `--path <dir>`",
            stripped.display()
        );
    }
    Ok(stripped)
}

// ---------------------------------------------------------------------------
// Skill artefact — per-agent SKILL.md writer
// ---------------------------------------------------------------------------

#[allow(dead_code)] // wired in Phase D
#[derive(Debug, Clone, Serialize)]
pub struct SkillInstallReport {
    pub agent: String,
    pub scope: &'static str,
    /// Resolved path, if the agent supports filesystem skills.
    pub path: Option<String>,
    /// `created` / `updated` / `unchanged` / `would-create` /
    /// `would-update` / `skipped`.
    pub status: &'static str,
    pub note: Option<String>,
}

/// Write the `vibevm` SKILL.md to the agent's skill directory under
/// the chosen scope. Idempotent: re-running with byte-identical output
/// reports `unchanged`. Cursor / Claude Desktop are reported as
/// `skipped` because they have no filesystem skill loader.
#[allow(dead_code)] // wired in Phase D
pub fn install_skill(
    agent: Agent,
    scope: SkillScope,
    project_root: &Path,
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
                "agent `{}` does not load filesystem skills",
                agent.as_str()
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
            fs::create_dir_all(parent).with_context(|| {
                format!("creating skill dir `{}`", parent.display())
            })?;
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

#[allow(dead_code)] // wired in Phase D
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

#[cfg(test)]
mod tests {
    use super::*;

    fn json_payload(agent: Agent, project: &Path) -> JsonValue {
        match agent.build_mcp_entry(project) {
            ConfigPayload::Json(v) => v,
            ConfigPayload::Toml(_) => panic!("expected JSON payload for {}", agent.as_str()),
        }
    }

    fn toml_payload(agent: Agent, project: &Path) -> toml::Value {
        match agent.build_mcp_entry(project) {
            ConfigPayload::Toml(v) => v,
            ConfigPayload::Json(_) => panic!("expected TOML payload for {}", agent.as_str()),
        }
    }

    // ---- detection ----

    #[test]
    fn detect_finds_claude_via_marker_dir() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join(".claude")).unwrap();
        let agents = detect_agents(dir.path());
        assert!(agents.contains(&Agent::ClaudeCode));
    }

    #[test]
    fn detect_finds_claude_via_claude_md() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("CLAUDE.md"), "x").unwrap();
        let agents = detect_agents(dir.path());
        assert!(agents.contains(&Agent::ClaudeCode));
    }

    #[test]
    fn detect_finds_cursor_via_marker_dir() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join(".cursor")).unwrap();
        let agents = detect_agents(dir.path());
        assert!(agents.contains(&Agent::Cursor));
    }

    #[test]
    fn detect_finds_opencode_via_marker_dir() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join(".opencode")).unwrap();
        let agents = detect_agents(dir.path());
        assert!(agents.contains(&Agent::OpenCode));
    }

    #[test]
    fn detect_finds_opencode_via_opencode_json() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("opencode.json"), "{}").unwrap();
        let agents = detect_agents(dir.path());
        assert!(agents.contains(&Agent::OpenCode));
    }

    #[test]
    fn detect_finds_opencode_via_opencode_jsonc() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("opencode.jsonc"), "{}").unwrap();
        let agents = detect_agents(dir.path());
        assert!(agents.contains(&Agent::OpenCode));
    }

    #[test]
    fn detect_finds_opencode_via_agents_md() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("AGENTS.md"), "x").unwrap();
        let agents = detect_agents(dir.path());
        assert!(agents.contains(&Agent::OpenCode));
    }

    // ---- parse_filter ----

    #[test]
    fn parse_filter_known_values() {
        assert_eq!(Agent::parse_filter("all").unwrap(), Agent::ALL.to_vec());
        assert_eq!(
            Agent::parse_filter("claude").unwrap(),
            vec![Agent::ClaudeCode]
        );
        assert_eq!(
            Agent::parse_filter("claude-code").unwrap(),
            vec![Agent::ClaudeCode]
        );
        assert_eq!(
            Agent::parse_filter("claude-desktop").unwrap(),
            vec![Agent::ClaudeCodeDesktop]
        );
        assert_eq!(
            Agent::parse_filter("claude-code-desktop").unwrap(),
            vec![Agent::ClaudeCodeDesktop]
        );
        assert_eq!(Agent::parse_filter("cursor").unwrap(), vec![Agent::Cursor]);
        assert_eq!(
            Agent::parse_filter("opencode").unwrap(),
            vec![Agent::OpenCode]
        );
        assert_eq!(Agent::parse_filter("codex").unwrap(), vec![Agent::Codex]);
        assert!(Agent::parse_filter("nope").is_err());
    }

    // ---- per-agent profiles ----

    #[test]
    fn config_format_codex_is_toml_others_json() {
        assert_eq!(Agent::Codex.config_format(), ConfigFormat::Toml);
        for &a in Agent::ALL {
            if a != Agent::Codex {
                assert_eq!(a.config_format(), ConfigFormat::Json, "{}", a.as_str());
            }
        }
    }

    #[test]
    fn config_location_user_only_for_desktop_and_codex() {
        for &a in Agent::ALL {
            let want = matches!(a, Agent::ClaudeCodeDesktop | Agent::Codex);
            let got = a.config_location() == ConfigLocation::User;
            assert_eq!(want, got, "{}", a.as_str());
        }
    }

    #[test]
    fn mcp_section_keys_match_per_agent_convention() {
        assert_eq!(Agent::ClaudeCode.mcp_section_key(), "mcpServers");
        assert_eq!(Agent::ClaudeCodeDesktop.mcp_section_key(), "mcpServers");
        assert_eq!(Agent::Cursor.mcp_section_key(), "mcpServers");
        assert_eq!(Agent::OpenCode.mcp_section_key(), "mcp");
        assert_eq!(Agent::Codex.mcp_section_key(), "mcp_servers");
    }

    // ---- payload shape ----

    #[test]
    fn claude_code_entry_has_command_and_args() {
        let dir = tempfile::tempdir().unwrap();
        let v = json_payload(Agent::ClaudeCode, dir.path());
        assert_eq!(v["command"], "vibe");
        assert!(v["args"].is_array());
        assert_eq!(v["args"][0], "mcp");
        assert_eq!(v["args"][1], "serve");
        assert_eq!(v["args"][2], "--path");
    }

    #[test]
    fn opencode_entry_uses_command_array_with_type_local() {
        let dir = tempfile::tempdir().unwrap();
        let v = json_payload(Agent::OpenCode, dir.path());
        assert_eq!(v["type"], "local");
        assert_eq!(v["enabled"], true);
        assert!(v["command"].is_array(), "command must be an array, got {v}");
        assert_eq!(v["command"][0], "vibe");
        assert_eq!(v["command"][1], "mcp");
        assert!(v.get("args").is_none(), "OpenCode shape must NOT split args");
    }

    #[test]
    fn codex_entry_returns_toml_table_with_command_and_args() {
        let dir = tempfile::tempdir().unwrap();
        let v = toml_payload(Agent::Codex, dir.path());
        let tbl = v.as_table().expect("codex entry must be a TOML table");
        assert_eq!(
            tbl.get("command")
                .and_then(|x| x.as_str())
                .unwrap_or_default(),
            "vibe"
        );
        let args = tbl
            .get("args")
            .and_then(|x| x.as_array())
            .expect("args must be an array");
        assert_eq!(args[0].as_str(), Some("mcp"));
        assert_eq!(args[1].as_str(), Some("serve"));
        assert_eq!(args[2].as_str(), Some("--path"));
    }

    // ---- JSON merger ----

    #[test]
    fn merge_json_inserts_into_empty_file_for_claude() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let entry = json_payload(Agent::ClaudeCode, dir.path());
        let merged = merge_json(&path, "mcpServers", SERVER_NAME, &entry).unwrap();
        assert_eq!(merged["mcpServers"]["vibevm"]["command"], "vibe");
    }

    #[test]
    fn merge_json_preserves_existing_keys() {
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
        let entry = json_payload(Agent::ClaudeCode, dir.path());
        let merged = merge_json(&path, "mcpServers", SERVER_NAME, &entry).unwrap();
        assert_eq!(merged["preexisting"], "value");
        assert_eq!(merged["mcpServers"]["other-server"]["command"], "x");
        assert_eq!(merged["mcpServers"]["vibevm"]["command"], "vibe");
    }

    #[test]
    fn merge_json_uses_mcp_section_for_opencode() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("opencode.json");
        let entry = json_payload(Agent::OpenCode, dir.path());
        let merged = merge_json(&path, "mcp", SERVER_NAME, &entry).unwrap();
        assert_eq!(merged["mcp"]["vibevm"]["type"], "local");
        assert_eq!(merged["mcp"]["vibevm"]["enabled"], true);
        assert!(merged["mcp"]["vibevm"]["command"].is_array());
    }

    // ---- TOML merger ----

    #[test]
    fn merge_toml_creates_mcp_servers_table_for_codex() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let entry = toml_payload(Agent::Codex, dir.path());
        let merged = merge_toml(&path, "mcp_servers", SERVER_NAME, &entry).unwrap();
        let v = merged
            .get("mcp_servers")
            .and_then(|x| x.as_table())
            .and_then(|t| t.get("vibevm"))
            .and_then(|x| x.as_table())
            .expect("[mcp_servers.vibevm] must exist");
        assert_eq!(v.get("command").and_then(|x| x.as_str()), Some("vibe"));
    }

    #[test]
    fn merge_toml_preserves_existing_top_level_keys() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, "model = \"gpt-5\"\n[mcp_servers.other]\ncommand = \"x\"\n")
            .unwrap();
        let entry = toml_payload(Agent::Codex, dir.path());
        let merged = merge_toml(&path, "mcp_servers", SERVER_NAME, &entry).unwrap();
        assert_eq!(
            merged.get("model").and_then(|x| x.as_str()),
            Some("gpt-5")
        );
        let servers = merged
            .get("mcp_servers")
            .and_then(|x| x.as_table())
            .unwrap();
        assert!(servers.contains_key("other"));
        assert!(servers.contains_key("vibevm"));
    }

    // ---- decide_action ----

    #[test]
    fn decide_action_reports_created_for_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nope.json");
        let payload = Agent::ClaudeCode.build_mcp_entry(dir.path());
        let (status, _) = decide_action(Agent::ClaudeCode, &path, &payload).unwrap();
        assert_eq!(status, "created");
    }

    #[test]
    fn decide_action_reports_unchanged_when_block_matches_for_opencode() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("opencode.json");
        let payload = Agent::OpenCode.build_mcp_entry(dir.path());
        let entry = match &payload {
            ConfigPayload::Json(v) => v.clone(),
            _ => panic!(),
        };
        let merged = merge_json(&path, "mcp", SERVER_NAME, &entry).unwrap();
        std::fs::write(&path, serde_json::to_string_pretty(&merged).unwrap()).unwrap();
        let (status, _) = decide_action(Agent::OpenCode, &path, &payload).unwrap();
        assert_eq!(status, "unchanged");
    }

    #[test]
    fn decide_action_reports_updated_when_block_differs_for_codex() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(
            &path,
            "[mcp_servers.vibevm]\ncommand = \"old\"\nargs = []\n",
        )
        .unwrap();
        let payload = Agent::Codex.build_mcp_entry(dir.path());
        let (status, _) = decide_action(Agent::Codex, &path, &payload).unwrap();
        assert_eq!(status, "updated");
    }

    // ---- skill scope + supports_skill ----

    #[test]
    fn skill_scope_parse_known_values() {
        assert_eq!(SkillScope::parse("project").unwrap(), SkillScope::Project);
        assert_eq!(SkillScope::parse("user").unwrap(), SkillScope::User);
        assert!(SkillScope::parse("global").is_err());
    }

    #[test]
    fn supports_skill_only_for_claude_opencode_codex() {
        assert!(Agent::ClaudeCode.supports_skill());
        assert!(Agent::OpenCode.supports_skill());
        assert!(Agent::Codex.supports_skill());
        assert!(!Agent::ClaudeCodeDesktop.supports_skill());
        assert!(!Agent::Cursor.supports_skill());
    }

    // ---- skill_path ----

    #[test]
    fn skill_path_project_scope_lands_in_per_agent_dir() {
        let dir = tempfile::tempdir().unwrap();
        let p = Agent::ClaudeCode
            .skill_path(SkillScope::Project, dir.path())
            .unwrap()
            .unwrap();
        let s = p.display().to_string().replace('\\', "/");
        assert!(s.ends_with("/.claude/skills/vibevm/SKILL.md"), "got {s}");

        let p = Agent::OpenCode
            .skill_path(SkillScope::Project, dir.path())
            .unwrap()
            .unwrap();
        let s = p.display().to_string().replace('\\', "/");
        assert!(s.ends_with("/.opencode/skills/vibevm/SKILL.md"), "got {s}");

        let p = Agent::Codex
            .skill_path(SkillScope::Project, dir.path())
            .unwrap()
            .unwrap();
        let s = p.display().to_string().replace('\\', "/");
        assert!(s.ends_with("/.agents/skills/vibevm/SKILL.md"), "got {s}");
    }

    #[test]
    fn skill_path_returns_none_for_unsupported_agents() {
        let dir = tempfile::tempdir().unwrap();
        assert!(
            Agent::Cursor
                .skill_path(SkillScope::Project, dir.path())
                .unwrap()
                .is_none()
        );
        assert!(
            Agent::Cursor
                .skill_path(SkillScope::User, dir.path())
                .unwrap()
                .is_none()
        );
        assert!(
            Agent::ClaudeCodeDesktop
                .skill_path(SkillScope::Project, dir.path())
                .unwrap()
                .is_none()
        );
        assert!(
            Agent::ClaudeCodeDesktop
                .skill_path(SkillScope::User, dir.path())
                .unwrap()
                .is_none()
        );
    }

    // ---- skill template content ----

    #[test]
    fn skill_template_has_required_frontmatter() {
        assert!(SKILL_TEMPLATE.starts_with("---"));
        assert!(SKILL_TEMPLATE.contains("name: vibevm"));
        assert!(SKILL_TEMPLATE.contains("description: "));
    }

    #[test]
    fn skill_template_documents_invoked_by_and_mcp_tools() {
        // Hard-coded contract: the skill must mention the three MCP
        // tools by name and the `--invoked-by` flag, otherwise the
        // agent has nothing actionable to invoke.
        assert!(SKILL_TEMPLATE.contains("query_package"));
        assert!(SKILL_TEMPLATE.contains("read_subskill"));
        assert!(SKILL_TEMPLATE.contains("materialise_subskill"));
        assert!(SKILL_TEMPLATE.contains("--invoked-by"));
        assert!(SKILL_TEMPLATE.contains("VIBE_INVOKED_BY"));
    }

    // ---- install_skill writer ----

    #[test]
    fn install_skill_creates_file_with_template_body() {
        let dir = tempfile::tempdir().unwrap();
        let report = install_skill(Agent::OpenCode, SkillScope::Project, dir.path(), false)
            .unwrap();
        assert_eq!(report.status, "created");
        assert_eq!(report.scope, "project");
        let path = dir.path().join(".opencode/skills/vibevm/SKILL.md");
        assert!(path.exists(), "expected skill at {}", path.display());
        let body = std::fs::read_to_string(&path).unwrap();
        assert_eq!(body, SKILL_TEMPLATE);
    }

    #[test]
    fn install_skill_is_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let r1 =
            install_skill(Agent::ClaudeCode, SkillScope::Project, dir.path(), false).unwrap();
        assert_eq!(r1.status, "created");
        let r2 =
            install_skill(Agent::ClaudeCode, SkillScope::Project, dir.path(), false).unwrap();
        assert_eq!(r2.status, "unchanged");
    }

    #[test]
    fn install_skill_detects_drift_and_reports_updated() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(".agents/skills/vibevm/SKILL.md");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "stale content").unwrap();
        let report =
            install_skill(Agent::Codex, SkillScope::Project, dir.path(), false).unwrap();
        assert_eq!(report.status, "updated");
        let body = std::fs::read_to_string(&path).unwrap();
        assert_eq!(body, SKILL_TEMPLATE);
    }

    #[test]
    fn install_skill_dry_run_does_not_write() {
        let dir = tempfile::tempdir().unwrap();
        let report =
            install_skill(Agent::ClaudeCode, SkillScope::Project, dir.path(), true).unwrap();
        assert_eq!(report.status, "would-create");
        let path = dir.path().join(".claude/skills/vibevm/SKILL.md");
        assert!(
            !path.exists(),
            "dry-run must not write; expected {} absent",
            path.display()
        );
    }

    #[test]
    fn install_skill_skips_unsupported_agents() {
        let dir = tempfile::tempdir().unwrap();
        let report =
            install_skill(Agent::Cursor, SkillScope::Project, dir.path(), false).unwrap();
        assert_eq!(report.status, "skipped");
        assert!(report.path.is_none());

        let report = install_skill(
            Agent::ClaudeCodeDesktop,
            SkillScope::User,
            dir.path(),
            false,
        )
        .unwrap();
        assert_eq!(report.status, "skipped");
        assert!(report.path.is_none());
    }
}
