//! Compatibility path resolvers that read the ambient user home.
//!
//! The lifecycle deploy lane uses the injected-home helpers in `home_paths`;
//! these methods retain the original public API for MCP and skill commands.

use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow, bail};

use super::{Agent, SKILL_NAME, Scope};

impl Agent {
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
            (Agent::QwenCode, Scope::Project) => {
                Ok(project_root.map(|p| p.join(".qwen").join("settings.json")))
            }
            (Agent::Codex, Scope::Project) => {
                Ok(project_root.map(|p| p.join(".codex").join("config.toml")))
            }
            (Agent::ClaudeCodeDesktop, Scope::Project) => Ok(None),
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
            (Agent::QwenCode, Scope::User) => {
                let home = dirs::home_dir()
                    .ok_or_else(|| anyhow!("could not resolve home dir for Qwen Code"))?;
                Ok(Some(home.join(".qwen").join("settings.json")))
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
            (Agent::QwenCode, Scope::Project) => Ok(project_root.map(|p| {
                p.join(".qwen")
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
            (Agent::QwenCode, Scope::User) => {
                let home = dirs::home_dir()
                    .ok_or_else(|| anyhow!("could not resolve home dir for Qwen Code skill"))?;
                Ok(Some(
                    home.join(".qwen")
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
    ///
    /// AMBIENT compatibility surface: it resolves the home through
    /// `dirs::home_dir()` and is retained for the pre-R8 callers
    /// (`vibe skill install` and friends). A lifecycle deploy provider
    /// calls only the pure [`Agent::user_skills_root_from_home`] twin in
    /// [`home_paths`] — never this one.
    ///
    /// [`home_paths`]: self::home_paths
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
            (Agent::QwenCode, Scope::Project) => {
                Ok(project_root.map(|p| p.join(".qwen").join("skills")))
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
            (Agent::QwenCode, Scope::User) => {
                let home = dirs::home_dir()
                    .ok_or_else(|| anyhow!("could not resolve home dir for Qwen Code skills"))?;
                Ok(Some(home.join(".qwen").join("skills")))
            }
        }
    }
}
