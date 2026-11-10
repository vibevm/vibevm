//! Agent **launchers**: a name bound to a harness kind and to the argv that
//! starts it (PROP-076 `#SYS-HARNESS-EXECUTABLE`).
//!
//! **The kind and the program are two different things, and conflating them
//! was the mistake this cell exists to undo.** A *kind* — `claude`, `codex`,
//! a closed set — is the flag dialect and the delivery rules. A *launcher*
//! is a name bound to a kind and to an argv **prefix**, so a personal
//! wrapper may carry its own standing arguments.
//!
//! **Only a user may name one.** A repository must not choose which program
//! runs on a contributor's machine: that is the one setting where a shared
//! file would let one developer's convenience execute arbitrary code on
//! everyone else's computer, so a project manifest that names a launcher is
//! a refusal and not a judgement call.
//!
//! The built-ins `claude` and `codex` always exist, each with `kind` equal
//! to its name; a user launcher of the same name **overrides** the built-in,
//! which is how a wrapper takes over the plain name without the user
//! retyping it. Any number of launchers may share a kind.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-076#SYS-HARNESS-EXECUTABLE");

use std::path::Path;

use anyhow::{Result, bail};
use vibe_app::{AgentsConfig, SettingsRoot};
use vibe_wire::generated::agent_list as wire;
use vibe_workspace::system_prompt::HarnessKind;

/// One resolved launcher.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Launcher {
    /// The name the user asks for it by.
    pub(crate) name: String,
    /// Its harness kind — the flag dialect and the delivery rules.
    pub(crate) kind: HarnessKind,
    /// The argv prefix to exec: the program and its standing arguments.
    pub(crate) command: Vec<String>,
    /// Where this launcher's harness keeps its session logs, when it is
    /// knowable at all.
    pub(crate) config_dir: Option<String>,
    /// Built-in, or the exact user-config path that decided it — because a
    /// developer debugging which program ran needs to know which file did.
    pub(crate) source: String,
}

/// Every launcher this machine has, built-ins first and user ones
/// overriding a built-in of the same name.
pub(crate) fn resolve_all(settings: &SettingsRoot, config: &AgentsConfig) -> Result<Vec<Launcher>> {
    let path = settings.agents_path().display().to_string();
    let mut launchers: Vec<Launcher> = HarnessKind::ALL
        .into_iter()
        .map(|kind| Launcher {
            name: kind.as_str().to_owned(),
            kind,
            command: vec![kind.as_str().to_owned()],
            config_dir: None,
            source: "built-in".to_owned(),
        })
        .collect();
    for (name, declared) in &config.launchers {
        let Some(kind) = HarnessKind::parse(&declared.kind) else {
            bail!(
                "`[launchers.{name}] kind = \"{}\"` in `{path}` names no harness kind this \
                 build knows; they are {}",
                declared.kind,
                HarnessKind::ALL
                    .map(|kind| format!("`{}`", kind.as_str()))
                    .join(", ")
            );
        };
        let command = if declared.command.is_empty() {
            vec![kind.as_str().to_owned()]
        } else {
            declared.command.clone()
        };
        let resolved = Launcher {
            name: name.clone(),
            kind,
            command,
            config_dir: declared.config_dir.clone(),
            source: path.clone(),
        };
        match launchers.iter_mut().find(|existing| existing.name == *name) {
            // A user launcher of a built-in's name overrides it, which is
            // how a wrapper takes over the plain name.
            Some(existing) => *existing = resolved,
            None => launchers.push(resolved),
        }
    }
    Ok(launchers)
}

/// Resolve one launcher by name. An unknown name is a refusal that **lists
/// the available launchers**, because the one thing a developer needs on a
/// typo is the set they could have meant.
pub(crate) fn resolve(
    settings: &SettingsRoot,
    config: &AgentsConfig,
    name: &str,
) -> Result<Launcher> {
    let launchers = resolve_all(settings, config)?;
    match launchers.iter().find(|launcher| launcher.name == name) {
        Some(launcher) => Ok(launcher.clone()),
        None => bail!(
            "`{name}` is not an agent launcher; this machine has {}. Declare one as \
             `[launchers.{name}]` with `kind` and `command` in `{}`",
            launchers
                .iter()
                .map(|launcher| format!("`{}`", launcher.name))
                .collect::<Vec<_>>()
                .join(", "),
            settings.agents_path().display()
        ),
    }
}

/// Refuse a project manifest that names a launcher, by name.
///
/// The spelling checked is the one `PROP-076` writes — `[agent.launchers]`
/// in a `vibe.toml`. The manifest parser would already reject the table as
/// an unknown field; this says *why* instead, because "unknown field
/// `agent`" is not the sentence a contributor needs to read.
pub(crate) fn refuse_project_launchers(project_root: &Path) -> Result<()> {
    let path = project_root.join(vibe_core::manifest::Manifest::FILENAME);
    let Ok(text) = std::fs::read_to_string(&path) else {
        return Ok(());
    };
    let Ok(document) = text.parse::<toml::Table>() else {
        return Ok(());
    };
    let names = document
        .get("agent")
        .and_then(toml::Value::as_table)
        .and_then(|agent| agent.get("launchers"))
        .and_then(toml::Value::as_table);
    let Some(names) = names else {
        return Ok(());
    };
    bail!(
        "`{}` declares `[agent.launchers.*]` ({}), and a project manifest may not name an agent \
         launcher: a repository must not choose which program runs on a contributor's machine. \
         Move the table to your own `[launchers.*]` in the user configuration",
        path.display(),
        names
            .keys()
            .map(|name| format!("`{name}`"))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

/// One launcher as the `vibe agent list` document carries it.
pub(crate) fn row(launcher: &Launcher) -> wire::Launcher {
    wire::Launcher {
        name: launcher.name.clone(),
        kind: launcher.kind.as_str().to_owned(),
        command: launcher.command.clone(),
        config_dir: launcher.config_dir.clone(),
        source: launcher.source.clone(),
    }
}
