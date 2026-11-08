//! Durable activation stays in the existing ownership receipt.
use super::*;
use vibe_agent_projection::activation::{ActivationIntent, BuiltinAdoption};

pub(super) const BUILTIN_PACKAGE: &str = "org.vibevm.core/vibevm";

/// Ownership refusals for the builtin `vibevm` MCP entry. Typed so read-only
/// and refresh commands (`status`, `upgrade`) can report a per-entry
/// conflict instead of aborting the whole command.
#[derive(Debug, thiserror::Error)]
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-073#MANAGED-ACTIVATION")]
pub(in crate::commands::mcp) enum BuiltinOwnershipError {
    #[error(
        "MCP name `vibevm` in `{config}` has no ownership receipt and differs from the canonical entry; refusing to overwrite \
         (violates spec://org.vibevm.core/vibevm/common/PROP-073#MANAGED-ACTIVATION; \
          fix: it was written by an older vibe or another tool — run `vibe mcp install --force` to adopt and replace it, \
          or remove the entry by hand)"
    )]
    UnreceiptedDiffers { config: String },
}

/// The note every legacy-ownership conflict row carries.
pub(in crate::commands::mcp) const UNRECEIPTED_HINT: &str =
    "unreceipted entry written by an older vibe; run `vibe mcp install --force` to adopt it";

pub(in crate::commands::mcp) fn builtin_server() -> DeclaredMcpServer {
    DeclaredMcpServer {
        package: BUILTIN_PACKAGE.into(),
        group: "org.vibevm.core".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        binary: None,
        decl: vibe_core::manifest::McpServerDecl {
            name: SERVER_NAME.into(),
            binary: None,
            url: None,
            transport: None,
            description: None,
            args: Vec::new(),
        },
    }
}

pub(in crate::commands::mcp) fn register_builtin(
    agent: Agent,
    scope: Scope,
    config: &Path,
    payload: &ConfigPayload,
    dry_run: bool,
) -> Result<AgentInstallReport> {
    register::register_one(agent, scope, config, &builtin_server(), payload, dry_run)
}

pub(super) fn render_projection(
    agent: Agent,
    config: &Path,
    name: &str,
    desired: Option<&serde_json::Value>,
) -> Result<String> {
    match agent.config_format() {
        ConfigFormat::Json => {
            let mut doc = if let Some(value) = desired {
                merge_json(config, agent.mcp_section_key(), name, value)?
            } else {
                strip_json_entry(config, agent.mcp_section_key(), name)?
            };
            if desired.is_some() {
                vibe_mcp::pkg_servers::mark_managed(&mut doc, name)?;
            } else {
                vibe_mcp::pkg_servers::unmark_managed(&mut doc, name);
            }
            Ok(serde_json::to_string_pretty(&doc)? + "\n")
        }
        ConfigFormat::Toml => {
            let doc = if let Some(value) = desired {
                let value: toml::Value = serde_json::from_value(value.clone())?;
                merge_toml(config, agent.mcp_section_key(), name, &value)?
            } else {
                strip_toml_entry(config, agent.mcp_section_key(), name)?
            };
            Ok(toml::to_string_pretty(&doc)?)
        }
    }
}

#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-073#MANAGED-ACTIVATION")]
pub(in crate::commands::mcp) fn set_activation(
    agent: Agent,
    scope: Scope,
    config: &Path,
    server: &DeclaredMcpServer,
    payload: &ConfigPayload,
    request: &ActivationIntent,
    dry_run: bool,
) -> Result<RegistrationStatus> {
    register::register_requested(
        agent,
        scope,
        config,
        server,
        payload,
        Some(request),
        dry_run,
    )?;
    // Read actual state; a preview never labels proposed intent as committed.
    registration_status(agent, scope, config, server)
}

pub(in crate::commands::mcp) fn remove_builtin(
    agent: Agent,
    scope: Scope,
    config: &Path,
    dry_run: bool,
) -> Result<AgentInstallReport> {
    let receipt = read_receipt(config)?;
    if receipt.entries.contains_key(SERVER_NAME) {
        return remove_selected(
            agent,
            scope,
            config,
            BUILTIN_PACKAGE,
            Some(SERVER_NAME),
            dry_run,
        )?
        .into_iter()
        .next()
        .ok_or_else(|| anyhow::anyhow!("builtin receipt missing after selection"));
    }
    let current = current_entry(agent, config, SERVER_NAME)?;
    if current.is_none() {
        return Ok(AgentInstallReport {
            agent: agent.as_str().into(),
            scope: scope.as_str(),
            config_path: machine_json_path(config),
            status: "not-installed",
            note: None,
        });
    }
    // Uninstall is explicit: byte-canonical legacy bytes are adopted (and
    // then removed); a differing unreceipted entry stays a typed refusal.
    let payload = agent.build_mcp_entry();
    let adoption = BuiltinAdoption::explicit("vibe mcp uninstall");
    register_builtin_explicit(agent, scope, config, &payload, adoption, true)?;
    if !dry_run {
        register_builtin_explicit(agent, scope, config, &payload, adoption, false)?;
        return remove_builtin(agent, scope, config, false);
    }
    Ok(AgentInstallReport {
        agent: agent.as_str().into(),
        scope: scope.as_str(),
        config_path: machine_json_path(config),
        status: "would-remove",
        note: None,
    })
}

/// Whether the builtin `vibevm` entry is present but carries no ownership
/// receipt row — the shape every project set up by a pre-receipt vibe has.
pub(in crate::commands::mcp) fn builtin_unreceipted(agent: Agent, config: &Path) -> Result<bool> {
    Ok(!read_receipt(config)?.entries.contains_key(SERVER_NAME)
        && current_entry(agent, config, SERVER_NAME)?.is_some())
}

/// Only an explicit command may adopt an unreceipted builtin entry.
///
/// Identical canonical bytes are adopted under any explicit command; a stale
/// entry an older vibe wrote is replaced only under [`BuiltinAdoption::force`]
/// (`vibe mcp install --force`), and only for the builtin `vibevm` key. The
/// replaced entry is journaled as the receipt's `pending_previous` across the
/// publish — the same recovery protocol every receipt-owned rewrite uses —
/// and reported in the row's note.
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-073#MANAGED-ACTIVATION")]
pub(in crate::commands::mcp) fn register_builtin_explicit(
    agent: Agent,
    scope: Scope,
    config: &Path,
    payload: &ConfigPayload,
    adoption: BuiltinAdoption,
    dry_run: bool,
) -> Result<AgentInstallReport> {
    let mut batch = PublicationBatch::default();
    batch.capture_config(config)?;
    let receipt = read_receipt(config)?;
    let canonical = payload_json(payload)?;
    let current = current_entry(agent, config, SERVER_NAME)?;
    let Some(current) = current.filter(|_| !receipt.entries.contains_key(SERVER_NAME)) else {
        return register_builtin(agent, scope, config, payload, dry_run);
    };
    let stale = current != canonical;
    if stale && !adoption.force {
        return Err(BuiltinOwnershipError::UnreceiptedDiffers {
            config: machine_json_path(config),
        }
        .into());
    }
    if !dry_run {
        let result = adopt_builtin(
            agent,
            config,
            receipt,
            canonical,
            stale.then_some(&current),
            adoption,
            &mut batch,
        );
        if let Err(error) = result {
            return Err(batch.failed(error, "explicit builtin registration"));
        }
    }
    let note = if stale {
        format!(
            "new ownership by forced registration; replaced unreceipted entry written by an older vibe: {}",
            serde_json::to_string(&current)?
        )
    } else {
        "new ownership by explicit registration; prior author unknown".into()
    };
    Ok(AgentInstallReport {
        agent: agent.as_str().into(),
        scope: scope.as_str(),
        config_path: machine_json_path(config),
        status: if dry_run { "would-adopt" } else { "adopted" },
        note: Some(note),
    })
}

/// Publish the adoption receipt, and for a stale entry the canonical bytes,
/// journaling the replaced entry as `pending_previous` until the config
/// publish is confirmed.
fn adopt_builtin(
    agent: Agent,
    config: &Path,
    mut receipt: Receipt,
    canonical: serde_json::Value,
    replaced: Option<&serde_json::Value>,
    adoption: BuiltinAdoption,
    batch: &mut PublicationBatch,
) -> Result<()> {
    let server = builtin_server();
    let rendered = replaced
        .map(|_| render_projection(agent, config, SERVER_NAME, Some(&canonical)))
        .transpose()?;
    receipt.entries.insert(
        SERVER_NAME.into(),
        OwnedEntry {
            package: server.package,
            version: Some(server.version),
            payload: Some(canonical),
            retracted: false,
            pending_previous: replaced.cloned(),
            activation: Some(adoption.intent(replaced.is_some())),
        },
    );
    batch.capture_config(config)?;
    batch.receipt(config, &receipt)?;
    if let Some(rendered) = rendered {
        batch.write(config, rendered.as_bytes())?;
        if let Some(row) = receipt.entries.get_mut(SERVER_NAME) {
            row.pending_previous = None;
        }
        batch.receipt(config, &receipt)?;
    }
    batch.capture_config(config)
}

#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-073#MANAGED-ACTIVATION")]
pub(in crate::commands::mcp) fn admit_activation(
    config: &Path,
    server: &DeclaredMcpServer,
    intent: &ActivationIntent,
) -> Result<()> {
    admitted_receipt(config, server, intent).map(|_| ())
}
fn admitted_receipt(
    config: &Path,
    server: &DeclaredMcpServer,
    intent: &ActivationIntent,
) -> Result<Receipt> {
    vibe_agent_projection::activation::validate(intent)?;
    let receipt = read_receipt(config)?;
    let row = receipt
        .entries
        .get(&server.decl.name)
        .ok_or_else(|| anyhow::anyhow!("MCP entry is not managed; explicitly register it first"))?;
    if row.package != server.package {
        bail!("MCP activation owner mismatch");
    }
    if row.pending_previous.is_some() {
        bail!("MCP registration is pending; reconcile it before a new activation request");
    }
    Ok(receipt)
}
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-073#MANAGED-ACTIVATION")]
pub(in crate::commands::mcp) fn retain_known_intent(
    config: &Path,
    server: &DeclaredMcpServer,
    intent: &ActivationIntent,
) -> Result<()> {
    let path = receipt_path(config)?;
    let mut journal = PublicationBatch::default();
    journal.capture(&path)?;
    let mut receipt = admitted_receipt(config, server, intent)?;
    let row = receipt
        .entries
        .get_mut(&server.decl.name)
        .ok_or_else(|| anyhow::anyhow!("admitted MCP owner vanished"))?;
    if row.activation.as_ref() == Some(intent) {
        return Ok(());
    }
    row.activation = Some(intent.clone());
    journal
        .receipt(config, &receipt)
        .context("activation intent not confirmed retained; projection untouched")
}
