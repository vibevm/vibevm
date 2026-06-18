//! `vibe mcp upgrade` — refresh stale MCP blocks and SKILL.md files in
//! place (PROP-015 §2.7). Split out of the mcp god-file (§7.3d).

use super::*;

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

pub(super) fn run_upgrade(ctx: &output::Context, args: McpUpgradeArgs) -> Result<()> {
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
        .map(crate::commands::init::strip_unc_public)
        .filter(|p| p.join(Manifest::FILENAME).exists());

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

    // Two-pass walk + apply-confirm. Same shape as run_install
    // (above) — preview first, ask confirm, then real apply only
    // when the operator approves AND the original invocation is
    // not `--dry-run`.
    let (preview_results, preview_skill) =
        walk_upgrade(&agents, scope, project_root.as_deref(), what, true)?;
    let needs_change = preview_results.iter().any(|r| r.status == "would-update")
        || preview_skill.iter().any(|r| r.status == "would-update");

    if !args.dry_run && needs_change {
        // Confirm only on real TTY without a skip-flag (see the
        // matching block in `run_install` for the rationale —
        // non-TTY scripts pre-date the confirm prompt and must
        // continue to work without `--yes`).
        let approved =
            if args.yes || ctx.is_unattended() || ctx.is_json() || !console::user_attended() {
                true
            } else {
                print_upgrade_results(ctx, true, &preview_results, &preview_skill);
                let stale = preview_results
                    .iter()
                    .filter(|r| r.status == "would-update")
                    .count()
                    + preview_skill
                        .iter()
                        .filter(|r| r.status == "would-update")
                        .count();
                Confirm::new()
                    .with_prompt(format!(
                        "Refresh {stale} stale entr{}?",
                        if stale == 1 { "y" } else { "ies" }
                    ))
                    .default(false)
                    .interact()
                    .context("reading user confirmation")?
            };
        if !approved {
            return Err(InstallError::UserDeclined.into());
        }
    }

    let (results, skill_results) = if args.dry_run || !needs_change {
        (preview_results, preview_skill)
    } else {
        walk_upgrade(&agents, scope, project_root.as_deref(), what, false)?
    };

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
        let verb = if args.dry_run {
            "previewed"
        } else {
            "refreshed"
        };
        ctx.summary(&format!(
            "vibe mcp upgrade: {stale} stale entr{} {verb}",
            if stale == 1 { "y" } else { "ies" }
        ));
        return Ok(());
    }

    print_upgrade_results(ctx, args.dry_run, &results, &skill_results);
    Ok(())
}

/// Per-(agent × scope) upgrade walker. Same role as `walk_install`
/// for the install path: invoked twice from `run_upgrade` (once
/// dry-run for the plan, once real for the apply) so the operator's
/// `--yes` / `--unattended` / `--auto` / `--json` actually gate a
/// confirmation prompt.
fn walk_upgrade(
    agents: &[Agent],
    scope: Scope,
    project_root: Option<&Path>,
    what: What,
    dry_run: bool,
) -> Result<(Vec<AgentInstallReport>, Vec<SkillInstallReport>)> {
    let mut results: Vec<AgentInstallReport> = Vec::new();
    let mut skill_results: Vec<SkillInstallReport> = Vec::new();
    for agent in agents {
        for concrete_scope in scope.expand() {
            if concrete_scope == Scope::Project && project_root.is_none() {
                continue;
            }
            if what.includes_mcp() {
                let path = agent.config_path(concrete_scope, project_root)?;
                if let Some(path) = path {
                    let outcome = upgrade_mcp_entry(*agent, concrete_scope, &path, dry_run)?;
                    results.push(outcome);
                }
            }
            if what.includes_skill() {
                let outcome = upgrade_skill(*agent, concrete_scope, project_root, dry_run)?;
                if let Some(o) = outcome {
                    skill_results.push(o);
                }
            }
        }
    }
    Ok((results, skill_results))
}

/// One-place upgrade probe + apply for an MCP-config block.
fn upgrade_mcp_entry(
    agent: Agent,
    scope: Scope,
    config_path: &Path,
    dry_run: bool,
) -> Result<AgentInstallReport> {
    // If the config file does not exist OR the vibevm block is absent,
    // upgrade is a no-op. We report `not-installed` rather than
    // creating it (that would be `install`'s job).
    let payload = agent.build_mcp_entry();
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
        (ConfigPayload::Json(_), ConfigFormat::Json) => read_json(config_path)?
            .get(section)
            .and_then(|v| v.get(SERVER_NAME))
            .is_some(),
        (ConfigPayload::Toml(_), ConfigFormat::Toml) => read_toml(config_path)?
            .get(section)
            .and_then(|v| v.as_table())
            .and_then(|t| t.get(SERVER_NAME))
            .is_some(),
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
            "would-update" | "updated" => {
                if dry_run {
                    "would"
                } else {
                    "updated"
                }
            }
            "not-installed" => "·",
            other => other,
        };
        let note = r
            .note
            .as_deref()
            .map(|n| format!(" ({n})"))
            .unwrap_or_default();
        ctx.step(&format!(
            "{} mcp     {} ({}) → {}{note}",
            prefix, r.agent, r.scope, r.config_path
        ));
    }
    for r in skill_results {
        let prefix = match r.status {
            "unchanged" => "✓",
            "would-update" | "updated" => {
                if dry_run {
                    "would"
                } else {
                    "updated"
                }
            }
            "not-installed" => "·",
            other => other,
        };
        let note = r
            .note
            .as_deref()
            .map(|n| format!(" ({n})"))
            .unwrap_or_default();
        let path_str = r.path.as_deref().unwrap_or("(no skill loader)");
        ctx.step(&format!(
            "{} skill   {} ({}) → {}{note}",
            prefix, r.agent, r.scope, path_str
        ));
    }
}
