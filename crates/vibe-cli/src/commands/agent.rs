//! `vibe agent list | presets | use | add | remove`: thin parsing and
//! rendering around the agent registry of `vibe-app` (PROP-064). The
//! registry always lives in the user settings root.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-064#agent-definition");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-064#presets");

use anyhow::Result;
use specmark::spec;
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
/// Publication and native runtime glue for Hermes/Pi launch carriers.
#[path = "agent/native.rs"]
pub(crate) mod native;
/// Resolving a launcher's program the way the operating system starts one:
/// on Windows through `PATHEXT`, so an npm-installed `claude.cmd` is found.
/// Public to the crate because the Codex version probe starts the very same
/// program and must resolve it the same way.
#[path = "agent/program.rs"]
pub(crate) mod program;
use crate::commands::app::global;
use crate::output;
use crate::output::language::{
    Cell, Glyph, List, Meta, Role, Row, cut, hint, paint, rule_at, rule_width, short_path, trailer,
    viewport, wrap,
};
use vibe_wire::generated::agent_list::AgentList;
use vibe_wire::generated::agent_presets::AgentPresets;

#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-064#agent-definition")]
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
    // applied site profile and this file — the same value an agent step and
    // a dependency's `slot:` contribution read.
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
    if ctx.is_quiet() {
        ctx.summary(&list_trailer(&listing));
        return Ok(());
    }
    ctx.block(&render_list(&listing, viewport(), rule_width()));
    Ok(())
}

/// The human listing of `vibe agent list`: the launchers, the agent
/// definitions, the effective trust and an external registration, each a
/// measured list under its own rule, then the trailer. `width` bounds the
/// rows and hints (unbounded on a redirected stream); `rule` is the width a
/// rule runs to.
///
/// ```text
/// ── launchers ─────────────────────────────────────────
///   claude  claude  claude                    [built-in]
///   cc2     claude  cc2 · config ~/.claude2   [~/.vibe/agents.toml]
///   vibe agent <name> starts one with this project's boot lane
///
/// ✻ 7 launchers · 0 agent definitions · default none · /home/u/.vibe/agents.toml
/// ```
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-067#list")]
fn render_list(listing: &AgentList, width: usize, rule: usize) -> String {
    let mut sections = vec![
        section("launchers", &launcher_list(listing).render_at(width), rule)
            + &hint_line(
                "vibe agent <name> starts one with this project's boot lane",
                width,
            ),
        section(
            "agent definitions",
            &definition_list(listing, width).render_at(width),
            rule,
        ),
    ];
    if let Some(trust) = trust_list(listing) {
        sections.push(
            section("trust", &trust.render_at(width), rule)
                + &hint_line(
                    "package-authored code of a trusted group runs unasked unless an untrusted \
                     entry covers it",
                    width,
                ),
        );
    }
    if let Some(external) = &listing.external {
        let row = Row::new(vec![
            Cell::text("VIBE_AGENT_COMMAND"),
            Cell::text(argv(&external.command, &external.args)),
        ]);
        sections.push(section(
            "external registration",
            &List::new(vec![row]).render_at(width),
            rule,
        ));
    }
    sections.push(list_trailer(listing));
    sections.join("\n\n")
}

/// A rule and the list under it, touching.
fn section(label: &str, body: &str, rule: usize) -> String {
    format!("{}\n{body}", rule_at(label, None, rule))
}

/// One dim hint line under a list, cut at the viewport (`#width`).
fn hint_line(text: &str, width: usize) -> String {
    format!("\n{}", cut(&format!("  {}", hint(text)), width))
}

/// `none`, dim, where a section has no rows.
fn none_list() -> List {
    List::new(vec![Row::new(vec![Cell::new("none", Role::Dim)])])
}

/// A command and its arguments as one line.
fn argv(command: &str, args: &[String]) -> String {
    std::iter::once(command)
        .chain(args.iter().map(String::as_str))
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// `1 launcher`, `7 launchers`.
fn count(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

/// The launchers: name, kind, the argv that starts it (and its config
/// dir), the source as a state word.
fn launcher_list(listing: &AgentList) -> List {
    let rows = listing
        .launchers
        .iter()
        .map(|launcher| {
            let mut command = launcher.command.join(" ");
            if let Some(dir) = &launcher.config_dir {
                command.push_str(&paint(Role::Dim, &format!(" · config {}", short_path(dir))));
            }
            Row::new(vec![
                Cell::text(&launcher.name),
                Cell::new(&launcher.kind, Role::Dim),
                Cell::text(command),
            ])
            .meta(vec![Meta::State(short_path(&launcher.source))])
        })
        .collect::<Vec<_>>();
    if rows.is_empty() {
        return none_list();
    }
    List::new(rows).meta_column()
}

/// The agent definitions: name, preset and `[default]`, the two argvs as
/// children; a definition that does not resolve is a `✗` row with its
/// problem as its child, wrapped rather than cut because it says the fix.
fn definition_list(listing: &AgentList, width: usize) -> List {
    if listing.agents.is_empty() {
        return none_list();
    }
    let rows = listing
        .agents
        .iter()
        .map(|agent| {
            let mut meta = Vec::new();
            if let Some(preset) = &agent.preset {
                meta.push(Meta::note(format!("preset {preset}")));
            }
            if agent.default {
                meta.push(Meta::State("default".into()));
            }
            let row = Row::new(vec![Cell::text(&agent.name)]);
            match &agent.problem {
                Some(problem) => {
                    let lines = wrap(problem, width.saturating_sub(CHILD).max(20))
                        .iter()
                        .map(|line| paint(Role::Alert, line))
                        .collect::<Vec<_>>();
                    row.glyph(Glyph::Failed, Role::Alert)
                        .meta(meta)
                        .child(lines.join(&format!("\n{}", " ".repeat(CHILD))))
                }
                None => {
                    let command = agent.command.as_deref().unwrap_or_default();
                    row.meta(meta)
                        .child(mode_line("interactive", command, &agent.interactive, width))
                        .child(mode_line("unattended", command, &agent.unattended, width))
                }
            }
        })
        .collect();
    List::new(rows).meta_column()
}

/// `interactive <cmd …>` / `unattended  <cmd …>`, the label dim and padded
/// so the two commands start together, cut at the viewport.
fn mode_line(label: &str, command: &str, args: &[String], width: usize) -> String {
    let line = format!(
        "{}{} {}",
        paint(Role::Dim, label),
        " ".repeat("interactive".len().saturating_sub(label.len())),
        argv(command, args)
    );
    cut(&line, width.saturating_sub(CHILD))
}

/// Where a child's text starts: `  └ `.
const CHILD: usize = 4;

/// The effective trust, entry by entry with the layer that wrote it, and
/// what each deny entry takes away. A reader must be able to see WHY a
/// group's installation steps run unasked, and which file to edit to stop
/// them (PROP-065 `#ordinary-trust`).
fn trust_list(listing: &AgentList) -> Option<List> {
    let trust = listing.trust.as_ref()?;
    let mut rows = Vec::new();
    for entry in &trust.trusted {
        rows.push(
            Row::new(vec![Cell::text("trusted"), Cell::text(&entry.pattern)])
                .meta(vec![Meta::State(entry.source.clone())]),
        );
    }
    if trust.trusted.is_empty() {
        rows.push(Row::new(vec![
            Cell::text("trusted"),
            Cell::new("nothing", Role::Dim),
        ]));
    }
    for entry in &trust.untrusted {
        let mut meta = Vec::new();
        if !entry.overrides.is_empty() {
            meta.push(Meta::note(format!(
                "overrides {}",
                entry.overrides.join(", ")
            )));
        }
        meta.push(Meta::State(entry.source.clone()));
        rows.push(
            Row::new(vec![
                Cell::new("untrusted", Role::Alert),
                Cell::text(&entry.pattern),
            ])
            .meta(meta),
        );
    }
    if !listing.trusted_groups.is_empty() {
        rows.push(
            Row::new(vec![
                Cell::new("written", Role::Dim),
                Cell::text(listing.trusted_groups.join(", ")),
            ])
            .meta(vec![Meta::State("trusted_groups".into())]),
        );
    }
    Some(List::new(rows).meta_column())
}

/// `✻ 7 launchers · 0 agent definitions · default none · <path in full>`.
fn list_trailer(listing: &AgentList) -> String {
    let default = listing.default.as_deref().unwrap_or("none");
    trailer(
        true,
        &count(listing.launchers.len(), "launcher", "launchers"),
        vec![
            Meta::note(count(
                listing.agents.len(),
                "agent definition",
                "agent definitions",
            )),
            Meta::note(format!("default {default}")),
            Meta::note(&listing.path),
        ],
        None,
    )
}

fn presets(ctx: &output::Context) -> Result<()> {
    let table = vibe_app::agent_preset_table();
    if ctx.is_json() {
        return ctx.emit_json(&table);
    }
    if ctx.is_quiet() {
        ctx.summary(&presets_trailer(&table));
        return Ok(());
    }
    ctx.block(&render_presets(&table, viewport()));
    Ok(())
}

/// The human table of `vibe agent presets`: one measured list (name,
/// harness, the two argvs as children), then the trailer.
///
/// ```text
///   claude-code  claude
///     └ interactive claude
///     └ unattended  claude -p {prompt}
///
/// ✻ 5 agent presets · placeholders {prompt} {cwd}
/// ```
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-067#list")]
fn render_presets(table: &AgentPresets, width: usize) -> String {
    let rows = table
        .presets
        .iter()
        .map(|preset| {
            Row::new(vec![
                Cell::text(&preset.name),
                Cell::new(&preset.harness, Role::Dim),
            ])
            .child(mode_line(
                "interactive",
                &preset.command,
                &preset.interactive,
                width,
            ))
            .child(mode_line(
                "unattended",
                &preset.command,
                &preset.unattended,
                width,
            ))
        })
        .collect::<Vec<_>>();
    let list = if rows.is_empty() {
        none_list()
    } else {
        List::new(rows)
    };
    format!("{}\n\n{}", list.render_at(width), presets_trailer(table))
}

/// `✻ 5 agent presets · placeholders {prompt} {cwd}`.
fn presets_trailer(table: &AgentPresets) -> String {
    trailer(
        true,
        &count(table.presets.len(), "agent preset", "agent presets"),
        vec![Meta::note(format!(
            "placeholders {}",
            table.placeholders.join(" ")
        ))],
        None,
    )
}

#[cfg(test)]
#[path = "agent/tests.rs"]
mod tests;
