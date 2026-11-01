//! `vibe app launchers --export-system | --remove-system` (PROP-059
//! `#launcher-runtime`): where the platform keeps the user's menu entries
//! and how an entry starts a launcher are decided here, at the CLI
//! boundary; `vibe-app` writes, records and prunes.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#launcher-runtime");

use std::path::Path;

use anyhow::{Context, Result};
use vibe_app::{ApplicationExported, ExportRequest, SystemTargets};

use super::home::AppHome;
use crate::output;

/// The flags of one projection call.
pub(super) struct Selection<'a> {
    pub remove: bool,
    pub collection: Option<&'a str>,
    pub project_root: Option<&'a Path>,
    pub dry_run: bool,
}

pub(super) fn system(
    ctx: &output::Context,
    app_home: &AppHome,
    selection: Selection<'_>,
) -> Result<()> {
    let targets = targets(app_home)?;
    let settings = app_home.settings();
    let answer = if selection.remove {
        vibe_app::remove_system(settings, &targets, selection.dry_run)?
    } else {
        vibe_app::export_system(&ExportRequest {
            settings,
            targets: &targets,
            collection: selection.collection,
            project_root: selection.project_root,
            dry_run: selection.dry_run,
        })?
    };
    render(ctx, &answer)
}

/// The platform's entry directory and the command line an entry runs: this
/// very `vibe`, told which home to start from — the settings root through
/// `VIBE_SETTINGS` (a desktop session does not carry the caller's
/// environment), or the project through `--path`.
fn targets(app_home: &AppHome) -> Result<SystemTargets> {
    let root = super::facts::system_entries_root().context(
        "the platform's menu-entry directory is unknown; set HOME (or APPDATA on Windows)",
    )?;
    let exe = std::env::current_exe().context("locating the running vibe executable")?;
    let exe = exe.display().to_string();
    let start = match app_home {
        AppHome::Project { root, .. } => vec![
            exe,
            "app".into(),
            "start".into(),
            "--path".into(),
            root.display().to_string(),
        ],
        AppHome::User(settings) if cfg!(windows) => {
            // A shortcut carries no environment of its own: it starts the
            // home `vibe` finds by default.
            let _ = settings;
            vec![exe, "app".into(), "start".into(), "-g".into()]
        }
        AppHome::User(settings) => vec![
            "env".into(),
            format!("VIBE_SETTINGS={}", settings.path().display()),
            exe,
            "app".into(),
            "start".into(),
            "-g".into(),
        ],
    };
    Ok(SystemTargets {
        platform: SystemTargets::host_platform(),
        root,
        start,
    })
}

fn render(ctx: &output::Context, answer: &ApplicationExported) -> Result<()> {
    if ctx.is_json() {
        return ctx.emit_json(answer);
    }
    let verb = if answer.dry_run {
        "would write"
    } else {
        "wrote"
    };
    for entry in &answer.entries {
        println!("{verb} {} ({})", entry.name, entry.key);
        if !ctx.is_quiet() {
            for path in &entry.paths {
                println!("  {path}");
            }
        }
    }
    let gone = if answer.dry_run {
        "would remove"
    } else {
        "removed"
    };
    for path in &answer.removed {
        println!("{gone} {path}");
    }
    ctx.summary(&format!(
        "{} system entr{} for {}",
        answer.entries.len(),
        if answer.entries.len() == 1 {
            "y"
        } else {
            "ies"
        },
        answer.home
    ));
    Ok(())
}
