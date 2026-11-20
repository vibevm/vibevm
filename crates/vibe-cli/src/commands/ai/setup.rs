//! Shared-menu setup-only adapter; one metadata planner and atomic executor.
use crate::{cli::AiBindingMode, output};
use anyhow::{Result, bail};
use std::{collections::BTreeMap, io::IsTerminal, path::Path};
use vibe_core::cancellation::CancellationToken;

pub(super) struct Request {
    pub agents: Vec<String>,
    pub from_agent_alias: Option<String>,
    pub mode: Option<AiBindingMode>,
    pub connection: Option<String>,
    pub response: Option<String>,
}

pub(super) fn run(
    ctx: &output::Context,
    settings: &Path,
    store: &vibe_connections::Store,
    request: Request,
    cancel: &CancellationToken,
) -> Result<()> {
    let Request {
        mut agents,
        from_agent_alias,
        mode,
        connection,
        response,
    } = request;
    let attended = !ctx.is_json()
        && !ctx.is_unattended()
        && std::io::stdin().is_terminal()
        && std::io::stderr().is_terminal();
    let alias_entry = from_agent_alias.is_some();
    if let Some(alias) = from_agent_alias {
        if !agents.is_empty() {
            bail!("alias setup conflicts with explicit agent selection");
        }
        agents.push(alias);
    }
    let interactive = agents.is_empty() || (alias_entry && mode.is_none());
    if interactive && !attended {
        bail!("setup requires explicit --agent selection outside an attended terminal");
    }
    if interactive && !alias_entry {
        let choices = crate::output::prompts::named(&["pi", "qwen"]);
        let Some(selected) = crate::output::prompts::choose_many(
            "Configure existing connection metadata only",
            &choices,
            &[false, false],
        )?
        else {
            return Ok(());
        };
        if selected.is_empty() {
            return Ok(());
        }
        agents = selected
            .into_iter()
            .map(|i| choices[i].name.clone())
            .collect();
    }
    let mode = match (mode, connection) {
        (Some(AiBindingMode::Connection), Some(id)) => vibe_secrets::ReferenceBinding::Connection {
            connection: vibe_secrets::SecretId::try_from(id)?,
        },
        (Some(AiBindingMode::InheritDefault), None) => {
            vibe_secrets::ReferenceBinding::InheritDefault
        }
        (None, None) if interactive => vibe_secrets::ReferenceBinding::InheritDefault,
        _ => bail!(
            "setup requires inherit-default or connection mode with exactly its applicable connection argument"
        ),
    };
    let root = vibe_app::SettingsRoot::new(settings);
    let config = vibe_app::AgentsConfig::load(&root)?;
    let mut aliases = BTreeMap::new();
    for name in agents {
        let launcher = super::super::agent::launchers::resolve(&root, &config, &name)?;
        if aliases
            .insert(name, launcher.kind.as_str().to_owned())
            .is_some()
        {
            bail!("duplicate setup agent selection");
        }
    }
    cancel.checkpoint()?;
    let preview = store.setup_preview(&aliases, mode)?;
    let mut apply = response;
    if interactive {
        if alias_entry {
            println!(
                "Selected launcher alias and preset: {}",
                serde_json::to_string(&aliases)?
            );
        }
        println!(
            "{}",
            serde_json::to_string_pretty(
                &serde_json::json!({"protocol":"vibe-ai-setup/1","applied":false,"preview":preview})
            )?
        );
        if !crate::output::prompts::confirm(
            "Apply exactly this metadata selection? No agents will be installed or launched",
            false,
        )? {
            return Ok(());
        }
        apply = Some(preview.preview_id().to_owned());
    }
    cancel.checkpoint()?;
    let (applied, revision) = if let Some(response) = apply {
        (true, store.apply_setup(&preview, &response)?)
    } else {
        (false, preview.store_revision)
    };
    let report = serde_json::json!({"protocol":"vibe-ai-setup/1","ok":true,"applied":applied,"store_revision":revision,"preview":preview});
    if ctx.is_json() {
        ctx.emit_json(&report)?;
    } else {
        println!("{}", serde_json::to_string_pretty(&report)?);
    }
    Ok(())
}
