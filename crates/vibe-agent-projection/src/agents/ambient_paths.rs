//! Compatibility path resolvers that read the ambient user home.
//!
//! The lifecycle deploy lane uses the injected-home helpers in `home_paths`;
//! these methods retain the original public API for MCP and skill commands.

use specmark::spec;
use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow, bail};

use super::{Agent, AgentUserDirectories, SKILL_NAME, Scope};

impl Agent {
    /// Resolve the per-agent config-file path for a single concrete
    /// scope (Project or User — never Both; expand Both first).
    /// Returns `Ok(None)` if the agent does not support this scope
    /// (e.g. Claude Desktop + Project). Returns `Err(...)` if the
    /// host cannot resolve required dirs (HOME / config-dir).
    pub fn config_path(self, scope: Scope, project_root: Option<&Path>) -> Result<Option<PathBuf>> {
        self.config_path_with_dirs(scope, project_root, &AgentUserDirectories::ambient())
    }

    /// Resolve a config path using only supplied project and user directories.
    /// No environment or filesystem reads occur. Expand `Scope::Both` first;
    /// unsupported scopes and a missing project root return `Ok(None)`.
    /// Missing user directories retain the errors from [`Agent::config_path`].
    ///
    /// ```
    /// use vibe_agent_projection::agents::{Agent, AgentUserDirectories, Scope};
    /// let sandbox = tempfile::tempdir()?;
    /// let home = sandbox.path().join("home");
    /// let directories = AgentUserDirectories {
    ///     home: Some(home.clone()),
    ///     config: None,
    ///     native: Default::default(),
    /// };
    /// assert_eq!(
    ///     Agent::Codex.config_path_with_dirs(Scope::User, None, &directories)?,
    ///     Some(home.join(".codex").join("config.toml")),
    /// );
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    #[spec(implements = "spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-015#CONFIG-PATH")]
    pub fn config_path_with_dirs(
        self,
        scope: Scope,
        project_root: Option<&Path>,
        directories: &AgentUserDirectories,
    ) -> Result<Option<PathBuf>> {
        if scope == Scope::User
            && let Some(directory) = directories.native.get(&self)
        {
            if !directory.is_absolute()
                || directory
                    .components()
                    .any(|part| matches!(part, std::path::Component::ParentDir))
            {
                bail!(
                    "native agent directory must be an absolute normalized path; refusing fallback write"
                );
            }
            if self == Agent::Pi {
                return Ok(Some(directory.join("mcp.json")));
            }
            if self == Agent::Hermes {
                return Ok(Some(directory.join("config.yaml")));
            }
        }
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
            (Agent::Pi, Scope::Project) => Ok(project_root.map(|p| p.join(".pi").join("mcp.json"))),
            (Agent::ClaudeCodeDesktop | Agent::Hermes, Scope::Project) => Ok(None),
            // ---- User scope ----
            (Agent::ClaudeCode, Scope::User) => {
                // User-scope MCP servers live in the top-level `mcpServers`
                // of `~/.claude.json` (exactly what `claude mcp add --scope
                // user` writes); Claude Code does not read server definitions
                // from `~/.claude/settings.json`.
                let home = directories
                    .home
                    .as_ref()
                    .ok_or_else(|| anyhow!("could not resolve home dir for Claude Code"))?;
                Ok(Some(home.join(".claude.json")))
            }
            (Agent::Cursor, Scope::User) => {
                let home = directories
                    .home
                    .as_ref()
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
                let home = directories
                    .home
                    .as_ref()
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
                let cfg = directories.config.as_ref().ok_or_else(|| {
                    anyhow!("could not resolve user-config dir for Claude Desktop")
                })?;
                Ok(Some(cfg.join("Claude").join("claude_desktop_config.json")))
            }
            (Agent::Hermes, Scope::User) => bail!(
                "Hermes requires an explicitly resolved native profile home; refusing default-home fallback"
            ),
            (Agent::Pi, Scope::User) => {
                let home = directories
                    .home
                    .as_ref()
                    .ok_or_else(|| anyhow!("could not resolve home dir for Pi"))?;
                Ok(Some(home.join(".pi").join("agent").join("mcp.json")))
            }
            (Agent::Codex, Scope::User) => {
                let home = directories
                    .home
                    .as_ref()
                    .ok_or_else(|| anyhow!("could not resolve home dir for Codex"))?;
                Ok(Some(home.join(".codex").join("config.toml")))
            }
            (Agent::QwenCode, Scope::User) => {
                let home = directories
                    .home
                    .as_ref()
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
    #[spec(implements = "spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-015#skill")]
    pub fn skill_path(self, scope: Scope, project_root: Option<&Path>) -> Result<Option<PathBuf>> {
        if !self.supports_skill() {
            return Ok(None);
        }
        if self == Agent::Hermes {
            if scope == Scope::Project {
                return Ok(None);
            }
            if scope == Scope::Both {
                bail!("expand Both before resolving Hermes skills");
            }
            let config = self
                .config_path(Scope::User, None)?
                .ok_or_else(|| anyhow!("Hermes profile home unavailable"))?;
            return Ok(Some(
                config
                    .parent()
                    .ok_or_else(|| anyhow!("Hermes profile home unavailable"))?
                    .join("skills")
                    .join(SKILL_NAME)
                    .join("SKILL.md"),
            ));
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
            (Agent::Codex | Agent::Pi, Scope::Project) => Ok(project_root.map(|p| {
                p.join(".agents")
                    .join("skills")
                    .join(SKILL_NAME)
                    .join("SKILL.md")
            })),
            (Agent::Cursor | Agent::ClaudeCodeDesktop | Agent::Hermes, _) => Ok(None),
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
            (Agent::Codex | Agent::Pi, Scope::User) => {
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
    #[spec(implements = "spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-015#skill")]
    pub fn skills_root(self, scope: Scope, project_root: Option<&Path>) -> Result<Option<PathBuf>> {
        if !self.supports_skill() {
            return Ok(None);
        }
        if self == Agent::Hermes {
            if scope == Scope::Project {
                return Ok(None);
            }
            if scope == Scope::Both {
                bail!("expand Both before resolving Hermes skills");
            }
            let config = self
                .config_path(Scope::User, None)?
                .ok_or_else(|| anyhow!("Hermes profile home unavailable"))?;
            return Ok(Some(
                config
                    .parent()
                    .ok_or_else(|| anyhow!("Hermes profile home unavailable"))?
                    .join("skills"),
            ));
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
            (Agent::Codex | Agent::Pi, Scope::Project) => {
                Ok(project_root.map(|p| p.join(".agents").join("skills")))
            }
            (Agent::Cursor | Agent::ClaudeCodeDesktop | Agent::Hermes, _) => Ok(None),
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
            (Agent::Codex | Agent::Pi, Scope::User) => {
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

#[cfg(test)]
mod injected_path_tests {
    use super::*;

    #[test]
    fn injected_user_paths_preserve_agent_mappings() {
        let sandbox = tempfile::tempdir().unwrap();
        let home = sandbox.path().join("home");
        let config = sandbox.path().join("config");
        let directories = AgentUserDirectories {
            home: Some(home.clone()),
            config: Some(config.clone()),
            native: Default::default(),
        };
        for (agent, expected) in [
            (Agent::ClaudeCode, home.join(".claude.json")),
            (Agent::Cursor, home.join(".cursor/mcp.json")),
            (Agent::OpenCode, home.join(".config/opencode/opencode.json")),
            (Agent::Codex, home.join(".codex/config.toml")),
            (Agent::QwenCode, home.join(".qwen/settings.json")),
            (
                Agent::ClaudeCodeDesktop,
                config.join("Claude/claude_desktop_config.json"),
            ),
        ] {
            assert_eq!(
                agent
                    .config_path_with_dirs(Scope::User, None, &directories)
                    .unwrap(),
                Some(expected),
                "{}",
                agent.as_str(),
            );
        }
        assert!(!home.exists());
        assert!(!config.exists());
    }

    #[test]
    fn injected_project_paths_do_not_require_user_directories() {
        let sandbox = tempfile::tempdir().unwrap();
        let project = sandbox.path().join("project");
        let directories = AgentUserDirectories::default();
        for (agent, expected) in [
            (Agent::ClaudeCode, Some(project.join(".mcp.json"))),
            (Agent::Cursor, Some(project.join(".cursor/mcp.json"))),
            (Agent::OpenCode, Some(project.join("opencode.json"))),
            (Agent::Codex, Some(project.join(".codex/config.toml"))),
            (Agent::QwenCode, Some(project.join(".qwen/settings.json"))),
            (Agent::ClaudeCodeDesktop, None),
        ] {
            assert_eq!(
                agent
                    .config_path_with_dirs(Scope::Project, Some(&project), &directories)
                    .unwrap(),
                expected,
            );
            assert_eq!(
                agent
                    .config_path_with_dirs(Scope::Project, None, &directories)
                    .unwrap(),
                None,
            );
        }
        assert!(!project.exists());
    }

    #[test]
    fn injected_missing_directories_preserve_errors() {
        let directories = AgentUserDirectories::default();
        for (agent, expected) in [
            (
                Agent::ClaudeCode,
                "could not resolve home dir for Claude Code",
            ),
            (Agent::Cursor, "could not resolve home dir for Cursor"),
            (Agent::OpenCode, "could not resolve home dir for OpenCode"),
            (Agent::Codex, "could not resolve home dir for Codex"),
            (Agent::QwenCode, "could not resolve home dir for Qwen Code"),
            (
                Agent::ClaudeCodeDesktop,
                "could not resolve user-config dir for Claude Desktop",
            ),
        ] {
            assert_eq!(
                agent
                    .config_path_with_dirs(Scope::User, None, &directories)
                    .unwrap_err()
                    .to_string(),
                expected,
            );
            assert_eq!(
                agent
                    .config_path_with_dirs(Scope::Both, None, &directories)
                    .unwrap_err()
                    .to_string(),
                "internal: Agent::config_path requires concrete scope; expand Both first",
            );
        }
    }

    #[test]
    fn pi_native_config_and_skill_paths_use_existing_injected_owners() {
        let home = PathBuf::from("/synthetic-home");
        let project = Path::new("/synthetic-project");
        let directories = AgentUserDirectories {
            home: Some(home.clone()),
            config: None,
            native: Default::default(),
        };
        assert_eq!(Agent::parse_filter("pi").unwrap(), vec![Agent::Pi]);
        assert_eq!(
            Agent::Pi
                .config_path_with_dirs(Scope::User, None, &directories)
                .unwrap(),
            Some(home.join(".pi/agent/mcp.json"))
        );
        assert_eq!(
            Agent::Pi
                .config_path_with_dirs(Scope::Project, Some(project), &directories)
                .unwrap(),
            Some(project.join(".pi/mcp.json"))
        );
        assert_eq!(
            Agent::Pi
                .skills_root(Scope::Project, Some(project))
                .unwrap(),
            Some(project.join(".agents/skills"))
        );
        assert_eq!(
            Agent::Pi.user_skill_entry_from_home(&home, "demo"),
            Some(home.join(".agents/skills/demo/SKILL.md"))
        );
        assert_eq!(Agent::Pi.mcp_section_key(), "mcpServers");
        let mut overridden = directories.clone();
        let custom = std::env::temp_dir().join("synthetic-pi-agent");
        overridden.native.insert(Agent::Pi, custom.clone());
        assert_eq!(
            Agent::Pi
                .config_path_with_dirs(Scope::User, None, &overridden)
                .unwrap(),
            Some(custom.join("mcp.json"))
        );
        overridden
            .native
            .insert(Agent::Pi, PathBuf::from("relative/unsafe"));
        assert!(
            Agent::Pi
                .config_path_with_dirs(Scope::User, None, &overridden)
                .unwrap_err()
                .to_string()
                .contains("refusing fallback")
        );
        assert_eq!(
            Agent::Pi
                .config_path_with_dirs(Scope::Project, Some(project), &overridden)
                .unwrap(),
            Some(project.join(".pi/mcp.json"))
        );
    }

    #[test]
    fn injected_user_paths_require_only_the_selected_directory() {
        let sandbox = tempfile::tempdir().unwrap();
        let root = sandbox.path().to_path_buf();
        let config_only = AgentUserDirectories {
            home: None,
            config: Some(root.clone()),
            native: Default::default(),
        };
        assert!(
            Agent::ClaudeCodeDesktop
                .config_path_with_dirs(Scope::User, None, &config_only)
                .is_ok()
        );
        assert!(
            Agent::Codex
                .config_path_with_dirs(Scope::User, None, &config_only)
                .is_err()
        );
        let home_only = AgentUserDirectories {
            home: Some(root),
            config: None,
            native: Default::default(),
        };
        for agent in Agent::ALL {
            if !matches!(*agent, Agent::ClaudeCodeDesktop | Agent::Hermes) {
                assert!(
                    agent
                        .config_path_with_dirs(Scope::User, None, &home_only)
                        .is_ok()
                );
            }
        }
        assert!(
            Agent::ClaudeCodeDesktop
                .config_path_with_dirs(Scope::User, None, &home_only)
                .is_err()
        );
    }
}
