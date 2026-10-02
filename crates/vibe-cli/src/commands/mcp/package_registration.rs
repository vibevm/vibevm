//! Registration of a selected package server in an agent's native config.
//! The adjacent receipt records the exact entry last written by vibe. A
//! changed or unreceipted entry is operator-owned and is never overwritten.

use super::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::atomic::{AtomicU64, Ordering};
use vibe_workspace::bins::DeclaredMcpServer;

#[derive(Debug, Clone, Serialize)]
pub(super) struct RegistrationStatus {
    pub(super) agent: &'static str,
    pub(super) scope: &'static str,
    pub(super) config_path: String,
    /// `registered`, `pending`, `missing`, `changed`, `foreign`, or `absent`.
    pub(super) state: &'static str,
}

pub(super) fn registration_status(
    agent: Agent,
    scope: Scope,
    config: &Path,
    server: &DeclaredMcpServer,
) -> Result<RegistrationStatus> {
    let current = current_entry(agent, config, &server.decl.name)?;
    let receipt = read_receipt(config)?;
    let state = match (receipt.entries.get(&server.decl.name), current.as_ref()) {
        (Some(owned), Some(value))
            if owned.package == server.package && value == &owned.payload =>
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
    })
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct Receipt {
    #[serde(default)]
    entries: BTreeMap<String, OwnedEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OwnedEntry {
    package: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    version: Option<String>,
    payload: serde_json::Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pending_previous: Option<serde_json::Value>,
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
        return Ok(Receipt::default());
    }
    let text =
        fs::read_to_string(&path).with_context(|| format!("reading `{}`", path.display()))?;
    serde_json::from_str(&text).with_context(|| format!("parsing `{}`", path.display()))
}

fn write_receipt(config: &Path, receipt: &Receipt) -> Result<()> {
    let path = receipt_path(config)?;
    let text = serde_json::to_string_pretty(receipt)? + "\n";
    atomic_write(&path, text.as_bytes())
}

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

/// A complete config or receipt is made visible by a same-directory rename.
/// This prevents a killed process from leaving a truncated user config.
fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    let original_permissions = if checked_file_exists(path)? {
        Some(fs::metadata(path)?.permissions())
    } else {
        None
    };
    let parent = path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("path has no parent: `{}`", path.display()))?;
    fs::create_dir_all(parent)?;
    let name = path
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("path has no file name"))?
        .to_string_lossy();
    let temporary = parent.join(format!(
        ".{name}.vibevm-{}.{}.tmp",
        std::process::id(),
        TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let result = (|| -> Result<()> {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        // A new receipt/config must not inherit a permissive umask on Unix.
        #[cfg(unix)]
        if original_permissions.is_none() {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&temporary, fs::Permissions::from_mode(0o600))?;
        }
        if let Some(permissions) = original_permissions {
            fs::set_permissions(&temporary, permissions)?;
        }
        file.write_all(bytes)?;
        file.sync_all()?;
        fs::rename(&temporary, path)
            .with_context(|| format!("replacing `{}` atomically", path.display()))?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

#[derive(Clone)]
struct Snapshot {
    path: PathBuf,
    before: Option<Vec<u8>>,
}

fn snapshot(path: PathBuf) -> Result<Snapshot> {
    let before = if checked_file_exists(&path)? {
        Some(fs::read(&path)?)
    } else {
        None
    };
    Ok(Snapshot { path, before })
}

fn restore(snapshot: &Snapshot) -> Result<()> {
    match &snapshot.before {
        Some(bytes) => atomic_write(&snapshot.path, bytes),
        None if checked_file_exists(&snapshot.path)? => {
            fs::remove_file(&snapshot.path)?;
            Ok(())
        }
        None => Ok(()),
    }
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

fn server_payload(
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

fn selected_servers(
    root: &Path,
    package: &str,
    only_server: Option<&str>,
) -> Result<Vec<DeclaredMcpServer>> {
    let coordinate = normalized_coordinate(package)?;
    let all =
        vibe_workspace::bins::collect_mcp_servers(root).map_err(|e| anyhow::anyhow!("{e}"))?;
    let found: Vec<_> = all
        .into_iter()
        .filter(|s| s.package == coordinate && only_server.is_none_or(|n| s.decl.name == n))
        .collect();
    for server in &found {
        ensure_requested_version(package, &server.version)?;
    }
    if found.is_empty() {
        bail!(
            "no installed MCP server matches `{package}`{}",
            only_server
                .map(|s| format!(" --server {s}"))
                .unwrap_or_default()
        );
    }
    Ok(found)
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

mod lifecycle;
mod register;
mod remove;
#[cfg(test)]
mod tests;

#[cfg(test)]
use lifecycle::preflight_owned_entries;
pub(super) use lifecycle::{
    GlobalRemovalPlan, apply_global_removal, plan_global_removal, preflight_user_refresh,
    refresh_user_package, remove_from_configs, restore_global_removal,
    user_agent_configs_for_package,
};
pub(super) use register::register;
use register::register_one;
pub(super) use remove::remove_all_managed;
use remove::remove_selected;
