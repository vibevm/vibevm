//! The trace's three diagnostic surfaces — why a run could not be opened,
//! what the observer could not do, and why a scope operation refused.
//!
//! One file because they are one responsibility split three ways. Every arm
//! of all three is observer-side evidence under
//! [OBS-TRACE](spec://org.vibevm.core/vibevm/common/PROP-054#OBS-TRACE):
//! a trace failure is never a compile failure
//! ([TRACE-NONVETO](spec://org.vibevm.core/vibevm/common/PROP-054#TRACE-NONVETO)),
//! so each message names the observation law it serves and the fix surface
//! that resolves it — never a rich error, never a secret, never an unbounded
//! value (the fields were bounded where they were built, and the finished
//! `Display` is clamped by the one writer formatter at render time).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#OBS-TRACE");

use specmark::spec;

/// Why a trace run could not be opened. Every arm is a reason to compile
/// UNTRACED — never a reason to fail a compile.
#[derive(Debug, thiserror::Error)]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#OBS-TRACE")]
pub enum TraceOpenError {
    #[error(
        "violates REQ spec://org.vibevm.core/vibevm/common/PROP-054#OBS-TRACE: \
         the trace project root must be absolute: `{root}`; fix surface: \
         open the run from the canonical workspace root — until then this \
         run simply compiles untraced"
    )]
    RelativeRoot { root: String },
    #[error(
        "violates REQ spec://org.vibevm.core/vibevm/common/PROP-054#OBS-TRACE: \
         `{run_id}` is not an exact 32-lowercase-hex lifecycle run id; fix \
         surface: open with the lifecycle-allocated run id unchanged — the \
         run compiles untraced meanwhile"
    )]
    RunId { run_id: String },
    #[error(
        "violates REQ spec://org.vibevm.core/vibevm/common/PROP-054#OBS-TRACE: \
         the run directory is {directory_units} path units deep, leaving \
         {remaining} for a snapshot name; the shortest canonical name needs \
         {floor}; fix surface: open the project nearer the drive root so a \
         canonical snapshot name fits — the run compiles untraced"
    )]
    RunDirectoryTooDeep {
        directory_units: usize,
        remaining: usize,
        floor: usize,
    },
    #[error(
        "violates REQ spec://org.vibevm.core/vibevm/common/PROP-054#OBS-TRACE: \
         the trace directory could not be opened: {reason}; fix surface: \
         make `.vibe/trace` under the project root creatable and writable — \
         the run compiles untraced"
    )]
    Directory { reason: String },
    #[error(
        "violates REQ spec://org.vibevm.core/vibevm/common/PROP-054#OBS-TRACE: \
         `{path}` is trace residue and was left untouched: {reason}; fix \
         surface: inspect and remove the named object by hand (retention \
         only collects complete runs) — this run compiles untraced"
    )]
    Residue { path: String, reason: String },
    #[error(
        "violates REQ spec://org.vibevm.core/vibevm/common/PROP-054#OBS-TRACE: \
         another VibeVM trace writer already owns `{project}`; this run is \
         not traced rather than waiting for it; fix surface: let the owning \
         command finish and retry — never delete or replace \
         `.vibe/compile-trace.lock`, whose unlinking while live would mint \
         a second lock identity; this run compiles untraced"
    )]
    Busy { project: String },
}

/// Something the trace could not do, reported to whoever renders the run.
///
/// Warnings accumulate; none of them ever reaches the compiler.
#[derive(Debug, Clone, thiserror::Error)]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#OBS-TRACE")]
pub enum TraceWarning {
    #[error(
        "violates REQ spec://org.vibevm.core/vibevm/common/PROP-054#OBS-TRACE: \
         `{path}` was left in place: {reason}; fix surface: compare the named \
         path against the retention law (nine complete runs kept) and remove \
         it by hand if it is expected"
    )]
    Residue { path: String, reason: String },
    #[error(
        "violates REQ spec://org.vibevm.core/vibevm/common/PROP-054#OBS-TRACE: \
         a trace index update did not land and will be retried: {reason}; \
         fix surface: the next update retries on its own — restore the run \
         directory's writability if the warning persists"
    )]
    IndexWrite { reason: String },
    #[error(
        "violates REQ spec://org.vibevm.core/vibevm/common/PROP-054#OBS-TRACE: \
         a trace index update landed, but its publication reported a fault \
         after the point of no return: {reason}; fix surface: treat the \
         on-disk `index.json` as the authority and re-read it — the writer \
         has already moved on"
    )]
    IndexAnomaly { reason: String },
    #[error(
        "violates REQ spec://org.vibevm.core/vibevm/common/PROP-054#OBS-TRACE: \
         event {sequence} produced no snapshot: {reason}; fix surface: the \
         event's outcome and cause stay in the run's `index.json` — re-run \
         the compile under `--trace-compile` if the certified bytes matter"
    )]
    Snapshot { sequence: u32, reason: String },
    #[error(
        "violates REQ spec://org.vibevm.core/vibevm/common/PROP-054#OBS-TRACE: \
         an observation was dropped: {reason}; fix surface: address the cause \
         carried in `reason` — the compile it describes already succeeded, \
         unobserved"
    )]
    Dropped { reason: String },
    #[error(
        "violates REQ spec://org.vibevm.core/vibevm/common/PROP-054#OBS-TRACE: \
         the run's terminal status was not written, so the index stays \
         `running`: {reason}; fix surface: do not trust the stale `running` \
         index — re-run the command so a fresh run terminalises"
    )]
    NotFinalised { reason: String },
}

/// Why one scope operation refused. Also never a compile failure — a caller
/// that cannot record a transition has still compiled the artifact.
#[derive(Debug, thiserror::Error)]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#OBS-TRACE")]
pub enum TraceError {
    #[error(
        "violates REQ spec://org.vibevm.core/vibevm/common/PROP-054#OBS-TRACE: \
         no scope `{id}` was declared on this run; fix surface: declare the \
         scope through the run's `declare_scope`/`acquire_scope` before \
         recording on it"
    )]
    UnknownScope { id: String },
    #[error(
        "violates REQ spec://org.vibevm.core/vibevm/common/PROP-054#OBS-TRACE: \
         scope `{id}` is already declared with a different identity; fix \
         surface: keep one descriptor identity per scope id — reacquire the \
         pending scope, never redefine it"
    )]
    ScopeConflict { id: String },
    #[error(
        "violates REQ spec://org.vibevm.core/vibevm/common/PROP-054#OBS-TRACE: \
         scope `{id}` already reached a terminal status; fix surface: report \
         a scope's transitions once — open the next attempt instead of \
         re-reporting the terminal one"
    )]
    ScopeAlreadyResolved { id: String },
    #[error(
        "violates REQ spec://org.vibevm.core/vibevm/common/PROP-054#OBS-TRACE: \
         scope `{id}` recorded events, so it cannot be reported as skipped; \
         fix surface: report `complete` or `fail` — a scope that emitted \
         events was compiled, not found fresh"
    )]
    SkipAfterEvents { id: String },
    #[error(
        "violates REQ spec://org.vibevm.core/vibevm/common/PROP-054#OBS-TRACE: \
         scope base `{base}` has spent every attempt id the attempt grammar \
         can address; the counter refuses rather than saturating or wrapping; \
         fix surface: start a fresh lifecycle run — the closed attempt space \
         is by design"
    )]
    AttemptExhausted { base: String },
    #[error(
        "violates REQ spec://org.vibevm.core/vibevm/common/PROP-054#OBS-TRACE: \
         the trace index refused the update: {reason}; fix surface: the \
         refusal is observer evidence only and the compile stands — inspect \
         the index's relational validator for the law it names"
    )]
    IndexRefused { reason: String },
    #[error(
        "violates REQ spec://org.vibevm.core/vibevm/common/PROP-054#OBS-TRACE: \
         the run is already finalised; fix surface: record everything through \
         the run before `finish` — a finalised run takes no further events"
    )]
    Finalised,
}
