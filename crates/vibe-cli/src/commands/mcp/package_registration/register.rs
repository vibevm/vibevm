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
    register_requested(agent, scope, config, server, payload, None, dry_run)
}

pub(super) fn register_requested(
    agent: Agent,
    scope: Scope,
    config: &Path,
    server: &DeclaredMcpServer,
    payload: &ConfigPayload,
    request: Option<&vibe_agent_projection::activation::ActivationIntent>,
    dry_run: bool,
) -> Result<AgentInstallReport> {
    let retained = request.is_some() && !dry_run;
    if retained {
        activation::retain_known_intent(
            config,
            server,
            request.ok_or_else(|| anyhow::anyhow!("missing activation request"))?,
        )?;
    }
    let mut batch = PublicationBatch::default();
    register_requested_in(
        agent,
        scope,
        config,
        server,
        payload,
        request,
        Mutation {
            dry_run,
            batch: &mut batch,
        },
    )
    .map_err(|error| {
        let error = batch.failed(error, "MCP registration");
        if retained {
            error.context("activation intent retained; projection reconciliation failed")
        } else {
            error
        }
    })
}

#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-073#MANAGED-ACTIVATION")]

pub(super) fn register_requested_in(
    agent: Agent,
    scope: Scope,
    config: &Path,
    server: &DeclaredMcpServer,
    payload: &ConfigPayload,
    request: Option<&vibe_agent_projection::activation::ActivationIntent>,
    mutation: Mutation<'_>,
) -> Result<AgentInstallReport> {
    let Mutation { dry_run, batch } = mutation;
    batch.capture_config(config)?;
    let name = &server.decl.name;
    let canonical = payload_json(payload)?;
    let config_before = if checked_file_exists(config)? {
        Some(fs::read(config)?)
    } else {
        None
    };
    let current = current_entry(agent, config, name)?;
    let mut receipt = read_receipt(config)?;
    let previous = receipt.entries.get(name).cloned();
    if let Some(old) = &previous {
        if old.package != server.package {
            bail!("MCP name `{name}` is owned by another package");
        }
        if current.as_ref() != old.expected()
            && current.as_ref() != old.pending_previous.as_ref()
            && current.is_some()
        {
            bail!("MCP name `{name}` changed outside vibe; refusing to overwrite");
        }
    } else if current.is_some()
        && ((server.package == activation::BUILTIN_PACKAGE || !legacy_owned(agent, config, name)?)
            || current.as_ref() != Some(&canonical))
    {
        bail!("MCP name `{name}` has no matching ownership receipt; refusing to overwrite");
    }
    if request.is_some() && previous.is_none() && current.is_none() {
        bail!("MCP name `{name}` is not managed; install it before changing activation");
    }
    let intent = request
        .cloned()
        .or_else(|| previous.as_ref().and_then(|p| p.activation.clone()));
    if let Some(intent) = &intent {
        vibe_agent_projection::activation::validate(intent)?;
    }
    let enabled = vibe_agent_projection::activation::desired_enabled(intent.as_ref());
    // OpenCode V1: https://docs.opencode.ai/docs/mcp-servers/
    // Codex: https://learn.chatgpt.com/docs/config-file/config-reference
    // Both native contracts explicitly define enabled=false.
    // Codex also supports enabled=false in mcp_servers; other adapters retract.
    let mut wanted = canonical;
    let native = matches!(agent, Agent::OpenCode | Agent::Codex);
    let retracted = !enabled && !native;
    if !enabled && native {
        wanted
            .as_object_mut()
            .ok_or_else(|| anyhow::anyhow!("native MCP payload must be an object"))?
            .insert("enabled".into(), false.into());
    }
    let desired = (!retracted).then_some(&wanted);
    let adopting = previous.is_none() && current.is_some();
    let unchanged = current.as_ref() == desired;
    let rendered = if unchanged {
        None
    } else {
        Some(activation::render_projection(agent, config, name, desired)?)
    };
    let status = if !enabled {
        if dry_run && !unchanged {
            "would-disable"
        } else {
            "disabled"
        }
    } else if unchanged {
        "unchanged"
    } else if current.is_some() {
        "updated"
    } else {
        "created"
    };
    let intent_changed = previous.as_ref().and_then(|p| p.activation.as_ref()) != intent.as_ref();
    if !dry_run
        && (!unchanged
            || previous.is_none()
            || intent_changed
            || previous.as_ref().is_some_and(|p| {
                p.pending_previous.is_some() || p.version.as_ref() != Some(&server.version)
            }))
    {
        receipt.entries.insert(
            name.clone(),
            OwnedEntry {
                package: server.package.clone(),
                version: Some(server.version.clone()),
                activation: intent,
                retracted,
                payload: Some(wanted),
                pending_previous: current.clone().filter(|_| !unchanged),
            },
        );
        batch.receipt(config, &receipt)?;
        if let Some(rendered) = rendered {
            // Recheck before publish: a foreign edit after preflight is not ours.
            let visible = if checked_file_exists(config)? {
                Some(fs::read(config)?)
            } else {
                None
            };
            if visible != config_before {
                bail!(
                    "MCP name `{name}` changed during activation; retained intent requires reconciliation"
                );
            }
            batch.write(config, rendered.as_bytes())?;
        }
        if let Some(row) = receipt.entries.get_mut(name) {
            row.pending_previous = None;
        }
        batch.receipt(config, &receipt)?;
    }
    Ok(AgentInstallReport {
        agent: agent.as_str().into(),
        scope: scope.as_str(),
        config_path: machine_json_path(config),
        status: if adopting && enabled {
            if dry_run { "would-adopt" } else { "adopted" }
        } else if dry_run && enabled {
            vibe_agent_projection::preview_status(status, true)
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
    register_with_dirs(
        root,
        package,
        only_server,
        agents,
        scope,
        RegistrationPolicy { consent, dry_run },
        &vibe_agent_projection::agents::AgentUserDirectories::ambient(),
    )
}

pub(in crate::commands::mcp) struct RegistrationPolicy {
    pub(in crate::commands::mcp) consent: bool,
    pub(in crate::commands::mcp) dry_run: bool,
}

pub(in crate::commands::mcp) fn register_with_dirs(
    root: &Path,
    package: &str,
    only_server: Option<&str>,
    agents: &[Agent],
    scope: Scope,
    policy: RegistrationPolicy,
    dirs: &vibe_agent_projection::agents::AgentUserDirectories,
) -> Result<Vec<AgentInstallReport>> {
    let mut batch = PublicationBatch::default();
    register_with_dirs_in(
        root,
        package,
        only_server,
        agents,
        scope,
        RegistrationExecution {
            policy,
            batch: &mut batch,
        },
        dirs,
    )
    .map_err(|error| batch.failed(error, "MCP batch registration"))
}

pub(super) struct RegistrationExecution<'a> {
    pub(super) policy: RegistrationPolicy,
    pub(super) batch: &'a mut PublicationBatch,
}

pub(super) fn register_with_dirs_in(
    root: &Path,
    package: &str,
    only_server: Option<&str>,
    agents: &[Agent],
    scope: Scope,
    execution: RegistrationExecution<'_>,
    dirs: &vibe_agent_projection::agents::AgentUserDirectories,
) -> Result<Vec<AgentInstallReport>> {
    let RegistrationExecution { policy, batch } = execution;
    let RegistrationPolicy { consent, dry_run } = policy;
    let servers = selected_servers(root, package, only_server)?;
    for server in &servers {
        ensure_scope_supported(scope, server)?;
    }
    // Preflight all selected destinations before writing any of them.
    let mut planned = Vec::new();
    let mut skipped = Vec::new();
    for &agent in agents {
        let Some(config) = agent.config_path_with_dirs(
            scope,
            if scope == Scope::Project {
                Some(root)
            } else {
                None
            },
            dirs,
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
    for (_, config, _, _) in &planned {
        batch.capture_config(config)?;
    }
    let mut applied = Vec::new();
    for (agent, config, server, payload) in &planned {
        applied.push(register_requested_in(
            *agent,
            scope,
            config,
            server,
            payload,
            None,
            Mutation {
                dry_run: false,
                batch: &mut *batch,
            },
        )?);
    }
    applied.extend(skipped);
    Ok(applied)
}
