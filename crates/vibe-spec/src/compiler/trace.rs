//! The one diagnostic observer seam of the compiler schedule (PROP-054
//! `##OBS-TRACE`), and nothing else.
//!
//! A caller that wants to watch a compile hands one [`CompileTraceSink`] to a
//! traced sibling of an existing entry point. Every actual pass invocation of
//! the declared schedule then produces exactly one [`PassTraceEvent`]: what
//! ran, what it consumed and produced, how each stage was timed, and — only
//! after the output was accepted and the sink asked for it — the exact pretty
//! `compiler_ir/e1` bytes of the accepted carrier.
//!
//! Four boundaries are deliberate:
//!
//! * **JTD-first metadata.** Status, level, cardinality, shape and duration are
//!   the GENERATED `compiler_trace_index/e1` types, used directly. This module
//!   declares no second vocabulary and no mapping table beside the epoch; the
//!   only handwritten values here are the borrowing event wrapper (it carries
//!   snapshot bytes and has no scope/sequence, neither of which the generated
//!   record can express) and the pre-encode decision.
//! * **Bytes only.** The sink never sees [`AnyIr`], a domain field or a JSON
//!   value. The snapshot is the ONE strict wire the R6.2b conversion already
//!   owns, so a recorder cannot invent a second projection.
//! * **Diagnostic only.** A trace, encode or sink problem is reported as a
//!   status on the event and can never alter the compile result or an error
//!   identity, so nothing the sink says comes back as a `Result` — and a sink
//!   that PANICS instead of answering is contained at one boundary rather than
//!   allowed to unwind out as the compile's answer. A future disk writer owns
//!   its own write-failure conversion.
//! * **Off means off.** With no sink the schedule takes the old path: the clock
//!   is never read, no event is built, nothing is encoded. Every observation
//!   sits behind `Option<&dyn CompileTraceSink>`.
//!
//! Scope, run identity, global sequence numbers, invocation ordinals,
//! filenames, retention and the byte budget itself are NOT here —
//! `vibe-workspace` adds them around one sink per artifact scope. This module
//! only provides the seam at which that writer can stand down.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#OBS-TRACE");

use std::fmt;
use std::panic::{self, AssertUnwindSafe};
use std::time::{Duration as Elapsed, Instant};

use vibe_wire::behaviour::compiler_trace_index::DIAGNOSTIC_CAP_BYTES;
use vibe_wire::generated::compiler_trace_index::e1::index;

use super::ir::{IrCardinality, IrLevel, IrShape};
use super::pass::AnyIr;
use super::wire::{self, IrWireError, bounded};

/// What the sink wants done with one accepted output.
///
/// Asked before any encoding clock starts, so a sink that is standing down
/// costs the compiler nothing at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotDecision {
    /// Encode the accepted carrier and deliver its bytes. The default.
    Encode,
    /// Stand down: no encode, no bytes. The retention budget this run may
    /// spend on snapshots is exhausted, which the trace epoch reports as
    /// `snapshot-skipped-budget` — an observability outcome a green run
    /// admits, never a compile failure.
    SkipBudget,
}

/// Where one compile's pass observations go.
///
/// Object-safe, `Send + Sync` and infallible from the compiler's side: an
/// observer is a witness, never a veto. Neither method may influence the
/// compile — a refusal has nowhere to go, and a PANIC is caught at the
/// compiler's boundary and treated as an observer defect, so the run still
/// returns the artifact or the error it would have returned unobserved.
pub trait CompileTraceSink: Send + Sync {
    /// Receive one attempted pass invocation. The event borrows for the call
    /// only; a sink that needs to keep anything copies it here.
    ///
    /// A panic here is contained: the observation is lost, the compile is not.
    fn record(&self, event: &PassTraceEvent<'_>);

    /// Decide whether the accepted output of `pass` should be encoded into a
    /// certified snapshot.
    ///
    /// Asked exactly once per accepted output — after output-shape checking
    /// and semantic/transition verification have both accepted it, and before
    /// the encode clock is read or the encoder is called. Infallible by
    /// construction: the only two answers are "encode" and "stand down", and
    /// the default answers "encode", so a sink that has no budget to track
    /// need not implement this at all. A panic here is contained and read as
    /// no answer at all, which falls back to that same default.
    fn before_snapshot(&self, _pass: &str, _output: &index::PassShape) -> SnapshotDecision {
        SnapshotDecision::Encode
    }
}

/// One attempted pass invocation, as the compiler saw it.
///
/// The pass name and the snapshot bytes are borrowed for the duration of the
/// [`CompileTraceSink::record`] call; every other member is the generated
/// trace-index type, so a recorder moves it into a `PassEvent` unchanged.
/// Which durations are present follows the status: a failure honestly omits
/// the stages it never reached.
pub struct PassTraceEvent<'a> {
    pass: &'a str,
    status: index::PassStatus,
    input: index::PassShape,
    output: index::PassShape,
    pass_duration: Option<index::Duration>,
    verify_duration: Option<index::Duration>,
    encode_duration: Option<index::Duration>,
    diagnostic: Option<String>,
    snapshot: Option<&'a [u8]>,
}

impl PassTraceEvent<'_> {
    /// The exact declared pass name, e.g. `parse`, `close`, `emit:static-xml`.
    pub fn pass(&self) -> &str {
        self.pass
    }

    pub fn status(&self) -> &index::PassStatus {
        &self.status
    }

    /// The shape of the value this invocation consumed.
    pub fn input(&self) -> &index::PassShape {
        &self.input
    }

    /// The shape of the value this invocation produced — the shape actually
    /// observed whenever the pass returned a carrier (so a wrong-shape refusal
    /// reports what the pass really returned), and the declared schedule
    /// output when the pass body produced nothing at all.
    pub fn output(&self) -> &index::PassShape {
        &self.output
    }

    /// The pass body around the erased call. Present whenever the body ran.
    pub fn pass_duration(&self) -> Option<&index::Duration> {
        self.pass_duration.as_ref()
    }

    /// Output-shape checking plus semantic and transition verification,
    /// measured apart from the body. Present whenever the body succeeded — a
    /// schedule with no verifier records the real cost of the shape check,
    /// not an absence.
    pub fn verify_duration(&self) -> Option<&index::Duration> {
        self.verify_duration.as_ref()
    }

    /// Snapshot encoding, measured apart from both. Present exactly when an
    /// encode was attempted — so a budget stand-down carries none.
    pub fn encode_duration(&self) -> Option<&index::Duration> {
        self.encode_duration.as_ref()
    }

    /// A bounded refusal text, present exactly on the three failure statuses.
    /// A budget stand-down is not a failure and carries none.
    pub fn diagnostic(&self) -> Option<&str> {
        self.diagnostic.as_deref()
    }

    /// The exact pretty `compiler_ir/e1` bytes of the accepted carrier,
    /// present exactly on [`index::PassStatus::Ok`].
    pub fn snapshot(&self) -> Option<&[u8]> {
        self.snapshot
    }
}

impl fmt::Debug for PassTraceEvent<'_> {
    /// Deliberately reports the snapshot's LENGTH, never its bytes: a debug
    /// line about an observation must not become a second copy of the IR.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PassTraceEvent")
            .field("pass", &self.pass)
            .field("status", &self.status)
            .field("input", &self.input)
            .field("output", &self.output)
            .field("pass_duration", &self.pass_duration)
            .field("verify_duration", &self.verify_duration)
            .field("encode_duration", &self.encode_duration)
            .field("diagnostic", &self.diagnostic)
            .field("snapshot_bytes", &self.snapshot.map(<[u8]>::len))
            .finish()
    }
}

/// The two timings a failure may already have spent.
#[derive(Debug, Clone, Copy)]
pub(crate) struct PassTiming {
    pub(crate) body: Elapsed,
    pub(crate) verify: Option<Elapsed>,
}

/// What the segment knows about one invocation before it ended.
#[derive(Debug, Clone, Copy)]
pub(crate) struct PassTraceFrame<'a> {
    pub(crate) pass: &'a str,
    pub(crate) input: IrShape,
    pub(crate) output: IrShape,
}

/// Record one accepted output.
///
/// The sink is asked first, so a stand-down reads no clock and calls no
/// encoder; only an `Encode` answer times [`wire::encode_pretty`] separately
/// from the pass body and its verification and delivers those exact bytes. An
/// encode refusal becomes `snapshot-failed`. Nothing here is returned: the
/// caller keeps the successful pass output whatever the observer decided.
pub(crate) fn record_accepted(
    sink: &dyn CompileTraceSink,
    pass: &str,
    input: IrShape,
    output: &AnyIr,
    timing: PassTiming,
) {
    let observed = shape(output.shape());
    let common = PassTraceEvent {
        pass,
        status: index::PassStatus::Ok,
        input: shape(input),
        output: observed.clone(),
        pass_duration: Some(measure(timing.body)),
        verify_duration: timing.verify.map(measure),
        encode_duration: None,
        diagnostic: None,
        snapshot: None,
    };

    if decide(sink, pass, &observed) == SnapshotDecision::SkipBudget {
        deliver(
            sink,
            &PassTraceEvent {
                status: index::PassStatus::SnapshotSkippedBudget,
                ..common
            },
        );
        return;
    }

    let clock = Instant::now();
    let encoded = encode_snapshot(output);
    let common = PassTraceEvent {
        encode_duration: Some(measure(clock.elapsed())),
        ..common
    };
    match encoded {
        Ok(bytes) => deliver(
            sink,
            &PassTraceEvent {
                snapshot: Some(&bytes),
                ..common
            },
        ),
        Err(error) => deliver(
            sink,
            &PassTraceEvent {
                status: index::PassStatus::SnapshotFailed,
                diagnostic: Some(diagnostic(&error)),
                ..common
            },
        ),
    }
}

/// Record one refused invocation. `status` is the caller's classification —
/// the pass body refused, or the output it returned was refused — and the
/// diagnostic is rendered from the very error the compiler is about to return.
pub(crate) fn record_refused(
    sink: &dyn CompileTraceSink,
    frame: PassTraceFrame<'_>,
    status: index::PassStatus,
    timing: PassTiming,
    error: &dyn fmt::Debug,
) {
    deliver(
        sink,
        &PassTraceEvent {
            pass: frame.pass,
            status,
            input: shape(frame.input),
            output: shape(frame.output),
            pass_duration: Some(measure(timing.body)),
            verify_duration: timing.verify.map(measure),
            encode_duration: None,
            diagnostic: Some(diagnostic(error)),
            snapshot: None,
        },
    );
}

/// The ONE boundary every crossing into a downstream observer goes through.
///
/// A sink is arbitrary foreign code. If it panics, that is an observer defect,
/// and the diagnostic-only law says an observer defect must not become the
/// compile's answer: the unwind is caught here so the caller returns the
/// artifact — or the compiler's own pass error — it would have returned with
/// no observer at all. The payload is dropped WITHOUT being formatted, cloned
/// or turned into a diagnostic, so a hostile panic message cannot ride out of
/// the observer either. (Rust's own panic hook still runs; this contains the
/// unwind, it does not silence the process-wide report, and no global hook is
/// touched.)
fn deliver(sink: &dyn CompileTraceSink, event: &PassTraceEvent<'_>) {
    let _ = panic::catch_unwind(AssertUnwindSafe(|| sink.record(event)));
}

/// The same containment for the pre-encode question. A sink that panics
/// instead of answering has not decided anything, so the compiler falls back
/// to the trait's own default — encode — exactly as if the sink had never
/// overridden it.
fn decide(sink: &dyn CompileTraceSink, pass: &str, output: &index::PassShape) -> SnapshotDecision {
    panic::catch_unwind(AssertUnwindSafe(|| sink.before_snapshot(pass, output)))
        .unwrap_or(SnapshotDecision::Encode)
}

/// The trace epoch's diagnostic law, applied through the ALREADY REVIEWED
/// bounded sink of the wire conversion.
///
/// Rendering goes through the derived `Debug`, never `Display`: a pass error
/// is an arbitrary type — a built-in's, and from R6.3 a plugin's — and some
/// `Display` impls build their whole text before a formatter ever sees it, so
/// an outer cap could only shorten an allocation that already happened. The
/// derived `Debug` writes field by field, so the sink stops KEEPING characters
/// while the value is still being walked, and the variant name leads the text.
/// The ceiling is `vibe_wire`'s ONE authority for it — the validator's own
/// `DIAGNOSTIC_CAP_BYTES`, itself pinned to the schema's
/// `x-diagnostic-cap-bytes` — so a producer and its validator can never drift
/// apart over a copied number.
fn diagnostic(error: &dyn fmt::Debug) -> String {
    bounded::debug_within(error, DIAGNOSTIC_CAP_BYTES)
}

/// The domain shape as the trace epoch spells it. The ONLY translation in this
/// module, and it is total over the closed level/cardinality vocabularies.
fn shape(value: IrShape) -> index::PassShape {
    index::PassShape {
        level: match value.level {
            IrLevel::Source => index::IrLevel::Source,
            IrLevel::Document => index::IrLevel::Document,
            IrLevel::Closure => index::IrLevel::Closure,
            IrLevel::Lane => index::IrLevel::Lane,
            IrLevel::Emitted => index::IrLevel::Emitted,
        },
        cardinality: match value.cardinality {
            IrCardinality::Document => index::IrCardinality::Document,
            IrCardinality::Artifact => index::IrCardinality::Artifact,
        },
    }
}

/// One measured stage in the epoch's representation: microseconds saturating
/// at [`u32::MAX`] rather than wrapping, with the marker set ONLY at that
/// ceiling — an exact measurement that lands on it stays unsaturated, exactly
/// as `event-coherence` requires.
fn measure(elapsed: Elapsed) -> index::Duration {
    match u32::try_from(elapsed.as_micros()) {
        Ok(micros) => index::Duration {
            micros,
            saturated: false,
        },
        Err(_) => index::Duration {
            micros: u32::MAX,
            saturated: true,
        },
    }
}

/// The compiler's ONE route from an accepted carrier to snapshot bytes.
///
/// Keeping it single lets the counter below prove that an untraced compile —
/// and a compile whose sink stood down on budget — never encodes anything, and
/// lets a test refuse an encode without inventing a production configuration
/// knob.
fn encode_snapshot(ir: &AnyIr) -> Result<Vec<u8>, IrWireError> {
    #[cfg(test)]
    {
        ENCODES.with(|count| count.set(count.get() + 1));
        if REFUSE_ENCODE.with(std::cell::Cell::get) {
            return Err(IrWireError::Encode(
                "the test encoder seam refused this snapshot".to_string(),
            ));
        }
    }
    wire::encode_pretty(ir)
}

#[cfg(test)]
std::thread_local! {
    static ENCODES: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static REFUSE_ENCODE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

#[cfg(test)]
pub(crate) fn reset_snapshot_encodes() {
    ENCODES.with(|count| count.set(0));
}

#[cfg(test)]
pub(crate) fn snapshot_encodes() -> usize {
    ENCODES.with(std::cell::Cell::get)
}

/// A test-only encoder refusal, scoped to the current thread and released on
/// drop. Narrow by construction: there is no production path that sets it.
#[cfg(test)]
pub(crate) struct RefusedEncoder;

#[cfg(test)]
impl RefusedEncoder {
    pub(crate) fn install() -> Self {
        REFUSE_ENCODE.with(|refuse| refuse.set(true));
        Self
    }
}

#[cfg(test)]
impl Drop for RefusedEncoder {
    fn drop(&mut self) {
        REFUSE_ENCODE.with(|refuse| refuse.set(false));
    }
}

#[cfg(test)]
mod tests;
