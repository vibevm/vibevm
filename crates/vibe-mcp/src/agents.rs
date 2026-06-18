//! Agent profiles and detection (PROP-015 §2.4, §2.5). The fixed set of
//! MCP-capable coding agents, each declaring its config shape (JSON vs
//! TOML, section key, scope support, on-disk paths) and its presence
//! markers. `vibe mcp install` and friends consume these; the CLI keeps
//! only argument parsing, the confirm/render UX, and the lifecycle
//! drivers.

specmark::scope!("spec://vibevm/modules/vibe-mcp/PROP-015#agent-config");

use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow, bail};
use serde_json::Value as JsonValue;
use specmark::spec;

/// Where a vibevm artefact (MCP-config block or SKILL.md) lives. The
/// install / upgrade / uninstall surface accept this through `--scope`;
/// the wizard asks via the first prompt.
///
/// ```
/// use vibe_mcp::agents::Scope;
/// assert_eq!(Scope::parse("project").unwrap(), Scope::Project);
/// // `both` expands into the two physical scopes a walk visits.
/// assert_eq!(Scope::Both.expand(), vec![Scope::Project, Scope::User]);
/// ```
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
            other => {
                bail!("unknown --scope value `{other}` (expected `project`, `user`, or `both`)")
            }
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

    /// Whether installing under this scope **requires** a `vibe.toml`
    /// in the working directory. Only `Project` — operator explicitly
    /// asked for project-only and there's no project to write into,
    /// so refuse. `User` doesn't need one (writes to home /
    /// `<config-dir>`); `Both` is best-effort — the user-leg always
    /// runs, the project-leg is silently skipped when no `vibe.toml`
    /// is present (matches the same model in `vibe mcp upgrade` /
    /// `vibe mcp uninstall` and supports the unattended-provisioning
    /// workflow on a fresh machine).
    pub fn requires_vibe_toml(self) -> bool {
        matches!(self, Scope::Project)
    }
}

/// What to install / uninstall — MCP server entry, SKILL.md, or both.
///
/// ```
/// use vibe_mcp::agents::What;
/// assert_eq!(What::parse("mcp").unwrap(), What::Mcp);
/// assert!(What::Both.includes_skill());
/// ```
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
            other => bail!("unknown --what value `{other}` (expected `mcp`, `skill`, or `both`)"),
        }
    }

    pub fn includes_mcp(self) -> bool {
        matches!(self, What::Mcp | What::Both)
    }

    pub fn includes_skill(self) -> bool {
        matches!(self, What::Skill | What::Both)
    }
}

/// Skill name. Matches the `name:` frontmatter in the SKILL.md template
/// and the directory name written under each agent's skills root.
pub const SKILL_NAME: &str = "vibevm";

/// Coding agent supported by `vibe mcp install` (PROP-015 §2.4).
///
/// ```
/// use vibe_mcp::agents::Agent;
/// assert_eq!(Agent::ClaudeCode.as_str(), "claude");
/// assert_eq!(Agent::parse_filter("all").unwrap().len(), 5);
/// // Codex configures via TOML; the others via JSON.
/// assert!(!Agent::Codex.supports_project_scope()); // user-only
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum Agent {
    ClaudeCode,
    ClaudeCodeDesktop,
    Cursor,
    OpenCode,
    Codex,
}

/// JSON or TOML — the config-file format an agent reads.
///
/// ```
/// use vibe_mcp::agents::ConfigFormat;
/// assert_ne!(ConfigFormat::Json, ConfigFormat::Toml);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigFormat {
    Json,
    Toml,
}

/// A per-server config block, in whichever format the target agent uses.
///
/// ```
/// use vibe_mcp::agents::ConfigPayload;
/// let p = ConfigPayload::Json(serde_json::json!({ "command": "vibe" }));
/// assert!(matches!(p, ConfigPayload::Json(_)));
/// ```
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
    pub fn config_path(self, scope: Scope, project_root: Option<&Path>) -> Result<Option<PathBuf>> {
        match (self, scope) {
            (_, Scope::Both) => {
                bail!("internal: Agent::config_path requires concrete scope; expand Both first")
            }
            // ---- Project scope ----
            (Agent::ClaudeCode, Scope::Project) => {
                // Claude Code discovers a project's MCP servers from the
                // committed `<project>/.mcp.json` — NOT `.claude/settings.json`,
                // which only *gates* servers (`enabledMcpjsonServers`) and
                // never defines them.
                Ok(project_root.map(|p| p.join(".mcp.json")))
            }
            (Agent::Cursor, Scope::Project) => {
                Ok(project_root.map(|p| p.join(".cursor").join("mcp.json")))
            }
            (Agent::OpenCode, Scope::Project) => Ok(project_root.map(|p| p.join("opencode.json"))),
            (Agent::ClaudeCodeDesktop | Agent::Codex, Scope::Project) => Ok(None),
            // ---- User scope ----
            (Agent::ClaudeCode, Scope::User) => {
                // User-scope MCP servers live in the top-level `mcpServers`
                // of `~/.claude.json` (exactly what `claude mcp add --scope
                // user` writes); Claude Code does not read server definitions
                // from `~/.claude/settings.json`.
                let home = dirs::home_dir()
                    .ok_or_else(|| anyhow!("could not resolve home dir for Claude Code"))?;
                Ok(Some(home.join(".claude.json")))
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
    pub fn skill_path(self, scope: Scope, project_root: Option<&Path>) -> Result<Option<PathBuf>> {
        if !self.supports_skill() {
            return Ok(None);
        }
        match (self, scope) {
            (_, Scope::Both) => {
                bail!("internal: Agent::skill_path requires concrete scope; expand Both first")
            }
            // ---- Project scope ----
            (Agent::ClaudeCode, Scope::Project) => Ok(project_root.map(|p| {
                p.join(".claude")
                    .join("skills")
                    .join(SKILL_NAME)
                    .join("SKILL.md")
            })),
            (Agent::OpenCode, Scope::Project) => Ok(project_root.map(|p| {
                p.join(".opencode")
                    .join("skills")
                    .join(SKILL_NAME)
                    .join("SKILL.md")
            })),
            (Agent::Codex, Scope::Project) => Ok(project_root.map(|p| {
                p.join(".agents")
                    .join("skills")
                    .join(SKILL_NAME)
                    .join("SKILL.md")
            })),
            (Agent::Cursor | Agent::ClaudeCodeDesktop, _) => Ok(None),
            // ---- User scope ----
            (Agent::ClaudeCode, Scope::User) => {
                let home = dirs::home_dir()
                    .ok_or_else(|| anyhow!("could not resolve home dir for Claude Code skill"))?;
                Ok(Some(
                    home.join(".claude")
                        .join("skills")
                        .join(SKILL_NAME)
                        .join("SKILL.md"),
                ))
            }
            (Agent::OpenCode, Scope::User) => {
                // Same XDG-on-every-OS contract as Agent::config_path
                // for OpenCode — see the comment there. Empirically
                // verified that opencode reads `~/.config/opencode/`
                // on Windows, NOT `%APPDATA%\opencode\`.
                let home = dirs::home_dir()
                    .ok_or_else(|| anyhow!("could not resolve home dir for OpenCode skill"))?;
                Ok(Some(
                    home.join(".config")
                        .join("opencode")
                        .join("skills")
                        .join(SKILL_NAME)
                        .join("SKILL.md"),
                ))
            }
            (Agent::Codex, Scope::User) => {
                let home = dirs::home_dir()
                    .ok_or_else(|| anyhow!("could not resolve home dir for Codex skill"))?;
                Ok(Some(
                    home.join(".agents")
                        .join("skills")
                        .join(SKILL_NAME)
                        .join("SKILL.md"),
                ))
            }
        }
    }

    /// The agent's skills *root* directory for a concrete scope — the
    /// parent under which each skill gets its own `<name>/` subdir.
    /// `Ok(None)` for agents with no filesystem skill loader (Cursor,
    /// Claude Desktop). Generalises [`Agent::skill_path`] (which bakes in
    /// the single `vibevm` skill + `SKILL.md`) for arbitrary package
    /// skills (PROP-018 §2.5).
    pub fn skills_root(self, scope: Scope, project_root: Option<&Path>) -> Result<Option<PathBuf>> {
        if !self.supports_skill() {
            return Ok(None);
        }
        match (self, scope) {
            (_, Scope::Both) => {
                bail!("internal: Agent::skills_root requires concrete scope; expand Both first")
            }
            (Agent::ClaudeCode, Scope::Project) => {
                Ok(project_root.map(|p| p.join(".claude").join("skills")))
            }
            (Agent::OpenCode, Scope::Project) => {
                Ok(project_root.map(|p| p.join(".opencode").join("skills")))
            }
            (Agent::Codex, Scope::Project) => {
                Ok(project_root.map(|p| p.join(".agents").join("skills")))
            }
            (Agent::Cursor | Agent::ClaudeCodeDesktop, _) => Ok(None),
            (Agent::ClaudeCode, Scope::User) => {
                let home = dirs::home_dir()
                    .ok_or_else(|| anyhow!("could not resolve home dir for Claude Code skills"))?;
                Ok(Some(home.join(".claude").join("skills")))
            }
            (Agent::OpenCode, Scope::User) => {
                // Same XDG-on-every-OS contract as Agent::skill_path.
                let home = dirs::home_dir()
                    .ok_or_else(|| anyhow!("could not resolve home dir for OpenCode skills"))?;
                Ok(Some(home.join(".config").join("opencode").join("skills")))
            }
            (Agent::Codex, Scope::User) => {
                let home = dirs::home_dir()
                    .ok_or_else(|| anyhow!("could not resolve home dir for Codex skills"))?;
                Ok(Some(home.join(".agents").join("skills")))
            }
        }
    }

    /// Wire shape of the per-server entry. The same shape serves every
    /// scope: `vibe mcp serve` with **no `--path`**, so the server
    /// resolves its project root from the process CWD. An MCP client sets
    /// that CWD to the project dir for a project-scope (`.mcp.json`)
    /// server and to the operator's working dir for a user-scope one, so
    /// one entry covers both — and a committed `.mcp.json` stays portable
    /// (no machine-specific absolute path baked in).
    ///
    /// On Windows the launcher is the `vibe.cmd` shim, which an MCP
    /// client's bare process-spawn cannot exec directly; the entry is
    /// wrapped as `cmd /c vibe …` so every agent's stdio launcher starts
    /// it. See [`Agent::build_mcp_entry_for`] for the OS-pure core.
    pub fn build_mcp_entry(self) -> ConfigPayload {
        self.build_mcp_entry_for(cfg!(windows))
    }

    /// OS-pure core of [`Agent::build_mcp_entry`]: `windows` selects the
    /// `cmd /c` shim wrapper, so both launch shapes are unit-testable on
    /// any host.
    ///
    /// ```
    /// use vibe_mcp::agents::{Agent, ConfigPayload};
    /// let ConfigPayload::Json(v) = Agent::ClaudeCode.build_mcp_entry_for(true) else {
    ///     panic!("Claude Code uses JSON");
    /// };
    /// assert_eq!(v["command"], "cmd");
    /// assert_eq!(v["args"][0], "/c");
    /// assert_eq!(v["args"][1], "vibe");
    /// ```
    pub fn build_mcp_entry_for(self, windows: bool) -> ConfigPayload {
        // The full launcher argv. On Windows the `.cmd` shim must be run
        // through `cmd /c`; elsewhere `vibe` is a real executable.
        let argv: Vec<String> = if windows {
            ["cmd", "/c", "vibe", "mcp", "serve"]
                .iter()
                .map(|s| s.to_string())
                .collect()
        } else {
            ["vibe", "mcp", "serve"]
                .iter()
                .map(|s| s.to_string())
                .collect()
        };
        // (command, args) split shape: head is the program, tail its args.
        let command = argv[0].clone();
        let args: Vec<String> = argv[1..].to_vec();
        match self {
            Agent::ClaudeCode | Agent::ClaudeCodeDesktop | Agent::Cursor => {
                ConfigPayload::Json(serde_json::json!({
                    "command": command,
                    "args": args,
                }))
            }
            Agent::OpenCode => ConfigPayload::Json(serde_json::json!({
                "type": "local",
                "command": argv,
                "enabled": true,
            })),
            Agent::Codex => {
                let mut tbl = toml::value::Table::new();
                tbl.insert("command".into(), toml::Value::String(command));
                tbl.insert(
                    "args".into(),
                    toml::Value::Array(args.into_iter().map(toml::Value::String).collect()),
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

    /// Directory whose existence on this host marks the agent as
    /// installed. For most agents it's the parent of the user config
    /// file (`~/.cursor`, `~/.codex`, the Claude Desktop config dir, …).
    /// Claude Code is special: its user MCP config is the top-level
    /// `~/.claude.json`, whose parent (`~`) always exists — so we probe
    /// the `~/.claude` data dir instead, the real "Claude Code has run
    /// here" signal.
    fn host_marker(self) -> Option<PathBuf> {
        match self {
            Agent::ClaudeCode => dirs::home_dir().map(|h| h.join(".claude")),
            _ => self
                .config_path(Scope::User, None)
                .ok()
                .flatten()
                .and_then(|c| c.parent().map(Path::to_path_buf)),
        }
    }

    /// Whether the agent is installed on this host — its marker dir
    /// exists. Lets `--auto` and the wizard mark host-installed agents
    /// even when the project tree has no markers.
    pub fn host_present(self) -> bool {
        self.host_marker().map(|p| p.exists()).unwrap_or(false)
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
/// this host (PROP-015 §2.4).
#[spec(implements = "spec://vibevm/modules/vibe-mcp/PROP-015#agent-detection")]
pub fn detect_agents(project_root: Option<&Path>) -> Vec<Agent> {
    Agent::ALL
        .iter()
        .copied()
        .filter(|a| a.is_present(project_root))
        .collect()
}
