//! `vibe agent list | presets | use | add | remove`: thin parsing and
//! rendering around the agent registry of `vibe-app` (PROP-064). The
//! registry always lives in the user settings root.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-064#agent-definition");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-064#presets");

use anyhow::Result;
use vibe_app::{AgentDefinition, AgentsConfig, AppError, SettingsRoot};

use crate::cli::{AgentArgs, AgentCommand};
use crate::commands::app::global;
use crate::output;

pub fn run(ctx: &output::Context, args: AgentArgs) -> Result<()> {
    let settings = global::settings_root()?;
    match args.command {
        AgentCommand::List => list(
            ctx,
            &settings,
            &AgentsConfig::load(&settings).map_err(AppError::from)?,
        ),
        AgentCommand::Presets => presets(ctx),
        AgentCommand::Use { name } => edit(
            ctx,
            &settings,
            &format!("default agent is `{name}`"),
            |config| config.use_agent(&name),
        ),
        AgentCommand::Add {
            name,
            preset,
            command,
            interactive_args,
            unattended_args,
            cwd,
            env,
            extra_args,
            default,
        } => {
            let definition = AgentDefinition {
                preset,
                command,
                interactive: (!interactive_args.is_empty()).then_some(interactive_args),
                unattended: (!unattended_args.is_empty()).then_some(unattended_args),
                cwd,
                env: env.into_iter().collect(),
                extra_args,
            };
            edit(ctx, &settings, &format!("added agent `{name}`"), |config| {
                config.add(&name, definition)?;
                if default {
                    config.use_agent(&name)?;
                }
                Ok(())
            })
        }
        AgentCommand::Remove { name } => edit(
            ctx,
            &settings,
            &format!("removed agent `{name}`"),
            |config| config.remove(&name),
        ),
    }
}

/// Apply one edit under the settings lock, save, then list.
fn edit(
    ctx: &output::Context,
    settings: &SettingsRoot,
    done: &str,
    change: impl FnOnce(&mut AgentsConfig) -> Result<(), vibe_app::AgentError>,
) -> Result<()> {
    let _lock = settings.lock()?;
    let mut config = AgentsConfig::load(settings).map_err(AppError::from)?;
    change(&mut config).map_err(AppError::from)?;
    config.save(settings).map_err(AppError::from)?;
    if !ctx.is_json() {
        ctx.summary(&format!("vibe agent: {done}"));
        if ctx.is_quiet() {
            return Ok(());
        }
    }
    list(ctx, settings, &config)
}

fn list(ctx: &output::Context, settings: &SettingsRoot, config: &AgentsConfig) -> Result<()> {
    let external = crate::commands::app::agents::external_registration()?;
    let listing = vibe_app::agent_list(settings, config, external.as_ref());
    if ctx.is_json() {
        return ctx.emit_json(&listing);
    }
    let default = listing.default.as_deref().unwrap_or("none");
    ctx.summary(&format!(
        "{} agent definition(s) in {}; default: {default}",
        listing.agents.len(),
        listing.path
    ));
    if ctx.is_quiet() {
        return Ok(());
    }
    for agent in &listing.agents {
        let marker = if agent.default { " [default]" } else { "" };
        let preset = agent
            .preset
            .as_deref()
            .map(|preset| format!(" (preset {preset})"))
            .unwrap_or_default();
        println!("  {}{marker}{preset}", agent.name);
        match &agent.problem {
            Some(problem) => println!("    problem: {problem}"),
            None => {
                let command = agent.command.as_deref().unwrap_or_default();
                println!("    interactive: {command} {}", agent.interactive.join(" "));
                println!("    unattended: {command} {}", agent.unattended.join(" "));
            }
        }
    }
    if !listing.trusted_groups.is_empty() {
        println!("  trusted groups: {}", listing.trusted_groups.join(", "));
    }
    if let Some(external) = &listing.external {
        println!(
            "  external registration (VIBE_AGENT_COMMAND): {} {}",
            external.command,
            external.args.join(" ")
        );
    }
    Ok(())
}

fn presets(ctx: &output::Context) -> Result<()> {
    let table = vibe_app::agent_preset_table();
    if ctx.is_json() {
        return ctx.emit_json(&table);
    }
    ctx.summary(&format!(
        "{} agent preset(s); placeholders {}",
        table.presets.len(),
        table.placeholders.join(" ")
    ));
    if ctx.is_quiet() {
        return Ok(());
    }
    for preset in &table.presets {
        println!("  {} — {}", preset.name, preset.harness);
        println!(
            "    interactive: {} {}",
            preset.command,
            preset.interactive.join(" ")
        );
        println!(
            "    unattended: {} {}",
            preset.command,
            preset.unattended.join(" ")
        );
    }
    Ok(())
}
