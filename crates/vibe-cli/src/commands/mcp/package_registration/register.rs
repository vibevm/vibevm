//! Register selected package servers and preserve prior agent configs.

use super::*;

pub(super) fn register_one(
    agent: Agent,
    scope: Scope,
    config: &Path,
    server: &DeclaredMcpServer,
    payload: &ConfigPayload,
    dry_run: bool,
) -> Result<AgentInstallReport> {
    let name = &server.decl.name;
    let wanted = payload_json(payload)?;
    let current = current_entry(agent, config, name)?;
    let mut receipt = read_receipt(config)?;
    let previous = receipt.entries.get(name).cloned();
    if let Some(old) = &previous {
        if old.package != server.package {
            bail!(
                "MCP name `{name}` is already managed for package `{}` in `{}`",
                old.package,
                config.display()
            );
        }
        if current.as_ref() != Some(&old.payload)
            && current.as_ref() != old.pending_previous.as_ref()
            && current.is_some()
        {
            bail!(
                "MCP name `{name}` was changed outside vibe in `{}`; refusing to overwrite",
                config.display()
            );
        }
    } else if current.is_some() {
        // Legacy releases marked package entries inside JSON. Adopt only an
        // exact match; a modified legacy entry has uncertain ownership.
        if !legacy_owned(agent, config, name)? || current.as_ref() != Some(&wanted) {
            bail!(
                "MCP name `{name}` already exists in `{}` without a matching vibe ownership receipt; refusing to overwrite",
                config.display()
            );
        }
    }
    let status = if current.as_ref() == Some(&wanted) {
        "unchanged"
    } else if current.is_some() {
        "updated"
    } else {
        "created"
    };
    let adopting_legacy = previous.is_none() && current.is_some();
    // Parse and render the full target document during preflight. If its MCP
    // section has a conflicting shape, no ownership receipt is created.
    let rendered = if status == "unchanged" {
        None
    } else {
        Some(match payload {
            ConfigPayload::Json(entry) => {
                let mut merged = merge_json(config, agent.mcp_section_key(), name, entry)?;
                vibe_mcp::pkg_servers::mark_managed(&mut merged, name)?;
                serde_json::to_string_pretty(&merged)? + "\n"
            }
            ConfigPayload::Toml(entry) => {
                let merged = merge_toml(config, agent.mcp_section_key(), name, entry)?;
                toml::to_string_pretty(&merged)?
            }
        })
    };
    if !dry_run
        && (status != "unchanged"
            || previous.is_none()
            || previous.as_ref().is_some_and(|p| {
                p.pending_previous.is_some()
                    || p.version.as_deref() != Some(server.version.as_str())
            }))
    {
        if let Some(parent) = config.parent() {
            fs::create_dir_all(parent)?;
        }
        receipt.entries.insert(
            name.clone(),
            OwnedEntry {
                package: server.package.clone(),
                version: Some(server.version.clone()),
                payload: wanted,
                pending_previous: current.clone().filter(|_| status != "unchanged"),
            },
        );
        // Both the previous and desired entry are accepted during a crash
        // recovery retry. Finish the receipt only after the config rename.
        write_receipt(config, &receipt)?;
        if let Some(rendered) = rendered {
            atomic_write(config, rendered.as_bytes())?;
        }
        receipt
            .entries
            .get_mut(name)
            .ok_or_else(|| anyhow::anyhow!("MCP receipt lost entry `{name}` during registration"))?
            .pending_previous = None;
        write_receipt(config, &receipt)?;
    }
    Ok(AgentInstallReport {
        agent: agent.as_str().to_owned(),
        scope: scope.as_str(),
        config_path: machine_json_path(config),
        status: if dry_run {
            if adopting_legacy {
                "would-adopt"
            } else {
                match status {
                    "created" => "would-create",
                    "updated" => "would-update",
                    _ => "unchanged",
                }
            }
        } else if adopting_legacy {
            "adopted"
        } else {
            status
        },
        note: Some(format!("package {} server `{name}`", server.package)),
    })
}

pub(in crate::commands::mcp) fn register(
    root: &Path,
    package: &str,
    only_server: Option<&str>,
    agents: &[Agent],
    scope: Scope,
    consent: bool,
    dry_run: bool,
) -> Result<Vec<AgentInstallReport>> {
    let servers = selected_servers(root, package, only_server)?;
    for server in &servers {
        ensure_scope_supported(scope, server)?;
    }
    // Preflight all selected destinations before writing any of them.
    let mut planned = Vec::new();
    let mut skipped = Vec::new();
    for &agent in agents {
        let Some(config) = agent.config_path(
            scope,
            if scope == Scope::Project {
                Some(root)
            } else {
                None
            },
        )?
        else {
            if agents.len() == 1 {
                bail!(
                    "agent `{}` has no {}-scope MCP configuration",
                    agent.as_str(),
                    scope.as_str()
                );
            }
            skipped.push(AgentInstallReport {
                agent: agent.as_str().to_owned(),
                scope: scope.as_str(),
                config_path: String::new(),
                status: "skipped",
                note: Some(format!(
                    "agent has no {}-scope MCP configuration",
                    scope.as_str()
                )),
            });
            continue;
        };
        for server in &servers {
            let payload = match planned_payload(agent, root, server, consent, agents.len() > 1)? {
                Some(payload) => payload,
                None => {
                    skipped.push(AgentInstallReport {
                        agent: agent.as_str().to_owned(),
                        scope: scope.as_str(),
                        config_path: machine_json_path(&config),
                        status: "unsupported",
                        note: Some(format!("server `{}`: Claude Desktop cannot connect directly to a remote HTTP MCP URL through claude_desktop_config.json", server.decl.name)),
                    });
                    continue;
                }
            };
            planned.push((agent, config.clone(), server.clone(), payload));
        }
    }
    let mut reports = Vec::new();
    for (agent, config, server, payload) in &planned {
        reports.push(register_one(*agent, scope, config, server, payload, true)?);
    }
    if planned.is_empty() {
        bail!(
            "no selected agent has a {}-scope MCP configuration",
            scope.as_str()
        );
    }
    reports.extend(skipped.clone());
    if dry_run {
        return Ok(reports);
    }
    let mut snapshots = BTreeMap::<PathBuf, Snapshot>::new();
    for (_, config, _, _) in &planned {
        if !snapshots.contains_key(config) {
            snapshots.insert(config.clone(), snapshot(config.clone())?);
            let receipt = receipt_path(config)?;
            snapshots.insert(receipt.clone(), snapshot(receipt)?);
        }
    }
    let mut applied = Vec::new();
    for (agent, config, server, payload) in &planned {
        match register_one(*agent, scope, config, server, payload, false) {
            Ok(row) => applied.push(row),
            Err(error) => {
                let failures: Vec<_> = snapshots
                    .values()
                    .filter_map(|saved| {
                        restore(saved)
                            .err()
                            .map(|e| format!("{}: {e}", saved.path.display()))
                    })
                    .collect();
                if failures.is_empty() {
                    return Err(
                        error.context("registration failed; prior config writes rolled back")
                    );
                }
                bail!(
                    "registration failed: {error}; rollback also failed: {}",
                    failures.join("; ")
                );
            }
        }
    }
    applied.extend(skipped);
    Ok(applied)
}
