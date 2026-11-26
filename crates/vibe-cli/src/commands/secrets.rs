//! Local operator keyring surface; no raw-value output or consumer discovery.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-000#TS-USER-KEYRING");
use crate::{
    cli::{SecretsArgs, SecretsCommand},
    output,
};
use anyhow::{Result, bail};
use specmark::spec;
use std::io::{IsTerminal, Read};
use vibe_core::cancellation::CancellationToken;
use vibe_secrets::{Expected, MetadataSnapshot, ProtectedValue, SecretId, Store};

mod diagnostics;
mod human;

#[cfg(test)]
mod tests;

#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-000#TS-USER-KEYRING")]
pub fn run(ctx: &output::Context, args: SecretsArgs, cancel: &CancellationToken) -> Result<()> {
    let settings = admitted_settings()?;
    let store = Store::at_settings(&settings)?;
    cancel.checkpoint()?;
    match args.command {
        None => interactive(ctx, &store, cancel),
        Some(command) => execute(ctx, &store, command, cancel),
    }
}
fn id(raw: String) -> Result<SecretId> {
    SecretId::try_from(raw).map_err(Into::into)
}
fn input(
    ctx: &output::Context,
    stdin: bool,
    file: Option<&std::path::Path>,
    cancel: &CancellationToken,
) -> Result<ProtectedValue> {
    if stdin && file.is_some() {
        bail!("protected input sources conflict");
    }
    let bytes = if let Some(file) = file {
        let read = || -> Result<Vec<u8>> {
            use std::path::Component;
            if file.components().any(|c| matches!(c, Component::ParentDir)) {
                bail!("ambiguous input path");
            }
            let absolute = std::path::absolute(file)?;
            let name = absolute
                .file_name()
                .and_then(|n| n.to_str())
                .ok_or_else(|| anyhow::anyhow!("invalid input name"))?;
            let parent = absolute
                .parent()
                .ok_or_else(|| anyhow::anyhow!("missing input parent"))?;
            let project = vibe_safefs::Project::open(parent)?;
            project
                .root_dir()?
                .read_private_metadata(name, 1024 * 1024)?
                .ok_or_else(|| anyhow::anyhow!("missing input file"))
        };
        read()
            .map_err(|_| anyhow::anyhow!("value_file_unsafe: the value file is missing, unsafe or larger than 1 MiB (violates spec://org.vibevm.core/vibevm/common/PROP-000#TS-USER-KEYRING; fix: use a 0600 file you own in a 0700 directory you own)"))?
    } else if stdin {
        if std::io::stdin().is_terminal() {
            bail!(
                "value_input_terminal: --value-stdin reads the value from a pipe, and stdin is a terminal (violates spec://org.vibevm.core/vibevm/common/PROP-000#TS-USER-KEYRING; fix: pipe the value in, or pass --value-file PATH)"
            );
        }
        let mut bytes = Vec::new();
        std::io::stdin()
            .lock()
            .take(1024 * 1024 + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| anyhow::anyhow!("value_input_unreadable: the value could not be read from stdin (violates spec://org.vibevm.core/vibevm/common/PROP-000#TS-USER-KEYRING; fix: pipe the value in again)"))?;
        bytes
    } else {
        attended(ctx)?;
        output::prompts::secret("New secret value (hidden)")
            .map_err(|_| anyhow::anyhow!("protected input cancelled or unavailable"))?
            .into_bytes()
    };
    cancel.checkpoint()?;
    if bytes.is_empty() {
        bail!(
            "value_empty: the value is empty (violates spec://org.vibevm.core/vibevm/common/PROP-000#TS-USER-KEYRING; fix: pass a nonempty value)"
        );
    }
    ProtectedValue::new(bytes).map_err(Into::into)
}
fn attended(ctx: &output::Context) -> Result<()> {
    if ctx.is_json()
        || ctx.is_unattended()
        || !std::io::stdin().is_terminal()
        || !std::io::stderr().is_terminal()
    {
        bail!(
            "attended_terminal_required: this needs an attended terminal (violates spec://org.vibevm.core/vibevm/common/PROP-000#TS-USER-KEYRING; fix: `vibe secrets create --kind KIND --label LABEL --value-stdin`)"
        );
    }
    Ok(())
}
fn render(
    ctx: &output::Context,
    snapshot: MetadataSnapshot,
    selected: Option<&SecretId>,
) -> Result<()> {
    let rows: Vec<_> = snapshot.items.iter().filter(|(id,_)| selected.is_none_or(|s| s == *id)).map(|(id,item)| serde_json::json!({
        "id": id.as_str(), "current_revision": item.current_revision.as_str(), "kind": item.kind, "label": item.label, "value_present": true
    })).collect();
    if selected.is_some() && rows.is_empty() {
        return Err(vibe_secrets::Error::NotFound.into());
    }
    let report = serde_json::json!({"protocol":"vibe-secrets/1", "ok":true,"store_revision":snapshot.store_revision,"items":rows});
    // Only metadata enters rendering; value bytes never reach this function.
    let tuples: Vec<human::ItemRow> = snapshot
        .items
        .iter()
        .filter(|(id, _)| selected.is_none_or(|s| s == *id))
        .map(|(id, item)| {
            (
                id.as_str().to_owned(),
                item.label.clone(),
                item.kind.clone(),
                item.current_revision.as_str().to_owned(),
            )
        })
        .collect();
    if ctx.is_json() {
        ctx.emit_json(&report)?;
    } else if ctx.is_quiet() {
        ctx.summary(&human::count_line(tuples.len(), snapshot.store_revision));
    } else {
        ctx.block(&human::items(snapshot.store_revision, &tuples));
    }
    Ok(())
}
fn execute(
    ctx: &output::Context,
    store: &Store,
    command: SecretsCommand,
    cancel: &CancellationToken,
) -> Result<()> {
    match command {
        SecretsCommand::List => render(ctx, store.snapshot()?, None),
        SecretsCommand::Details { id: raw } => render(ctx, store.snapshot()?, Some(&id(raw)?)),
        SecretsCommand::References { id: raw } => {
            let id = id(raw)?;
            let consumers = store.references(&id)?;
            let report = serde_json::json!({"protocol":"vibe-secrets-references/1","ok":true,"id":id.as_str(),"consumers":consumers});
            if ctx.is_json() {
                ctx.emit_json(&report)?;
            } else if ctx.is_quiet() {
                ctx.summary(&human::consumers_line(id.as_str(), consumers.len()));
            } else {
                ctx.block(&human::references(id.as_str(), &consumers));
            }
            Ok(())
        }
        SecretsCommand::Create {
            kind,
            label,
            value_stdin,
            value_file,
        } => {
            let value = input(ctx, value_stdin, value_file.as_deref(), cancel)?;
            cancel.checkpoint()?;
            let created = store.create(&kind, &label, value)?;
            let snapshot = store.snapshot()?;
            if ctx.is_json() {
                return render(ctx, snapshot, Some(&created));
            }
            ctx.trailer(&format!(
                "created keyring item {label} · {} · store revision {}",
                created.as_str(),
                snapshot.store_revision
            ));
            Ok(())
        }
        SecretsCommand::Edit {
            id: raw,
            label,
            expect_store,
            expect_current,
        } => {
            let id = id(raw)?;
            let expected = Expected {
                store_revision: expect_store,
                current_revision: id_value(expect_current)?,
            };
            cancel.checkpoint()?;
            let snapshot = store
                .edit_label(&id, &expected, &label)
                .map_err(|error| diagnostics::expected(store, &id, &expected, error))?;
            if ctx.is_json() {
                return render(ctx, snapshot, Some(&id));
            }
            ctx.trailer(&format!(
                "renamed {} to {label} · store revision {}",
                id.as_str(),
                snapshot.store_revision
            ));
            Ok(())
        }
        SecretsCommand::Rotate {
            id: raw,
            expect_current,
            value_stdin,
            value_file,
        } => {
            let id = id(raw)?;
            let revision = id_value(expect_current)?;
            let value = input(ctx, value_stdin, value_file.as_deref(), cancel)?;
            cancel.checkpoint()?;
            let rotated = store.rotate(&id, &revision, value)?;
            let snapshot = store.snapshot()?;
            if ctx.is_json() {
                return render(ctx, snapshot, Some(&id));
            }
            ctx.trailer(&format!(
                "rotated the value of {} · revision {} · store revision {}",
                id.as_str(),
                rotated.as_str(),
                snapshot.store_revision
            ));
            Ok(())
        }
        SecretsCommand::Remove {
            id: raw,
            expect_store,
            expect_current,
            yes,
        } => {
            let id = id(raw)?;
            let expected = Expected {
                store_revision: expect_store,
                current_revision: id_value(expect_current)?,
            };
            if !yes {
                bail!(
                    "consent_required: removing keyring item {} needs --yes (violates spec://org.vibevm.core/vibevm/common/PROP-000#TS-USER-KEYRING; fix: \
                     `vibe secrets remove {} --expect-store {} --expect-current {} --yes`): removal is \
                     logical; the retained bytes are not erased",
                    id.as_str(),
                    id.as_str(),
                    expected.store_revision,
                    expected.current_revision.as_str()
                );
            }
            cancel.checkpoint()?;
            // Only durable fresh-genesis protocol admission can attest inventory.
            let snapshot = store
                .remove_coordinated(&id, &expected)
                .map_err(|error| diagnostics::expected(store, &id, &expected, error))?;
            if ctx.is_json() {
                return render(ctx, snapshot, None);
            }
            ctx.trailer(&format!(
                "removed keyring item {} · retained bytes not erased · store revision {}",
                id.as_str(),
                snapshot.store_revision
            ));
            Ok(())
        }
    }
}
fn id_value(raw: String) -> Result<SecretId> {
    id(raw)
}

fn interactive(ctx: &output::Context, store: &Store, cancel: &CancellationToken) -> Result<()> {
    attended(ctx)?;
    loop {
        cancel.checkpoint()?;
        let snapshot = store.snapshot()?;
        let mut choices = vec![vibe_app::Choice::named("Create item", "create")];
        choices.extend(snapshot.items.iter().map(|(id, item)| vibe_app::Choice {
            name: item.label.clone(),
            description: Some(format!("({})  {}", item.kind, id.as_str())),
            ..vibe_app::Choice::default()
        }));
        let Some(picked) = output::prompts::choose_opt("Keyring item", &choices, 0)? else {
            return Ok(());
        };
        if picked == 0 {
            let kind = output::prompts::text("Kind", "api-key", false)?;
            let label = output::prompts::text("Label", "", false)?;
            execute(
                ctx,
                store,
                SecretsCommand::Create {
                    kind,
                    label,
                    value_stdin: false,
                    value_file: None,
                },
                cancel,
            )?;
            continue;
        }
        let (selected, item) = snapshot
            .items
            .iter()
            .nth(picked - 1)
            .ok_or(vibe_secrets::Error::NotFound)?;
        let actions = output::prompts::named(&[
            "Details",
            "Edit label",
            "Rotate value",
            "Remove item",
            "Show references",
        ]);
        let Some(action) = output::prompts::choose_opt("Action", &actions, 0)? else {
            continue;
        };
        let expected = Expected {
            store_revision: snapshot.store_revision,
            current_revision: item.current_revision.clone(),
        };
        match action {
            0 => render(ctx, snapshot.clone(), Some(selected))?,
            1 => {
                let label = output::prompts::text("Label", &item.label, false)?;
                cancel.checkpoint()?;
                render(
                    ctx,
                    store.edit_label(selected, &expected, &label)?,
                    Some(selected),
                )?;
            }
            2 => {
                let value = input(ctx, false, None, cancel)?;
                cancel.checkpoint()?;
                store.rotate(selected, &expected.current_revision, value)?;
                render(ctx, store.snapshot()?, Some(selected))?;
            }
            3 => {
                let consumers = match store.references(selected) {
                    Ok(consumers) => consumers,
                    Err(_) => {
                        println!("Reference inventory unavailable or unknown; removal refused.");
                        continue;
                    }
                };
                if !consumers.is_empty() {
                    ctx.error_with_suffix(
                        &anyhow::anyhow!(
                            "Item is used; removal refused. Consumers: {}",
                            consumers.join(", ")
                        ),
                        None,
                    );
                    continue;
                }
                ctx.hint(&format!(
                    "{} · no registered consumers · private bytes remain",
                    selected.as_str()
                ));
                let choices = output::prompts::named(&["Keep item", "Remove item"]);
                let prompt = format!("Remove {}?", item.label);
                if output::prompts::choose_opt_interruptible(&prompt, &choices, 0)? == Some(1) {
                    cancel.checkpoint()?;
                    store.remove_coordinated(selected, &expected)?;
                }
            }
            4 => match store.references(selected) {
                Ok(consumers) => ctx.block(&human::references(selected.as_str(), &consumers)),
                Err(_) => println!("Reference inventory unavailable or unknown."),
            },
            _ => unreachable!(),
        }
    }
}

pub(crate) fn admitted_settings() -> Result<std::path::PathBuf> {
    let settings = vibe_core::user_project::selected_settings(vibe_core::settings::settings_dir())
        .map_err(|_| anyhow::anyhow!("an absolute effective user settings root is required"))?;
    for component in settings.components() {
        match component {
            std::path::Component::ParentDir | std::path::Component::CurDir => {
                bail!("ambiguous settings root is unavailable")
            }
            std::path::Component::Normal(name)
                if name
                    .to_str()
                    .is_none_or(|s| vibe_safefs::ensure_safe_component(s).is_err()) =>
            {
                bail!("unsupported settings root")
            }
            _ => {}
        }
    }
    Ok(settings)
}
