//! `vibe prefs set <key> <value> [--layer]` — write one key to a layer.

specmark::scope!("spec://vibevm/modules/vibe-settings/PROP-040#prefs-command");

use anyhow::Result;
use serde::Serialize;
use vibe_settings::cli::{PrefsOp, PrefsOutcome, run_prefs};

use crate::cli::PrefsSetArgs;
use crate::commands::prefs::{
    Loaded, display_value, load, parse_layer, parse_value, persist_layer, resolve_repo,
    warn_load_warnings,
};
use crate::output;

#[derive(Serialize)]
struct SetReport {
    ok: bool,
    command: &'static str,
    key: String,
    value: serde_json::Value,
    layer: String,
    path: String,
}

pub fn run(ctx: &output::Context, args: PrefsSetArgs) -> Result<()> {
    let repo = resolve_repo(&args.path);
    let layer = parse_layer(args.layer.as_deref().unwrap_or("L3"))?;
    let value = parse_value(&args.value)?;
    let Loaded {
        paths,
        raw,
        schema,
        warnings,
        ..
    } = load(&repo)?;
    warn_load_warnings(&warnings);

    let empty = toml::Table::new();
    // A declared key written to a forbidden layer yields `PrefsError::WrongLayer`
    // (thiserror, cites PROP-040 §7 #scope-matrix) — `?` propagates it to the
    // command edge with that message intact.
    let outcome = run_prefs(
        PrefsOp::Set {
            key: &args.key,
            value: value.clone(),
            layer,
        },
        &schema,
        &raw,
        &empty,
        &empty,
    )?;
    let table = match outcome {
        PrefsOutcome::LayerWritten { table, .. } => table,
        _ => unreachable!("run_prefs(Set) returns LayerWritten"),
    };

    let path = paths.for_layer(layer).to_path_buf();
    persist_layer(&path, &table, layer)?;

    if ctx.is_json() {
        ctx.emit_json(&SetReport {
            ok: true,
            command: "prefs:set",
            key: args.key.clone(),
            value: serde_json::to_value(&value)?,
            layer: layer.label().to_string(),
            path: path.display().to_string(),
        })?;
        return Ok(());
    }
    let msg = format!(
        "set {} = {} in {} ({})",
        args.key,
        display_value(&value),
        layer,
        path.display()
    );
    if ctx.is_quiet() {
        ctx.summary(&msg);
    } else {
        println!("{msg}");
    }
    Ok(())
}
