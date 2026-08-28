//! The ONE command-level owner of a compile trace, and the ONE join between a
//! command's outcome and its report.
//!
//! Everything below this cell borrows `Option<&TraceRun>`. Nothing below it
//! opens a recorder, finishes one, clones one into an outcome, or keeps one
//! past the command — because a recorder holds the project's cooperative lock,
//! and a second owner of that lock is a second answer to "is this workspace
//! being traced right now". So the owner is created here, once, after the
//! lifecycle identity is selected and before the first compile, and it is
//! CONSUMED by exactly one typed exit.
//!
//! ## The three states, and why `unavailable` is one of them
//!
//! ```text
//! disabled                                  — nothing was requested
//! unavailable { run_id, reasons }           — requested, and no recorder opened
//! open        { run_id, run }               — requested, and one did
//! ```
//!
//! `unavailable` exists because the lifecycle's sticky trace bit proves a
//! *request*, not a recorder. A busy project, a directory too deep to hold a
//! snapshot name, a resume whose original invocation never opened — each of
//! them compiles untraced, and each of them must SAY so in the report rather
//! than look like a run that simply had nothing to record. That is also why
//! the state is not `Option<TraceRun>`: an `Option` cannot tell "off" from
//! "asked for and unavailable", and those two owe the operator different words.
//!
//! ## What never happens here
//!
//! **The command's error is never persisted.** A failed invocation finishes
//! its trace with the fixed word `command failed` and nothing else. An
//! `anyhow::Error` from a real command can carry a provider's response body, a
//! script's captured stderr, or a token that was on a command line; byte-
//! bounding such a string is not redaction, and a diagnostic file that quotes
//! it is a diagnostic file that leaks it. The funnel therefore only ever
//! *borrows* the error while finalising, and hands the caller back the very
//! same object — same downcast identity, same chain.
//!
//! **Nothing here reads a clock.** Both instants are injected: one for
//! terminalising a displaced predecessor, one for the finish. A `Drop` that
//! invented a timestamp or performed I/O would make the trace's own record of
//! time depend on when a value happened to fall out of scope, so there is no
//! such `Drop` — releasing the lock is all dropping does. The one `chrono` call
//! in this cell is a PARSER: the lifecycle's own recorded RFC 3339 start,
//! spelled as the trace epoch's instant.
//!
//! **Nothing here renders or serialises.** The funnel returns a
//! [`FinalizedCommand`], and the surface's report roots attach its `trace`
//! member themselves. Terminal tables, quiet suffixes, JSON emission, deferred
//! plan flushing and the registered report families all stay in the surface.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#OBS-TRACE");

use std::fmt;
use std::path::Path;

use specmark::spec;
use vibe_lifecycle::{RunIdentity, SupersededTrace};
use vibe_wire::generated::shared::Timestamp;
use vibe_workspace::compile_trace::{RunOutcome, TraceRun, TraceSummary, TraceWarning};

mod bounded;
mod exit;
mod report;

pub use bounded::BoundedDiagnostic;
pub use exit::{CommandExit, FinalizedCommand, PlanDisposition, classify, finalize};

#[cfg(test)]
mod tests;

/// An injected instant. Every timestamp this cell writes arrives this way —
/// see the module note on why there is no clock and no `Drop` that reads one.
///
/// ```
/// use vibe_orchestrator::trace::Clock;
/// use vibe_wire::generated::shared::Timestamp;
///
/// let fixed = Timestamp::from_timestamp(0, 0).unwrap();
/// let clock: Clock<'_> = &|| fixed;
/// assert_eq!(clock(), fixed);
/// ```
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#OBS-TRACE")]
pub type Clock<'a> = &'a dyn Fn() -> Timestamp;

/// What a displaced run's index records as its failure.
///
/// FIXED text. Why a run was superseded is a structural fact about this
/// workspace — a later invocation took the identity — and never a quotation of
/// anything the displaced command produced.
const SUPERSEDED: &str = "superseded by a later invocation of this workspace";

/// The command's trace, in exactly one of its three states.
///
/// Deliberately not `Clone`: cloning it would clone a `TraceRun`, and the
/// whole contract is that the last handle drops inside the funnel.
enum TraceSession {
    Disabled,
    Unavailable {
        run_id: String,
        reasons: Vec<BoundedDiagnostic>,
    },
    Open {
        run_id: String,
        run: TraceRun,
    },
}

/// The owner a command holds for its whole execution: the session, plus the
/// startup notices produced before it even existed.
///
/// The notices are OWNED here rather than returned beside it, because they are
/// produced by superseding a displaced predecessor — work that happens before
/// the current session is decided, and whose failures a caller must not be
/// able to drop by ignoring the second element of a tuple. Close folds them
/// into the trace member's warnings AND returns them for human presentation;
/// a disabled session still returns them, because "the previous run's trace
/// could not be closed" is true whether or not this run is being traced.
///
/// ```
/// use vibe_lifecycle::RunIdentity;
/// use vibe_orchestrator::trace::without_workspace;
///
/// let identity = RunIdentity {
///     run_id: "0".repeat(32),
///     started: "2026-08-28T10:00:00Z".to_string(),
///     adopted: false,
///     compile_trace: false,
///     superseded_trace: None,
/// };
/// let preparation = without_workspace(&identity);
/// assert!(preparation.recorder().is_none());
/// assert!(!preparation.trace_requested());
/// ```
#[must_use = "the trace owner must be consumed by `finalize`: dropping it leaves the run's \
              index `running` on disk and silently discards the startup notices it owns"]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#OBS-TRACE")]
pub struct TracePreparation {
    session: TraceSession,
    notices: Vec<BoundedDiagnostic>,
}

impl TracePreparation {
    /// The ONLY thing lower layers ever see. Not an owned clone, not a
    /// retry-capable handle: a borrow that cannot outlive the command.
    ///
    /// See the type-level example.
    #[must_use]
    pub fn recorder(&self) -> Option<&TraceRun> {
        match &self.session {
            TraceSession::Open { run, .. } => Some(run),
            TraceSession::Disabled | TraceSession::Unavailable { .. } => None,
        }
    }

    /// Whether tracing was REQUESTED — true for both `open` and `unavailable`.
    ///
    /// An explicit pre-close fact, read before anything can go wrong with the
    /// report: emission policy may never be inferred from whether a member
    /// survived validation. See the type-level example.
    #[must_use]
    pub fn trace_requested(&self) -> bool {
        !matches!(self.session, TraceSession::Disabled)
    }
}

/// The owner for an invocation whose workspace discovery FAILED, so there is
/// no canonical root to store a trace under.
///
/// Deliberately NOT a fall back to the selected project root: entering a
/// workspace through a member would then lock and write a trace home that is
/// not the one the run's compiles belong to, and two members would race for
/// the same work. Nothing here creates a lock or a tree.
///
/// The SESSION depends on the request, not on the failure:
///
/// * a run that asked for nothing is `disabled`, and owes no explanation;
/// * a run that ASKED is `unavailable`, with the fixed reason below. Calling
///   it `disabled` would answer an explicit `--trace-compile` with silence,
///   and would make a command that COULD NOT be traced indistinguishable from
///   one nobody asked to trace — the exact confusion the third state exists
///   to prevent.
///
/// The NOTICE does not depend on the request at all. A displaced predecessor
/// is a fact about the state, not about this invocation's flags: the prior
/// parked run's index still says `running`, this command has taken its
/// identity, and nothing here can close it because closing needs the very root
/// that could not be found. That is true whether or not this run wanted a
/// trace, so an untraced invocation still owes the operator the sentence.
///
/// See [`TracePreparation`] for the worked example.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#OBS-TRACE")]
pub fn without_workspace(identity: &RunIdentity) -> TracePreparation {
    let session = if identity.compile_trace {
        unavailable(
            &identity.run_id,
            format_args!(
                "the workspace enclosing this project could not be discovered, so there is no \
                 canonical root to store a trace under and this invocation compiles untraced"
            ),
        )
    } else {
        TraceSession::Disabled
    };
    let notices = identity
        .superseded_trace
        .as_ref()
        .map(|superseded| {
            vec![BoundedDiagnostic::new(format_args!(
                "the displaced trace run `{}` could not be superseded: the workspace enclosing \
                 this project could not be discovered, so its trace home cannot be named and its \
                 index still reads `running`",
                superseded.run_id
            ))]
        })
        .unwrap_or_default();
    TracePreparation { session, notices }
}

/// Open the command's trace from the already-selected lifecycle identity.
///
/// The identity is BORROWED: the caller's next move is to put its run id,
/// start and effective trace bit into `RunMetadata`, and a seam that consumed
/// it would force a clone at every call site for no reason.
///
/// The order below is mandatory, and each step is a different law:
///
/// 1. a displaced predecessor is terminalised FIRST — it holds the one
///    cooperative project lock, so closing it is what lets the fresh open
///    succeed at all;
/// 2. a disabled run spends nothing: no start parse, no path, no clock;
/// 3. an adopted run may only REOPEN — see [`TraceRun::open_existing`];
/// 4. a fresh run may create;
/// 5. every parse or open failure is requested-but-unavailable, never a
///    command failure. This whole subsystem is a witness, not a veto.
///
/// ```no_run
/// use std::path::Path;
/// use vibe_lifecycle::RunIdentity;
/// use vibe_orchestrator::trace::prepare;
/// use vibe_wire::generated::shared::Timestamp;
///
/// let identity = RunIdentity {
///     run_id: "0".repeat(32),
///     started: "2026-08-28T10:00:00Z".to_string(),
///     adopted: false,
///     compile_trace: true,
///     superseded_trace: None,
/// };
/// let fixed = Timestamp::from_timestamp(0, 0).unwrap();
/// let preparation = prepare(Path::new("/workspace"), &identity, &|| fixed);
/// assert!(preparation.trace_requested());
/// ```
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#OBS-TRACE")]
pub fn prepare(
    root: &Path,
    identity: &RunIdentity,
    supersede_clock: Clock<'_>,
) -> TracePreparation {
    let mut notices = Vec::new();
    if let Some(superseded) = identity.superseded_trace.as_ref() {
        supersede(root, superseded, supersede_clock, &mut notices);
    }
    if !identity.compile_trace {
        return TracePreparation {
            session: TraceSession::Disabled,
            notices,
        };
    }
    let Some(started) = parse_started(&identity.started) else {
        return TracePreparation {
            session: unavailable(
                &identity.run_id,
                format_args!(
                    "the run records a start `{}` that is not an RFC 3339 timestamp, so no \
                     trace run could be identified",
                    identity.started
                ),
            ),
            notices,
        };
    };
    let session = if identity.adopted {
        match TraceRun::open_existing(root, &identity.run_id, started) {
            Ok(Some(run)) => TraceSession::Open {
                run_id: identity.run_id.clone(),
                run,
            },
            // The sticky bit proved a REQUEST, not a recorder. Creating the
            // run here would publish a history whose early compiles are
            // missing and call it complete.
            Ok(None) => unavailable(
                &identity.run_id,
                format_args!(
                    "this invocation adopted a parked run whose trace could not be reopened \
                     because no existing trace directory was found, so it compiles untraced \
                     rather than starting a partial mid-run history"
                ),
            ),
            Err(error) => unavailable(
                &identity.run_id,
                format_args!("the adopted run's trace could not be reopened: {error}"),
            ),
        }
    } else {
        match TraceRun::open(root, &identity.run_id, started) {
            Ok(run) => TraceSession::Open {
                run_id: identity.run_id.clone(),
                run,
            },
            Err(error) => unavailable(
                &identity.run_id,
                format_args!("no trace run could be opened: {error}"),
            ),
        }
    };
    TracePreparation { session, notices }
}

/// Terminalise the parked traced run this invocation displaced.
///
/// The state proved WHICH run was displaced; it did not prove that the run
/// ever opened a trace. So this is an existing-only reopen: a directory that
/// is not there is left not-there, and no phantom terminal run is manufactured
/// merely so that something could be superseded. The clock is called only on
/// the one path that really writes a terminal index.
fn supersede(
    root: &Path,
    superseded: &SupersededTrace,
    clock: Clock<'_>,
    notices: &mut Vec<BoundedDiagnostic>,
) {
    let Some(started) = parse_started(&superseded.started) else {
        notices.push(BoundedDiagnostic::new(format_args!(
            "the displaced run `{}` records a start `{}` that is not an RFC 3339 timestamp, so \
             its trace was left exactly as it is",
            superseded.run_id, superseded.started
        )));
        return;
    };
    match TraceRun::open_existing(root, &superseded.run_id, started) {
        Ok(None) => {}
        Ok(Some(run)) => {
            let summary = run.finish(&RunOutcome::Failed(SUPERSEDED.to_string()), clock());
            // The last handle: dropping it releases the cooperative project
            // lock, which the fresh open below is about to need.
            drop(run);
            notices.extend(supersede_notices(&superseded.run_id, &summary));
        }
        Err(error) => notices.push(BoundedDiagnostic::new(format_args!(
            "the displaced trace run `{}` could not be reopened to be superseded: {error}",
            superseded.run_id
        ))),
    }
}

/// Everything the displaced run's own close reported, as startup notices.
///
/// Pure, so the shapes it has to get right can be proved directly instead of
/// through cross-crate fault injection.
///
/// A finalised supersession always ends in exactly one bounded STRUCTURAL
/// notice — fixed prose naming the run, the presentation twin of the fixed
/// [`SUPERSEDED`] word its index now carries — because "the previous run's
/// trace was closed" is a fact the operator is owed whether or not anything
/// went wrong while closing it. It comes AFTER every writer warning, in their
/// original order: the warnings are the ONLY account of what went wrong (a
/// terminal index that landed despite a post-publication fault reports an
/// `IndexAnomaly`), and the structural fact closes rather than leads.
///
/// A run that was NOT finalised never gets the structural notice — that would
/// claim a close that did not happen. Its warnings stand alone, and the
/// generic `still reads running` line is a FALLBACK, added only when no
/// warning explains the refusal: when `NotFinalised` is present it would be a
/// duplicate of a strictly better message.
fn supersede_notices(run_id: &str, summary: &TraceSummary) -> Vec<BoundedDiagnostic> {
    let mut notices: Vec<BoundedDiagnostic> = summary
        .warnings
        .iter()
        .map(|warning| {
            BoundedDiagnostic::new(format_args!(
                "closing the displaced trace run `{run_id}`: {warning}"
            ))
        })
        .collect();
    if summary.finalised {
        notices.push(BoundedDiagnostic::new(format_args!(
            "the displaced trace run `{run_id}` was finalised: {SUPERSEDED}"
        )));
        return notices;
    }
    let explained = summary
        .warnings
        .iter()
        .any(|warning| matches!(warning, TraceWarning::NotFinalised { .. }));
    if !explained {
        notices.push(BoundedDiagnostic::new(format_args!(
            "the displaced trace run `{run_id}` could not be finalised, so its index still \
             reads `running`"
        )));
    }
    notices
}

/// Build the `unavailable` state with one bounded reason.
///
/// The reason is clamped through the WRITER's formatter as it is built, so the
/// unbounded intermediate string never exists — a refusal can quote a
/// filesystem path or a hostile directory name, and a report is exactly the
/// place that text gets copied to.
fn unavailable(run_id: &str, reason: fmt::Arguments<'_>) -> TraceSession {
    TraceSession::Unavailable {
        run_id: run_id.to_string(),
        reasons: vec![BoundedDiagnostic::new(reason)],
    }
}

/// The lifecycle's RFC 3339 start, as the trace epoch's timestamp.
///
/// `None` rather than an error: an unparsable start means this invocation
/// cannot name a trace run, which is a reason to compile untraced and say so —
/// never a reason to fail.
fn parse_started(started: &str) -> Option<Timestamp> {
    chrono::DateTime::parse_from_rfc3339(started)
        .ok()
        .map(|stamp| stamp.with_timezone(&chrono::Utc))
}
