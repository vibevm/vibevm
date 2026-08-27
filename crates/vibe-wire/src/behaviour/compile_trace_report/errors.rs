//! The typed refusals of the command-report trace member's relational
//! laws. The same refusal discipline the trace index cell carries: a
//! report is read from disk or a provider stream, so no variant here
//! clones a wire string — every untrusted scalar rides a bounded
//! [`ScalarPreview`] (shared with the index cell, one type, not a
//! second preview), and every index or column name is bounded by
//! construction.

use crate::behaviour::compiler_trace_index::ScalarPreview;
use crate::generated::shared::TraceReportStatus;

/// One broken relational law, with the context needed to name the
/// offender. Typed end to end — no stringly `detail` — so a test can
/// assert the exact family a mutation lands in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TraceReportError {
    /// `run-id` — `run_id` is not exactly 32 lowercase hex.
    RunIdNotLowercaseHex { run_id: ScalarPreview },
    /// `canonical-counts` — a count member is not a canonical unsigned
    /// decimal string. `field` names the member in wire spelling.
    NonCanonicalCount {
        field: &'static str,
        value: ScalarPreview,
    },
    /// `run-path` — the path is not absolute, forward-slashed and
    /// control-free.
    UnsafeRunPath {
        path: ScalarPreview,
        reason: RunPathUnsafety,
    },
    /// `run-path` — the path does not end with `.vibe/trace/<run_id>`,
    /// so it names a directory this report's run does not own.
    RunPathSuffix {
        path: ScalarPreview,
        run_id: ScalarPreview,
    },
    /// `status-matrix` — `unavailable` carries a run path.
    UnavailableWithRunPath,
    /// `status-matrix` — a `running`, `ok` or `failed` trace names no
    /// run directory; the member is absent exactly for `unavailable`.
    ActiveWithoutRunPath { status: TraceReportStatus },
    /// `status-matrix` — `unavailable` claims to be finalised; a trace
    /// that never opened has no terminal state.
    UnavailableFinalised,
    /// `status-matrix` — `unavailable` claims a spent snapshot budget;
    /// a recorder that never opened never owned one.
    UnavailableBudgetExhausted,
    /// `status-matrix` — `unavailable` carries a nonzero count.
    UnavailableNonZero {
        field: &'static str,
        carried: ScalarPreview,
    },
    /// `status-matrix` — `unavailable` carries timing rows it never
    /// recorded.
    UnavailableWithTimings,
    /// `status-matrix` — `unavailable` says nothing about WHY tracing
    /// is unavailable; the bounded reason is the member's whole point.
    UnavailableSilent,
    /// `status-matrix` — `unavailable` carries warnings that are all
    /// blank. A nonempty vector of empty or whitespace-only strings
    /// satisfies the LENGTH of the reason law and none of its meaning.
    /// `warnings` is how many blanks were offered.
    UnavailableBlankReason { warnings: usize },
    /// `status-matrix` — a parked (`running`) trace claims to be
    /// finalised.
    RunningFinalised,
    /// `status-matrix` — `ok`/`failed` is a terminal state; its trace
    /// must be finalised.
    TerminalNotFinalised { status: TraceReportStatus },
    /// `count-coherence` — more certified snapshots than events.
    SnapshotsExceedEvents {
        events: ScalarPreview,
        snapshots: ScalarPreview,
    },
    /// `warning-cap` — a warning text exceeds the shared diagnostic
    /// cap. `index` is the warning's list position.
    WarningOverCap { index: usize, bytes: usize },
    /// `timing-rows` — a pass name is blank or carries CR, LF or NUL.
    /// `row` is the row's list position.
    TimingPassUnsafe { row: usize, pass: ScalarPreview },
    /// `timing-rows` — one pass name got two rows; the CLI table is a
    /// diffable artifact, so a duplicate row is never a rendering
    /// choice.
    TimingPassDuplicate { row: usize, pass: ScalarPreview },
    /// `timing-rows` — a carried duration claims saturation without
    /// sitting at the ceiling; the rule is the index's own
    /// (`saturated` only at `micros = u32::MAX`), reused, not restated.
    NonCanonicalDuration {
        row: usize,
        pass: ScalarPreview,
        column: &'static str,
    },
}

/// Why a run path failed its spelling law.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunPathUnsafety {
    /// A Windows separator — the wire spelling is forward slashes.
    Backslash,
    /// CR, LF or NUL inside the path.
    ControlByte,
    /// Neither `/…` nor `X:/…`.
    NotAbsolute,
}

impl TraceReportError {
    /// The implemented-law label this violation witnesses — the join
    /// key the wire-corpus parity test reads.
    #[must_use]
    pub fn law(&self) -> &'static str {
        match self {
            TraceReportError::RunIdNotLowercaseHex { .. } => "run-id",
            TraceReportError::NonCanonicalCount { .. } => "canonical-counts",
            TraceReportError::UnsafeRunPath { .. } | TraceReportError::RunPathSuffix { .. } => {
                "run-path"
            }
            TraceReportError::UnavailableWithRunPath
            | TraceReportError::ActiveWithoutRunPath { .. }
            | TraceReportError::UnavailableFinalised
            | TraceReportError::UnavailableBudgetExhausted
            | TraceReportError::UnavailableNonZero { .. }
            | TraceReportError::UnavailableWithTimings
            | TraceReportError::UnavailableSilent
            | TraceReportError::UnavailableBlankReason { .. }
            | TraceReportError::RunningFinalised
            | TraceReportError::TerminalNotFinalised { .. } => "status-matrix",
            TraceReportError::SnapshotsExceedEvents { .. } => "count-coherence",
            TraceReportError::WarningOverCap { .. } => "warning-cap",
            TraceReportError::TimingPassUnsafe { .. }
            | TraceReportError::TimingPassDuplicate { .. }
            | TraceReportError::NonCanonicalDuration { .. } => "timing-rows",
        }
    }
}

impl std::error::Error for TraceReportError {}

/// The wire spelling of a status — the closed enum carries no `Display`
/// of its own, and a refusal quoting the exact wire word beats one that
/// names the Rust variant.
fn status_spelling(status: &TraceReportStatus) -> &'static str {
    match status {
        TraceReportStatus::Unavailable => "unavailable",
        TraceReportStatus::Running => "running",
        TraceReportStatus::Ok => "ok",
        TraceReportStatus::Failed => "failed",
    }
}

impl std::fmt::Display for TraceReportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use TraceReportError as E;
        match self {
            E::RunIdNotLowercaseHex { run_id } => {
                write!(
                    f,
                    "run_id {run_id} is not exactly 32 lowercase hex characters"
                )
            }
            E::NonCanonicalCount { field, value } => write!(
                f,
                "{field} = {value} is not a canonical unsigned decimal string \
                 (nonempty ASCII digits, no leading zero unless the value is 0)"
            ),
            E::UnsafeRunPath { path, reason } => match reason {
                RunPathUnsafety::Backslash => {
                    write!(
                        f,
                        "run_path {path} contains a backslash; the wire spelling is forward slashes"
                    )
                }
                RunPathUnsafety::ControlByte => {
                    write!(f, "run_path {path} carries CR, LF or NUL")
                }
                RunPathUnsafety::NotAbsolute => {
                    write!(f, "run_path {path} is not an absolute forward-slashed path")
                }
            },
            E::RunPathSuffix { path, run_id } => {
                write!(f, "run_path {path} does not end with .vibe/trace/{run_id}")
            }
            E::UnavailableWithRunPath => {
                write!(
                    f,
                    "an unavailable trace carries a run path; a trace that never opened has none"
                )
            }
            E::ActiveWithoutRunPath { status } => write!(
                f,
                "a `{}` trace carries no run_path; only an unavailable trace names no directory",
                status_spelling(status)
            ),
            E::UnavailableFinalised => write!(
                f,
                "an unavailable trace claims to be finalised; it never reached a terminal state"
            ),
            E::UnavailableBudgetExhausted => write!(
                f,
                "an unavailable trace reports an exhausted snapshot budget; a recorder that never \
                 opened never owned one"
            ),
            E::UnavailableNonZero { field, carried } => write!(
                f,
                "an unavailable trace carries {field} = {carried}; nothing was recorded, so every count is 0"
            ),
            E::UnavailableWithTimings => write!(
                f,
                "an unavailable trace carries timing rows; no pass ever ran under it"
            ),
            E::UnavailableSilent => write!(
                f,
                "an unavailable trace carries no warning; the bounded reason is the member's whole point"
            ),
            E::UnavailableBlankReason { warnings } => write!(
                f,
                "an unavailable trace carries {warnings} blank warning(s); an empty or \
                 whitespace-only string is not a reason"
            ),
            E::RunningFinalised => write!(
                f,
                "a running trace claims to be finalised; a parked run is resumed, not terminal"
            ),
            E::TerminalNotFinalised { status } => write!(
                f,
                "a `{}` trace is not finalised; terminal statuses finalise their index",
                status_spelling(status)
            ),
            E::SnapshotsExceedEvents { events, snapshots } => write!(
                f,
                "snapshots = {snapshots} exceeds events = {events}; only an ok event certifies a snapshot"
            ),
            E::WarningOverCap { index, bytes } => write!(
                f,
                "warning {index} is {bytes} bytes, over the shared {DIAGNOSTIC_CAP} byte cap",
                DIAGNOSTIC_CAP = crate::behaviour::compiler_trace_index::DIAGNOSTIC_CAP_BYTES
            ),
            E::TimingPassUnsafe { row, pass } => {
                write!(f, "timing row {row} carries an unusable pass name {pass}")
            }
            E::TimingPassDuplicate { row, pass } => write!(
                f,
                "timing row {row} repeats pass {pass}; one pass name gets one row"
            ),
            E::NonCanonicalDuration { row, pass, column } => write!(
                f,
                "timing row {row} (pass {pass}) carries a non-canonical {column}: \
                 `saturated` is legal only at micros = u32::MAX"
            ),
        }
    }
}
