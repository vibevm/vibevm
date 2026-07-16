//! `vibe prefs list` — every resolved key with its value and origin.

specmark::scope!("spec://vibevm/modules/vibe-settings/PROP-040#prefs-command");

use anyhow::Result;
use serde::Serialize;
use vibe_settings::cli::{PrefsOp, PrefsOutcome, run_prefs};

use crate::cli::PrefsPathArgs;
use crate::commands::prefs::{
    Loaded, display_value, load, resolve_repo, value_to_json, warn_load_warnings,
};
use crate::output;

#[derive(Serialize)]
struct KeyRow {
    key: String,
    value: serde_json::Value,
    origin: String,
}

#[derive(Serialize)]
struct ListReport {
    ok: bool,
    command: &'static str,
    count: usize,
    keys: Vec<KeyRow>,
}

pub fn run(ctx: &output::Context, args: PrefsPathArgs) -> Result<()> {
    let repo = resolve_repo(&args.path);
    let Loaded {
        raw,
        schema,
        warnings,
        ..
    } = load(&repo)?;
    warn_load_warnings(&warnings);

    let empty = toml::Table::new();
    let keys = match run_prefs(PrefsOp::List, &schema, &raw, &empty, &empty)? {
        PrefsOutcome::Keys(k) => k,
        _ => unreachable!("run_prefs(List) returns Keys"),
    };

    if ctx.is_json() {
        let rows: Vec<KeyRow> = keys
            .iter()
            .map(|k| {
                Ok::<_, anyhow::Error>(KeyRow {
                    key: k.path.clone(),
                    value: value_to_json(&k.value)?,
                    origin: k.origin.label().to_string(),
                })
            })
            .collect::<Result<_>>()?;
        ctx.emit_json(&ListReport {
            ok: true,
            command: "prefs:list",
            count: rows.len(),
            keys: rows,
        })?;
        return Ok(());
    }

    if keys.is_empty() {
        if !ctx.is_quiet() {
            println!("(no preferences set)");
        }
        ctx.summary("0 keys");
        return Ok(());
    }
    if ctx.is_quiet() {
        ctx.summary(&format!("{} keys", keys.len()));
        return Ok(());
    }
    for k in &keys {
        println!("{} = {}   [{}]", k.path, display_value(&k.value), k.origin);
    }
    Ok(())
}
