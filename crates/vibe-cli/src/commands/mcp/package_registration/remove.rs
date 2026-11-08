//! Remove selected and legacy package registrations.

use super::*;

pub(super) fn remove_selected(
    agent: Agent,
    scope: Scope,
    config: &Path,
    package: &str,
    only_server: Option<&str>,
    dry_run: bool,
) -> Result<Vec<AgentInstallReport>> {
    let mut batch = PublicationBatch::default();
    remove_selected_in(
        agent,
        scope,
        config,
        package,
        only_server,
        Mutation {
            dry_run,
            batch: &mut batch,
        },
    )
    .map_err(|error| batch.failed(error, "MCP unregistration"))
}

pub(super) fn remove_selected_in(
    agent: Agent,
    scope: Scope,
    config: &Path,
    package: &str,
    only_server: Option<&str>,
    mutation: Mutation<'_>,
) -> Result<Vec<AgentInstallReport>> {
    let Mutation { dry_run, batch } = mutation;
    batch.capture_config(config)?;
    let selected = server_filter(only_server)?;
    let coordinate = normalized_coordinate(package)?;
    let requested_version = selector_exact_version(package)?;
    let mut receipt = read_receipt(config)?;
    let names: Vec<String> = receipt
        .entries
        .iter()
        .filter(|(name, entry)| {
            entry.package == coordinate
                && selected.as_ref().is_none_or(|names| names.contains(name))
        })
        .map(|(name, _)| name.clone())
        .collect();
    if names.is_empty() {
        return Ok(Vec::new());
    }
    if let Some(requested) = requested_version {
        for name in &names {
            let owned = &receipt.entries[name];
            if owned.version.as_deref() != Some(requested) {
                bail!(
                    "MCP selector `{package}` requests version `{requested}`, but receipt for `{name}` records `{}`",
                    owned.version.as_deref().unwrap_or("unknown")
                );
            }
        }
    }
    for name in &names {
        let current = current_entry(agent, config, name)?;
        let expected = receipt.entries[name].expected();
        if current.as_ref() != expected && current.is_some() {
            bail!(
                "MCP name `{name}` was changed outside vibe in `{}`; refusing to remove",
                config.display()
            );
        }
    }
    let mut rows = Vec::new();
    if !dry_run {
        for name in &names {
            if config.exists() {
                match agent.config_format() {
                    ConfigFormat::Json => {
                        let mut stripped = strip_json_entry(config, agent.mcp_section_key(), name)?;
                        vibe_mcp::pkg_servers::unmark_managed(&mut stripped, name);
                        batch.write(
                            config,
                            (serde_json::to_string_pretty(&stripped)? + "\n").as_bytes(),
                        )?;
                    }
                    ConfigFormat::Toml => {
                        let stripped = strip_toml_entry(config, agent.mcp_section_key(), name)?;
                        batch.write(config, toml::to_string_pretty(&stripped)?.as_bytes())?;
                    }
                }
            }
            receipt.entries.remove(name);
        }
        let path = receipt_path(config)?;
        if receipt.entries.is_empty() {
            batch.remove(&path)?;
        } else {
            batch.receipt(config, &receipt)?;
        }
    }
    for name in names {
        rows.push(AgentInstallReport {
            agent: agent.as_str().to_owned(),
            scope: scope.as_str(),
            config_path: machine_json_path(config),
            status: if dry_run { "would-remove" } else { "removed" },
            note: Some(format!("package `{coordinate}` server `{name}`")),
        });
    }
    Ok(rows)
}

pub(in crate::commands::mcp) fn remove_all_managed(
    agent: Agent,
    scope: Scope,
    config: &Path,
    project_root: Option<&Path>,
    dry_run: bool,
) -> Result<Vec<AgentInstallReport>> {
    let receipt = read_receipt(config)?;
    let targets: Vec<_> = receipt
        .entries
        .iter()
        .map(|(name, entry)| (name.clone(), entry.package.clone()))
        .collect();
    let legacy = legacy_removal_candidates(agent, scope, config, project_root, &receipt)?;
    let mut planned = Vec::new();
    for (name, package) in &targets {
        planned.extend(remove_selected(
            agent,
            scope,
            config,
            package,
            Some(&exact_server_filter(std::slice::from_ref(name))?),
            true,
        )?);
    }
    for server in &legacy {
        planned.push(AgentInstallReport {
            agent: agent.as_str().to_owned(),
            scope: scope.as_str(),
            config_path: machine_json_path(config),
            status: "would-remove",
            note: Some(format!(
                "legacy package {} server `{}`",
                server.package, server.decl.name
            )),
        });
    }
    if dry_run {
        return Ok(planned);
    }
    let mut applied = Vec::new();
    for (name, package) in &targets {
        applied.extend(remove_selected(
            agent,
            scope,
            config,
            package,
            Some(&exact_server_filter(std::slice::from_ref(name))?),
            false,
        )?);
    }
    for server in &legacy {
        let root = project_root
            .ok_or_else(|| anyhow::anyhow!("legacy MCP removal requires a project root"))?;
        let payload = server_payload(agent, root, server, true)?;
        register_one(agent, scope, config, server, &payload, false)?;
        applied.extend(remove_selected(
            agent,
            scope,
            config,
            &server.package,
            Some(&exact_server_filter(std::slice::from_ref(
                &server.decl.name,
            ))?),
            false,
        )?);
    }
    Ok(applied)
}

fn legacy_removal_candidates(
    agent: Agent,
    scope: Scope,
    config: &Path,
    project_root: Option<&Path>,
    receipt: &Receipt,
) -> Result<Vec<DeclaredMcpServer>> {
    if scope != Scope::Project
        || agent.config_format() != ConfigFormat::Json
        || !checked_file_exists(config)?
    {
        return Ok(Vec::new());
    }
    let Some(root) = project_root else {
        return Ok(Vec::new());
    };
    let doc = read_json(config)?;
    let legacy_names = vibe_mcp::pkg_servers::managed_entries(&doc);
    if legacy_names.is_empty() {
        return Ok(Vec::new());
    }
    let servers =
        vibe_workspace::bins::collect_mcp_servers(root).map_err(|e| anyhow::anyhow!("{e}"))?;
    let mut candidates = Vec::new();
    let mut names = std::collections::BTreeSet::new();
    for server in servers {
        if !legacy_names.contains(&server.decl.name)
            || receipt.entries.contains_key(&server.decl.name)
        {
            continue;
        }
        if !names.insert(server.decl.name.clone()) {
            bail!(
                "legacy MCP name `{}` is declared by more than one package; refusing ambiguous removal",
                server.decl.name
            );
        }
        let payload = server_payload(agent, root, &server, true)?;
        let current = current_entry(agent, config, &server.decl.name)?;
        if current.as_ref() != Some(&payload_json(&payload)?) {
            bail!(
                "legacy managed MCP name `{}` was changed outside vibe in `{}`; refusing to remove",
                server.decl.name,
                config.display()
            );
        }
        candidates.push(server);
    }
    Ok(candidates)
}

/// Declaration churn retracts projection, retaining any explicit activation intent.
/// Explicit uninstall continues to remove the complete ownership row.
pub(super) fn retire_vanished_in(
    agent: Agent,
    scope: Scope,
    config: &Path,
    package: &str,
    name: &str,
    mutation: Mutation<'_>,
) -> Result<Vec<AgentInstallReport>> {
    let Mutation { dry_run, batch } = mutation;
    batch.capture_config(config)?;
    let mut receipt = read_receipt(config)?;
    let Some(prior) = receipt.entries.get(name).cloned() else {
        return Ok(Vec::new());
    };
    if prior.package != normalized_coordinate(package)? {
        bail!("vanished MCP entry belongs to another package");
    }
    if prior.activation.is_none() {
        return remove_selected_in(
            agent,
            scope,
            config,
            package,
            Some(&exact_server_filter(&[name.into()])?),
            Mutation { dry_run, batch },
        );
    }
    let current = current_entry(agent, config, name)?;
    if current.is_some() && current.as_ref() != prior.expected() {
        bail!("vanished MCP projection changed outside vibe; preserving it and intent");
    }
    if !dry_run {
        let mut retained = prior;
        retained.retracted = true;
        retained.pending_previous = current.clone();
        receipt.entries.insert(name.into(), retained);
        batch.receipt(config, &receipt)?;
        if current.is_some() {
            batch.write(
                config,
                activation::render_projection(agent, config, name, None)?.as_bytes(),
            )?;
        }
        if let Some(row) = receipt.entries.get_mut(name) {
            row.pending_previous = None;
        }
        batch.receipt(config, &receipt)?;
    }
    Ok(vec![AgentInstallReport {
        agent: agent.as_str().into(),
        scope: scope.as_str(),
        config_path: machine_json_path(config),
        status: if dry_run {
            "would-retain-intent"
        } else {
            "retained-intent"
        },
        note: Some(format!(
            "declaration `{name}` absent; activation intent retained"
        )),
    }])
}
