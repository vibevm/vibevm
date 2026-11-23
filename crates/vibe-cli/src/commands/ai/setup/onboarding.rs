//! Connection onboarding inside attended `vibe ai setup`: a usable default
//! connection exists before agents are configured. The key is read only
//! through the protected hidden prompt and goes straight into the keyring.
use crate::output;
use crate::output::language::{Block, Glyph, Meta};
use anyhow::{Result, anyhow, bail};
use std::path::Path;
use vibe_core::cancellation::CancellationToken;
use vibe_secrets::SecretId;

/// Whether attended setup continues after onboarding.
pub(super) enum Next {
    Continue,
    Stop,
}

const SKIPPED: &str = "ai setup skipped; connections and bindings unchanged";

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
    ctx.hint("no default AI connection exists");
    if !output::prompts::confirm("Create a default AI connection now?", false)? {
        ctx.hint(SKIPPED);
        return Ok(Next::Stop);
    }
    let Some((id, metadata)) = create(ctx, settings, store, cancel)? else {
        ctx.hint(SKIPPED);
        return Ok(Next::Stop);
    };
    store.set_default(&id, store.list()?.store_revision)?;
    ctx.block(
        &Block::new(
            Glyph::Done,
            format!("created default connection {}", metadata.label),
        )
        .with_meta(vec![
            Meta::note(id.as_str()),
            Meta::note(metadata.endpoint.clone()),
        ])
        .render(),
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
    ctx.hint(&format!(
        "default connection {} · {} · {}",
        metadata.label,
        current.as_str(),
        metadata.endpoint
    ));
    if output::prompts::confirm("Reuse the default connection?", true)? {
        return Ok(Next::Continue);
    }
    if !output::prompts::confirm(
        "Replace the default connection? Agents that inherit the default will use the new connection",
        false,
    )? {
        ctx.hint("default connection unchanged");
        return Ok(Next::Continue);
    }
    let Some((id, _)) = create(ctx, settings, store, cancel)? else {
        ctx.hint(SKIPPED);
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
            ctx.hint(&format!(
                "default unchanged; new connection {} kept as a non-default connection",
                id.as_str()
            ));
            return Ok(Next::Continue);
        }
    };
    super::default_scope(ctx, store, &id, scope, None, cancel)?;
    Ok(Next::Continue)
}

/// Ask for a new OpenAI-compatible connection and store it; `None` when the
/// API-kind menu is escaped. Metadata is validated before the key is read.
fn create(
    ctx: &output::Context,
    settings: &Path,
    store: &vibe_connections::Store,
    cancel: &CancellationToken,
) -> Result<Option<(SecretId, vibe_connections::Metadata)>> {
    let hints = vibe_core::site_profile::installed()?
        .map(|site| site.profile.ai)
        .unwrap_or_default();
    let label = output::prompts::text("Label", "Default", false)?;
    let endpoint = output::prompts::text(
        "Endpoint",
        hints.default_endpoint.as_deref().unwrap_or(""),
        false,
    )?;
    let kinds = ["openai-chat-completions", "openai-responses"];
    let Some(kind) =
        output::prompts::choose_opt_interruptible("API kind", &output::prompts::named(&kinds), 0)?
    else {
        return Ok(None);
    };
    let default_model = output::prompts::text("Model", "", false)?;
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
    if let Some(hint) = &hints.key_hint_label {
        match &hints.key_hint_url {
            Some(url) => ctx.hint(&format!("get an API key: {hint} · {url}")),
            None => ctx.hint(&format!("get an API key: {hint}")),
        }
    }
    let key = output::prompts::secret("API key (hidden)")
        .map_err(|_| anyhow!("protected input cancelled or unavailable; nothing was written"))?;
    let value = vibe_secrets::ProtectedValue::new(key.into_bytes())?;
    let secret =
        vibe_secrets::Store::at_settings(settings)?.create("api-key", &metadata.label, value)?;
    let id = store.add(metadata.clone(), &secret)?;
    Ok(Some((id, metadata)))
}
