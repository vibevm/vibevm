//! Local connection metadata only; never launches a provider or resolves a key.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-000#TS-KEYRING-CONSUMER");
use crate::{
    cli::{AiArgs, AiCommand, ConnectionCommand},
    output,
};
use anyhow::{Result, bail};
use vibe_core::cancellation::CancellationToken;
use vibe_secrets::{Expected, SecretId};
fn parse_id(raw: String) -> Result<SecretId> {
    Ok(SecretId::try_from(raw)?)
}
pub fn run(ctx: &output::Context, args: AiArgs, cancel: &CancellationToken) -> Result<()> {
    let settings = super::secrets::admitted_settings()?;
    let store = vibe_connections::Store::at_settings(&settings)?;
    let AiCommand::Connections { command } = args.command;
    cancel.checkpoint()?;
    let selected = match command {
        ConnectionCommand::List => None,
        ConnectionCommand::Details { id: raw } => Some(parse_id(raw)?),
        ConnectionCommand::Add {
            label,
            endpoint,
            api_kind,
            default_model,
            secret_ref,
        } => {
            let secret = parse_id(secret_ref)?;
            let metadata = vibe_connections::Metadata {
                label,
                endpoint,
                api_kind,
                default_model,
            };
            cancel.checkpoint()?;
            Some(store.add(metadata, &secret)?)
        }
        ConnectionCommand::Remove {
            id: raw,
            expect_store,
            expect_current,
            yes,
        } => {
            let id = parse_id(raw)?;
            let expected = Expected {
                store_revision: expect_store,
                current_revision: parse_id(expect_current)?,
            };
            if !yes {
                bail!(
                    "connection unbinding requires explicit --yes; it never erases the referenced secret"
                );
            }
            cancel.checkpoint()?;
            store.remove(&id, &expected)?;
            None
        }
    };
    let mut snapshot = store.list()?;
    if let Some(id) = selected {
        let row = snapshot
            .items
            .remove(&id)
            .ok_or(vibe_secrets::Error::NotFound)?;
        snapshot.items.clear();
        snapshot.items.insert(id, row);
    }
    let report = serde_json::json!({"protocol":"vibe-ai-connections/1","ok":true,"store_revision":snapshot.store_revision,"items":snapshot.items});
    if ctx.is_json() {
        ctx.emit_json(&report)?;
    } else {
        println!("{}", serde_json::to_string_pretty(&report)?);
    }
    Ok(())
}
