//! The launcher-catalog surface of `vibe app` (PROP-059
//! `#launcher-catalog`): `launchers --revision`, the collection, pin and
//! override edits of the user layer, and the detached `start --json`.
//!
//! Every edit answers the whole user layer, so a client that changed one
//! collection sees what the change did to the rest without a second call.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#launcher-catalog");

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use vibe_app::{Index, LauncherChange, PinTarget, StartRequest, Storage};
use vibe_wire::generated::application_collections::ApplicationCollections;

use super::home::AppHome;
use super::launch::choose_launcher;
use crate::cli::{CollectionCommand, PinArg};
use crate::output;

/// `vibe app launchers --revision`: the counter alone, so a client polls one
/// number instead of refetching the catalog.
pub(super) fn revision(ctx: &output::Context, app_home: &AppHome) -> Result<()> {
    let answer = vibe_app::revision_document(app_home.settings());
    if ctx.is_json() {
        return ctx.emit_json(&answer);
    }
    println!("{}", answer.revision);
    Ok(())
}

/// `vibe app collection …`.
pub(super) fn collection(
    ctx: &output::Context,
    app_home: &AppHome,
    command: CollectionCommand,
) -> Result<()> {
    let settings = app_home.settings();
    let layer = match command {
        CollectionCommand::List => vibe_app::collections(settings)?,
        CollectionCommand::Create { name, icon } => {
            vibe_app::app_collection_create(settings, &name, resolved(icon.as_deref()).as_deref())?
        }
        CollectionCommand::Rename { old, new } => {
            vibe_app::app_collection_rename(settings, &old, &new)?
        }
        CollectionCommand::Delete { name } => vibe_app::app_collection_delete(settings, &name)?,
        CollectionCommand::Add { name, keys } => {
            vibe_app::app_collection_add(settings, &name, &keys)?
        }
        CollectionCommand::Remove { name, keys } => {
            vibe_app::app_collection_remove(settings, &name, &keys)?
        }
        CollectionCommand::Order { name, keys } => {
            vibe_app::app_collection_order(settings, &name, &keys)?
        }
    };
    render(ctx, &layer)
}

/// `vibe app pin KEY --to …` and `vibe app unpin KEY --from …`.
pub(super) fn pin(
    ctx: &output::Context,
    app_home: &AppHome,
    key: &str,
    target: PinArg,
    pinning: bool,
) -> Result<()> {
    let settings = app_home.settings();
    let target = match target {
        PinArg::Taskbar => PinTarget::Taskbar,
        PinArg::Favorites => PinTarget::Favorites,
    };
    let layer = if pinning {
        vibe_app::app_pin(settings, key, target)?
    } else {
        vibe_app::app_unpin(settings, key, target)?
    };
    render(ctx, &layer)
}

/// Which change the five flags of `vibe app launcher KEY` name. Clap's
/// argument group admits exactly one of them, so the fallthrough is
/// unreachable through the CLI and is still an error rather than a guess.
pub(super) fn launcher_change(
    hide: bool,
    show: bool,
    name: Option<String>,
    icon: Option<PathBuf>,
    reset: bool,
) -> Result<LauncherChange> {
    Ok(match (hide, show, name, icon, reset) {
        (true, ..) => LauncherChange::Hide,
        (_, true, ..) => LauncherChange::Show,
        (_, _, Some(name), ..) => LauncherChange::Name(name),
        (_, _, _, Some(icon), _) => {
            LauncherChange::Icon(resolved(Some(&icon)).unwrap_or(icon).display().to_string())
        }
        (.., true) => LauncherChange::Reset,
        _ => bail!("usage: name one of --hide, --show, --name, --icon or --reset"),
    })
}

/// `vibe app launcher KEY --hide | --show | --name … | --icon … | --reset`.
pub(super) fn launcher(
    ctx: &output::Context,
    app_home: &AppHome,
    key: &str,
    change: LauncherChange,
) -> Result<()> {
    let layer = vibe_app::launcher_override(app_home.settings(), key, change)?;
    render(ctx, &layer)
}

/// A user-given path as the layer records it: absolute when the file is
/// there to canonicalise, otherwise exactly as typed. The home is read by
/// clients that never share the caller's working directory, so a relative
/// path that still resolves is worth making absolute here, where the
/// working directory is the user's own.
fn resolved(path: Option<&Path>) -> Option<PathBuf> {
    path.map(|path| fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf()))
}

/// `vibe app start … --json`: start the launcher detached and answer its
/// process id, command line and log instead of waiting for it.
pub(super) fn start_detached(
    ctx: &output::Context,
    app_home: &AppHome,
    package: &str,
    launcher_id: &str,
    args: &[String],
    project_root: Option<&Path>,
) -> Result<()> {
    let settings = app_home.settings();
    let index = Index::load(settings)?;
    let storage = Storage::load(settings)?;
    let (_, installation) = vibe_app::installed(settings, &index, package)?;
    let extra: Vec<OsString> = args.iter().map(OsString::from).collect();
    let answer = vibe_app::start_launcher(
        installation,
        &storage,
        &StartRequest {
            settings,
            launcher_id,
            extra_args: &extra,
            project_root,
            // Recorded in the running registry, so `running`, `stop` and
            // `watch` know it (PROP-059 `#launcher-runtime`).
            processes: Some(&super::processes::HostProcesses),
        },
    )?;
    ctx.emit_json(&answer)
}

/// The package spelling and the launcher a `vibe app start` argument names:
/// a launcher key's `#<launcher id>` selects the launch point, and
/// `--launcher` is the other spelling of the same choice, never a second
/// one.
pub(super) fn start_target(
    spelling: &str,
    launcher: Option<String>,
) -> Result<(String, Option<String>)> {
    match vibe_app::split_launcher_key(spelling) {
        Some((package, id)) => match launcher {
            Some(explicit) if explicit != id => bail!(
                "usage: `{spelling}` already names launcher `{id}`; drop --launcher {explicit}"
            ),
            _ => Ok((package.to_owned(), Some(id.to_owned()))),
        },
        None => Ok((spelling.to_owned(), launcher)),
    }
}

/// The launcher a start acts on: the one named, else the package's declared
/// default, else a question in an attended terminal and the refusal
/// `selection_required` without one.
pub(super) fn start_launcher_id(
    ctx: &output::Context,
    app_home: &AppHome,
    package: &str,
    launcher: Option<String>,
    project_root: Option<&Path>,
) -> Result<Option<String>> {
    match launcher {
        Some(id) => Ok(Some(id)),
        None => {
            let listing = super::launch::launcher_listing(app_home, package, project_root)?;
            choose_launcher(&listing, ctx)
        }
    }
}

/// The user layer as a terminal shows it: every collection with its keys in
/// order, both pinned lists and every override.
fn render(ctx: &output::Context, layer: &ApplicationCollections) -> Result<()> {
    if ctx.is_json() {
        return ctx.emit_json(layer);
    }
    ctx.summary(&format!(
        "{} collection(s) in {} at revision {}",
        layer.collections.len(),
        layer.home,
        layer.revision
    ));
    if ctx.is_quiet() {
        return Ok(());
    }
    for collection in &layer.collections {
        let seeded = if collection.seeded { " [seeded]" } else { "" };
        match &collection.icon {
            Some(icon) => println!("  {}{seeded} ({icon})", collection.name),
            None => println!("  {}{seeded}", collection.name),
        }
        for key in &collection.launchers {
            println!("    {key}");
        }
        for key in &collection.removed {
            println!("    {key} [removed]");
        }
    }
    for (name, list) in [
        ("taskbar", &layer.pins.taskbar),
        ("favorites", &layer.pins.favorites),
    ] {
        if !list.is_empty() {
            println!("pinned to {name}: {}", list.join(", "));
        }
    }
    for row in &layer.overrides {
        let hidden = if row.hidden { " hidden" } else { "" };
        let name = row.name.as_deref().unwrap_or("-");
        let icon = row.icon.as_deref().unwrap_or("-");
        println!("override {}{hidden}: name {name}, icon {icon}", row.key);
    }
    Ok(())
}
