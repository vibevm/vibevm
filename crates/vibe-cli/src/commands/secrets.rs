//! Local operator keyring surface; no raw-value output or consumer discovery.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-000#TS-USER-KEYRING");
use crate::{
    cli::{SecretsArgs, SecretsCommand},
    output,
};
use anyhow::{Result, bail};
use std::io::{IsTerminal, Read};
use vibe_core::cancellation::CancellationToken;
use vibe_secrets::{Expected, MetadataSnapshot, ProtectedValue, SecretId, Store};

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
            .map_err(|_| anyhow::anyhow!("protected input file unavailable, unsafe or oversized"))?
    } else if stdin {
        if std::io::stdin().is_terminal() {
            bail!("--value-stdin requires non-terminal input");
        }
        let mut bytes = Vec::new();
        std::io::stdin()
            .lock()
            .take(1024 * 1024 + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| anyhow::anyhow!("protected input could not be read"))?;
        bytes
    } else {
        attended(ctx)?;
        output::prompts::secret("New secret value (hidden)")
            .map_err(|_| anyhow::anyhow!("protected input cancelled or unavailable"))?
            .into_bytes()
    };
    cancel.checkpoint()?;
    if bytes.is_empty() {
        bail!("protected input must not be empty");
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
            "interactive secrets require an attended terminal; use an explicit command and --value-stdin"
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
    if ctx.is_json() {
        ctx.emit_json(&report)?;
    } else {
        // Only metadata enters rendering; value bytes never reach this function.
        println!("{}", serde_json::to_string_pretty(&report)?);
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
        SecretsCommand::Create {
            kind,
            label,
            value_stdin,
            value_file,
        } => {
            let value = input(ctx, value_stdin, value_file.as_deref(), cancel)?;
            cancel.checkpoint()?;
            let created = store.create(&kind, &label, value)?;
            render(ctx, store.snapshot()?, Some(&created))
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
            render(ctx, store.edit_label(&id, &expected, &label)?, Some(&id))
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
            store.rotate(&id, &revision, value)?;
            render(ctx, store.snapshot()?, Some(&id))
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
                    "logical removal requires explicit --yes consent; retained bytes are not erased"
                );
            }
            cancel.checkpoint()?;
            // Only durable fresh-genesis protocol admission can attest inventory.
            render(ctx, store.remove_coordinated(&id, &expected)?, None)
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
        choices.extend(snapshot.items.iter().map(|(id, item)| {
            vibe_app::Choice::named(&format!("{} ({})", item.label, id.as_str()), id.as_str())
        }));
        let Some(picked) = output::prompts::choose_opt(
            "Local keyring — select item; Escape returns",
            &choices,
            0,
        )?
        else {
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
        let actions =
            output::prompts::named(&["Details", "Edit label", "Rotate value", "Remove item"]);
        let Some(action) =
            output::prompts::choose_opt("Item action — Escape returns", &actions, 0)?
        else {
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
                if output::prompts::confirm(
                    &format!(
                        "Remove {} ({})? Private bytes remain; unknown references refuse",
                        item.label,
                        selected.as_str()
                    ),
                    false,
                )? {
                    cancel.checkpoint()?;
                    store.remove_coordinated(selected, &expected)?;
                }
            }
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
                if !name
                    .to_str()
                    .is_some_and(|s| vibe_safefs::ensure_safe_component(s).is_ok()) =>
            {
                bail!("unsupported settings root")
            }
            _ => {}
        }
    }
    Ok(settings)
}
