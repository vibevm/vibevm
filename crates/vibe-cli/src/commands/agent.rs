//! `vibe agent list | presets | use | add | remove`: thin parsing and
//! rendering around the agent registry of `vibe-app` (PROP-064). The
//! registry always lives in the user settings root.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-064#agent-definition");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-064#presets");

use anyhow::Result;
use vibe_app::{AgentDefinition, AgentsConfig, AppError, SettingsRoot};

use crate::cli::{AgentArgs, AgentCommand};

/// Starting one launcher with the composed lane in its instruction slot.
#[path = "agent/launch.rs"]
mod launch;
/// Agent launchers — a name bound to a harness kind and to the argv that
/// starts it. Out of line along the PROP-064/PROP-076 seam: the definitions
/// above configure which agent runs an installer's prompt, and a launcher
/// configures which program a developer's own coding session starts.
#[path = "agent/launchers.rs"]
pub(crate) mod launchers;
/// Resolving a launcher's program the way the operating system starts one:
/// on Windows through `PATHEXT`, so an npm-installed `claude.cmd` is found.
/// Public to the crate because the Codex version probe starts the very same
/// program and must resolve it the same way.
#[path = "agent/program.rs"]
pub(crate) mod program;
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
        AgentCommand::Launch(argv) => launch::run(ctx, argv),
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
    // The ONE effective trust, resolved from the base list vibe ships, the
    // applied site profile and this file — the same value an agent step, a
    // dependency's `slot:` contribution and a `[hooks]` entry read.
    let trust = vibe_app::effective_trust(settings, Some(config)).map_err(AppError::from)?;
    let mut listing = vibe_app::agent_list(settings, config, external.as_ref(), &trust);
    // The launcher rows are resolved here, not in the registry: merging the
    // user's `[launchers.*]` with the built-ins of a closed harness kind set
    // is PROP-076's question, and the registry is PROP-064's.
    let launchers = launchers::resolve_all(settings, config)?;
    listing.launchers = launchers.iter().map(launchers::row).collect();
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
    print_trust(&listing);
    if let Some(external) = &listing.external {
        println!(
            "  external registration (VIBE_AGENT_COMMAND): {} {}",
            external.command,
            external.args.join(" ")
        );
    }
    println!(
        "  {} agent launcher(s) — `vibe agent <name>` starts one with this project's boot lane \
         in its own instruction slot:",
        listing.launchers.len()
    );
    for launcher in &listing.launchers {
        println!(
            "    {} (kind {}) — {} [{}]",
            launcher.name,
            launcher.kind,
            launcher.command.join(" "),
            launcher.source
        );
        if let Some(dir) = &launcher.config_dir {
            println!("      config dir: {dir}");
        }
    }
    Ok(())
}

/// The effective trust, entry by entry with the layer that wrote it, and
/// what each deny entry takes away. A reader must be able to see WHY a
/// group's installation steps run unasked, and which file to edit to stop
/// them (PROP-065 `#ordinary-trust`).
fn print_trust(listing: &vibe_wire::generated::agent_list::AgentList) {
    let Some(trust) = &listing.trust else {
        return;
    };
    println!(
        "  effective trust — a group's package-authored code runs unasked when a trusted entry \
         covers it and no untrusted entry does:"
    );
    for entry in &trust.trusted {
        println!("    trusted:   {} [{}]", entry.pattern, entry.source);
    }
    if trust.trusted.is_empty() {
        println!("    trusted:   (nothing)");
    }
    for entry in &trust.untrusted {
        let overrides = if entry.overrides.is_empty() {
            String::new()
        } else {
            format!(" — overrides {}", entry.overrides.join(", "))
        };
        println!(
            "    untrusted: {} [{}]{overrides}",
            entry.pattern, entry.source,
        );
    }
    if !listing.trusted_groups.is_empty() {
        println!(
            "    written in {}: trusted_groups = {}",
            listing.path,
            listing.trusted_groups.join(", "),
        );
    }
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
