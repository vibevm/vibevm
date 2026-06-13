//! `vibe mcp install` — detection-driven, two-pass (preview/confirm/apply)
//! per-agent MCP-config + SKILL.md install (PROP-015 §2.7). Split out of
//! the mcp god-file (CONVERT-PLAN v0.1 §7.3d).

use super::*;

// ---------------------------------------------------------------------------
// Reporting
// ---------------------------------------------------------------------------

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

pub(super) fn run_install(ctx: &output::Context, args: McpInstallArgs) -> Result<()> {
    // The mode is `auto` if --auto was passed; `flags` if any of
    // (--scope/--what/--agent) was passed without --auto and we don't
    // need to ask anything; `interactive` otherwise (asks via wizard).
    let any_explicit_target = args.agent.is_some() || args.scope.is_some() || args.what.is_some();
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

    // Under `--unattended` (or `VIBE_UNATTENDED`), no wizard may
    // open. The operator is in a script and a hung dialoguer prompt
    // would deadlock CI. Detect missing dimensions early and bail
    // with a concrete, actionable hint rather than letting the
    // interactive branches below try to prompt.
    if ctx.is_unattended() && !args.auto {
        let mut missing: Vec<&'static str> = Vec::new();
        if args.scope.is_none() {
            missing.push("--scope");
        }
        if args.what.is_none() {
            missing.push("--what");
        }
        if args.agent.is_none() {
            missing.push("--agent");
        }
        if !missing.is_empty() {
            bail!(
                "unattended mode requires every wizard dimension to be explicit; missing: {}. \
                 Either supply the missing flag(s), or use `--auto` to detect every \
                 dimension automatically.",
                missing.join(", ")
            );
        }
    }

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

    // 2. Resolve project_root. Two policies, mirroring the model in
    //    `vibe mcp upgrade` / `vibe mcp uninstall`:
    //
    //    - `Scope::requires_vibe_toml()` (only `Project`) → bail when
    //      `vibe.toml` is missing. The operator was explicit and
    //      there is nothing to write into.
    //    - Otherwise (`User` or `Both`) → best-effort: read the
    //      project_root only if `vibe.toml` exists; leave it as
    //      `None` if not. The walker below skips the project-leg
    //      when project_root is None, so the user-leg of `Both`
    //      runs unattended even on a fresh machine. This is what
    //      makes `--scope both` usable from first-time-user
    //      provisioning scripts.
    let project_root: Option<PathBuf> = if scope.requires_vibe_toml() {
        Some(resolve_project_root_required(&args.path)?)
    } else {
        args.path
            .canonicalize()
            .ok()
            .map(crate::commands::init::strip_unc_public)
            .filter(|p| p.join(Manifest::FILENAME).exists())
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
    //    scope, or when `Both` was selected without a `vibe.toml`
    //    making the project-leg unreachable).
    let project_leg_skipped_no_manifest = scope == Scope::Both && project_root.is_none();

    // Two-pass walk so the operator's `--yes` / `--unattended` /
    // `--auto` / `--json` / `--dry-run` flags actually gate a
    // confirmation prompt (PROP-002 §2.3.1 hint about destructive
    // operations). First pass is always dry-run — gathers the
    // would-do/won't-do list without touching any config files.
    // Second pass writes only when (a) the operator approved AND
    // (b) the original invocation wasn't `--dry-run`.
    let (preview_results, preview_skill) = walk_install(
        &targeted,
        scope,
        project_root.as_deref(),
        what,
        args.force,
        true,
    )?;

    let needs_change = preview_results
        .iter()
        .any(|r| matches!(r.status, "would-create" | "would-update"))
        || preview_skill
            .iter()
            .any(|r| matches!(r.status, "would-create" | "would-update"));

    if !args.dry_run && needs_change {
        // Confirmation gating: skip the prompt when the operator
        // already signalled "go" via flag / env, OR when we are
        // not attached to a TTY (CI / opencode harness — the
        // pre-this-commit behaviour with no confirm at all is the
        // baseline; we never break those scripts). Show the
        // interactive prompt only on a real TTY without an
        // explicit skip-flag.
        let approved = if args.yes
            || ctx.is_unattended()
            || args.auto
            || ctx.is_json()
            || !console::user_attended()
        {
            true
        } else {
            print_install_results(ctx, true, &preview_results, &preview_skill);
            let mcp_count = preview_results
                .iter()
                .filter(|r| matches!(r.status, "would-create" | "would-update"))
                .count();
            let skill_count = preview_skill
                .iter()
                .filter(|r| matches!(r.status, "would-create" | "would-update"))
                .count();
            Confirm::new()
                .with_prompt(format!(
                    "Apply this plan? ({mcp_count} MCP entr{}, {skill_count} SKILL.md file{})",
                    if mcp_count == 1 { "y" } else { "ies" },
                    if skill_count == 1 { "" } else { "s" },
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
        walk_install(
            &targeted,
            scope,
            project_root.as_deref(),
            what,
            args.force,
            false,
        )?
    };

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
        let mcp_written = results
            .iter()
            .filter(|r| matches!(r.status, "created" | "updated"))
            .count();
        let skill_written = skill_results
            .iter()
            .filter(|r| matches!(r.status, "created" | "updated"))
            .count();
        let verb = if args.dry_run { "previewed" } else { "written" };
        ctx.summary(&format!(
            "vibe mcp install: scope={} what={} — {mcp_written} MCP + {skill_written} skill {verb}",
            scope.as_str(),
            what.as_str()
        ));
        return Ok(());
    }
    print_install_results(ctx, args.dry_run, &results, &skill_results);
    if project_leg_skipped_no_manifest {
        ctx.step(&format!(
            "note: --scope both was requested but `{}` carries no `vibe.toml`; \
             project-scope leg skipped, only user-level installs were written. \
             Run `vibe init` here first if you want both legs.",
            args.path.display()
        ));
    }
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
        let note = r
            .note
            .as_deref()
            .map(|n| format!(" ({n})"))
            .unwrap_or_default();
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

/// Per-(agent × scope) install walker. Extracted from `run_install`
/// so the two-pass `confirm-then-apply` flow can call it twice —
/// once with `dry_run = true` to gather the would-do plan, then
/// (after the operator approves) once with `dry_run = false` to
/// actually write. The semantics inside the loop are unchanged
/// from the prior single-pass implementation; only the surrounding
/// state lives in `run_install` now.
fn walk_install(
    targeted: &[Agent],
    scope: Scope,
    project_root: Option<&Path>,
    what: What,
    _force: bool,
    dry_run: bool,
) -> Result<(Vec<AgentInstallReport>, Vec<SkillInstallReport>)> {
    let mut results: Vec<AgentInstallReport> = Vec::new();
    let mut skill_results: Vec<SkillInstallReport> = Vec::new();
    for agent in targeted {
        for concrete_scope in scope.expand() {
            // `Both` without `vibe.toml`: the user-leg runs as
            // normal, the project-leg is silently skipped.
            if concrete_scope == Scope::Project && project_root.is_none() {
                continue;
            }
            // ---- MCP entry ----
            if what.includes_mcp() {
                let path = agent.config_path(concrete_scope, project_root)?;
                if let Some(path) = path {
                    let payload = agent.build_mcp_entry(concrete_scope, project_root);
                    let outcome = if dry_run {
                        preview_install_mcp(*agent, concrete_scope, &path, &payload)?
                    } else {
                        apply_install_mcp(*agent, concrete_scope, &path, &payload)?
                    };
                    results.push(outcome);
                } else if scope == Scope::Both {
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
                let outcome = install_skill(*agent, concrete_scope, project_root, dry_run)?;
                skill_results.push(outcome);
            }
        }
    }
    Ok((results, skill_results))
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
