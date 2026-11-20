//! Explicit persistent native setup, distinct from metadata binding setup.
use super::*;
use vibe_connections::NativeSettingsAdapter;

pub(crate) fn run(
    ctx: &output::Context,
    store: &vibe_connections::Store,
    agents: Vec<String>,
    connection: Option<String>,
    response: Option<String>,
    cancel: &CancellationToken,
) -> Result<()> {
    if agents != ["pi"] {
        bail!(
            "native settings currently require exactly --agent pi; other adapters are not yet available"
        );
    }
    // A native directory override changes the target. Never silently ignore it.
    if std::env::var_os("PI_CODING_AGENT_DIR").is_some() {
        bail!("explicit Pi agent-directory override is not yet admitted for native settings");
    }
    let home = super::super::super::app::facts::user_home()
        .ok_or_else(|| anyhow::anyhow!("native settings require an explicit effective HOME"))?;
    let id = vibe_secrets::SecretId::try_from(
        connection
            .ok_or_else(|| anyhow::anyhow!("native settings require an explicit --connection"))?,
    )?;
    cancel.checkpoint()?;
    let metadata = store.details(&id)?.metadata;
    let adapter = vibe_connections::PiSettingsAdapter;
    let preview = adapter.preview(store, &home, &id)?;
    let summary = serde_json::json!({"agent":"pi","connection":id,"endpoint":metadata.endpoint,"model":metadata.default_model,"destination":"HOME/.pi/agent/models.json","preview_id":preview.preview_id()});
    let attended = !ctx.is_json()
        && !ctx.is_unattended()
        && std::io::stdin().is_terminal()
        && std::io::stderr().is_terminal();
    let mut response = response;
    if attended && response.is_none() {
        println!("{}", serde_json::to_string_pretty(&summary)?);
        let choices = crate::output::prompts::named(&[
            "Keep native settings unchanged",
            "Write this connection into Pi native settings",
        ]);
        if crate::output::prompts::choose_opt_interruptible(
            "Explicit native settings write — Escape keeps state",
            &choices,
            0,
        )? != Some(1)
        {
            return Ok(());
        }
        response = Some(preview.preview_id().to_owned());
    }
    cancel.checkpoint()?;
    let applied = response.is_some();
    let changed = match response {
        Some(response) => adapter.apply(store, &preview, &response)?,
        None => false,
    };
    let report = serde_json::json!({"protocol":"vibe-ai-native-settings/1","ok":true,"applied":applied,"changed":changed,"preview":summary});
    if ctx.is_json() {
        ctx.emit_json(&report)?;
    } else {
        println!("{}", serde_json::to_string_pretty(&report)?);
    }
    Ok(())
}
