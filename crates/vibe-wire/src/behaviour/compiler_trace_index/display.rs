//! Human rendering of the typed refusals — split out of `errors.rs` so
//! both files stay inside the project's 600-line cap.
//!
//! Every line begins with the law label, so a log reader gets the same
//! join key `TraceIndexError::law()` returns, and every untrusted scalar
//! goes through [`super::ScalarPreview`]'s bounded `Display` — a refusal
//! is a constant-size line whatever the document weighed.

use std::fmt;

use super::errors::{DurationSite, SnapshotUnsafety, TraceIndexError};
use super::{DIAGNOSTIC_CAP_BYTES, SCHEMA_EPOCH};

impl fmt::Display for SnapshotUnsafety {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SnapshotUnsafety::TooLong { bytes } => {
                write!(f, "the name is {bytes} bytes, past the 96-byte ceiling")
            }
            SnapshotUnsafety::NotCanonical { full, short } => match full {
                Some(full) => write!(f, "this event writes {full} or {short}"),
                None => write!(
                    f,
                    "the full form would pass the ceiling, so this event writes {short}"
                ),
            },
        }
    }
}

impl fmt::Display for TraceIndexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use TraceIndexError as E;
        match self {
            E::SchemaEpoch { schema } => write!(
                f,
                "schema-epoch: root schema is {schema}, expected {SCHEMA_EPOCH}"
            ),
            E::RunIdNotLowercaseHex { run_id } => write!(
                f,
                "scalar-gates: run_id {run_id} is not exactly 32 lowercase hex characters"
            ),
            E::RootDigestMalformed { root_digest } => write!(
                f,
                "scalar-gates: root_digest {root_digest} is not `sha256:` + 64 lowercase hex"
            ),
            E::UnsafeScalar { field, value } => write!(
                f,
                "scalar-gates: {field} {value} is blank or carries CR/LF/NUL"
            ),
            E::ProjectDisplayNotRoot { display } => write!(
                f,
                "scalar-gates: project.display {display} is not the epoch-1 root spelling \".\""
            ),
            E::CustomTargetCharset { scope, target } => write!(
                f,
                "scalar-gates: scope {scope} custom target {target} is not \
                 [a-z0-9][a-z0-9._-]{{0,63}}"
            ),
            E::FinishedWhileRunning => {
                write!(f, "timestamp-coherence: a running index carries finished")
            }
            E::FinishedBeforeStarted { started, finished } => write!(
                f,
                "timestamp-coherence: finished {finished} precedes started {started}"
            ),
            E::DuplicateScopeId { id } => write!(f, "scope-identity: scope id {id} repeats"),
            E::UnknownEventScope { sequence, scope } => write!(
                f,
                "scope-identity: event {sequence} names undeclared scope {scope}"
            ),
            E::ScopeStatusIncoherent {
                scope,
                status,
                fingerprint,
                failure,
            } => write!(
                f,
                "scope-status-coherence: scope {scope} is {status:?} carrying \
                 fingerprint={fingerprint} failure={failure}"
            ),
            E::SkippedScopeHasEvents { scope, first_event } => write!(
                f,
                "skipped-scope-is-silent: skipped scope {scope} has pass events \
                 (first at sequence {first_event})"
            ),
            E::SequenceNotDense { position, sequence } => write!(
                f,
                "sequence-density: events[{position}] carries sequence {sequence}"
            ),
            E::SequenceOverflow { position } => write!(
                f,
                "sequence-density: events[{position}] is past the uint32 sequence ceiling"
            ),
            E::InvocationNotDense {
                scope,
                pass,
                expected,
                invocation,
            } => write!(
                f,
                "invocation-key: (scope {scope}, pass {pass}) is at ordinal {invocation}, \
                 encounter order wants {expected}"
            ),
            E::InvocationOverflow { scope, pass } => write!(
                f,
                "invocation-key: (scope {scope}, pass {pass}) ran past the uint32 \
                 ordinal ceiling"
            ),
            E::IllegalShape {
                sequence,
                which,
                level,
                cardinality,
            } => write!(
                f,
                "shape-ladder: event {sequence} {which} shape {level:?}/{cardinality:?} \
                 is off the IR ladder"
            ),
            E::EventIncoherent {
                sequence,
                status,
                field,
                expected,
            } => write!(
                f,
                "event-coherence: event {sequence} ({status:?}) {} {}",
                if *expected {
                    "must carry"
                } else {
                    "must not carry"
                },
                field.wire_name()
            ),
            E::NonCanonicalDuration { site, micros } => match site {
                DurationSite::Event { sequence, field } => write!(
                    f,
                    "event-coherence: event {sequence} {} is saturated at {micros} micros, \
                     not at the u32 ceiling",
                    field.wire_name()
                ),
                DurationSite::Aggregate { pass, column } => write!(
                    f,
                    "aggregate-reconciliation: pass {pass} {} is saturated at {micros} micros, \
                     not at the u32 ceiling",
                    column.wire_name()
                ),
            },
            E::UnsafeSnapshot {
                sequence,
                filename,
                reason,
            } => write!(
                f,
                "snapshot-portability: event {sequence} snapshot {filename} is not canonical: \
                 {reason}"
            ),
            E::DuplicateSnapshot {
                filename,
                first,
                second,
            } => write!(
                f,
                "snapshot-portability: snapshot {filename} is claimed by events {first} \
                 and {second}"
            ),
            E::FailureOutsideFailedRun { status } => {
                write!(f, "root-coherence: a root failure rides status {status:?}")
            }
            E::FailedRunWithoutFailure => {
                write!(f, "root-coherence: a failed root carries no failure")
            }
            E::TerminalWithoutFinished { status } => write!(
                f,
                "root-coherence: terminal status {status:?} carries no finished"
            ),
            E::OkWithPendingScope { scope } => write!(
                f,
                "root-coherence: root ok with scope {scope} still pending"
            ),
            E::OkWithFailedScope { scope } => {
                write!(f, "root-coherence: scope {scope} failed under root ok")
            }
            E::OkWithFailedEvent { sequence, status } => write!(
                f,
                "root-coherence: event {sequence} is {status:?} — a compile failure — under root ok"
            ),
            E::AggregateRowMissing { pass } => write!(
                f,
                "aggregate-reconciliation: pass {pass} has events but no timing row"
            ),
            E::AggregateRowUnknown { pass } => write!(
                f,
                "aggregate-reconciliation: timing row {pass} names a pass no event carries"
            ),
            E::AggregateRowDuplicate { pass } => write!(
                f,
                "aggregate-reconciliation: pass {pass} carries two timing rows"
            ),
            E::AggregateRowOutOfOrder {
                position,
                carried,
                expected,
            } => write!(
                f,
                "aggregate-reconciliation: aggregates[{position}] is {carried}, \
                 first-appearance order wants {expected}"
            ),
            E::AggregateCountMismatch {
                pass,
                carried,
                actual,
            } => write!(
                f,
                "aggregate-reconciliation: pass {pass} row counts {carried} invocations, \
                 events carry {actual}"
            ),
            E::AggregateCountOverflow { pass } => write!(
                f,
                "aggregate-reconciliation: pass {pass} has more events than a uint32 can count"
            ),
            E::AggregateDurationMismatch {
                pass,
                column,
                carried,
                recomputed,
            } => write!(
                f,
                "aggregate-reconciliation: pass {pass} {} carries \
                 {} micros (saturated {}), recomputed {} micros (saturated {})",
                column.wire_name(),
                carried.micros,
                carried.saturated,
                recomputed.micros,
                recomputed.saturated
            ),
            E::DiagnosticOverCap { site, bytes } => write!(
                f,
                "diagnostic-cap: {site:?} text is {bytes} bytes, cap is {DIAGNOSTIC_CAP_BYTES}"
            ),
        }
    }
}
