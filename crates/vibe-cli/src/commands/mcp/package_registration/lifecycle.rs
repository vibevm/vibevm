//! Refresh and transactional global removal for package registrations.

use super::*;

pub(in crate::commands::mcp) fn user_agent_configs_for_package(
    package: &str,
) -> Result<Vec<(Agent, PathBuf)>> {
    let coordinate = normalized_coordinate(package)?;
    let mut found = Vec::new();
    for agent in Agent::ALL.iter().copied() {
        let Some(config) = agent.config_path(Scope::User, None)? else {
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

pub(super) fn preflight_owned_entries(agent: Agent, config: &Path, coordinate: &str) -> Result<()> {
    let receipt = read_receipt(config)?;
    for (name, owned) in receipt
        .entries
        .iter()
        .filter(|(_, owned)| owned.package == coordinate)
    {
        let current = current_entry(agent, config, name)?;
        if current.as_ref() != Some(&owned.payload) {
            bail!(
                "MCP name `{name}` in `{}` differs from its vibe ownership receipt; refusing package update before this is resolved",
                config.display()
            );
        }
    }
    Ok(())
}

pub(in crate::commands::mcp) fn preflight_user_refresh(package: &str) -> Result<()> {
    let coordinate = normalized_coordinate(package)?;
    for (agent, config) in user_agent_configs_for_package(package)? {
        preflight_owned_entries(agent, &config, &coordinate)?;
    }
    Ok(())
}

pub(in crate::commands::mcp) fn refresh_user_package(
    root: &Path,
    package: &str,
    agents: &[Agent],
    consent: bool,
) -> Result<Vec<AgentInstallReport>> {
    let servers = selected_servers(root, package, None)?;
    let wanted_names: std::collections::BTreeSet<_> =
        servers.iter().map(|s| s.decl.name.as_str()).collect();
    // Validate every current registration before changing any agent.
    let mut stale = Vec::<(Agent, Scope, PathBuf, String)>::new();
    let coordinate = normalized_coordinate(package)?;
    let mut snapshots = BTreeMap::<PathBuf, Snapshot>::new();
    for &agent in agents {
        let config = agent.config_path(Scope::User, None)?.ok_or_else(|| {
            anyhow::anyhow!("agent `{}` has no user MCP configuration", agent.as_str())
        })?;
        let receipt = read_receipt(&config)?;
        for (name, entry) in receipt.entries {
            if entry.package == coordinate && !wanted_names.contains(name.as_str()) {
                remove_selected(agent, Scope::User, &config, package, Some(&name), true)?;
                stale.push((agent, Scope::User, config.clone(), name));
            }
        }
        snapshots.insert(config.clone(), snapshot(config.clone())?);
        let receipt_path = receipt_path(&config)?;
        snapshots.insert(receipt_path.clone(), snapshot(receipt_path)?);
    }
    register(root, package, None, agents, Scope::User, consent, true)?;
    let result = (|| -> Result<Vec<AgentInstallReport>> {
        let mut rows = Vec::new();
        for (agent, scope, config, name) in &stale {
            rows.extend(remove_selected(
                *agent,
                *scope,
                config,
                package,
                Some(name),
                false,
            )?);
        }
        rows.extend(register(
            root,
            package,
            None,
            agents,
            Scope::User,
            consent,
            false,
        )?);
        Ok(rows)
    })();
    match result {
        Ok(rows) => Ok(rows),
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
                Err(error.context("global MCP refresh failed; prior config writes rolled back"))
            } else {
                bail!(
                    "global MCP refresh failed: {error}; rollback also failed: {}",
                    failures.join("; ")
                )
            }
        }
    }
}

pub(in crate::commands::mcp) fn remove_from_configs(
    destinations: &[(Agent, Scope, PathBuf)],
    package: &str,
    only_server: Option<&str>,
    dry_run: bool,
) -> Result<Vec<AgentInstallReport>> {
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
    let mut snapshots = BTreeMap::<PathBuf, Snapshot>::new();
    for (_, _, config) in destinations {
        if !snapshots.contains_key(config) {
            snapshots.insert(config.clone(), snapshot(config.clone())?);
            let receipt = receipt_path(config)?;
            snapshots.insert(receipt.clone(), snapshot(receipt)?);
        }
    }
    let mut applied = Vec::new();
    for (agent, scope, config) in destinations {
        match remove_selected(*agent, *scope, config, package, only_server, false) {
            Ok(rows) => applied.extend(rows),
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
                        error.context("unregistration failed; prior config writes rolled back")
                    );
                }
                bail!(
                    "unregistration failed: {error}; rollback also failed: {}",
                    failures.join("; ")
                );
            }
        }
    }
    Ok(applied)
}

pub(in crate::commands::mcp) struct GlobalRemovalPlan {
    package: String,
    destinations: Vec<(Agent, Scope, PathBuf)>,
    snapshots: Vec<Snapshot>,
    after: Vec<Snapshot>,
    pub(in crate::commands::mcp) results: Vec<AgentInstallReport>,
}

pub(in crate::commands::mcp) fn plan_global_removal(package: &str) -> Result<GlobalRemovalPlan> {
    let destinations: Vec<_> = user_agent_configs_for_package(package)?
        .into_iter()
        .map(|(agent, config)| (agent, Scope::User, config))
        .collect();
    // A complete ownership/drift preflight, followed by exact byte snapshots
    // that can restore the user's configs if package removal itself fails.
    remove_from_configs(&destinations, package, None, true)?;
    let mut snapshots = Vec::new();
    for (_, _, config) in &destinations {
        snapshots.push(snapshot(config.clone())?);
        snapshots.push(snapshot(receipt_path(config)?)?);
    }
    Ok(GlobalRemovalPlan {
        package: package.to_owned(),
        destinations,
        snapshots,
        after: Vec::new(),
        results: Vec::new(),
    })
}

pub(in crate::commands::mcp) fn apply_global_removal(plan: &mut GlobalRemovalPlan) -> Result<()> {
    remove_from_configs(&plan.destinations, &plan.package, None, true)?;
    plan.snapshots.clear();
    for (_, _, config) in &plan.destinations {
        plan.snapshots.push(snapshot(config.clone())?);
        plan.snapshots.push(snapshot(receipt_path(config)?)?);
    }
    plan.results = remove_from_configs(&plan.destinations, &plan.package, None, false)?;
    plan.after = plan
        .snapshots
        .iter()
        .map(|saved| snapshot(saved.path.clone()))
        .collect::<Result<_>>()?;
    Ok(())
}

pub(in crate::commands::mcp) fn restore_global_removal(plan: &GlobalRemovalPlan) -> Result<()> {
    if plan.after.len() != plan.snapshots.len() {
        bail!("MCP registration removal has not completed; there is no applied state to restore");
    }
    for after in &plan.after {
        if snapshot(after.path.clone())?.before.as_ref() != after.before.as_ref() {
            bail!(
                "MCP config or receipt changed after removal: `{}`; refusing to overwrite the newer edit",
                after.path.display()
            );
        }
    }
    let failures: Vec<_> = plan
        .snapshots
        .iter()
        .filter_map(|saved| {
            restore(saved)
                .err()
                .map(|e| format!("{}: {e}", saved.path.display()))
        })
        .collect();
    if failures.is_empty() {
        Ok(())
    } else {
        bail!(
            "restoring MCP registrations failed: {}",
            failures.join("; ")
        )
    }
}
