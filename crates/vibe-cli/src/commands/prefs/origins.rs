//! `vibe prefs show-origins [key]` — the full per-layer breakdown.

specmark::scope!("spec://vibevm/modules/vibe-settings/PROP-040#show-origins");

use anyhow::Result;
use serde::Serialize;
use vibe_settings::cli::{OriginEntry, PrefsOp, PrefsOutcome, run_prefs};
use vibe_settings::resolver::InspectValue;

use crate::cli::PrefsOriginsArgs;
use crate::commands::prefs::{
    Loaded, display_value, load, resolve_repo, value_to_json, warn_load_warnings,
};
use crate::output;

#[derive(Serialize)]
struct OriginLayer {
    default: Option<serde_json::Value>,
    l1: Option<serde_json::Value>,
    l2: Option<serde_json::Value>,
    l3: Option<serde_json::Value>,
    cli: Option<serde_json::Value>,
    env: Option<serde_json::Value>,
}

#[derive(Serialize)]
struct OriginRow {
    key: String,
    value: serde_json::Value,
    origin: String,
    layers: OriginLayer,
}

#[derive(Serialize)]
struct OriginsReport {
    ok: bool,
    command: &'static str,
    count: usize,
    keys: Vec<OriginRow>,
}

pub fn run(ctx: &output::Context, args: PrefsOriginsArgs) -> Result<()> {
    let repo = resolve_repo(&args.path);
    let Loaded {
        raw,
        schema,
        warnings,
        ..
    } = load(&repo)?;
    warn_load_warnings(&warnings);

    let empty = toml::Table::new();
    let key_ref = args.key.as_deref();
    let origins = match run_prefs(
        PrefsOp::ShowOrigins { key: key_ref },
        &schema,
        &raw,
        &empty,
        &empty,
    )? {
        PrefsOutcome::Origins(o) => o,
        _ => unreachable!("run_prefs(ShowOrigins) returns Origins"),
    };

    if ctx.is_json() {
        let rows = origins
            .iter()
            .map(entry_to_row)
            .collect::<Result<Vec<_>>>()?;
        ctx.emit_json(&OriginsReport {
            ok: true,
            command: "prefs:show-origins",
            count: rows.len(),
            keys: rows,
        })?;
        return Ok(());
    }

    if origins.is_empty() {
        match key_ref {
            Some(k) => println!("{k} = (unset; no value in any layer or default)"),
            None => println!("(no preferences set)"),
        }
        ctx.summary("0 keys");
        return Ok(());
    }
    if ctx.is_quiet() {
        ctx.summary(&format!("{} keys", origins.len()));
        return Ok(());
    }
    for entry in &origins {
        print_breakdown(&entry.path, &entry.value);
    }
    Ok(())
}

fn entry_to_row(entry: &OriginEntry) -> Result<OriginRow> {
    let iv = &entry.value;
    Ok(OriginRow {
        key: entry.path.clone(),
        value: value_to_json(&iv.value)?,
        origin: iv.origin.label().to_string(),
        layers: OriginLayer {
            default: iv.default.as_ref().map(value_to_json).transpose()?,
            l1: iv.l1.as_ref().map(value_to_json).transpose()?,
            l2: iv.l2.as_ref().map(value_to_json).transpose()?,
            l3: iv.l3.as_ref().map(value_to_json).transpose()?,
            cli: iv.cli.as_ref().map(value_to_json).transpose()?,
            env: iv.env.as_ref().map(value_to_json).transpose()?,
        },
    })
}

fn print_breakdown(path: &str, iv: &InspectValue) {
    println!(
        "{path} = {}   [origin: {}]",
        display_value(&iv.value),
        iv.origin
    );
    println!("  default: {}", opt_display(iv.default.as_ref()));
    println!("  L1:      {}", opt_display(iv.l1.as_ref()));
    println!("  L2:      {}", opt_display(iv.l2.as_ref()));
    println!("  L3:      {}", opt_display(iv.l3.as_ref()));
    println!();
}

fn opt_display(v: Option<&toml::Value>) -> String {
    match v {
        Some(v) => display_value(v),
        None => "(unset)".to_string(),
    }
}
