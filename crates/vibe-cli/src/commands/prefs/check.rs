//! `vibe prefs check` — validate every layer against the schema.

specmark::scope!("spec://vibevm/modules/vibe-settings/PROP-040#prefs-command");

use anyhow::Result;
use serde::Serialize;
use vibe_settings::cli::{PrefsOp, PrefsOutcome, run_prefs};

use crate::cli::PrefsPathArgs;
use crate::commands::prefs::{Loaded, load, resolve_repo};
use crate::output;

#[derive(Serialize)]
struct CheckReport {
    ok: bool,
    command: &'static str,
    count: usize,
    diagnostics: Vec<String>,
}

pub fn run(ctx: &output::Context, args: PrefsPathArgs) -> Result<()> {
    let repo = resolve_repo(&args.path);
    let Loaded {
        raw,
        schema,
        mut warnings,
        ..
    } = load(&repo)?;

    let empty = toml::Table::new();
    let mut diagnostics = match run_prefs(PrefsOp::Check, &schema, &raw, &empty, &empty)? {
        PrefsOutcome::Diagnostics(d) => d,
        _ => unreachable!("run_prefs(Check) returns Diagnostics"),
    };
    // Surface load/parse failures alongside schema diagnostics.
    diagnostics.append(&mut warnings);

    if ctx.is_json() {
        ctx.emit_json(&CheckReport {
            ok: diagnostics.is_empty(),
            command: "prefs:check",
            count: diagnostics.len(),
            diagnostics,
        })?;
        return Ok(());
    }

    if diagnostics.is_empty() {
        if ctx.is_quiet() {
            ctx.summary("ok: 0 diagnostics");
        } else {
            println!("ok: no diagnostics (all layers validate)");
        }
        return Ok(());
    }
    if ctx.is_quiet() {
        ctx.summary(&format!("{} diagnostic(s)", diagnostics.len()));
        return Ok(());
    }
    println!("{} diagnostic(s):", diagnostics.len());
    for d in &diagnostics {
        println!("  - {d}");
    }
    Ok(())
}
