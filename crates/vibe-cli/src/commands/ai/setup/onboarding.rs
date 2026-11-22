//! Connection onboarding inside attended `vibe ai setup`: a usable default
//! connection exists before agents are configured. The key is read only
//! through the protected hidden prompt and goes straight into the keyring.
use crate::output;
use anyhow::{Result, anyhow, bail};
use std::path::Path;
use vibe_core::cancellation::CancellationToken;
use vibe_secrets::SecretId;

/// Whether attended setup continues after onboarding.
pub(super) enum Next {
    Continue,
    Stop,
}

const SKIPPED: &str = "AI setup skipped; connections and bindings unchanged.";

/// With no default connection, offer (default No) to create one and select it.
pub(super) fn ensure_default(
    ctx: &output::Context,
    settings: &Path,
    store: &vibe_connections::Store,
    cancel: &CancellationToken,
) -> Result<Next> {
    if let Some(current) = store.default_selection()?.connection {
        return reuse_or_replace(ctx, settings, store, &current, cancel);
    }
    println!("No default AI connection exists.");
    if !output::prompts::confirm("Create a default AI connection now?", false)? {
        println!("{SKIPPED}");
        return Ok(Next::Stop);
    }
    let Some((id, metadata)) = create(settings, store, cancel)? else {
        println!("{SKIPPED}");
        return Ok(Next::Stop);
    };
    store.set_default(&id, store.list()?.store_revision)?;
    println!(
        "Default AI connection created: {} — {} — {}",
        metadata.label,
        id.as_str(),
        metadata.endpoint
    );
    Ok(Next::Continue)
}

/// Show the default (never its key) and keep it, or replace it with a new
/// connection under an explicit scope reviewed by the default-scope preview.
fn reuse_or_replace(
    ctx: &output::Context,
    settings: &Path,
    store: &vibe_connections::Store,
    current: &SecretId,
    cancel: &CancellationToken,
) -> Result<Next> {
    let metadata = store.details(current)?.metadata;
    println!(
        "Default AI connection: {} — {} — {}",
        metadata.label,
        current.as_str(),
        metadata.endpoint
    );
    if output::prompts::confirm("Reuse this default connection?", true)? {
        return Ok(Next::Continue);
    }
    if !output::prompts::confirm(
        "Replace the default connection? Agents that inherit the default will use the new connection",
        false,
    )? {
        println!("Default connection unchanged.");
        return Ok(Next::Continue);
    }
    let Some((id, _)) = create(settings, store, cancel)? else {
        println!("{SKIPPED}");
        return Ok(Next::Stop);
    };
    let scopes = [
        "Change only the default connection",
        "Replace all Vibe-managed agent overrides with this connection",
    ];
    let scope = match output::prompts::choose_opt_interruptible(
        "Default update scope",
        &output::prompts::named(&scopes),
        0,
    )? {
        Some(0) => vibe_connections::DefaultScope::DefaultOnly,
        Some(_) => vibe_connections::DefaultScope::ManagedOverrides,
        None => {
            println!(
                "Default unchanged; new connection {} kept as a non-default connection.",
                id.as_str()
            );
            return Ok(Next::Continue);
        }
    };
    super::default_scope(ctx, store, &id, scope, None, cancel)?;
    Ok(Next::Continue)
}

/// Ask for a new OpenAI-compatible connection and store it; `None` when the
/// API-kind menu is escaped. Metadata is validated before the key is read.
fn create(
    settings: &Path,
    store: &vibe_connections::Store,
    cancel: &CancellationToken,
) -> Result<Option<(SecretId, vibe_connections::Metadata)>> {
    let label = output::prompts::text("Connection label", "Default", false)?;
    let endpoint = output::prompts::text("Endpoint URL", "", false)?;
    let kinds = ["openai-chat-completions", "openai-responses"];
    let Some(kind) =
        output::prompts::choose_opt_interruptible("API kind", &output::prompts::named(&kinds), 0)?
    else {
        return Ok(None);
    };
    let default_model = output::prompts::text("Default model", "", false)?;
    let metadata = vibe_connections::Metadata {
        label,
        endpoint,
        api_kind: kinds[kind].to_owned(),
        default_model,
    };
    if metadata.validate().is_err() {
        bail!("connection metadata is invalid; nothing was written");
    }
    cancel.checkpoint()?;
    let key = output::prompts::secret("API key (hidden)")
        .map_err(|_| anyhow!("protected input cancelled or unavailable; nothing was written"))?;
    let value = vibe_secrets::ProtectedValue::new(key.into_bytes())?;
    let secret =
        vibe_secrets::Store::at_settings(settings)?.create("api-key", &metadata.label, value)?;
    let id = store.add(metadata.clone(), &secret)?;
    Ok(Some((id, metadata)))
}
