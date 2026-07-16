//! `vibe prefs get <key>` — the resolved value of one key + which layer set it.

specmark::scope!("spec://vibevm/modules/vibe-settings/PROP-040#prefs-command");

use anyhow::Result;
use serde::Serialize;
use vibe_settings::cli::{PrefsOp, PrefsOutcome, run_prefs};

use crate::cli::PrefsGetArgs;
use crate::commands::prefs::{
    Loaded, display_value, load, resolve_repo, value_to_json, warn_load_warnings,
};
use crate::output;

#[derive(Serialize)]
struct LayerValue {
    default: Option<serde_json::Value>,
    l1: Option<serde_json::Value>,
    l2: Option<serde_json::Value>,
    l3: Option<serde_json::Value>,
    cli: Option<serde_json::Value>,
    env: Option<serde_json::Value>,
}

#[derive(Serialize)]
struct GetReport {
    ok: bool,
    command: &'static str,
    key: String,
    set: bool,
    value: Option<serde_json::Value>,
    origin: Option<String>,
    layers: LayerValue,
}

pub fn run(ctx: &output::Context, args: PrefsGetArgs) -> Result<()> {
    let repo = resolve_repo(&args.path);
    let Loaded {
        raw,
        schema,
        warnings,
        ..
    } = load(&repo)?;
    warn_load_warnings(&warnings);

    let empty = toml::Table::new();
    let outcome = run_prefs(
        PrefsOp::Get { key: &args.key },
        &schema,
        &raw,
        &empty,
        &empty,
    )?;
    let Some(iv) = (match outcome {
        PrefsOutcome::Value(iv) => iv,
        _ => unreachable!("run_prefs(Get) returns Value"),
    }) else {
        // Absent everywhere.
        if ctx.is_json() {
            ctx.emit_json(&GetReport {
                ok: true,
                command: "prefs:get",
                key: args.key.clone(),
                set: false,
                value: None,
                origin: None,
                layers: LayerValue {
                    default: None,
                    l1: None,
                    l2: None,
                    l3: None,
                    cli: None,
                    env: None,
                },
            })?;
        } else if ctx.is_quiet() {
            ctx.summary(&format!("{} = (unset)", args.key));
        } else {
            println!("{} = (unset; no value in any layer or default)", args.key);
        }
        return Ok(());
    };

    let origin = iv.origin.label().to_string();
    if ctx.is_json() {
        let report = GetReport {
            ok: true,
            command: "prefs:get",
            key: args.key.clone(),
            set: true,
            value: Some(value_to_json(&iv.value)?),
            origin: Some(origin.clone()),
            layers: LayerValue {
                default: iv.default.as_ref().map(value_to_json).transpose()?,
                l1: iv.l1.as_ref().map(value_to_json).transpose()?,
                l2: iv.l2.as_ref().map(value_to_json).transpose()?,
                l3: iv.l3.as_ref().map(value_to_json).transpose()?,
                cli: iv.cli.as_ref().map(value_to_json).transpose()?,
                env: iv.env.as_ref().map(value_to_json).transpose()?,
            },
        };
        ctx.emit_json(&report)?;
        return Ok(());
    }

    let value_str = display_value(&iv.value);
    if ctx.is_quiet() {
        ctx.summary(&format!("{} = {} [{}]", args.key, value_str, origin));
        return Ok(());
    }
    println!("{} = {}   [{}]", args.key, value_str, origin);
    println!("  default: {}", layer_display(iv.default.as_ref()));
    println!("  L1:      {}", layer_display(iv.l1.as_ref()));
    println!("  L2:      {}", layer_display(iv.l2.as_ref()));
    println!("  L3:      {}", layer_display(iv.l3.as_ref()));
    Ok(())
}

/// Render an `Option<&Value>` as its display form or `(unset)`.
fn layer_display(v: Option<&toml::Value>) -> String {
    match v {
        Some(v) => display_value(v),
        None => "(unset)".to_string(),
    }
}
