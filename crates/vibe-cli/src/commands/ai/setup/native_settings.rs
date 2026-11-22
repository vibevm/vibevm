//! Explicit persistent native setup, distinct from metadata binding setup.
use super::*;
use vibe_connections::NativeSettingsAdapter;
mod set;
pub(crate) use set::NativeSettingsSetFailure;
pub(super) use set::attended;

pub(crate) fn run(
    ctx: &output::Context,
    store: &vibe_connections::Store,
    agents: Vec<String>,
    connection: Option<String>,
    response: Option<String>,
    cancel: &CancellationToken,
) -> Result<()> {
    if agents.len() > 1 {
        return set::run(ctx, store, agents, connection, response, cancel);
    }
    match agents.as_slice() {
        [agent] if agent == "pi" => run_adapter(
            ctx,
            store,
            connection,
            response,
            cancel,
            vibe_connections::PiSettingsAdapter,
            target(agent)?,
        ),
        [agent] if agent == "qwen" => run_adapter(
            ctx,
            store,
            connection,
            response,
            cancel,
            vibe_connections::QwenSettingsAdapter,
            target(agent)?,
        ),
        [agent] if agent == "codex" => run_adapter(
            ctx,
            store,
            connection,
            response,
            cancel,
            vibe_connections::CodexSettingsAdapter,
            target(agent)?,
        ),
        [agent] if agent == "opencode" => run_adapter(
            ctx,
            store,
            connection,
            response,
            cancel,
            vibe_connections::OpenCodeSettingsAdapter,
            target(agent)?,
        ),
        [agent] if agent == "hermes" => run_adapter(
            ctx,
            store,
            connection,
            response,
            cancel,
            vibe_connections::HermesSettingsAdapter,
            target(agent)?,
        ),
        [agent] if agent == "claude" => run_adapter(
            ctx,
            store,
            connection,
            response,
            cancel,
            vibe_connections::ClaudeSettingsAdapter,
            target(agent)?,
        ),
        _ => bail!(
            "native settings require exactly one supported --agent pi, qwen, codex, opencode, hermes or claude"
        ),
    }
}
struct NativeTarget {
    destination: &'static str,
    overrides: &'static [&'static str],
}
fn run_adapter<A: NativeSettingsAdapter>(
    ctx: &output::Context,
    store: &vibe_connections::Store,
    connection: Option<String>,
    response: Option<String>,
    cancel: &CancellationToken,
    adapter: A,
    target: NativeTarget,
) -> Result<()>
where
    A::Preview: vibe_connections::NativeSettingsPreview,
{
    let NativeTarget {
        destination,
        overrides,
    } = target;
    if overrides
        .iter()
        .any(|name| crate::commands::app::facts::env_set(name))
    {
        bail!("native settings destination override is not yet admitted");
    }
    use vibe_connections::NativeSettingsPreview;
    let home = super::super::super::app::facts::user_home()
        .ok_or_else(|| anyhow::anyhow!("native settings require an explicit effective HOME"))?;
    let id = vibe_secrets::SecretId::try_from(
        connection
            .ok_or_else(|| anyhow::anyhow!("native settings require an explicit --connection"))?,
    )?;
    cancel.checkpoint()?;
    let metadata = store.details(&id)?.metadata;
    let preview = adapter.preview(store, &home, &id)?;
    let summary = serde_json::json!({"agent":adapter.id(),"connection":id,"endpoint":metadata.endpoint,"model":metadata.default_model,"destination":destination,"preview_id":preview.preview_id()});
    let attended = !ctx.is_json()
        && !ctx.is_unattended()
        && std::io::stdin().is_terminal()
        && std::io::stderr().is_terminal();
    let mut response = response;
    if attended && response.is_none() {
        println!("{}", serde_json::to_string_pretty(&summary)?);
        let choices = crate::output::prompts::named(&[
            "Keep native settings unchanged",
            "Write this connection into selected agent native settings",
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

fn target(agent: &str) -> Result<NativeTarget> {
    Ok(match agent {
        "pi" => NativeTarget {
            destination: "HOME/.pi/agent/models.json",
            overrides: &["PI_CODING_AGENT_DIR"],
        },
        "qwen" => NativeTarget {
            destination: "HOME/.qwen/settings.json",
            overrides: &[
                "QWEN_HOME",
                "QWEN_CODE_SYSTEM_SETTINGS_PATH",
                "QWEN_CODE_SYSTEM_DEFAULTS_PATH",
            ],
        },
        "codex" => NativeTarget {
            destination: "HOME/.codex/config.toml",
            overrides: &["CODEX_HOME"],
        },
        "opencode" => NativeTarget {
            destination: "HOME/.config/opencode/opencode.json",
            overrides: &[
                "XDG_CONFIG_HOME",
                "OPENCODE_CONFIG",
                "OPENCODE_CONFIG_CONTENT",
                "OPENCODE_CONFIG_DIR",
            ],
        },
        "hermes" => NativeTarget {
            destination: "HOME/.hermes/config.yaml",
            overrides: &["HERMES_HOME", "HERMES_CONFIG_FILE"],
        },
        "claude" => NativeTarget {
            destination: "HOME/.claude/settings.json",
            overrides: &["CLAUDE_CONFIG_DIR"],
        },
        _ => bail!("unsupported native settings agent"),
    })
}
