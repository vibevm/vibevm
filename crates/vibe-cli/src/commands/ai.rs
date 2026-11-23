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
            let change = action.as_ref().map(|(mode, connection, _)| match mode {
                AiBindingMode::Native => format!("restored native binding for {harness}"),
                AiBindingMode::InheritDefault => format!("bound {harness} to inherit-default"),
                AiBindingMode::Connection => format!(
                    "bound {harness} to connection {}",
                    connection.as_deref().unwrap_or_default()
                ),
            });
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
                    _ => bail!(
                        "binding_arguments_invalid: binding mode and connection argument do not match (violates spec://org.vibevm.core/vibevm/common/PROP-000#TS-KEYRING-CONSUMER; fix: pass --connection ID only with --mode connection; omit --connection for native and inherit-default)"
                    ),
                };
                store
                    .set_binding(harness, binding, expected)
                    .map_err(|error| diagnostics::binding(harness, error))?
            } else {
                store.binding(harness)?
            };
            let report =
                serde_json::json!({"protocol":"vibe-ai-binding/1","ok":true,"binding":result});
            if ctx.is_json() {
                ctx.emit_json(&report)?;
            } else if let Some(change) = change {
                ctx.trailer(&format!(
                    "{change} · store revision {}",
                    result.store_revision
                ));
            } else if ctx.is_quiet() {
                ctx.summary(&view::binding_quiet(&result));
            } else {
                ctx.block(&view::binding_block(&result));
            }
            return Ok(());
        }
    };
    cancel.checkpoint()?;
    let mut action = None;
    let selected = match command {
        ConnectionCommand::Default { command } => {
            let change = match &command {
                ConnectionDefaultCommand::Show => None,
                ConnectionDefaultCommand::Set { .. } => Some(true),
                ConnectionDefaultCommand::Clear { .. } => Some(false),
            };
            let selection = match command {
                ConnectionDefaultCommand::Show => store.default_selection()?,
                ConnectionDefaultCommand::Set { id, expect_store } => {
                    store.set_default(&parse_id(id)?, expect_store)?
                }
                ConnectionDefaultCommand::Clear { expect_store, yes } => {
                    if !yes {
                        bail!(
                            "consent_required: clearing the default connection needs --yes (violates spec://org.vibevm.core/vibevm/common/PROP-000#TS-KEYRING-CONSUMER; fix: `vibe ai connections default clear --expect-store {expect_store} --yes`)"
                        );
                    }
                    store.clear_default(expect_store)?
                }
            };
            let report = serde_json::json!({"protocol":"vibe-ai-default/1","ok":true,"store_revision":selection.store_revision,"connection":selection.connection});
            if ctx.is_json() {
                ctx.emit_json(&report)?;
            } else {
                let label = selection
                    .connection
                    .as_ref()
                    .and_then(|id| store.details(id).ok())
                    .map(|connection| connection.metadata.label);
                if let Some(set) = change {
                    let what = if set {
                        let id = selection
                            .connection
                            .as_ref()
                            .ok_or(vibe_secrets::Error::NotFound)?
                            .as_str();
                        match label {
                            Some(label) => format!("default connection set to {label} ({id})"),
                            None => format!("default connection set to {id}"),
                        }
                    } else {
                        "cleared default connection".into()
                    };
                    ctx.trailer(&format!(
                        "{what} · store revision {}",
                        selection.store_revision
                    ));
                } else if ctx.is_quiet() {
                    ctx.summary(&view::default_quiet(&selection, label.as_deref()));
                } else {
                    ctx.block(&view::default_block(&selection, label.as_deref()));
                }
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
                label: label.clone(),
                endpoint,
                api_kind,
                default_model,
            };
            cancel.checkpoint()?;
            let id = store.add(metadata, &secret)?;
            action = Some(format!("added connection {label} · {}", id.as_str()));
            Some(id)
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
                    "consent_required: removing connection {id} needs --yes (violates spec://org.vibevm.core/vibevm/common/PROP-000#TS-KEYRING-CONSUMER; fix: `vibe ai connections remove {id} --expect-store {expect_store} --expect-current {} --yes`): unbinding does not erase the referenced secret",
                    expected.current_revision.as_str(),
                    id = id.as_str()
                );
            }
            cancel.checkpoint()?;
            store
                .remove(&id, &expected)
                .map_err(|error| diagnostics::removal(&store, &id, &expected, error))?;
            action = Some(format!("removed connection {}", id.as_str()));
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
    } else if let Some(action) = action {
        ctx.trailer(&format!(
            "{action} · store revision {}",
            snapshot.store_revision
        ));
    } else if ctx.is_quiet() {
        ctx.summary(&view::connections_count(
            snapshot.items.len(),
            snapshot.store_revision,
        ));
    } else {
        ctx.block(&view::connections(&snapshot));
    }
    Ok(())
}

mod diagnostics;
mod setup;
mod view;

#[cfg(test)]
mod tests;

pub(crate) use setup::NativeSettingsSetFailure;
