//! Local connection metadata only; never launches a provider or resolves a key.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-000#TS-KEYRING-CONSUMER");
use crate::{
    cli::{
        AiArgs, AiBindingCommand, AiBindingMode, AiCommand, ConnectionCommand,
        ConnectionDefaultCommand,
    },
    output,
};
use anyhow::{Result, bail};
use specmark::spec;
use vibe_core::cancellation::CancellationToken;
use vibe_secrets::{Expected, SecretId};
fn parse_id(raw: String) -> Result<SecretId> {
    Ok(SecretId::try_from(raw)?)
}
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-000#TS-KEYRING-CONSUMER")]
pub fn run(ctx: &output::Context, args: AiArgs, cancel: &CancellationToken) -> Result<()> {
    let settings = super::secrets::admitted_settings()?;
    let store = vibe_connections::Store::at_settings(&settings)?;
    let command = match args.command {
        AiCommand::Setup {
            agents,
            from_agent_alias,
            native_settings,
            mode,
            connection,
            set_default,
            scope,
            apply_preview,
        } => {
            if native_settings {
                return setup::native_settings(
                    ctx,
                    &store,
                    agents,
                    connection,
                    apply_preview,
                    cancel,
                );
            }
            if let Some(target) = set_default {
                let scope = match scope
                    .ok_or_else(|| anyhow::anyhow!("explicit default scope required"))?
                {
                    crate::cli::AiDefaultScope::DefaultOnly => {
                        vibe_connections::DefaultScope::DefaultOnly
                    }
                    crate::cli::AiDefaultScope::ManagedOverrides => {
                        vibe_connections::DefaultScope::ManagedOverrides
                    }
                };
                return setup::default_scope(
                    ctx,
                    &store,
                    &parse_id(target)?,
                    scope,
                    apply_preview,
                    cancel,
                );
            }
            return setup::run(
                ctx,
                &settings,
                &store,
                setup::Request {
                    agents,
                    from_agent_alias,
                    mode,
                    connection,
                    response: apply_preview,
                },
                cancel,
            );
        }
        AiCommand::Connections { command } => command,
        AiCommand::Bindings { command } => {
            let (name, action) = match command {
                AiBindingCommand::Show { harness } => (harness, None),
                AiBindingCommand::Set {
                    harness,
                    mode,
                    connection,
                    expect_store,
                } => (harness, Some((mode, connection, expect_store))),
            };
            // Existing user alias resolution owns identity/executable separation;
            // no launcher is executed or credential config inspected.
            let settings_root = vibe_app::SettingsRoot::new(&settings);
            let config = vibe_app::AgentsConfig::load(&settings_root)?;
            let launcher = super::agent::launchers::resolve(&settings_root, &config, &name)?;
            let harness = launcher.kind.as_str();
            cancel.checkpoint()?;
            let result = if let Some((mode, connection, expected)) = action {
                let binding = match (mode, connection) {
                    (AiBindingMode::Native, None) => None,
                    (AiBindingMode::InheritDefault, None) => {
                        Some(vibe_secrets::ReferenceBinding::InheritDefault)
                    }
                    (AiBindingMode::Connection, Some(id)) => {
                        Some(vibe_secrets::ReferenceBinding::Connection {
                            connection: parse_id(id)?,
                        })
                    }
                    _ => bail!("binding mode requires exactly its applicable connection argument"),
                };
                store.set_binding(harness, binding, expected)?
            } else {
                store.binding(harness)?
            };
            let report =
                serde_json::json!({"protocol":"vibe-ai-binding/1","ok":true,"binding":result});
            if ctx.is_json() {
                ctx.emit_json(&report)?;
            } else {
                println!("{}", serde_json::to_string_pretty(&report)?);
            }
            return Ok(());
        }
    };
    cancel.checkpoint()?;
    let selected = match command {
        ConnectionCommand::Default { command } => {
            let selection = match command {
                ConnectionDefaultCommand::Show => store.default_selection()?,
                ConnectionDefaultCommand::Set { id, expect_store } => {
                    store.set_default(&parse_id(id)?, expect_store)?
                }
                ConnectionDefaultCommand::Clear { expect_store, yes } => {
                    if !yes {
                        bail!("clearing default connection requires explicit --yes");
                    }
                    store.clear_default(expect_store)?
                }
            };
            let report = serde_json::json!({"protocol":"vibe-ai-default/1","ok":true,"store_revision":selection.store_revision,"connection":selection.connection});
            if ctx.is_json() {
                ctx.emit_json(&report)?;
            } else {
                println!("{}", serde_json::to_string_pretty(&report)?);
            }
            return Ok(());
        }
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

mod setup;

pub(crate) use setup::NativeSettingsSetFailure;
