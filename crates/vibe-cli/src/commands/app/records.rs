//! `vibe app runs` (PROP-065 `#runs`) and `vibe app errors`
//! (`#error-codes`, `#exit-codes`): the run records of a package and the
//! table of stable error codes with their exit status.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#runs");

use anyhow::Result;
use vibe_app::SettingsRoot;
use vibe_wire::generated::application_errors::{ApplicationErrors, Code};

use crate::exit_code::app_exit_code;
use crate::output;

/// List (and with `prune`, first prune) the run records of `package`.
pub(super) fn runs(
    ctx: &output::Context,
    settings: &SettingsRoot,
    package: &str,
    prune: Option<usize>,
) -> Result<()> {
    let listing = match prune {
        Some(keep) => vibe_app::prune_runs(settings, package, keep)?,
        None => vibe_app::runs(settings, package)?,
    };
    if ctx.is_json() {
        return ctx.emit_json(&listing);
    }
    if !listing.pruned.is_empty() {
        ctx.summary(&format!("pruned {} run record(s)", listing.pruned.len()));
    }
    ctx.summary(&format!(
        "{}: {} run record(s)",
        package,
        listing.runs.len()
    ));
    if ctx.is_quiet() {
        return Ok(());
    }
    for run in &listing.runs {
        println!(
            "  {} {} {} ({}) {}",
            run.timestamp, run.command, run.key, run.outcome, run.path
        );
        for log in &run.logs {
            println!("    {}: {}", log.step, log.path);
        }
    }
    Ok(())
}

/// The error-code table: code, enum, meaning and exit status (0 success,
/// 1 a failed or interrupted step, 2 a refusal before any effect).
pub(super) fn errors(ctx: &output::Context) -> Result<()> {
    let document = ApplicationErrors {
        protocol: "vibe-app-errors/1".into(),
        codes: vibe_app::error_codes()
            .into_iter()
            .map(|row| Code {
                code: row.code.into(),
                enum_: row.family.into(),
                meaning: row.meaning.into(),
                exit: app_exit_code(row.class),
            })
            .collect(),
    };
    if ctx.is_json() {
        return ctx.emit_json(&document);
    }
    ctx.summary(&format!(
        "{} error code(s); exit status 0 success, 1 a failed or interrupted step, 2 a refusal before any effect",
        document.codes.len()
    ));
    if ctx.is_quiet() {
        return Ok(());
    }
    for code in &document.codes {
        println!(
            "  {} [{}] exit {} — {}",
            code.code, code.enum_, code.exit, code.meaning
        );
    }
    Ok(())
}
