//! How a command ends, and the ONE consuming join between that outcome and its
//! report.
//!
//! Nothing here renders or serialises. The funnel returns a
//! [`FinalizedCommand`] and the surface's own report roots attach its `trace`
//! member themselves — which is exactly why the join is generic over the draft
//! `R`: four CLI command roots and a hosted MCP result share one funnel instead
//! of five near-copies.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#OBS-TRACE");

use specmark::spec;
use vibe_wire::generated::shared::CompileTraceReport;
use vibe_workspace::compile_trace::RunOutcome;

use crate::failure;

use super::{BoundedDiagnostic, Clock, TracePreparation, TraceSession, report};

/// What a failed command's index records as its failure.
///
/// FIXED text, and for a sharp reason: see the module note on
/// [`super`] about never persisting an `anyhow::Error`.
const COMMAND_FAILED: &str = "command failed";

/// How a command ended, joined with the report draft it produced.
///
/// Generic over the draft so every command root shares one funnel instead of
/// one near-copy each. `emit_when_trace_disabled` carries the policy the
/// historically-inner emitters already have (install slot failure, lifecycle
/// contribution failure): it is a property of the typed failure, never
/// something inferred by inspecting an error string.
///
/// ```
/// use vibe_orchestrator::trace::CommandExit;
/// let exit: CommandExit<&str> = CommandExit::Parked("the draft");
/// assert!(matches!(exit, CommandExit::Parked("the draft")));
/// ```
#[must_use = "a command exit must reach `finalize`: it carries the report draft and, on the \
              failure arm, the original error the caller still has to return"]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#OBS-TRACE")]
pub enum CommandExit<R> {
    /// The command completed.
    Success(R),
    /// The command parked for a hosting agent.
    Parked(R),
    /// The command failed, and still owes its measured report.
    Failed {
        /// The measured draft its own site froze.
        report: R,
        /// The caller's ORIGINAL error object, to be returned unchanged.
        original_error: anyhow::Error,
        /// Whether this failure emits its machine document when tracing is OFF.
        emit_when_trace_disabled: bool,
    },
}

/// What the deferred JSON plan previews owe this outcome.
///
/// A TYPED fact, decided from the command exit itself. Reading it back off the
/// finished report — "does the draft carry a delegation member?" — would be an
/// inference: the two agree today, and a renderer that quietly disagreed with
/// the funnel about whether a run parked would either drop a preview a
/// completed run owes, or print one beside a handoff that is supposed to stand
/// alone.
///
/// ```
/// use vibe_orchestrator::trace::PlanDisposition;
/// assert_ne!(PlanDisposition::Flush, PlanDisposition::Discard);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#OBS-TRACE")]
pub enum PlanDisposition {
    /// Success and failure alike: a preview records what this invocation was
    /// doing, and both are outcomes.
    Flush,
    /// A park emits ONE document in total.
    Discard,
}

/// Everything the caller needs to render, and nothing rendered.
///
/// ```
/// use vibe_orchestrator::trace::FinalizedCommand;
/// fn plan(finalized: &FinalizedCommand<&str>) -> bool {
///     finalized.trace_requested
/// }
/// ```
#[must_use = "the join must be handled: it owns the report draft, the original error to \
              return, the emission decision and the notices to present"]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#OBS-TRACE")]
pub struct FinalizedCommand<R> {
    /// The command's own report draft, untouched by the funnel.
    pub report: R,
    /// What the deferred plan previews owe — from the EXIT, not the report.
    pub plan: PlanDisposition,
    /// The shared generated member, or `None` when tracing was off — or when
    /// the member itself would have broken its own wire law.
    pub trace: Option<CompileTraceReport>,
    /// The caller's ORIGINAL error object, unchanged: same downcast identity,
    /// same chain, never re-wrapped or re-stringified.
    pub original_error: Option<anyhow::Error>,
    /// Whether the command's registered root is emitted at all.
    pub emit_report: bool,
    /// The pre-close fact, not `trace.is_some()`.
    pub trace_requested: bool,
    /// The owner's startup notices, already bounded, as ordinary text.
    pub notices: Vec<String>,
}

/// The trace's own view of how the command ended.
///
/// The failure arm carries a BORROW of the error precisely so that this cell
/// cannot format or store it: there is nothing here to persist it into, and
/// the type says so.
enum Disposition<'a> {
    Success,
    Parked,
    Failed(
        #[allow(
            dead_code,
            reason = "borrowed precisely so this cell CANNOT format or store the command's \
                      error; never reading it is the invariant, and the borrow is what makes \
                      an attempt to read it visible in a diff"
        )]
        &'a anyhow::Error,
    ),
}

/// The one consuming funnel: take the owner and the typed exit, finish the
/// recorder, and hand back the join.
///
/// Emission policy is decided from facts read BEFORE the close: success and
/// park always emit; a failure emits when its own typed bit says so or when
/// tracing was requested, so a requested-but-unavailable trace is observable
/// on a path that was historically silent. A member that then fails validation
/// is omitted — and changes none of that.
///
/// ```
/// use vibe_lifecycle::RunIdentity;
/// use vibe_orchestrator::trace::{CommandExit, PlanDisposition, finalize, without_workspace};
/// use vibe_wire::generated::shared::Timestamp;
///
/// let identity = RunIdentity {
///     run_id: "0".repeat(32),
///     started: "2026-08-28T10:00:00Z".to_string(),
///     adopted: false,
///     compile_trace: false,
///     superseded_trace: None,
/// };
/// let fixed = Timestamp::from_timestamp(0, 0).unwrap();
/// let finalized = finalize(
///     without_workspace(&identity),
///     CommandExit::Parked("the draft"),
///     &|| fixed,
/// );
/// assert_eq!(finalized.report, "the draft");
/// assert_eq!(finalized.plan, PlanDisposition::Discard);
/// assert!(finalized.trace.is_none(), "a disabled run carries no member");
/// ```
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#OBS-TRACE")]
pub fn finalize<R>(
    preparation: TracePreparation,
    exit: CommandExit<R>,
    finish_clock: Clock<'_>,
) -> FinalizedCommand<R> {
    let trace_requested = preparation.trace_requested();
    let TracePreparation { session, notices } = preparation;
    let (report, original_error, parked, emit_when_disabled) = match exit {
        CommandExit::Success(report) => (report, None, false, true),
        CommandExit::Parked(report) => (report, None, true, true),
        CommandExit::Failed {
            report,
            original_error,
            emit_when_trace_disabled,
        } => (
            report,
            Some(original_error),
            false,
            emit_when_trace_disabled,
        ),
    };
    let emit_report = original_error.is_none() || emit_when_disabled || trace_requested;
    let (trace, notices) = {
        // The borrow lives exactly as long as the close, and the close has no
        // way to keep it.
        let disposition = match (original_error.as_ref(), parked) {
            (Some(error), _) => Disposition::Failed(error),
            (None, true) => Disposition::Parked,
            (None, false) => Disposition::Success,
        };
        close(session, &disposition, finish_clock, notices)
    };
    FinalizedCommand {
        report,
        plan: if parked {
            PlanDisposition::Discard
        } else {
            PlanDisposition::Flush
        },
        trace,
        original_error,
        emit_report,
        trace_requested,
        // The proof is dropped exactly here, at the crate boundary: past this
        // point the text is ordinary presentation material.
        notices: notices
            .into_iter()
            .map(BoundedDiagnostic::into_string)
            .collect(),
    }
}

/// Consume the session against one disposition, and build the member.
fn close(
    session: TraceSession,
    disposition: &Disposition<'_>,
    finish_clock: Clock<'_>,
    notices: Vec<BoundedDiagnostic>,
) -> (Option<CompileTraceReport>, Vec<BoundedDiagnostic>) {
    match session {
        // Off means off: no member, and the finish clock is never called.
        TraceSession::Disabled => (None, notices),
        TraceSession::Unavailable { run_id, reasons } => {
            report::unavailable(&run_id, &reasons, notices)
        }
        TraceSession::Open { run_id, run } => {
            let summary = match disposition {
                // Park is SUSPENSION: take a running summary, do not finish,
                // and release the lock so the resume can take it.
                Disposition::Parked => run.summary(),
                Disposition::Success => run.finish(&RunOutcome::Ok, finish_clock()),
                // Borrowed, never rendered — see the module note.
                Disposition::Failed(_) => run.finish(
                    &RunOutcome::Failed(COMMAND_FAILED.to_string()),
                    finish_clock(),
                ),
            };
            // The command's last handle on the run. A retained `TraceScope`
            // elsewhere legitimately keeps the shared state — and therefore
            // the cooperative lock — alive until it drops; there is no hidden
            // force-release, because a forced release would be a lie about
            // who is writing.
            drop(run);
            report::from_summary(&run_id, &summary, notices)
        }
    }
}

/// Turn any error into the one typed failure exit.
///
/// A carried draft is unwrapped to exactly what its site measured — the same
/// object, the same context chain, the same site-frozen emission bit. Anything
/// else is a generic stage failure: it gets the draft `fallback` builds for the
/// stage it happened in, and the historical emission policy for such failures,
/// which is silence.
///
/// Generic over the evidence, so the surface that owns a registered report
/// family and the lower layer that owns a measurement classify through ONE
/// implementation of the carrier — see [`crate::failure::Carried`].
///
/// ```
/// use vibe_orchestrator::failure::{Carried, carry};
/// use vibe_orchestrator::trace::{CommandExit, classify};
///
/// let carried = carry(Carried {
///     original: anyhow::Error::msg("the handler refused"),
///     evidence: "the measured draft",
///     emit_machine_failure: true,
/// });
/// let CommandExit::Failed { report, emit_when_trace_disabled, .. } =
///     classify(carried, || "the stage fallback")
/// else {
///     panic!("a carried failure is a failure");
/// };
/// assert_eq!(report, "the measured draft");
/// assert!(emit_when_trace_disabled, "the site's own policy survives");
/// ```
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#OBS-TRACE")]
pub fn classify<E>(error: anyhow::Error, fallback: impl FnOnce() -> E) -> CommandExit<E>
where
    E: std::fmt::Debug + Send + Sync + 'static,
{
    match failure::take::<E>(error) {
        Ok(failure::Carried {
            original,
            evidence,
            emit_machine_failure,
        }) => CommandExit::Failed {
            report: evidence,
            original_error: original,
            emit_when_trace_disabled: emit_machine_failure,
        },
        Err(original) => CommandExit::Failed {
            report: fallback(),
            original_error: original,
            emit_when_trace_disabled: false,
        },
    }
}
