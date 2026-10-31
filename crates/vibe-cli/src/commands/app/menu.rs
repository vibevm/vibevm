//! The graphical-client surface of `vibe app` (PROP-059 `#menu`,
//! `#desktop-entries`): `launchers --all [--menu]`, `icon` and
//! `import-desktop`.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#menu");

use std::path::{Path, PathBuf};

use anyhow::Result;
use vibe_app::{Index, Storage};

use super::home::AppHome;
use super::views::print_listing;
use crate::output;

/// `vibe app launchers --all [--menu]`: every ready, exposed installation of
/// the selected home.
pub(super) fn all(
    ctx: &output::Context,
    app_home: &AppHome,
    project_root: Option<&Path>,
    menu_only: bool,
) -> Result<()> {
    let settings = app_home.settings();
    let index = Index::load(settings)?;
    let storage = Storage::load(settings)?;
    let listing =
        vibe_app::launcher_listing_all(&index, &storage, settings.path(), project_root, menu_only);
    let value = serde_json::to_value(&listing)?;
    if ctx.is_json() {
        return ctx.emit_json(&value);
    }
    let installations = value
        .get("installations")
        .and_then(serde_json::Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    println!("{} installation(s)", installations.len());
    for installation in installations {
        if let Some(group) = installation
            .get("menu_group")
            .and_then(serde_json::Value::as_str)
        {
            println!("[{group}]");
        }
        print_listing(installation, ctx.is_quiet());
    }
    Ok(())
}

/// `vibe app icon PKG [--launcher ID] --size N [--prefer SLOT]`: the path of
/// the best existing file; exit 1 when none exists.
pub(super) fn icon(
    ctx: &output::Context,
    app_home: &AppHome,
    package: &str,
    launcher: Option<&str>,
    size: u32,
    prefer: Option<&str>,
) -> Result<()> {
    let settings = app_home.settings();
    let index = Index::load(settings)?;
    let storage = Storage::load(settings)?;
    let (_, installation) = vibe_app::installed(settings, &index, package)?;
    let chosen = vibe_app::icon_choice(
        installation,
        &storage,
        settings.path(),
        launcher,
        size,
        prefer,
    )?;
    if ctx.is_json() {
        return ctx.emit_json(&serde_json::to_value(&chosen)?);
    }
    println!("{}", chosen.path);
    Ok(())
}

/// `vibe app import-desktop FILE… [--app-root DIR]`: manifest TOML rows on
/// stdout.
pub(super) fn import_desktop(files: &[PathBuf], app_root: Option<&Path>) -> Result<()> {
    let text = vibe_app::import_desktop(files, app_root).map_err(anyhow::Error::msg)?;
    print!("{text}");
    Ok(())
}
