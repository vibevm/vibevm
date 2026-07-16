//! `vibe prefs migrate` — rewrite deprecated keys to their replacements.

specmark::scope!("spec://vibevm/modules/vibe-settings/PROP-040#prefs-command");

use anyhow::Result;
use serde::Serialize;
use vibe_settings::cli::{PrefsOp, PrefsOutcome, run_prefs};

use crate::cli::PrefsPathArgs;
use crate::commands::prefs::{Loaded, load, persist_layer, resolve_repo, warn_load_warnings};
use crate::output;

#[derive(Serialize)]
struct MigrateLayer {
    layer: String,
    path: String,
    rewrote: Vec<String>,
}

#[derive(Serialize)]
struct MigrateReport {
    ok: bool,
    command: &'static str,
    layers: Vec<MigrateLayer>,
}

pub fn run(ctx: &output::Context, args: PrefsPathArgs) -> Result<()> {
    let repo = resolve_repo(&args.path);
    let Loaded {
        paths,
        raw,
        schema,
        warnings,
        ..
    } = load(&repo)?;
    warn_load_warnings(&warnings);

    let empty = toml::Table::new();
    let migrated = match run_prefs(PrefsOp::Migrate, &schema, &raw, &empty, &empty)? {
        PrefsOutcome::Migrated(m) => m,
        _ => unreachable!("run_prefs(Migrate) returns Migrated"),
    };

    // Persist each rewritten layer (basic write; phase 2.7 comment-preserves).
    let mut report_layers = Vec::new();
    for entry in &migrated {
        let path = paths.for_layer(entry.layer).to_path_buf();
        persist_layer(&path, &entry.table, entry.layer)?;
        report_layers.push(MigrateLayer {
            layer: entry.layer.label().to_string(),
            path: path.display().to_string(),
            rewrote: entry.rewrote.clone(),
        });
    }

    if ctx.is_json() {
        ctx.emit_json(&MigrateReport {
            ok: true,
            command: "prefs:migrate",
            layers: report_layers,
        })?;
        return Ok(());
    }

    if migrated.is_empty() {
        if ctx.is_quiet() {
            ctx.summary("nothing to migrate");
        } else {
            println!("nothing to migrate (no deprecated keys in use)");
        }
        return Ok(());
    }
    if ctx.is_quiet() {
        let total: usize = migrated.iter().map(|m| m.rewrote.len()).sum();
        ctx.summary(&format!(
            "migrated {total} key(s) across {} layer(s)",
            migrated.len()
        ));
        return Ok(());
    }
    for entry in &migrated {
        let path = paths.for_layer(entry.layer);
        println!("{} ({}):", entry.layer, path.display());
        for line in &entry.rewrote {
            println!("  - {line}");
        }
    }
    Ok(())
}
