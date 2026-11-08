//! Refresh and transactional global removal for package registrations.

use super::*;

pub(in crate::commands::mcp) fn user_agent_configs_for_package(
    package: &str,
    dirs: &vibe_agent_projection::agents::AgentUserDirectories,
) -> Result<Vec<(Agent, PathBuf)>> {
    let coordinate = normalized_coordinate(package)?;
    let mut found = Vec::new();
    for agent in Agent::ALL.iter().copied() {
        let Some(config) = agent.config_path_with_dirs(Scope::User, None, dirs)? else {
            continue;
        };
        let receipt = read_receipt(&config)?;
        if receipt
            .entries
            .values()
            .any(|entry| entry.package == coordinate)
        {
            found.push((agent, config));
        }
    }
    Ok(found)
}

/// Native config evidence for UI defaults; ownership discovery remains separate.
pub(crate) struct GlobalAgentRegistration {
    pub(crate) agent: Agent,
    pub(crate) registered: usize,
    pub(crate) disabled: usize,
    pub(crate) missing: usize,
    pub(crate) changed: usize,
    pub(crate) pending: usize,
}

/// Spec: spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-027#REG-GLOBAL-UNINSTALL-DEFAULTS
pub(in crate::commands::mcp) fn user_registration_statuses(
    package: &str,
    dirs: &vibe_agent_projection::agents::AgentUserDirectories,
) -> Result<Vec<GlobalAgentRegistration>> {
    let coordinate = normalized_coordinate(package)?;
    let mut statuses = Vec::new();
    for (agent, config) in user_agent_configs_for_package(package, dirs)
        .map_err(|_| anyhow::anyhow!("cannot inspect native MCP ownership records"))?
    {
        let receipt = read_receipt(&config).map_err(|_| {
            anyhow::anyhow!(
                "cannot inspect MCP ownership receipt for `{}` at `{}`",
                agent.as_str(),
                config.display()
            )
        })?;
        let mut status = GlobalAgentRegistration {
            agent,
            registered: 0,
            disabled: 0,
            missing: 0,
            changed: 0,
            pending: 0,
        };
        // Parse once so every owned server is judged against one config snapshot.
        // Discard parser chains: TOML diagnostics can include credentials.
        let native = (|| -> Result<Option<serde_json::Value>> {
            if !checked_file_exists(&config)? {
                return Ok(None);
            }
            let value = match agent.config_format() {
                ConfigFormat::Json => read_json(&config)?,
                ConfigFormat::Toml => serde_json::to_value(read_toml(&config)?)?,
            };
            Ok(Some(value))
        })()
        .map_err(|_| {
            anyhow::anyhow!(
                "cannot inspect native MCP configuration for `{}` at `{}`",
                agent.as_str(),
                config.display()
            )
        })?;
        for (name, owned) in receipt
            .entries
            .iter()
            .filter(|(_, owned)| owned.package == coordinate)
        {
            let current = native.as_ref().and_then(|value| {
                value
                    .get(agent.mcp_section_key())
                    .and_then(|section| section.get(name))
            });
            match current {
                current
                    if !vibe_agent_projection::activation::desired_enabled(
                        owned.activation.as_ref(),
                    ) && owned.disabled_projection(agent, current) =>
                {
                    status.disabled += 1
                }
                Some(value)
                    if !vibe_agent_projection::activation::desired_enabled(
                        owned.activation.as_ref(),
                    ) && Some(value) == owned.payload.as_ref() =>
                {
                    status.pending += 1
                }
                Some(value) if !owned.retracted && Some(value) == owned.payload.as_ref() => {
                    status.registered += 1
                }
                Some(value) if owned.pending_previous.as_ref() == Some(value) => {
                    status.pending += 1
                }
                Some(_) => status.changed += 1,
                None => status.missing += 1,
            }
        }
        statuses.push(status);
    }
    Ok(statuses)
}

pub(super) fn preflight_owned_entries(agent: Agent, config: &Path, coordinate: &str) -> Result<()> {
    let receipt = read_receipt(config)?;
    for (name, owned) in receipt
        .entries
        .iter()
        .filter(|(_, owned)| owned.package == coordinate)
    {
        let current = current_entry(agent, config, name)?;
        if current.as_ref() != owned.expected()
            && !owned
                .pending_previous
                .as_ref()
                .is_some_and(|previous| current.as_ref() == Some(previous))
        {
            bail!(
                "MCP name `{name}` in `{}` differs from its vibe ownership receipt; refusing package update before this is resolved",
                config.display()
            );
        }
    }
    Ok(())
}

pub(in crate::commands::mcp) fn preflight_user_refresh(
    package: &str,
    dirs: &vibe_agent_projection::agents::AgentUserDirectories,
) -> Result<()> {
    let coordinate = normalized_coordinate(package)?;
    for (agent, config) in user_agent_configs_for_package(package, dirs)? {
        preflight_owned_entries(agent, &config, &coordinate)?;
    }
    Ok(())
}

pub(in crate::commands::mcp) fn refresh_user_package(
    root: &Path,
    package: &str,
    agents: &[Agent],
    consent: bool,
    dirs: &vibe_agent_projection::agents::AgentUserDirectories,
) -> Result<Vec<AgentInstallReport>> {
    let mut batch = PublicationBatch::default();
    refresh_user_package_in(root, package, agents, consent, dirs, &mut batch)
        .map_err(|error| batch.failed(error, "global MCP refresh"))
}

pub(super) fn refresh_user_package_in(
    root: &Path,
    package: &str,
    agents: &[Agent],
    consent: bool,
    dirs: &vibe_agent_projection::agents::AgentUserDirectories,
    batch: &mut PublicationBatch,
) -> Result<Vec<AgentInstallReport>> {
    let servers = selected_servers(root, package, None)?;
    let wanted_names: std::collections::BTreeSet<_> =
        servers.iter().map(|s| s.decl.name.as_str()).collect();
    // Validate every current registration before changing any agent.
    let mut stale = Vec::<(Agent, Scope, PathBuf, String)>::new();
    let mut refresh = Vec::<(Agent, String)>::new();
    let coordinate = normalized_coordinate(package)?;
    for &agent in agents {
        let config = agent
            .config_path_with_dirs(Scope::User, None, dirs)?
            .ok_or_else(|| {
                anyhow::anyhow!("agent `{}` has no user MCP configuration", agent.as_str())
            })?;
        let receipt = read_receipt(&config)?;
        for (name, entry) in receipt.entries {
            if entry.package != coordinate {
                continue;
            }
            if wanted_names.contains(name.as_str()) {
                refresh.push((agent, name));
            } else {
                remove_selected(
                    agent,
                    Scope::User,
                    &config,
                    package,
                    Some(&exact_server_filter(std::slice::from_ref(&name))?),
                    true,
                )?;
                stale.push((agent, Scope::User, config.clone(), name));
            }
        }
        batch.capture_config(&config)?;
    }
    for (agent, name) in &refresh {
        register_with_dirs(
            root,
            package,
            Some(&exact_server_filter(std::slice::from_ref(name))?),
            &[*agent],
            Scope::User,
            RegistrationPolicy {
                consent,
                dry_run: true,
            },
            dirs,
        )?;
    }
    let mut rows = Vec::new();
    for (agent, scope, config, name) in &stale {
        rows.extend(remove::retire_vanished_in(
            *agent,
            *scope,
            config,
            package,
            name,
            Mutation {
                dry_run: false,
                batch: &mut *batch,
            },
        )?);
    }
    for (agent, name) in &refresh {
        rows.extend(register::register_with_dirs_in(
            root,
            package,
            Some(&exact_server_filter(std::slice::from_ref(name))?),
            &[*agent],
            Scope::User,
            register::RegistrationExecution {
                policy: RegistrationPolicy {
                    consent,
                    dry_run: false,
                },
                batch: &mut *batch,
            },
            dirs,
        )?);
    }
    Ok(rows)
}

pub(in crate::commands::mcp) fn remove_from_configs(
    destinations: &[(Agent, Scope, PathBuf)],
    package: &str,
    only_server: Option<&str>,
    dry_run: bool,
) -> Result<Vec<AgentInstallReport>> {
    let mut batch = PublicationBatch::default();
    remove_from_configs_in(
        destinations,
        package,
        only_server,
        Mutation {
            dry_run,
            batch: &mut batch,
        },
    )
    .map_err(|error| batch.failed(error, "MCP batch unregistration"))
}

fn remove_from_configs_in(
    destinations: &[(Agent, Scope, PathBuf)],
    package: &str,
    only_server: Option<&str>,
    mutation: Mutation<'_>,
) -> Result<Vec<AgentInstallReport>> {
    let Mutation { dry_run, batch } = mutation;
    if let Some(names) = server_filter(only_server)? {
        let coordinate = normalized_coordinate(package)?;
        let mut available = Vec::new();
        for (_, _, config) in destinations {
            available.extend(
                read_receipt(config)?
                    .entries
                    .into_iter()
                    .filter(|(_, entry)| entry.package == coordinate)
                    .map(|(name, _)| name),
            );
        }
        validate_server_names(
            &names,
            &available.iter().map(String::as_str).collect::<Vec<_>>(),
        )?;
    }
    let mut preview = Vec::new();
    for (agent, scope, config) in destinations {
        preview.extend(remove_selected(
            *agent,
            *scope,
            config,
            package,
            only_server,
            true,
        )?);
    }
    if dry_run {
        return Ok(preview);
    }
    for (_, _, config) in destinations {
        batch.capture_config(config)?;
    }
    let mut applied = Vec::new();
    for (agent, scope, config) in destinations {
        applied.extend(remove::remove_selected_in(
            *agent,
            *scope,
            config,
            package,
            only_server,
            Mutation {
                dry_run: false,
                batch: &mut *batch,
            },
        )?);
    }
    Ok(applied)
}

pub(in crate::commands::mcp) struct GlobalRemovalPlan {
    dirs: vibe_agent_projection::agents::AgentUserDirectories,
    package: String,
    destinations: Vec<(Agent, Scope, PathBuf)>,
    applied: Option<PublicationBatch>,
    pub(in crate::commands::mcp) package_removed: bool,
    pub(in crate::commands::mcp) results: Vec<AgentInstallReport>,
}

pub(in crate::commands::mcp) fn plan_global_removal(
    package: &str,
    agents: &[Agent],
    dirs: &vibe_agent_projection::agents::AgentUserDirectories,
) -> Result<GlobalRemovalPlan> {
    let owned = user_agent_configs_for_package(package, dirs)?;
    let package_removed = owned.iter().all(|(agent, _)| agents.contains(agent));
    let destinations: Vec<_> = owned
        .into_iter()
        .filter(|(agent, _)| agents.contains(agent))
        .map(|(agent, config)| (agent, Scope::User, config))
        .collect();
    // Read-only ownership preflight. Apply later retains exact before-images
    // and the actual writer's publication receipts for any required inverse.
    remove_from_configs(&destinations, package, None, true)?;
    Ok(GlobalRemovalPlan {
        dirs: dirs.clone(),
        package: package.to_owned(),
        package_removed,
        destinations,
        applied: None,
        results: Vec::new(),
    })
}

pub(in crate::commands::mcp) fn apply_global_removal(plan: &mut GlobalRemovalPlan) -> Result<()> {
    let owned = user_agent_configs_for_package(&plan.package, &plan.dirs)?;
    plan.package_removed = owned.iter().all(|(agent, _)| {
        plan.destinations
            .iter()
            .any(|(selected, _, _)| selected == agent)
    });
    remove_from_configs(&plan.destinations, &plan.package, None, true)?;
    let mut batch = PublicationBatch::default();
    let result = remove_from_configs_in(
        &plan.destinations,
        &plan.package,
        None,
        Mutation {
            dry_run: false,
            batch: &mut batch,
        },
    );
    match result {
        Ok(rows) => {
            plan.results = rows;
            plan.applied = Some(batch);
            Ok(())
        }
        Err(error) => {
            let error = batch.failed(error, "global MCP removal");
            plan.applied = Some(batch);
            Err(error)
        }
    }
}

pub(in crate::commands::mcp) fn restore_global_removal(plan: &mut GlobalRemovalPlan) -> Result<()> {
    let batch = plan
        .applied
        .as_mut()
        .ok_or_else(|| anyhow::anyhow!("MCP removal has no applied publication record"))?;
    batch
        .rollback()
        .context("restoring MCP registrations; recovery required on conflict")
}
