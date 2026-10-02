//! Agent profiles and detection (PROP-015 §2.4, §2.5). The fixed set of
//! MCP-capable coding agents, each declaring its config shape (JSON vs
//! TOML, section key, scope support, on-disk paths) and its presence
//! markers. `vibe mcp install` and friends consume these; the CLI keeps
//! only argument parsing, the confirm/render UX, and the lifecycle
//! drivers.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-015#agent-config");

use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use serde_json::Value as JsonValue;
use specmark::spec;

/// Where a vibevm artefact (MCP-config block or SKILL.md) lives. The
/// install / upgrade / uninstall surface accept this through `--scope`;
/// the wizard asks via the first prompt.
///
/// ```
/// use vibe_agent_projection::agents::Scope;
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
/// use vibe_agent_projection::agents::What;
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
/// use vibe_agent_projection::agents::Agent;
/// assert_eq!(Agent::ClaudeCode.as_str(), "claude");
/// assert_eq!(Agent::parse_filter("all").unwrap().len(), 6);
/// // Codex configures via TOML; the others via JSON.
/// assert!(Agent::Codex.supports_project_scope());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum Agent {
    ClaudeCode,
    ClaudeCodeDesktop,
    Cursor,
    OpenCode,
    Codex,
    QwenCode,
}

/// User directories supplied by the caller when resolving agent config paths.
/// Missing directories fail only for scopes that need them.
///
/// ```
/// use vibe_agent_projection::agents::AgentUserDirectories;
/// let directories = AgentUserDirectories::default();
/// assert!(directories.home.is_none());
/// assert!(directories.config.is_none());
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AgentUserDirectories {
    /// User home directory, used by all agents except Claude Desktop.
    pub home: Option<PathBuf>,
    /// Platform user-config directory, used by Claude Desktop.
    pub config: Option<PathBuf>,
}

impl AgentUserDirectories {
    /// Resolve the ambient home and config directories once, without reading
    /// configuration files or creating directories.
    ///
    /// ```
    /// use vibe_agent_projection::agents::AgentUserDirectories;
    /// let _: AgentUserDirectories = AgentUserDirectories::ambient();
    /// ```
    pub fn ambient() -> Self {
        Self {
            home: dirs::home_dir(),
            config: dirs::config_dir(),
        }
    }
}

/// JSON or TOML — the config-file format an agent reads.
///
/// ```
/// use vibe_agent_projection::agents::ConfigFormat;
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
/// use vibe_agent_projection::agents::ConfigPayload;
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
        Agent::QwenCode,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Agent::ClaudeCode => "claude",
            Agent::ClaudeCodeDesktop => "claude-desktop",
            Agent::Cursor => "cursor",
            Agent::OpenCode => "opencode",
            Agent::Codex => "codex",
            Agent::QwenCode => "qwen-code",
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
            "qwen" | "qwen-code" => Ok(vec![Agent::QwenCode]),
            other => bail!(
                "unknown --agent value `{other}` (expected one of `all`, \
                 `claude`, `claude-desktop`, `cursor`, `opencode`, `codex`, `qwen-code`)"
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
    /// Claude Desktop is user-only; Codex also reads trusted project config.
    pub fn supports_project_scope(self) -> bool {
        match self {
            Agent::ClaudeCode
            | Agent::Cursor
            | Agent::OpenCode
            | Agent::QwenCode
            | Agent::Codex => true,
            Agent::ClaudeCodeDesktop => false,
        }
    }

    /// Whether this agent loads filesystem-backed skill files. Cursor
    /// and Claude Desktop are JSON-config-only.
    pub fn supports_skill(self) -> bool {
        match self {
            Agent::ClaudeCode | Agent::OpenCode | Agent::Codex | Agent::QwenCode => true,
            Agent::ClaudeCodeDesktop | Agent::Cursor => false,
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
    /// use vibe_agent_projection::agents::{Agent, ConfigPayload};
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
            Agent::ClaudeCode | Agent::ClaudeCodeDesktop | Agent::Cursor | Agent::QwenCode => {
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
            Agent::QwenCode => &[".qwen", "QWEN.md"],
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
#[spec(implements = "spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-015#agent-detection")]
pub fn detect_agents(project_root: Option<&Path>) -> Vec<Agent> {
    Agent::ALL
        .iter()
        .copied()
        .filter(|a| a.is_present(project_root))
        .collect()
}

// The INJECTED-HOME pure path helpers — the deploy lane's half of this
// module's surface, in its own file because its callers (and its
// no-ambient-reads law) are a different audience from the compatibility
// surfaces above.
mod ambient_paths;
mod home_paths;
