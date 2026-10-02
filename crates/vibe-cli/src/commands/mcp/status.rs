//! Status reporting for product and package MCP servers.

use super::*;

// ---------------------------------------------------------------------------
// status
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
struct StatusReport {
    ok: bool,
    command: &'static str,
    project: Option<String>,
    detected: Vec<String>,
    /// MCP-config preview entries, one per (agent × concrete-scope)
    /// combination that has a surface.
    results: Vec<AgentInstallReport>,
    /// SKILL.md drift preview entries — same shape as install /
    /// upgrade. Empty for agents without filesystem skill loaders
    /// (Cursor, Claude Desktop). Status is `would-create` /
    /// `would-update` / `unchanged`.
    skill_results: Vec<SkillInstallReport>,
    /// Package-declared servers (PROP-027 §2.4) and their lifecycle
    /// state: registered where, artifact built or the build recipe.
    pkg_servers: Vec<PkgServerStatus>,
}

#[derive(Debug, Clone, Serialize)]
struct PkgServerStatus {
    name: String,
    package: String,
    version: String,
    artifact: Option<String>,
    endpoint: Option<String>,
    /// `built` / `unbuilt` / `remote`.
    artifact_state: &'static str,
    /// The recipe when unbuilt.
    note: Option<String>,
    scope: &'static str,
    registrations: Vec<package_registration::RegistrationStatus>,
}

/// Collect the PROP-027 lifecycle rows for `vibe mcp status`.
fn pkg_server_status(root: &Path, scope: Scope) -> Result<Vec<PkgServerStatus>> {
    let servers = vibe_workspace::bins::collect_mcp_servers(root)
        .map_err(|e| anyhow::anyhow!("collecting MCP servers from `{}`: {e}", root.display()))?;
    servers
        .into_iter()
        .map(|s| -> Result<PkgServerStatus> {
            let mut registrations = Vec::new();
            for agent in Agent::ALL.iter().copied() {
                if let Some(config) =
                    agent.config_path(scope, (scope == Scope::Project).then_some(root))?
                {
                    registrations.push(package_registration::registration_status(
                        agent, scope, &config, &s,
                    )?);
                }
            }
            let artifact = s.binary.as_ref().map(|binary| {
                let path = binary.artifact();
                if path.is_absolute() {
                    path
                } else {
                    root.join(path)
                }
            });
            let built = artifact.as_ref().is_some_and(|path| path.exists());
            Ok(PkgServerStatus {
                name: s.decl.name.clone(),
                package: s.package.clone(),
                version: s.version.clone(),
                artifact: artifact.as_ref().map(|path| machine_json_path(path)),
                endpoint: s.decl.url.clone(),
                artifact_state: if s.decl.url.is_some() {
                    "remote"
                } else if built {
                    "built"
                } else {
                    "unbuilt"
                },
                note: if !built && s.decl.url.is_none() {
                    let binary = s.binary.as_ref().ok_or_else(|| {
                        anyhow::anyhow!("local MCP server `{}` has no binary", s.decl.name)
                    })?;
                    Some(format!(
                        "run `vibe bin build {}` before an agent launches it",
                        binary.decl.name
                    ))
                } else {
                    None
                },
                scope: scope.as_str(),
                registrations,
            })
        })
        .collect()
}

pub(super) fn run_status(ctx: &output::Context, args: McpStatusArgs) -> Result<()> {
    // Status is read-only and scope-agnostic: report on every agent ×
    // every scope that has a surface. Project entries require
    // resolved project_root; user entries don't.
    let project_root: Option<PathBuf> = args
        .path
        .canonicalize()
        .ok()
        .map(crate::commands::init::strip_unc_public)
        .filter(|p| p.join(Manifest::FILENAME).exists());
    let detected = detect_agents(project_root.as_deref());
    let mut results: Vec<AgentInstallReport> = Vec::new();
    let mut skill_results: Vec<SkillInstallReport> = Vec::new();
    for agent in Agent::ALL.iter().copied() {
        for scope in [Scope::Project, Scope::User] {
            if scope == Scope::Project && project_root.is_none() {
                continue;
            }
            // MCP-config preview.
            if let Some(path) = agent.config_path(scope, project_root.as_deref())? {
                let payload = agent.build_mcp_entry();
                results.push(preview_install_mcp(agent, scope, &path, &payload)?);
            }
            // Skill preview — only for agents that load skills + have
            // a path for this scope. install_skill with dry_run=true
            // reuses the decide-then-(don't-)apply logic and emits
            // would-create / would-update / unchanged.
            if agent.supports_skill() && agent.skill_path(scope, project_root.as_deref())?.is_some()
            {
                let outcome = install_skill(agent, scope, project_root.as_deref(), true)?;
                skill_results.push(outcome);
            }
        }
    }
    let mut pkg_servers = if let Some(root) = project_root.as_deref() {
        pkg_server_status(root, Scope::Project)?
    } else {
        Vec::new()
    };
    if let Ok(user_root) = crate::commands::install::user_project_root()
        && user_root.join(Manifest::FILENAME).exists()
    {
        pkg_servers.extend(pkg_server_status(&user_root, Scope::User)?);
    }
    let report = StatusReport {
        ok: true,
        command: "mcp:status",
        project: project_root.as_ref().map(|p| p.display().to_string()),
        detected: detected.iter().map(|a| a.as_str().to_string()).collect(),
        results: results.clone(),
        skill_results: skill_results.clone(),
        pkg_servers: pkg_servers.clone(),
    };
    if ctx.is_json() {
        ctx.emit_json(&report)?;
        return Ok(());
    }
    if ctx.is_quiet() {
        let registered = pkg_servers
            .iter()
            .flat_map(|server| &server.registrations)
            .filter(|registration| registration.state == "registered")
            .count();
        ctx.summary(&format!(
            "vibe mcp status: {} package server{}, {registered} agent registration{}",
            pkg_servers.len(),
            if pkg_servers.len() == 1 { "" } else { "s" },
            if registered == 1 { "" } else { "s" }
        ));
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
            "{} mcp     {} ({}) → {}{note}",
            r.status, r.agent, r.scope, r.config_path
        ));
    }
    for r in &skill_results {
        let note = r
            .note
            .as_deref()
            .map(|n| format!(" ({n})"))
            .unwrap_or_default();
        let path_str = r.path.as_deref().unwrap_or("(no skill loader)");
        ctx.step(&format!(
            "{} skill   {} ({}) → {}{note}",
            r.status, r.agent, r.scope, path_str
        ));
    }
    for s in &pkg_servers {
        let note = s
            .note
            .as_deref()
            .map(|n| format!(" ({n})"))
            .unwrap_or_default();
        ctx.step(&format!(
            "{} server  {} ({}@{}, {}) → {}{note}",
            s.artifact_state,
            s.name,
            s.package,
            s.version,
            s.scope,
            s.endpoint
                .as_deref()
                .or(s.artifact.as_deref())
                .unwrap_or("(missing)")
        ));
        for registration in &s.registrations {
            if registration.state != "absent" {
                ctx.step(&format!(
                    "{} agent   {} ({}) → {}",
                    registration.state,
                    registration.agent,
                    registration.scope,
                    registration.config_path,
                ));
            }
        }
    }
    Ok(())
}
