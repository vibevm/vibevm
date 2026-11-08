//! Agent-install reporting + skill materialisation (PROP-015 §2.6, §2.7).
//! The per-(agent, scope) outcome records the lifecycle returns, and the
//! SKILL.md writer that renders the vendored template into a supporting
//! agent's skill directory. The CLI drives these and renders the records;
//! the MCP-entry writers and the install/upgrade/uninstall walkers
//! consume the same report types.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-015#lifecycle");

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde::Serialize;
use specmark::spec;
use vibe_core::machine_json_path;

use crate::agents::{Agent, Scope};
// The shared dry-run status vocabulary moved down with the package-skill
// projection (R7.4 §5); the SKILL.md writer consumes the one lower
// implementation rather than re-spelling it here.

/// Bytes of the `vibevm` SKILL.md template, vendored at compile time.
pub const SKILL_TEMPLATE: &str = include_str!("skill_template.md");

/// Per-(agent, scope) outcome of writing (or previewing) an MCP config
/// entry — the structured record the CLI renders or emits as JSON.
///
/// ```
/// use vibe_mcp::install::AgentInstallReport;
/// let r = AgentInstallReport {
///     agent: "claude".into(),
///     scope: "project",
///     config_path: "/p/.mcp.json".into(),
///     status: "created",
///     note: None,
/// };
/// assert_eq!(r.status, "created");
/// ```
#[derive(Debug, Clone, Serialize)]
#[spec(implements = "spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-015#lifecycle")]
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
///
/// ```
/// use vibe_mcp::install::SkillInstallReport;
/// let r = SkillInstallReport {
///     agent: "claude".into(),
///     scope: "project",
///     path: None,
///     status: "skipped",
///     note: None,
/// };
/// assert_eq!(r.status, "skipped");
/// ```
#[derive(Debug, Clone, Serialize)]
#[spec(implements = "spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-015#lifecycle")]
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
#[spec(implements = "spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-015#skill")]
pub fn install_skill(
    agent: Agent,
    scope: Scope,
    project_root: Option<&Path>,
    dry_run: bool,
) -> Result<SkillInstallReport> {
    install_skill_adopting(
        agent,
        scope,
        project_root,
        vibe_agent_projection::activation::BuiltinAdoption::explicit("vibe mcp install"),
        dry_run,
    )
}

/// Explicit builtin SKILL.md registration under a stated adoption policy
/// (PROP-073 managed activation): `vibe mcp install` adopts identical
/// unreceipted bytes, `vibe mcp install --force` also replaces stale
/// unreceipted bytes an older vibe wrote, and `vibe mcp upgrade` adopts
/// identical bytes only. A refused stale target surfaces as
/// [`vibe_agent_projection::pkgskill::PackageSkillError::Unowned`].
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-073#MANAGED-ACTIVATION")]
pub fn install_skill_adopting(
    agent: Agent,
    scope: Scope,
    project_root: Option<&Path>,
    adoption: vibe_agent_projection::activation::BuiltinAdoption,
    dry_run: bool,
) -> Result<SkillInstallReport> {
    install_skill_with(agent, scope, project_root, dry_run, Some(adoption))
}

/// Refresh or observe existing ownership without adopting unreceipted files.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-073#MANAGED-ACTIVATION")]
pub fn refresh_skill(
    agent: Agent,
    scope: Scope,
    project_root: Option<&Path>,
    dry_run: bool,
) -> Result<SkillInstallReport> {
    install_skill_with(agent, scope, project_root, dry_run, None)
}

fn install_skill_with(
    agent: Agent,
    scope: Scope,
    project_root: Option<&Path>,
    dry_run: bool,
    adoption: Option<vibe_agent_projection::activation::BuiltinAdoption>,
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

    let desired =
        std::collections::BTreeMap::from([("SKILL.md".into(), SKILL_TEMPLATE.as_bytes().to_vec())]);
    let row = if let Some(adoption) = adoption {
        vibe_agent_projection::pkgskill::adopt_builtin_skill_snapshot(
            agent,
            scope,
            project_root,
            &desired,
            adoption,
            dry_run,
        )?
    } else {
        vibe_agent_projection::pkgskill::install_managed_skill_snapshot(
            agent,
            scope,
            project_root,
            Some("org.vibevm.core/vibevm"),
            "vibevm",
            &desired,
            dry_run,
        )?
    };
    Ok(SkillInstallReport {
        agent: agent_str,
        scope: scope_str,
        path: Some(machine_json_path(&path)),
        status: row.status,
        note: row.note,
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

    /// A SKILL.md an older (pre-receipt) vibe wrote with stale bytes is
    /// refused by plain explicit install and replaced only under `--force`.
    #[test]
    fn stale_legacy_skill_needs_forced_adoption() {
        use vibe_agent_projection::activation::BuiltinAdoption;
        let dir = tempfile::tempdir().unwrap();
        let skill = dir.path().join(".claude/skills/vibevm/SKILL.md");
        fs::create_dir_all(skill.parent().unwrap()).unwrap();
        fs::write(&skill, "stale content").unwrap();
        let refusal =
            install_skill(Agent::ClaudeCode, Scope::Project, Some(dir.path()), false).unwrap_err();
        assert!(
            refusal.to_string().contains("vibe mcp install --force"),
            "{refusal}"
        );
        assert_eq!(fs::read_to_string(&skill).unwrap(), "stale content");
        let forced = install_skill_adopting(
            Agent::ClaudeCode,
            Scope::Project,
            Some(dir.path()),
            BuiltinAdoption::forced("vibe mcp install"),
            false,
        )
        .unwrap();
        assert_eq!(forced.status, "adopted");
        assert_eq!(fs::read_to_string(&skill).unwrap(), SKILL_TEMPLATE);
        let again =
            install_skill(Agent::ClaudeCode, Scope::Project, Some(dir.path()), false).unwrap();
        assert_eq!(again.status, "unchanged");
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

    /// PROP-018 §2.8/§2.9: the usage skill teaches agents which MCP tools to
    /// call, so every tool the server actually registers must be named in the
    /// template — otherwise the skill either omits a real tool or, worse, the
    /// server drops one the skill still tells agents to call. Cross-check the
    /// served set (`default_tools`) against the template so the two cannot
    /// drift apart silently.
    #[test]
    fn every_served_tool_is_named_in_the_skill_template() {
        use crate::default_tools;
        for tool in default_tools() {
            let name = tool.descriptor().name;
            assert!(
                SKILL_TEMPLATE.contains(&name),
                "MCP tool `{name}` is served by default_tools() but the vibevm \
                 skill template names no such tool — teach it in \
                 skill_template.md, or an agent will never learn to call it"
            );
        }
    }

    /// The template has no substitution pass (it ships verbatim via
    /// `include_str!`), so a retired identity in its prose becomes a recipe
    /// agents follow into a resolution error. The wave-2 family rename
    /// (PROP-028 §2.4) retired every `discipline-`-prefixed package, binary,
    /// and skill name; pin their absence so the template cannot teach one
    /// again. The bare word "discipline" (the concept) stays legal.
    #[test]
    fn skill_template_teaches_no_retired_identities() {
        for retired in [
            "discipline-", // covers discipline-rust/-typescript/-mcp-*/-cli-*/-sweep*/-core
            "conform-rust",
            "specmap-rust",
            "tcg-rust",
            "conform-typescript",
            "specmap-typescript",
            "tcg-typescript",
            "terraform-rust",
            "terraform-typescript",
        ] {
            assert!(
                !SKILL_TEMPLATE.contains(retired),
                "skill_template.md still teaches the retired identity \
                 `{retired}` — the family rename (PROP-028 §2.4) replaced it; \
                 update the recipe to the <family>-prefixed form"
            );
        }
    }
}
