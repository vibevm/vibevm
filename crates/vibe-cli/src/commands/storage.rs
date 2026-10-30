//! `vibe storage list | add | attach | detach | move | gc`: thin parsing and
//! rendering around the `vibe-app` storage seams. Relative paths resolve
//! against the working directory here, at the boundary.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#storage-libraries");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#move");

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use vibe_app::{Library, SettingsRoot};
use vibe_wire::generated::application_storage_gc::ApplicationStorageGc;
use vibe_wire::generated::application_storage_list::{ApplicationStorageList, Status};
use vibe_wire::generated::application_storage_move::ApplicationStorageMove;

use crate::cli::{StorageArgs, StorageCommand};
use crate::commands::app::global;
use crate::output;

pub fn run(ctx: &output::Context, args: StorageArgs) -> Result<()> {
    let settings = global::settings_root()?;
    match args.command {
        StorageCommand::List => {
            let listing = vibe_app::storage_list(&settings)?;
            if ctx.is_json() {
                return ctx.emit_json(&listing);
            }
            print_list(&listing, ctx.is_quiet());
            Ok(())
        }
        StorageCommand::Add { path, id } => {
            let library = vibe_app::storage_add(&settings, &absolute(&path)?, id.as_deref())?;
            changed(ctx, &settings, "add", &library, "created and listed")
        }
        StorageCommand::Attach { path } => {
            let library = vibe_app::storage_attach(&settings, &absolute(&path)?)?;
            changed(ctx, &settings, "attach", &library, "listed")
        }
        StorageCommand::Detach { id } => {
            let library = vibe_app::storage_detach(&settings, &id)?;
            changed(
                ctx,
                &settings,
                "detach",
                &library,
                "unlisted; its files stay in place",
            )
        }
        StorageCommand::Move { key, to } => {
            let moved = vibe_app::storage_move(&settings, &key, &to)?;
            if ctx.is_json() {
                return ctx.emit_json(&ApplicationStorageMove {
                    protocol: "vibe-app-storage-move/1".into(),
                    key: moved.key,
                    from: moved.from,
                    to: moved.to,
                    old_app_root: vibe_core::machine_json_path(&moved.old_app_root),
                    new_app_root: vibe_core::machine_json_path(&moved.new_app_root),
                    message: moved.message,
                });
            }
            ctx.summary(&format!("vibe storage move: {}", moved.message));
            if !ctx.is_quiet() {
                println!("  old app_root: {}", moved.old_app_root.display());
                println!("  new app_root: {}", moved.new_app_root.display());
            }
            Ok(())
        }
        StorageCommand::Gc { delete } => {
            let report = vibe_app::storage_gc(&settings, delete)?;
            if ctx.is_json() {
                return ctx.emit_json(&report);
            }
            print_gc(&report, ctx.is_quiet());
            Ok(())
        }
    }
}

/// After a row edit the machine answer is the listing; the human answer is
/// one line.
fn changed(
    ctx: &output::Context,
    settings: &SettingsRoot,
    command: &str,
    library: &Library,
    what: &str,
) -> Result<()> {
    if ctx.is_json() {
        return ctx.emit_json(&vibe_app::storage_list(settings)?);
    }
    ctx.summary(&format!(
        "vibe storage {command}: library `{}` at `{}` {what}",
        library.id,
        library.path.display()
    ));
    Ok(())
}

fn absolute(path: &Path) -> Result<PathBuf> {
    std::path::absolute(path).with_context(|| format!("resolving `{}`", path.display()))
}

fn status_text(status: &Status, reason: Option<&str>) -> String {
    match (status, reason) {
        (Status::Ready, _) => "ready".to_owned(),
        (Status::Unavailable, Some(reason)) => format!("unavailable: {reason}"),
        (Status::Unavailable, None) => "unavailable".to_owned(),
    }
}

fn print_list(listing: &ApplicationStorageList, quiet: bool) {
    let count = listing.libraries.len();
    println!(
        "{count} storage {}",
        if count == 1 { "library" } else { "libraries" }
    );
    if quiet {
        return;
    }
    for library in &listing.libraries {
        let marker = if library.default { " [default]" } else { "" };
        let path = library.path.as_deref().unwrap_or("(not configured)");
        println!(
            "  {}{marker}: {path} ({})",
            library.id,
            status_text(&library.status, library.reason.as_deref())
        );
        for installation in &library.installations {
            println!(
                "    {}: {} ({})",
                installation.slot,
                installation.key,
                status_text(&installation.status, installation.reason.as_deref())
            );
        }
    }
}

fn print_gc(report: &ApplicationStorageGc, quiet: bool) {
    let verb = if report.deleted { "removed" } else { "found" };
    let count = report.orphans.len();
    println!(
        "{verb} {count} unreferenced {}",
        if count == 1 {
            "directory"
        } else {
            "directories"
        }
    );
    if quiet {
        return;
    }
    for orphan in &report.orphans {
        println!("  {}: {}", orphan.library, orphan.path);
    }
    for skipped in &report.skipped {
        println!("  skipped {}: {}", skipped.library, skipped.reason);
    }
    if !report.deleted && !report.orphans.is_empty() {
        println!("run `vibe storage gc --delete` to remove them");
    }
}
