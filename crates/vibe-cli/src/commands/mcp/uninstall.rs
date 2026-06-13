//! `vibe mcp uninstall` — strip vibevm MCP entries + SKILL.md, preserving
//! foreign config (PROP-015 §2.7). Split out of the mcp god-file (§7.3d).

use super::*;

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

pub(super) fn run_uninstall(ctx: &output::Context, args: McpUninstallArgs) -> Result<()> {
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
        .map(crate::commands::init::strip_unc_public)
        .filter(|p| p.join(Manifest::FILENAME).exists());

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

    // Two-pass walk + apply-confirm. Uninstall is the most
    // destructive of the three MCP commands (it deletes SKILL.md
    // files and drops the vibevm block from MCP configs), so the
    // confirm step here matters more than for install / upgrade.
    let (preview_results, preview_skill) =
        walk_uninstall(&agents, scope, project_root.as_deref(), what, true)?;
    let needs_change = preview_results.iter().any(|r| r.status == "would-remove")
        || preview_skill.iter().any(|r| r.status == "would-remove");

    if !args.dry_run && needs_change {
        // Same TTY-only confirm policy as install / upgrade.
        let approved =
            if args.yes || ctx.is_unattended() || ctx.is_json() || !console::user_attended() {
                true
            } else {
                print_uninstall_results(ctx, true, &preview_results, &preview_skill);
                let to_remove = preview_results
                    .iter()
                    .filter(|r| r.status == "would-remove")
                    .count()
                    + preview_skill
                        .iter()
                        .filter(|r| r.status == "would-remove")
                        .count();
                Confirm::new()
                    .with_prompt(format!(
                        "Remove {to_remove} entr{}?",
                        if to_remove == 1 { "y" } else { "ies" }
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
        walk_uninstall(&agents, scope, project_root.as_deref(), what, false)?
    };

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
        ctx.summary(&format!(
            "vibe mcp uninstall: {removed} entr{} {verb}",
            if removed == 1 { "y" } else { "ies" }
        ));
        return Ok(());
    }

    print_uninstall_results(ctx, args.dry_run, &results, &skill_results);
    Ok(())
}

/// Per-(agent × scope) uninstall walker. Mirrors `walk_install` /
/// `walk_upgrade`: invoked twice from `run_uninstall`, once
/// dry-run, once apply.
fn walk_uninstall(
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
                    let outcome = uninstall_mcp_entry(*agent, concrete_scope, &path, dry_run)?;
                    results.push(outcome);
                }
            }
            if what.includes_skill() {
                let outcome = uninstall_skill(*agent, concrete_scope, project_root, dry_run)?;
                if let Some(o) = outcome {
                    skill_results.push(o);
                }
            }
        }
    }
    Ok((results, skill_results))
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
        ConfigFormat::Json => read_json(config_path)?
            .get(section)
            .and_then(|v| v.get(SERVER_NAME))
            .is_some(),
        ConfigFormat::Toml => read_toml(config_path)?
            .get(section)
            .and_then(|v| v.as_table())
            .and_then(|t| t.get(SERVER_NAME))
            .is_some(),
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
    fs::remove_file(&path).with_context(|| format!("removing SKILL.md `{}`", path.display()))?;
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

fn print_uninstall_results(
    ctx: &output::Context,
    dry_run: bool,
    results: &[AgentInstallReport],
    skill_results: &[SkillInstallReport],
) {
    for r in results {
        let prefix = match r.status {
            "removed" | "would-remove" => {
                if dry_run {
                    "would"
                } else {
                    "removed"
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
            "removed" | "would-remove" => {
                if dry_run {
                    "would"
                } else {
                    "removed"
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
