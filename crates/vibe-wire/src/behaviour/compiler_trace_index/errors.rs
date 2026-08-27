//! The typed refusals of the trace index's relational laws, and the
//! bounded rendering every untrusted wire scalar passes through before
//! it reaches one. The `Display` half lives in `display.rs`, so both
//! files stay inside the project's 600-line cap.
//!
//! An index is read from disk: `run_id`, a scope id, a snapshot
//! filename or a pass name can be a multi-megabyte string a corrupt or
//! hostile writer left behind. An error that cloned such a value would
//! turn one bad document into a second full-size allocation and then
//! print it — so no variant here carries a wire string. Every one
//! carries a [`ScalarPreview`]: a bounded head plus the true byte
//! length, or an index/field/name that is bounded by construction (a
//! canonical snapshot name is at most 96 bytes and is BUILT, not
//! copied). The diagnostics themselves are separately capped by
//! `diagnostic-cap`; this is the same discipline for the identity
//! scalars, which have no cap of their own.

use std::fmt;

use crate::generated::compiler_trace_index::e1::index::{
    Duration, IrCardinality, IrLevel, PassStatus, RunStatus, ScopeStatus, Timestamp,
};

/// How many bytes of an untrusted scalar an error may retain. Enough to
/// recognise the offender in a log line, small enough that a refusal
/// costs a constant allocation whatever the document did.
pub const SCALAR_PREVIEW_BYTES: usize = 64;

/// A bounded rendering of one untrusted wire scalar: at most
/// [`SCALAR_PREVIEW_BYTES`] bytes of the head (cut on a character
/// boundary, never mid-UTF-8) plus the full byte length of the original.
///
/// The pair is what a reader actually needs — *which* value, and *how
/// big it really was* — without the error owning the value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScalarPreview {
    head: String,
    bytes: usize,
}

impl ScalarPreview {
    /// Bound one wire scalar.
    pub fn of(value: &str) -> Self {
        let mut end = SCALAR_PREVIEW_BYTES.min(value.len());
        while end > 0 && !value.is_char_boundary(end) {
            end -= 1;
        }
        ScalarPreview {
            head: value[..end].to_string(),
            bytes: value.len(),
        }
    }

    /// The retained head — at most [`SCALAR_PREVIEW_BYTES`] bytes.
    pub fn head(&self) -> &str {
        &self.head
    }

    /// The byte length of the original scalar, however large it was.
    pub fn bytes(&self) -> usize {
        self.bytes
    }

    /// Whether the head is shorter than the original.
    pub fn is_truncated(&self) -> bool {
        self.head.len() < self.bytes
    }
}

impl fmt::Display for ScalarPreview {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_truncated() {
            write!(f, "{:?}… ({} bytes)", self.head, self.bytes)
        } else {
            write!(f, "{:?}", self.head)
        }
    }
}

/// One broken relational law, with the context needed to name the
/// offender. Typed end to end — no stringly `detail` — so a test can
/// assert the exact family a mutation lands in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TraceIndexError {
    /// `schema-epoch` — the root `schema` is not 1.
    SchemaEpoch { schema: u32 },
    /// `scalar-gates` — `run_id` is not exactly 32 lowercase hex.
    RunIdNotLowercaseHex { run_id: ScalarPreview },
    /// `scalar-gates` — `root_digest` is not `sha256:` + 64 lowercase hex.
    RootDigestMalformed { root_digest: ScalarPreview },
    /// `scalar-gates` — an identity scalar is blank/whitespace-only or
    /// carries CR, LF or NUL. `field` names the member in wire spelling.
    UnsafeScalar {
        field: &'static str,
        value: ScalarPreview,
    },
    /// `scalar-gates` — epoch-1 `project.display` is exactly `"."`; any
    /// other spelling is an absolute developer path leaking into the
    /// trace (the outer node/unit labels live in the scopes).
    ProjectDisplayNotRoot { display: ScalarPreview },
    /// `scalar-gates` — a custom (open-vocabulary) artifact target does
    /// not match the backend id charset `[a-z0-9][a-z0-9._-]{0,63}`.
    CustomTargetCharset {
        scope: ScalarPreview,
        target: ScalarPreview,
    },
    /// `timestamp-coherence` — a `running` index carries `finished`.
    FinishedWhileRunning,
    /// `timestamp-coherence` — `finished` precedes `started`.
    FinishedBeforeStarted {
        started: Timestamp,
        finished: Timestamp,
    },
    /// `scope-identity` — a scope id repeats.
    DuplicateScopeId { id: ScalarPreview },
    /// `scope-identity` — an event names a scope the index never declared.
    UnknownEventScope { sequence: u32, scope: ScalarPreview },
    /// `scope-status-coherence` — the fingerprint/failure pair does not
    /// match the scope's status.
    ScopeStatusIncoherent {
        scope: ScalarPreview,
        status: ScopeStatus,
        fingerprint: bool,
        failure: bool,
    },
    /// `skipped-scope-is-silent` — a skipped scope carries pass events.
    SkippedScopeHasEvents {
        scope: ScalarPreview,
        first_event: u32,
    },
    /// `sequence-density` — `events[i].sequence != i`.
    SequenceNotDense { position: usize, sequence: u32 },
    /// `sequence-density` — the list is longer than a `uint32` sequence
    /// can address, so no dense numbering exists in this epoch. Refused,
    /// never truncated to a wrapped `u32`.
    SequenceOverflow { position: usize },
    /// `invocation-key` — the `(scope, pass)` invocation ordinals are not
    /// `0..D-1` in encounter order. One variant covers all three ways to
    /// break that: starting above zero, leaving a gap, and repeating or
    /// reordering an ordinal already spent.
    InvocationNotDense {
        scope: ScalarPreview,
        pass: ScalarPreview,
        expected: u32,
        invocation: u32,
    },
    /// `invocation-key` — one `(scope, pass)` ran more times than a
    /// `uint32` ordinal can number. Refused, never wrapped.
    InvocationOverflow {
        scope: ScalarPreview,
        pass: ScalarPreview,
    },
    /// `shape-ladder` — an input/output shape's level and cardinality
    /// pair off the IR ladder.
    IllegalShape {
        sequence: u32,
        which: &'static str,
        level: IrLevel,
        cardinality: IrCardinality,
    },
    /// `event-coherence` — a member the status requires is absent, or one
    /// it forbids is present.
    EventIncoherent {
        sequence: u32,
        status: PassStatus,
        field: EventField,
        expected: bool,
    },
    /// `event-coherence` / `aggregate-reconciliation` — a duration claims
    /// saturation without sitting at the ceiling. `saturated` means
    /// "the true value was at least `u32::MAX`", so it is legal only at
    /// `micros == u32::MAX`; an unsaturated `u32::MAX` stays legal (an
    /// exact measurement may land exactly there).
    NonCanonicalDuration { site: DurationSite, micros: u32 },
    /// `snapshot-portability` — the filename is not one of the two names
    /// this event's `(sequence, pass, kind, label, artifact, ordinal)`
    /// may spell.
    UnsafeSnapshot {
        sequence: u32,
        filename: ScalarPreview,
        reason: SnapshotUnsafety,
    },
    /// `snapshot-portability` — two events claim one snapshot file.
    DuplicateSnapshot {
        filename: ScalarPreview,
        first: u32,
        second: u32,
    },
    /// `root-coherence` — a failure rides a non-`failed` root.
    FailureOutsideFailedRun { status: RunStatus },
    /// `root-coherence` — a `failed` root carries no failure.
    FailedRunWithoutFailure,
    /// `root-coherence` — a terminal root (`ok`/`failed`) has no `finished`.
    TerminalWithoutFinished { status: RunStatus },
    /// `root-coherence` — root `ok` with a scope still pending.
    OkWithPendingScope { scope: ScalarPreview },
    /// `root-coherence` — a scope failure hidden by root `ok`.
    OkWithFailedScope { scope: ScalarPreview },
    /// `root-coherence` — a COMPILE failure (`pass-failed` or
    /// `verification-failed`) hidden by root `ok`. A `snapshot-failed` or
    /// `snapshot-skipped-budget` event is deliberately NOT one of these:
    /// the trace observer failing to write does not fail the compile.
    OkWithFailedEvent { sequence: u32, status: PassStatus },
    /// `aggregate-reconciliation` — a pass with events has no row.
    AggregateRowMissing { pass: ScalarPreview },
    /// `aggregate-reconciliation` — a row names a pass no event carries.
    AggregateRowUnknown { pass: ScalarPreview },
    /// `aggregate-reconciliation` — one pass got two rows.
    AggregateRowDuplicate { pass: ScalarPreview },
    /// `aggregate-reconciliation` — the rows are not in first-appearance
    /// order, so the CLI table would reorder between two runs of one
    /// compile. Same set, different order, still red.
    AggregateRowOutOfOrder {
        position: usize,
        carried: ScalarPreview,
        expected: ScalarPreview,
    },
    /// `aggregate-reconciliation` — the row's invocation count is wrong.
    AggregateCountMismatch {
        pass: ScalarPreview,
        carried: u32,
        actual: u32,
    },
    /// `aggregate-reconciliation` — more events carry one pass name than
    /// a `uint32` count can hold. Refused, never wrapped.
    AggregateCountOverflow { pass: ScalarPreview },
    /// `aggregate-reconciliation` — a carried total does not equal the
    /// saturating recomputation.
    AggregateDurationMismatch {
        pass: ScalarPreview,
        column: TimingColumn,
        carried: Duration,
        recomputed: Duration,
    },
    /// `diagnostic-cap` — a failure/diagnostic text exceeds the cap.
    DiagnosticOverCap { site: DiagnosticSite, bytes: usize },
}

/// Which event member an [`TraceIndexError::EventIncoherent`] is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventField {
    Snapshot,
    Diagnostic,
    PassMicros,
    VerifyMicros,
    EncodeMicros,
}

impl EventField {
    pub(super) fn wire_name(self) -> &'static str {
        match self {
            EventField::Snapshot => "snapshot",
            EventField::Diagnostic => "diagnostic",
            EventField::PassMicros => "pass_micros",
            EventField::VerifyMicros => "verify_micros",
            EventField::EncodeMicros => "encode_micros",
        }
    }
}

/// Where a non-canonical duration was found. The site decides the law:
/// an event's own duration is `event-coherence`, an aggregate total is
/// `aggregate-reconciliation`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DurationSite {
    Event {
        sequence: u32,
        field: EventField,
    },
    Aggregate {
        pass: ScalarPreview,
        column: TimingColumn,
    },
}

/// Why a snapshot filename is not one this event may have written.
///
/// There are exactly two, because the law is CONSTRUCTED rather than
/// pattern-matched: the name is either too long for a canonical form to
/// exist, or it is not the canonical form. A raw `-` inside a component,
/// an over-encoded `%41`, a two-digit ordinal, a lowercase escape, the
/// wrong pass, the wrong scope kind and an invented digest are all the
/// same finding — "that is not what this event writes" — and the
/// expected spellings say so exactly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SnapshotUnsafety {
    /// Longer than the 96-byte ceiling both canonical forms obey.
    TooLong { bytes: usize },
    /// Neither the canonical full name (absent when it would itself pass
    /// the ceiling, leaving the short form the only legal spelling) nor
    /// the canonical short name.
    NotCanonical {
        full: Option<ScalarPreview>,
        short: ScalarPreview,
    },
}

/// Which aggregate column failed to reconcile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimingColumn {
    Pass,
    Verify,
    Encode,
}

impl TimingColumn {
    pub(super) fn wire_name(self) -> &'static str {
        match self {
            TimingColumn::Pass => "pass_total",
            TimingColumn::Verify => "verify_total",
            TimingColumn::Encode => "encode_total",
        }
    }
}

/// Where an over-cap diagnostic text lives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiagnosticSite {
    RunFailure,
    ScopeFailure { scope: ScalarPreview },
    EventDiagnostic { sequence: u32 },
}

impl TraceIndexError {
    /// The implemented-law label this violation witnesses — the join key
    /// the schema parity test reads.
    pub fn law(&self) -> &'static str {
        match self {
            TraceIndexError::SchemaEpoch { .. } => "schema-epoch",
            TraceIndexError::RunIdNotLowercaseHex { .. }
            | TraceIndexError::RootDigestMalformed { .. }
            | TraceIndexError::UnsafeScalar { .. }
            | TraceIndexError::ProjectDisplayNotRoot { .. }
            | TraceIndexError::CustomTargetCharset { .. } => "scalar-gates",
            TraceIndexError::FinishedWhileRunning
            | TraceIndexError::FinishedBeforeStarted { .. } => "timestamp-coherence",
            TraceIndexError::DuplicateScopeId { .. }
            | TraceIndexError::UnknownEventScope { .. } => "scope-identity",
            TraceIndexError::ScopeStatusIncoherent { .. } => "scope-status-coherence",
            TraceIndexError::SkippedScopeHasEvents { .. } => "skipped-scope-is-silent",
            TraceIndexError::SequenceNotDense { .. } | TraceIndexError::SequenceOverflow { .. } => {
                "sequence-density"
            }
            TraceIndexError::InvocationNotDense { .. }
            | TraceIndexError::InvocationOverflow { .. } => "invocation-key",
            TraceIndexError::IllegalShape { .. } => "shape-ladder",
            TraceIndexError::EventIncoherent { .. } => "event-coherence",
            TraceIndexError::NonCanonicalDuration { site, .. } => match site {
                DurationSite::Event { .. } => "event-coherence",
                DurationSite::Aggregate { .. } => "aggregate-reconciliation",
            },
            TraceIndexError::UnsafeSnapshot { .. } | TraceIndexError::DuplicateSnapshot { .. } => {
                "snapshot-portability"
            }
            TraceIndexError::FailureOutsideFailedRun { .. }
            | TraceIndexError::FailedRunWithoutFailure
            | TraceIndexError::TerminalWithoutFinished { .. }
            | TraceIndexError::OkWithPendingScope { .. }
            | TraceIndexError::OkWithFailedScope { .. }
            | TraceIndexError::OkWithFailedEvent { .. } => "root-coherence",
            TraceIndexError::AggregateRowMissing { .. }
            | TraceIndexError::AggregateRowUnknown { .. }
            | TraceIndexError::AggregateRowDuplicate { .. }
            | TraceIndexError::AggregateRowOutOfOrder { .. }
            | TraceIndexError::AggregateCountMismatch { .. }
            | TraceIndexError::AggregateCountOverflow { .. }
            | TraceIndexError::AggregateDurationMismatch { .. } => "aggregate-reconciliation",
            TraceIndexError::DiagnosticOverCap { .. } => "diagnostic-cap",
        }
    }
}

impl std::error::Error for TraceIndexError {}
