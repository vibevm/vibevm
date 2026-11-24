//! Shared-menu setup-only adapter; one metadata planner and atomic executor.
use crate::{cli::AiBindingMode, output};
use anyhow::{Result, bail};
use std::{collections::BTreeMap, io::IsTerminal, path::Path};
use vibe_core::cancellation::CancellationToken;
mod install;
mod native_settings;
mod onboarding;
mod render;
pub(crate) use native_settings::NativeSettingsSetFailure;
pub(super) use native_settings::run as native_settings;

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
    let bare = attended
        && agents.is_empty()
        && from_agent_alias.is_none()
        && mode.is_none()
        && connection.is_none()
        && response.is_none();
    if bare {
        install::offer(cancel)?;
        if matches!(
            onboarding::ensure_default(ctx, settings, store, cancel)?,
            onboarding::Next::Stop
        ) {
            return Ok(());
        }
        if crate::output::prompts::confirm(
            "Write a connection into selected agents' native settings? Otherwise continue metadata only",
            false,
        )? {
            return native_settings::attended(ctx, store, cancel);
        }
    }
    let alias_entry = from_agent_alias.is_some();
    if let Some(alias) = from_agent_alias.as_ref() {
        if !agents.is_empty() {
            bail!("alias setup conflicts with explicit agent selection");
        }
        agents.push(alias.clone());
    }
    let interactive = agents.is_empty() || (alias_entry && mode.is_none());
    if interactive && !attended {
        bail!("setup requires explicit --agent selection outside an attended terminal");
    }
    if interactive && !alias_entry {
        let choices = crate::output::prompts::named(&["pi", "qwen", "claude"]);
        let Some(selected) = crate::output::prompts::choose_many(
            "Configure existing connection metadata only",
            &choices,
            &[false, false, false],
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
    let config = vibe_app::AgentsConfig::load(&root).map_err(vibe_app::AppError::from)?;
    let mut aliases = BTreeMap::new();
    for name in agents {
        let launcher = super::super::agent::launchers::resolve(&root, &config, &name)?;
        if interactive && !alias_entry && launcher.kind.as_str() == "claude" {
            ctx.hint("claude skipped: its native settings cannot take an OpenAI-compatible connection; its native authentication is kept");
            continue;
        }
        if aliases
            .insert(name, launcher.kind.as_str().to_owned())
            .is_some()
        {
            bail!("duplicate setup agent selection");
        }
    }
    if aliases.is_empty() {
        ctx.hint("no supported agent selected; metadata unchanged");
        return Ok(());
    }
    cancel.checkpoint()?;
    let connection_id = match &mode {
        vibe_secrets::ReferenceBinding::Connection { connection } => {
            Some(connection.as_str().to_owned())
        }
        vibe_secrets::ReferenceBinding::InheritDefault => None,
    };
    let preview = store.setup_preview(&aliases, mode)?;
    let mut apply = response;
    if !ctx.is_json() {
        let unchanged = vibe_workspace::system_prompt::HarnessKind::ALL
            .into_iter()
            .filter(|kind| !aliases.values().any(|id| id == kind.as_str()))
            .map(|kind| store.binding(kind.as_str()))
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let mut rows = Vec::new();
        for (alias, harness) in &aliases {
            if let Some(row) = preview.rows.iter().find(|row| &row.harness == harness) {
                rows.push(render::PlanRow {
                    name: alias.clone(),
                    kind: Some(harness.clone()),
                    change: format!("{} → {}", row.old_mode, row.proposed_mode),
                    connection: Some(row.effective_connection.as_str().to_owned()),
                    state: None,
                });
            }
        }
        for observation in unchanged {
            let state = match observation.capability {
                vibe_connections::MetadataCapability::Compatible { .. } => "compatible",
                vibe_connections::MetadataCapability::Unsupported => "unsupported",
                vibe_connections::MetadataCapability::Unknown => "unknown",
            };
            rows.push(render::PlanRow {
                name: observation.harness,
                kind: None,
                change: format!("unchanged · {}", observation.mode),
                connection: None,
                state: Some(state.to_owned()),
            });
        }
        ctx.rule(&format!(
            "ai setup plan · store revision {}",
            preview.store_revision
        ));
        ctx.block(&render::plan_list(
            &rows,
            crate::output::language::viewport(),
        ));
    }
    if interactive {
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
    } else if applied {
        if revision == preview.store_revision {
            ctx.trailer(&format!("bindings unchanged · store revision {revision}"));
        } else {
            ctx.trailer(&render::bound_what(
                aliases.len(),
                connection_id.as_deref(),
                revision,
            ));
        }
    } else {
        let names: Vec<&str> = aliases.keys().map(String::as_str).collect();
        let command = render::apply_command(
            &names,
            from_agent_alias.as_deref(),
            connection_id.as_deref(),
            preview.preview_id(),
        );
        if ctx.is_quiet() {
            ctx.summary(&command);
        } else {
            ctx.block(&render::preview_trailer(
                &format!("previewed {}", render::bindings(aliases.len())),
                &command,
            ));
        }
    }
    Ok(())
}

pub(super) fn default_scope(
    ctx: &output::Context,
    store: &vibe_connections::Store,
    target: &vibe_secrets::SecretId,
    scope: vibe_connections::DefaultScope,
    response: Option<String>,
    cancel: &CancellationToken,
) -> Result<()> {
    cancel.checkpoint()?;
    let preview = store.default_scope_preview(target, scope)?;
    let attended = !ctx.is_json()
        && !ctx.is_unattended()
        && std::io::stdin().is_terminal()
        && std::io::stderr().is_terminal();
    let mut response = response;
    let scope_word = match scope {
        vibe_connections::DefaultScope::DefaultOnly => "default-only",
        vibe_connections::DefaultScope::ManagedOverrides => "managed-overrides",
    };
    if !ctx.is_json() {
        let mut rows = vec![render::PlanRow {
            name: "default".to_owned(),
            kind: None,
            change: format!(
                "{} → {}",
                preview
                    .old_default
                    .as_ref()
                    .map_or("none", |id| id.as_str()),
                preview.proposed_default.as_str()
            ),
            connection: None,
            state: None,
        }];
        rows.extend(preview.rows.iter().map(|row| {
            render::PlanRow {
                name: row.harness.clone(),
                kind: None,
                change: format!("{} → {}", row.old_mode, row.proposed_mode),
                connection: row
                    .proposed_connection
                    .as_ref()
                    .map(|id| id.as_str().to_owned()),
                state: None,
            }
        }));
        ctx.rule(&format!("default connection plan · {scope_word}"));
        ctx.block(&render::plan_list(
            &rows,
            crate::output::language::viewport(),
        ));
    }
    if attended && response.is_none() {
        let choices = crate::output::prompts::named(&[
            "Keep current default and bindings",
            "Apply exactly this default scope",
        ]);
        if crate::output::prompts::choose_opt_interruptible(
            "Review default and managed overrides",
            &choices,
            0,
        )? != Some(1)
        {
            return Ok(());
        }
        response = Some(preview.preview_id.clone());
    }
    cancel.checkpoint()?;
    let (applied, revision) = if let Some(response) = response {
        (true, store.apply_default_scope(&preview, &response)?)
    } else {
        (false, store.list()?.store_revision)
    };
    let report = serde_json::json!({"protocol":"vibe-ai-default-scope-result/1","ok":true,"applied":applied,"store_revision":revision,"preview":preview});
    if ctx.is_json() {
        ctx.emit_json(&report)?;
    } else if applied {
        ctx.trailer(&format!(
            "set the default connection to {} · store revision {revision}",
            target.as_str()
        ));
    } else {
        let command =
            render::default_apply_command(target.as_str(), scope_word, &preview.preview_id);
        if ctx.is_quiet() {
            ctx.summary(&command);
        } else {
            ctx.block(&render::preview_trailer(
                "previewed the default connection change",
                &command,
            ));
        }
    }
    Ok(())
}
