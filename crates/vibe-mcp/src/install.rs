//! Agent-install reporting + skill materialisation (PROP-015 §2.6, §2.7).
//! The per-(agent, scope) outcome records the lifecycle returns, and the
//! SKILL.md writer that renders the vendored template into a supporting
//! agent's skill directory. The CLI drives these and renders the records;
//! the MCP-entry writers and the install/upgrade/uninstall walkers
//! consume the same report types.

specmark::scope!("spec://vibevm/modules/vibe-mcp/PROP-015#lifecycle");

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde::Serialize;
use specmark::spec;

use crate::agents::{Agent, Scope};

/// Bytes of the `vibevm` SKILL.md template, vendored at compile time.
pub const SKILL_TEMPLATE: &str = include_str!("skill_template.md");

/// Per-(agent, scope) outcome of writing (or previewing) an MCP config
/// entry — the structured record the CLI renders or emits as JSON.
#[derive(Debug, Clone, Serialize)]
#[spec(implements = "spec://vibevm/modules/vibe-mcp/PROP-015#lifecycle")]
pub struct AgentInstallReport {
    pub agent: String,
    pub scope: &'static str,
    pub config_path: String,
    /// `created` / `updated` / `unchanged` / `would-create` /
    /// `would-update` / `skipped`.
    pub status: &'static str,
    pub note: Option<String>,
}

/// Per-(agent, scope) outcome of writing (or previewing) a SKILL.md.
#[derive(Debug, Clone, Serialize)]
#[spec(implements = "spec://vibevm/modules/vibe-mcp/PROP-015#lifecycle")]
pub struct SkillInstallReport {
    pub agent: String,
    pub scope: &'static str,
    pub path: Option<String>,
    pub status: &'static str,
    pub note: Option<String>,
}

/// Write (or, on `dry_run`, preview) the `vibevm` SKILL.md for one agent
/// and scope (PROP-015 §2.6). Agents with no filesystem skill loader,
/// or no surface for this scope, report `skipped`. Idempotent: an
/// identical existing file is left `unchanged`; a divergent one is
/// `updated`.
#[spec(implements = "spec://vibevm/modules/vibe-mcp/PROP-015#skill")]
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
        fs::write(&path, body).with_context(|| format!("writing skill `{}`", path.display()))?;
    }

    Ok(SkillInstallReport {
        agent: agent_str,
        scope: scope_str,
        path: Some(path_str),
        status: final_status,
        note: None,
    })
}

/// Compare an existing SKILL.md to the template: `created` (absent),
/// `unchanged` (byte-identical), or `updated` (divergent).
pub fn decide_skill_action(path: &Path, body: &str) -> Result<&'static str> {
    if !path.exists() {
        return Ok("created");
    }
    let existing =
        fs::read_to_string(path).with_context(|| format!("reading skill `{}`", path.display()))?;
    if existing == body {
        Ok("unchanged")
    } else {
        Ok("updated")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn install_skill_creates_under_project() {
        let dir = tempfile::tempdir().unwrap();
        let r = install_skill(Agent::ClaudeCode, Scope::Project, Some(dir.path()), false).unwrap();
        assert_eq!(r.status, "created");
        let p = dir
            .path()
            .join(".claude")
            .join("skills")
            .join("vibevm")
            .join("SKILL.md");
        assert!(p.is_file());
        // A second run sees identical bytes → unchanged.
        let r2 = install_skill(Agent::ClaudeCode, Scope::Project, Some(dir.path()), false).unwrap();
        assert_eq!(r2.status, "unchanged");
    }

    #[test]
    fn install_skill_dry_run_writes_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let r = install_skill(Agent::OpenCode, Scope::Project, Some(dir.path()), true).unwrap();
        assert_eq!(r.status, "would-create");
        assert!(!dir.path().join(".opencode").exists());
    }

    #[test]
    fn install_skill_skipped_for_unsupported_agent() {
        let dir = tempfile::tempdir().unwrap();
        // Cursor is JSON-config-only — no filesystem skill loader.
        let r = install_skill(Agent::Cursor, Scope::Project, Some(dir.path()), false).unwrap();
        assert_eq!(r.status, "skipped");
        assert!(r.path.is_none());
    }

    #[test]
    fn skill_template_carries_its_contract() {
        assert!(SKILL_TEMPLATE.starts_with("---"));
        assert!(SKILL_TEMPLATE.contains("name: vibevm"));
        assert!(SKILL_TEMPLATE.contains("query_package"));
        assert!(SKILL_TEMPLATE.contains("read_subskill"));
        assert!(SKILL_TEMPLATE.contains("materialise_subskill"));
        // PROP-018 §2.9: the skill teaches the agentic relay protocol (the
        // two-step produce/`vibe command` drain, no auto write-back, the
        // transport heuristic) and the standalone `vibe skill` command.
        assert!(SKILL_TEMPLATE.contains("vibe agentic explain"));
        assert!(SKILL_TEMPLATE.contains("vibe command"));
        assert!(SKILL_TEMPLATE.contains("agentic_explain"));
        assert!(SKILL_TEMPLATE.contains("no automatic write-back"));
        assert!(SKILL_TEMPLATE.contains("vibe skill"));
    }
}
