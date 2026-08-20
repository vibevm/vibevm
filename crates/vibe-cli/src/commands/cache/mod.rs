//! `vibe cache …` — the operator surface over the machine-global
//! package store `~/.vibe/cache/` (PROP-010 §2.8). The family is
//! top-level on the owner's ruling NAMESPACE-IS-TOP-LEVEL-VIBE-CACHE:
//! the store is machine-global and its headline case is work that has
//! no project yet, so its commands must not hang off a project-scoped
//! family. `path` / `list` are read-only walks of the store's own
//! directory tree (the layout IS the index — no side-car state); `add`
//! and `clean` live in their own modules along the family's seams.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#command-summary");

mod add;
mod clean;

use anyhow::{Context, Result};

use crate::cli::{CacheArgs, CacheSubcommand};
use crate::output;

pub(crate) fn run(ctx: &output::Context, args: CacheArgs, root_offline: bool) -> Result<()> {
    match args.command {
        CacheSubcommand::Path => run_path(ctx),
        CacheSubcommand::List => run_list(ctx),
        CacheSubcommand::Add(args) => add::run(ctx, args, root_offline),
        CacheSubcommand::Clean(args) => clean::run(ctx, args),
    }
}

/// `vibe cache path` — print the store root. Works anywhere: the root
/// is resolved through the settings chokepoint alone, and an
/// unresolvable home is the only failure.
fn run_path(ctx: &output::Context) -> Result<()> {
    let root = vibe_registry::store_root().context("resolving the machine store root")?;
    if ctx.is_json() {
        ctx.emit_json(&serde_json::json!({
            "ok": true,
            "command": "cache:path",
            "root": root.display().to_string(),
        }))?;
        return Ok(());
    }
    // One line, the path — `--quiet` and human agree here because the
    // answer already is a single summary line.
    ctx.summary(&root.display().to_string());
    Ok(())
}

/// `vibe cache list` — the offline-resolvable inventory, straight off
/// the `list_all` walk (sorting is the API's). An empty store is not
/// an error: it is the honest answer.
fn run_list(ctx: &output::Context) -> Result<()> {
    let root = vibe_registry::store_root().context("resolving the machine store root")?;
    let entries = vibe_registry::list_all();

    if ctx.is_json() {
        let packages: Vec<serde_json::Value> = entries
            .iter()
            .map(|(group, name, version)| {
                serde_json::json!({
                    "group": group.as_str(),
                    "name": name,
                    "version": version.to_string(),
                })
            })
            .collect();
        ctx.emit_json(&serde_json::json!({
            "ok": true,
            "command": "cache:list",
            "root": root.display().to_string(),
            "count": packages.len(),
            "packages": packages,
        }))?;
        return Ok(());
    }

    if entries.is_empty() {
        ctx.summary(&format!("store is empty ({})", root.display()));
        return Ok(());
    }

    if ctx.is_quiet() {
        for (group, name, version) in &entries {
            ctx.summary(&format!("{group}/{name}@{version}"));
        }
        return Ok(());
    }

    // Table, the `vibe list` shape.
    let mut g_w = "GROUP".len();
    let mut n_w = "NAME".len();
    let mut v_w = "VERSION".len();
    for (group, name, version) in &entries {
        g_w = g_w.max(group.as_str().len());
        n_w = n_w.max(name.len());
        v_w = v_w.max(version.to_string().len());
    }
    println!("{:<g_w$}  {:<n_w$}  {:<v_w$}", "GROUP", "NAME", "VERSION");
    for (group, name, version) in &entries {
        println!(
            "{:<g_w$}  {:<n_w$}  {:<v_w$}",
            group.as_str(),
            name,
            version.to_string()
        );
    }
    println!(
        "\n{} package{} in {}.",
        entries.len(),
        if entries.len() == 1 { "" } else { "s" },
        root.display()
    );
    Ok(())
}
