//! Registration of a selected package server in an agent's native config.
//! The adjacent receipt records the exact entry last written by vibe. A
//! changed or unreceipted entry is operator-owned and is never overwritten.

use super::*;
use serde::Serialize;
use std::collections::BTreeMap;
use vibe_workspace::bins::DeclaredMcpServer;

#[derive(Debug, Clone, Serialize)]
pub(super) struct RegistrationStatus {
    pub(super) agent: &'static str,
    pub(super) scope: &'static str,
    pub(super) config_path: String,
    /// `registered`, `pending`, `missing`, `changed`, `foreign`, or `absent`.
    pub(super) state: &'static str,
    pub(super) desired_enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) activation: Option<vibe_agent_projection::activation::ActivationIntent>,
}

pub(super) fn registration_status(
    agent: Agent,
    scope: Scope,
    config: &Path,
    server: &DeclaredMcpServer,
) -> Result<RegistrationStatus> {
    let current = current_entry(agent, config, &server.decl.name)?;
    let receipt = read_receipt(config)?;
    let owned = receipt
        .entries
        .get(&server.decl.name)
        .filter(|owned| owned.package == server.package);
    let state = match (receipt.entries.get(&server.decl.name), current.as_ref()) {
        (Some(owned), current)
            if owned.package == server.package
                && !vibe_agent_projection::activation::desired_enabled(
                    owned.activation.as_ref(),
                )
                && owned.disabled_projection(agent, current) =>
        {
            "disabled"
        }
        (Some(owned), Some(value))
            if owned.package == server.package
                && !vibe_agent_projection::activation::desired_enabled(
                    owned.activation.as_ref(),
                )
                && Some(value) == owned.payload.as_ref() =>
        {
            "pending"
        }
        (Some(owned), Some(value))
            if owned.package == server.package
                && !owned.retracted
                && Some(value) == owned.payload.as_ref() =>
        {
            "registered"
        }
        (Some(owned), Some(value))
            if owned.package == server.package
                && owned.pending_previous.as_ref() == Some(value) =>
        {
            "pending"
        }
        (Some(owned), None) if owned.package == server.package => "missing",
        (Some(owned), _) if owned.package == server.package => "changed",
        (Some(_), _) | (None, Some(_)) => "foreign",
        (None, None) => "absent",
    };
    Ok(RegistrationStatus {
        agent: agent.as_str(),
        scope: scope.as_str(),
        config_path: machine_json_path(config),
        state,
        desired_enabled: vibe_agent_projection::activation::desired_enabled(
            owned.and_then(|o| o.activation.as_ref()),
        ),
        activation: owned.and_then(|o| o.activation.clone()),
    })
}

use vibe_wire::generated::mcp_registration_receipt::{
    McpRegistrationReceipt as Receipt, OwnedEntry,
};

trait OwnedProjection {
    fn expected(&self) -> Option<&serde_json::Value>;
    fn disabled_projection(&self, agent: Agent, current: Option<&serde_json::Value>) -> bool;
}

impl OwnedProjection for OwnedEntry {
    fn expected(&self) -> Option<&serde_json::Value> {
        if self.retracted {
            None
        } else {
            self.payload.as_ref()
        }
    }
    fn disabled_projection(&self, agent: Agent, current: Option<&serde_json::Value>) -> bool {
        current == self.expected()
            && (self.retracted
                || (matches!(agent, Agent::OpenCode | Agent::Codex)
                    && self.payload.as_ref().is_some_and(|p| {
                        p.get("enabled") == Some(&serde_json::Value::Bool(false))
                    })))
    }
}

fn receipt_path(config: &Path) -> Result<PathBuf> {
    let name = config
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("invalid agent config path"))?;
    Ok(config.with_file_name(format!("{}.vibevm-mcp.json", name.to_string_lossy())))
}

fn read_receipt(config: &Path) -> Result<Receipt> {
    let path = receipt_path(config)?;
    if !checked_file_exists(&path)? {
        return Ok(empty_receipt());
    }
    let text =
        fs::read_to_string(&path).with_context(|| format!("reading `{}`", path.display()))?;
    let receipt: Receipt = serde_json::from_str(&text)
        .map_err(|_| anyhow::anyhow!("malformed MCP ownership receipt at `{}`", path.display()))?;
    if !matches!(receipt.schema.unwrap_or(1), 1 | 2) {
        bail!(
            "unsupported MCP receipt schema {}; requires vibe >= 1.0.12",
            receipt.schema.unwrap_or(1)
        );
    }
    for entry in receipt.entries.values() {
        if entry.payload.is_none() {
            bail!("MCP receipt lacks a native payload; refusing ownership");
        }
        if let Some(intent) = &entry.activation {
            vibe_agent_projection::activation::validate(intent)?;
        }
        if receipt.schema.unwrap_or(1) == 1 && (entry.activation.is_some() || entry.retracted) {
            bail!("MCP activation requires receipt schema 2");
        }
    }
    Ok(receipt)
}

fn encode_receipt(receipt: &Receipt) -> Result<Vec<u8>> {
    let mut wire = serde_json::to_value(receipt)?;
    wire["schema"] = 2.into();
    Ok((serde_json::to_string_pretty(&wire)? + "\n").into_bytes())
}

#[cfg(test)]
fn write_receipt(config: &Path, receipt: &Receipt) -> Result<()> {
    PublicationBatch::default().receipt(config, receipt)
}

#[cfg(test)]
fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    PublicationBatch::default().write(path, bytes)
}

#[cfg(test)]
#[derive(Clone)]
struct Snapshot {
    path: PathBuf,
    before: Option<Vec<u8>>,
}

#[cfg(test)]
fn snapshot(path: PathBuf) -> Result<Snapshot> {
    let before = if checked_file_exists(&path)? {
        Some(fs::read(&path)?)
    } else {
        None
    };
    Ok(Snapshot { path, before })
}

fn payload_json(payload: &ConfigPayload) -> Result<serde_json::Value> {
    match payload {
        ConfigPayload::Json(v) => Ok(v.clone()),
        ConfigPayload::Toml(v) => Ok(serde_json::to_value(v)?),
    }
}

fn current_entry(agent: Agent, config: &Path, name: &str) -> Result<Option<serde_json::Value>> {
    if !checked_file_exists(config)? {
        return Ok(None);
    }
    let section = agent.mcp_section_key();
    match agent.config_format() {
        ConfigFormat::Json => Ok(read_json(config)?
            .get(section)
            .and_then(|v| v.get(name))
            .cloned()),
        ConfigFormat::Toml => Ok(read_toml(config)?
            .get(section)
            .and_then(toml::Value::as_table)
            .and_then(|t| t.get(name))
            .map(serde_json::to_value)
            .transpose()?),
    }
}

fn legacy_owned(agent: Agent, config: &Path, name: &str) -> Result<bool> {
    if agent.config_format() != ConfigFormat::Json || !checked_file_exists(config)? {
        return Ok(false);
    }
    Ok(vibe_mcp::pkg_servers::managed_entries(&read_json(config)?)
        .iter()
        .any(|managed| managed == name))
}

fn require_regular_file(path: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        bail!(
            "refusing MCP config or receipt that is not a regular file: `{}`",
            path.display()
        );
    }
    Ok(())
}

fn checked_file_exists(path: &Path) -> Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(_) => {
            require_regular_file(path)?;
            Ok(true)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error).with_context(|| format!("checking `{}`", path.display())),
    }
}

pub(super) fn server_payload(
    agent: Agent,
    root: &Path,
    server: &DeclaredMcpServer,
    consent: bool,
) -> Result<ConfigPayload> {
    if let Some(url) = &server.decl.url {
        return vibe_mcp::pkg_servers::remote_entry_payload(agent, url);
    }
    let binary = server
        .binary
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("local MCP server `{}` has no binary", server.decl.name))?;
    vibe_workspace::bins::consent_to_build(binary, consent).map_err(|e| anyhow::anyhow!("{e}"))?;
    let artifact = binary.artifact();
    let artifact = if artifact.is_absolute() {
        artifact
    } else {
        root.join(artifact)
    };
    let command = machine_json_path(&vibe_mcp::pkg_servers::verbatim_free(&artifact));
    let args = vibe_mcp::pkg_servers::substituted_args(&server.decl.args, root);
    Ok(vibe_mcp::pkg_servers::entry_payload(agent, &command, &args))
}

fn ensure_scope_supported(scope: Scope, server: &DeclaredMcpServer) -> Result<()> {
    if scope == Scope::User
        && server.decl.binary.is_some()
        && server
            .decl
            .args
            .iter()
            .any(|arg| arg.contains("{project_root}"))
    {
        bail!(
            "local MCP server `{}` uses `{{project_root}}`; user/global registration would bind it to the package inventory instead of the active project. Use project scope or remove that argument",
            server.decl.name
        );
    }
    Ok(())
}

fn planned_payload(
    agent: Agent,
    root: &Path,
    server: &DeclaredMcpServer,
    consent: bool,
    multiple_agents: bool,
) -> Result<Option<ConfigPayload>> {
    match server_payload(agent, root, server, consent) {
        Ok(payload) => Ok(Some(payload)),
        Err(_)
            if agent == Agent::ClaudeCodeDesktop
                && server.decl.url.is_some()
                && multiple_agents =>
        {
            Ok(None)
        }
        Err(reason) => Err(reason),
    }
}

fn server_filter(filter: Option<&str>) -> Result<Option<Vec<String>>> {
    filter
        .map(|filter| {
            let requested: Vec<String> = if filter.trim_start().starts_with('[') {
                serde_json::from_str(filter)
                    .context("--server JSON selection must be a string array")?
            } else {
                filter
                    .split(',')
                    .map(|name| name.trim().to_owned())
                    .collect()
            };
            if requested.is_empty() || requested.iter().any(String::is_empty) {
                bail!("--server requires nonempty server names");
            }
            let mut names = Vec::new();
            for name in requested {
                if !names.contains(&name) {
                    names.push(name);
                }
            }
            Ok(names)
        })
        .transpose()
}

/// Internal selectors preserve exact manifest identities, including punctuation.
pub(super) fn exact_server_filter(names: &[String]) -> Result<String> {
    Ok(serde_json::to_string(names)?)
}

fn validate_server_names(names: &[String], available: &[&str]) -> Result<()> {
    for name in names {
        if !available.contains(&name.as_str()) {
            bail!("requested MCP server `{name}` is missing or unknown in the selected package");
        }
    }
    Ok(())
}

pub(super) fn selected_servers(
    root: &Path,
    package: &str,
    only_server: Option<&str>,
) -> Result<Vec<DeclaredMcpServer>> {
    let names = server_filter(only_server)?;
    let coordinate = normalized_coordinate(package)?;
    let all =
        vibe_workspace::bins::collect_mcp_servers(root).map_err(|e| anyhow::anyhow!("{e}"))?;
    let found: Vec<_> = all
        .into_iter()
        .filter(|s| s.package == coordinate)
        .collect();
    if found.is_empty() {
        bail!("no installed MCP server matches `{package}`");
    }
    for server in &found {
        ensure_requested_version(package, &server.version)?;
    }
    if let Some(names) = &names {
        validate_server_names(
            names,
            &found
                .iter()
                .map(|s| s.decl.name.as_str())
                .collect::<Vec<_>>(),
        )?;
    }
    Ok(found
        .into_iter()
        .filter(|s| {
            names
                .as_ref()
                .is_none_or(|names| names.contains(&s.decl.name))
        })
        .collect())
}

pub(super) fn package_version(root: &Path, package: &str) -> Result<String> {
    Ok(selected_servers(root, package, None)?[0].version.clone())
}

fn normalized_coordinate(package: &str) -> Result<String> {
    if package.contains(':') && !package.starts_with("mcp:") {
        bail!("MCP package selector must use the `mcp:` kind or no kind prefix: `{package}`");
    }
    let spelling = package.strip_prefix("mcp:").unwrap_or(package);
    let reference = vibe_core::PackageRef::parse(spelling)
        .with_context(|| format!("parsing MCP package selector `{package}`"))?;
    let group = reference
        .group
        .ok_or_else(|| anyhow::anyhow!("MCP package selector must include `<group>/`"))?;
    Ok(format!("{group}/{}", reference.name))
}

fn selector_exact_version(package: &str) -> Result<Option<&str>> {
    let Some((_, request)) = package.rsplit_once('@') else {
        return Ok(None);
    };
    let Some(exact) = request.strip_prefix('=') else {
        bail!("versioned MCP selectors require an exact `@=X.Y.Z` version: `{package}`");
    };
    if exact.is_empty() {
        bail!("empty MCP version selector in `{package}`");
    }
    Ok(Some(exact))
}

fn ensure_requested_version(package: &str, installed: &str) -> Result<()> {
    if let Some(exact) = selector_exact_version(package)?
        && installed != exact
    {
        bail!(
            "MCP selector `{package}` requests version `{exact}`, but the installed package is `{installed}`"
        );
    }
    Ok(())
}

fn empty_receipt() -> Receipt {
    Receipt {
        entries: BTreeMap::new(),
        schema: None,
    }
}

mod transaction;
use transaction::{Mutation, PublicationBatch};
mod activation;
pub(super) use activation::{
    BuiltinOwnershipError, UNRECEIPTED_HINT, admit_activation, builtin_server, builtin_unreceipted,
    register_builtin, register_builtin_explicit, remove_builtin, retain_known_intent,
    set_activation,
};
mod lifecycle;
mod register;
mod remove;
#[cfg(test)]
mod tests;

pub(crate) use lifecycle::GlobalAgentRegistration;
#[cfg(test)]
use lifecycle::preflight_owned_entries;
pub(super) use lifecycle::{
    GlobalRemovalPlan, apply_global_removal, plan_global_removal, preflight_user_refresh,
    refresh_user_package, remove_from_configs, restore_global_removal,
    user_agent_configs_for_package, user_registration_statuses,
};
use register::register_one;
pub(super) use register::{RegistrationPolicy, register, register_with_dirs};
pub(super) use remove::remove_all_managed;
use remove::remove_selected;
