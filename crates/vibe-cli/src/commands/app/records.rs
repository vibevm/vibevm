//! `vibe app runs` (PROP-065 `#runs`), `vibe app work` (`#work`) and
//! `vibe app errors` (`#error-codes`, `#exit-codes`): the run records of a
//! package, the kept work directories of the home and the table of stable
//! error codes with their exit status.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#runs");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#work");

use anyhow::Result;
use vibe_app::{SettingsRoot, human_bytes};
use vibe_wire::generated::application_errors::{ApplicationErrors, Code};
use vibe_wire::generated::application_work::{ApplicationWork, Entry, EntryState};

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

/// List the kept work directories of the home, or with `clean` delete the
/// ones `package` selects (every one of the home when it names none).
#[specmark::spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-065#work")]
pub(super) fn work(
    ctx: &output::Context,
    settings: &SettingsRoot,
    package: Option<&str>,
    clean: bool,
) -> Result<()> {
    let listing = if clean {
        vibe_app::clean_work(settings, package)?
    } else {
        vibe_app::work(settings, package)?
    };
    if ctx.is_json() {
        return ctx.emit_json(&listing);
    }
    if clean {
        return removed(ctx, &listing);
    }
    if listing.entries.is_empty() {
        ctx.summary(&format!("no kept work directories in {}", listing.home));
        return Ok(());
    }
    ctx.summary(&format!(
        "{} kept work directory(ies) in {}",
        listing.entries.len(),
        listing.home
    ));
    if ctx.is_quiet() {
        return Ok(());
    }
    for entry in &listing.entries {
        println!("  {}", entry.key);
        let field = |name: &str, value: &str| println!("    {name:<8} {value}");
        field("state", &state_text(entry));
        field("changed", &entry.changed);
        field("size", &size_text(entry));
        field("path", &entry.path);
    }
    Ok(())
}

/// What a `--clean` deleted, directory by directory.
fn removed(ctx: &output::Context, listing: &ApplicationWork) -> Result<()> {
    for entry in &listing.entries {
        ctx.summary(&format!(
            "removed {} ({})",
            entry.path,
            human_bytes(entry.bytes)
        ));
    }
    ctx.summary(&format!("freed {}", human_bytes(listing.freed)));
    Ok(())
}

/// How far the kept run got, with the step responsible when one is.
fn state_text(entry: &Entry) -> String {
    let state = match entry.state {
        EntryState::Done => "done",
        EntryState::Failed => "failed",
        EntryState::Interrupted => "interrupted",
        EntryState::Delegated => "delegated",
        EntryState::Unknown => "unknown",
    };
    match &entry.step {
        Some(step) => format!("{state} at step `{step}`"),
        None => state.to_owned(),
    }
}

fn size_text(entry: &Entry) -> String {
    format!(
        "{} (sources {}, out {}, inputs {}, other {})",
        human_bytes(entry.bytes),
        human_bytes(entry.parts.sources),
        human_bytes(entry.parts.out),
        human_bytes(entry.parts.inputs),
        human_bytes(entry.parts.other)
    )
}

/// The error-code table: code, enum, meaning and exit status (0 success,
/// 1 a failed or interrupted step, 2 a refusal before any effect, 75 a run
/// parked for a hosting agent).
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
        "{} error code(s); exit status 0 success, 1 a failed or interrupted step, 2 a refusal before any effect, \
         75 a run parked for a hosting agent",
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
