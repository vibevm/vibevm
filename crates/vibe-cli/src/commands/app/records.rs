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
use crate::output::language::{Cell, List, Meta, Role, Row};

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
    // One row per run, newest first as the library lists them, each step
    // log a child under it (PROP-067 `#list`, `#children`).
    let rows = listing
        .runs
        .iter()
        .map(|run| {
            let mut row = Row::new(vec![
                Cell::new(run.timestamp.clone(), Role::Dim),
                Cell::text(run.command.clone()),
                Cell::new(run.key.clone(), Role::Accent),
                Cell::new(format!("({})", run.outcome), Role::Dim),
            ])
            .meta(vec![Meta::Note(run.path.clone())]);
            for log in &run.logs {
                row = row.child(format!("{}  {}", log.step, log.path));
            }
            row
        })
        .collect();
    ctx.block(&List::new(rows).children_under(1).render());
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
    import: Option<&str>,
) -> Result<()> {
    let listing = if let Some(id) = import {
        if clean {
            vibe_app::clean_import(settings, id)?
        } else {
            vibe_app::work_import(settings, id)?
        }
    } else if clean {
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
    if listing.entries.is_empty() && listing.imports.is_empty() {
        ctx.summary(&format!("no kept work directories in {}", listing.home));
        return Ok(());
    }
    ctx.summary(&format!(
        "{} kept work directory(ies) in {}",
        listing.entries.len() + listing.imports.len(),
        listing.home
    ));
    if ctx.is_quiet() {
        return Ok(());
    }
    // One row per kept directory: its key, then how far the run got, when
    // it changed, its size and where it is (PROP-067 `#list`, `#meta`).
    let mut rows: Vec<Row> = listing
        .entries
        .iter()
        .map(|entry| {
            Row::new(vec![
                Cell::new(entry.key.clone(), Role::Accent),
                Cell::text(state_text(entry)),
            ])
            .meta(vec![
                Meta::Note(entry.changed.clone()),
                Meta::Count(size_text(entry)),
                Meta::Note(entry.path.clone()),
            ])
        })
        .collect();
    for cohort in &listing.imports {
        let phase = match cohort.phase {
            vibe_wire::generated::shared::ApplicationImportPhase::Running => "running",
            vibe_wire::generated::shared::ApplicationImportPhase::Arrangement => "arrangement",
            vibe_wire::generated::shared::ApplicationImportPhase::Complete => "complete",
        };
        rows.push(
            Row::new(vec![
                Cell::new(format!("import {}", cohort.target.operation), Role::Accent),
                Cell::text(format!(
                    "{phase}; {} / {} rows; continue: vibe app import --resume {}",
                    cohort.cursor, cohort.total, cohort.target.operation
                )),
            ])
            .meta(vec![
                Meta::Note(cohort.changed.clone()),
                Meta::Count(human_bytes(cohort.bytes)),
                Meta::Note(cohort.path.clone()),
            ]),
        );
    }
    ctx.block(&List::new(rows).render());
    Ok(())
}

/// What a `--clean` deleted, directory by directory.
fn removed(ctx: &output::Context, listing: &ApplicationWork) -> Result<()> {
    for path in &listing.removed {
        ctx.summary(&format!("removed {path}"));
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
    let mut text = match &entry.step {
        Some(step) => format!("{state} at step {step}"),
        None => state.to_owned(),
    };
    if let Some(operation) = &entry.operation {
        use vibe_wire::generated::shared::ApplicationPendingOperationPhase as P;
        let phase = match operation.phase {
            P::Prepared => "prepared",
            P::Dependencies => "dependencies",
            P::Removing => "removing",
            P::Removed => "removed",
            P::Running => "running",
            P::Placing => "placing",
            P::Placed => "placed",
            P::Publishing => "publishing",
            P::Published => "published",
            P::Complete => "complete",
            P::Retiring => "retiring",
        };
        text.push_str(&format!(
            "; operation {} ({phase})",
            operation.target.operation
        ));
        if operation.phase != P::Complete {
            text.push_str(&format!(
                "; continue: {}",
                vibe_app::resume_command(&operation.target)
            ));
        }
    }
    if let Some(parent) = &entry.retained_for {
        match &parent.owner {
            vibe_wire::generated::shared::ApplicationContinuationTarget::Application(owner) => text
                .push_str(&format!(
                    "; retained for parent {} (operation {})",
                    owner.target.key, owner.target.operation
                )),
            vibe_wire::generated::shared::ApplicationContinuationTarget::Import(owner) => text
                .push_str(&format!(
                    "; retained for import {} row {}",
                    owner.target.operation, owner.row
                )),
        }
    }
    text
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
