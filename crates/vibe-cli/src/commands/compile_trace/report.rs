//! The one conversion from a live recorder's summary to the shared generated
//! trace member — pure, total, and incapable of failing the command.
//!
//! Four properties are the whole cell:
//!
//! * **`finalised` means DURABLE.** The writer already decides that from the
//!   disk, not from whether its publication call returned `Ok`. So a terminal
//!   index whose bytes provably never landed arrives here still `running`, and
//!   is reported `running` with `finalised = false`. Saying `ok` because the
//!   command succeeded would be the one lie a cold reader cannot detect: their
//!   copy of the index says `running` and always will.
//! * **Counts are lossless.** JTD has no `uint64`, so each count rides a
//!   canonical decimal string. `u64::to_string` IS that spelling — no
//!   narrowing, no locale, and no checked conversion that could turn an
//!   observer into a veto.
//! * **Warnings are clamped AFTER formatting, and EXACTLY once.** Every field
//!   inside a `TraceWarning` is already bounded, which is why the naive version
//!   is wrong: `Display` adds a prefix, and a bounded field plus a prefix is
//!   over the cap the epoch's validator enforces. So a raw warning is rendered
//!   straight into the writer's streaming formatter — no intermediate `String`,
//!   one definition of the cap. A [`BoundedDiagnostic`] arriving from the owner
//!   already carries that proof in its type and is cloned, never re-measured; a
//!   second pass would be a no-op that hides which pass is the real defence.
//! * **A refusal is a notice, never a panic.** If the assembled member breaks
//!   its own relational law, it is omitted and the reason is reported. It does
//!   not fail the command, does not touch emission policy, and does not
//!   replace the caller's error.

use vibe_core::machine_json_path;
use vibe_wire::behaviour::compile_trace_report::validate;
use vibe_wire::generated::compiler_trace_index::e1::index::RunStatus;
use vibe_wire::generated::shared::{CompileTraceReport, TraceReportStatus};
use vibe_workspace::compile_trace::{TraceSummary, TraceWarning};

use super::BoundedDiagnostic;

/// The member for a trace that was requested and never opened.
///
/// Zero counts, no path, no timings, and at least one nonblank reason: the
/// epoch refuses a silent `unavailable`, because a record that says only
/// "there is no trace" and not why is a record that reads like a bug.
pub(super) fn unavailable(
    run_id: &str,
    reasons: &[BoundedDiagnostic],
    notices: Vec<BoundedDiagnostic>,
) -> (Option<CompileTraceReport>, Vec<BoundedDiagnostic>) {
    let report = CompileTraceReport {
        budget_exhausted: false,
        events: "0".to_string(),
        finalised: false,
        run_id: run_id.to_string(),
        snapshot_bytes: "0".to_string(),
        snapshots: "0".to_string(),
        status: TraceReportStatus::Unavailable,
        timings: Vec::new(),
        // Nothing raw on this arm: both lists are already bounded by type.
        warnings: member_warnings(&[], reasons.iter().chain(notices.iter())),
        run_path: None,
    };
    validated(report, notices)
}

/// The member for a trace that really opened, in whatever state the writer
/// left it.
pub(super) fn from_summary(
    run_id: &str,
    summary: &TraceSummary,
    notices: Vec<BoundedDiagnostic>,
) -> (Option<CompileTraceReport>, Vec<BoundedDiagnostic>) {
    // Only a status the writer PROVED durable is reported terminal.
    //
    // A terminal word that never landed normalises to running/false, and that
    // is not a repair: the writer already restored its own root to `running`
    // when the publication was refused, so the disk and the report agree.
    //
    // `running + finalised` is the one shape that is IMPOSSIBLE, and it is
    // deliberately NOT normalised. Quietly rewriting it would turn a defect in
    // this crate into a well-formed member that says something false and no
    // one ever sees. Carried through as it is, the shared validator refuses it
    // on `status-matrix`, the member is omitted, and the notice names the law.
    let (status, finalised) = match (&summary.status, summary.finalised) {
        (RunStatus::Ok, true) => (TraceReportStatus::Ok, true),
        (RunStatus::Failed, true) => (TraceReportStatus::Failed, true),
        (RunStatus::Ok | RunStatus::Failed, false) => (TraceReportStatus::Running, false),
        (RunStatus::Running, finalised) => (TraceReportStatus::Running, finalised),
    };
    let report = CompileTraceReport {
        budget_exhausted: summary.budget_exhausted,
        events: summary.events.to_string(),
        finalised,
        run_id: run_id.to_string(),
        snapshot_bytes: summary.snapshot_bytes.to_string(),
        snapshots: summary.snapshots.to_string(),
        status,
        // The index's OWN aggregate rows, carried verbatim through the one
        // shared generated type. Nothing here recomputes a total, and no
        // per-module row type exists to convert between.
        timings: summary.aggregates.clone(),
        warnings: member_warnings(&summary.warnings, notices.iter()),
        run_path: Some(machine_json_path(&summary.run_dir)),
    };
    validated(report, notices)
}

/// The member's warning list: the observer's own messages, then the owner's
/// already-bounded notices.
///
/// The split is the law. `observed` is RAW — a [`TraceWarning`] whose fields
/// the writer bounded individually, but whose `Display` adds a prefix that can
/// push the finished message back over the epoch's cap — so it is rendered
/// straight into the one clamp, with no intermediate `String` in between.
/// `bounded` is already proved, carries that proof in its type, and is
/// therefore CLONED rather than measured a second time.
///
/// A single implementation over `Display` cannot express this: it would either
/// under-clamp the raw half or re-clamp the proved half, and the re-clamp is
/// precisely what makes the real clamp untestable.
fn member_warnings<'a>(
    observed: &[TraceWarning],
    bounded: impl Iterator<Item = &'a BoundedDiagnostic>,
) -> Vec<String> {
    let mut all: Vec<BoundedDiagnostic> = observed
        .iter()
        .map(|warning| BoundedDiagnostic::new(format_args!("{warning}")))
        .collect();
    all.extend(bounded.cloned());
    all.into_iter()
        .map(BoundedDiagnostic::into_string)
        .collect()
}

/// Run the shared behaviour validator exactly once.
///
/// A refusal here is a programming fault in this cell, not a user's problem:
/// the member is dropped, the fault is reported as a secondary notice, and
/// every other decision the funnel already made stands.
fn validated(
    report: CompileTraceReport,
    mut notices: Vec<BoundedDiagnostic>,
) -> (Option<CompileTraceReport>, Vec<BoundedDiagnostic>) {
    match validate(&report) {
        Ok(()) => (Some(report), notices),
        Err(error) => {
            notices.push(BoundedDiagnostic::new(format_args!(
                "the compile-trace report for run `{}` broke its own `{}` law and was omitted \
                 from this command's output: {error}",
                report.run_id,
                error.law()
            )));
            (None, notices)
        }
    }
}
