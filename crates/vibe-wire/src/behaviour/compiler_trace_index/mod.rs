//! The compile trace index's relational laws — the hand-written
//! validation cell beside the generated reader
//! (`generated::compiler_trace_index::e1::index`), implementing
//! `spec://org.vibevm.core/vibevm/common/PROP-054#OBS-TRACE` (R3.4, the
//! `-print-after-all` / `-time-passes` genre) at the metadata half of
//! the surface.
//!
//! JTD owns the FORM (closed status vocabularies that refuse unknown
//! spellings, timestamp typing, the saturated `duration` record); the
//! laws a form cannot say — density, uniqueness, cross-references,
//! status matrices, snapshot portability, aggregate reconciliation —
//! are named in the schema's `metadata.x-relational-laws` and enforced
//! HERE, in one pure pass over the generated type with typed errors.
//! The two label sets are pinned equal by
//! `tests/compiler_trace_index_wire_corpus.rs`, so an undocumented law
//! and an unimplemented label are both red (the same seam the compiler
//! IR's `x-conversion-gates` already carries).
//!
//! The pass is pure metadata: no filesystem, no clock, no compiler or
//! workspace dependency — the index describes passes, it never touches
//! them. Reading is PERMISSIVE at the object boundary by registry
//! policy (`foreign_parsers = "many"`: a newer writer's extra member is
//! carried and ignored); this cell is not a stricter ad-hoc reader and
//! adds no unknown-member refusal PROP-044 does not compute.
//!
//! Every value it reads is untrusted — an index is a file on disk, and
//! a failed compile deliberately leaves a partial one behind. So no
//! refusal clones the offending scalar: errors carry a bounded
//! [`ScalarPreview`] and the true byte length (`errors`), and the
//! arithmetic is checked or saturating throughout (`aggregates`).

use std::collections::{BTreeMap, BTreeSet};

use crate::generated::compiler_trace_index::e1::index::{
    ArtifactTarget, CompilerTraceIndex, Duration, IrCardinality, IrLevel, PassEvent, PassStatus,
    RunStatus, Scope, ScopeStatus,
};

mod aggregates;
mod display;
mod errors;
mod scalars;
mod snapshot;

pub use aggregates::build_aggregates;
pub use errors::{
    DiagnosticSite, DurationSite, EventField, SCALAR_PREVIEW_BYTES, ScalarPreview,
    SnapshotUnsafety, TimingColumn, TraceIndexError,
};
pub use snapshot::{SHORT_DIGEST_HEX, SNAPSHOT_NAME_CAP, SnapshotName};

use aggregates::{aggregate_gate, canonical_gate};
use scalars::{ROOT_DISPLAY, is_backend_id, scalar_gate};

// The two pure predicates the sibling command-report trace cell shares:
// the canonical-duration rule and the 32-hex run-id rule are ONE law
// each — the report validator reuses them rather than restating them.
pub(crate) use aggregates::is_canonical;
pub(crate) use scalars::is_lowercase_hex;
use snapshot::{SnapshotIdentity, kind_spelling, snapshot_unsafety};

#[cfg(test)]
mod tests;

/// The byte cap every failure/diagnostic text obeys — the schema's
/// `metadata.x-diagnostic-cap-bytes` value (8 KiB, the conservative cap
/// the schema documents). Pinned equal by the wire-corpus test.
pub const DIAGNOSTIC_CAP_BYTES: usize = 8 * 1024;

/// Every implemented law label, in schema order. Set-equal to the
/// schema's `x-relational-laws` prefixes by the wire-corpus test: a law
/// the validator enforces but the schema does not name, and a label the
/// schema names but no code answers, are both red.
pub const IMPLEMENTED_LAWS: &[&str] = &[
    "schema-epoch",
    "scalar-gates",
    "timestamp-coherence",
    "scope-identity",
    "scope-status-coherence",
    "skipped-scope-is-silent",
    "sequence-density",
    "invocation-key",
    "shape-ladder",
    "event-coherence",
    "snapshot-portability",
    "root-coherence",
    "aggregate-reconciliation",
    "diagnostic-cap",
];

/// The wire epoch this validator reads. Anything else is refused before
/// a single field is interpreted.
pub const SCHEMA_EPOCH: u32 = 1;

/// Validate one compile trace index against every relational law. Pure:
/// the value in, the first broken law out.
pub fn validate(index: &CompilerTraceIndex) -> Result<(), TraceIndexError> {
    if index.schema != SCHEMA_EPOCH {
        return Err(TraceIndexError::SchemaEpoch {
            schema: index.schema,
        });
    }
    identity_gate(index)?;
    timestamp_gate(index)?;

    let scopes = scope_pass(index)?;
    event_pass(index, &scopes)?;
    root_pass(index)?;
    aggregate_gate(&index.events, &index.aggregates)
}

/// `scalar-gates` at the root: the run id, the project digest, and the
/// epoch-1 project display.
fn identity_gate(index: &CompilerTraceIndex) -> Result<(), TraceIndexError> {
    if !is_lowercase_hex(&index.run_id, 32) {
        return Err(TraceIndexError::RunIdNotLowercaseHex {
            run_id: ScalarPreview::of(&index.run_id),
        });
    }
    let digest = &index.project.root_digest;
    if !(digest.starts_with("sha256:") && is_lowercase_hex(digest.get(7..).unwrap_or(""), 64)) {
        return Err(TraceIndexError::RootDigestMalformed {
            root_digest: ScalarPreview::of(digest),
        });
    }
    scalar_gate("project.display", &index.project.display)?;
    if index.project.display != ROOT_DISPLAY {
        return Err(TraceIndexError::ProjectDisplayNotRoot {
            display: ScalarPreview::of(&index.project.display),
        });
    }
    Ok(())
}

/// `timestamp-coherence`: `running` carries no `finished`, and a
/// `finished` never precedes `started`.
fn timestamp_gate(index: &CompilerTraceIndex) -> Result<(), TraceIndexError> {
    if index.status == RunStatus::Running && index.finished.is_some() {
        return Err(TraceIndexError::FinishedWhileRunning);
    }
    if let Some(finished) = index.finished
        && finished < index.started
    {
        return Err(TraceIndexError::FinishedBeforeStarted {
            started: index.started,
            finished,
        });
    }
    Ok(())
}

/// What the scope pass hands the event pass: the declared scopes by id
/// (the snapshot name needs each one's kind, label and artifact), and
/// which of them are skipped and therefore must stay silent.
struct DeclaredScopes<'a> {
    declared: BTreeMap<&'a str, &'a Scope>,
    skipped: BTreeSet<&'a str>,
}

/// `scope-identity`, `scope-status-coherence` and the fingerprint half
/// of `skipped-scope-is-silent`.
fn scope_pass(index: &CompilerTraceIndex) -> Result<DeclaredScopes<'_>, TraceIndexError> {
    let mut declared = DeclaredScopes {
        declared: BTreeMap::new(),
        skipped: BTreeSet::new(),
    };
    for scope in &index.scopes {
        if declared.declared.insert(scope.id.as_str(), scope).is_some() {
            return Err(TraceIndexError::DuplicateScopeId {
                id: ScalarPreview::of(&scope.id),
            });
        }
        scope_scalars(scope)?;
        let expects_fingerprint =
            matches!(scope.status, ScopeStatus::Compiled | ScopeStatus::Skipped);
        let expects_failure = scope.status == ScopeStatus::Failed;
        if scope.fingerprint.is_some() != expects_fingerprint
            || scope.failure.is_some() != expects_failure
        {
            return Err(TraceIndexError::ScopeStatusIncoherent {
                scope: ScalarPreview::of(&scope.id),
                status: scope.status.clone(),
                fingerprint: scope.fingerprint.is_some(),
                failure: scope.failure.is_some(),
            });
        }
        if let Some(failure) = &scope.failure {
            cap_gate(
                &DiagnosticSite::ScopeFailure {
                    scope: ScalarPreview::of(&scope.id),
                },
                failure,
            )?;
        }
        if scope.status == ScopeStatus::Skipped {
            declared.skipped.insert(scope.id.as_str());
        }
    }
    Ok(declared)
}

/// `scalar-gates` on one scope's identity members, including the open
/// target vocabulary: a custom backend's spelling must be the identity
/// the compiler itself would have accepted.
fn scope_scalars(scope: &Scope) -> Result<(), TraceIndexError> {
    scalar_gate("scope.id", &scope.id)?;
    scalar_gate("scope.label", &scope.label)?;
    scalar_gate("scope.artifact", &scope.artifact)?;
    scalar_gate("scope.target", target_spelling(&scope.target))?;
    if let ArtifactTarget::Unknown(custom) = &scope.target
        && !is_backend_id(custom)
    {
        return Err(TraceIndexError::CustomTargetCharset {
            scope: ScalarPreview::of(&scope.id),
            target: ScalarPreview::of(custom),
        });
    }
    if let Some(fingerprint) = &scope.fingerprint {
        scalar_gate("scope.fingerprint", fingerprint)?;
    }
    Ok(())
}

/// `sequence-density`, the reference half of `scope-identity`,
/// `invocation-key`, `shape-ladder`, `event-coherence`,
/// `snapshot-portability` and the silence half of
/// `skipped-scope-is-silent`.
fn event_pass(
    index: &CompilerTraceIndex,
    scopes: &DeclaredScopes<'_>,
) -> Result<(), TraceIndexError> {
    // The NEXT ordinal each `(scope, pass)` may spend. Uniqueness is not
    // enough: `parse` over D documents produces exactly `0..D-1` in
    // encounter order, so a start at 7, a gap, and a repeat are one law.
    let mut next_ordinal: BTreeMap<(&str, &str), u32> = BTreeMap::new();
    let mut snapshots: BTreeMap<&str, u32> = BTreeMap::new();
    for (position, event) in index.events.iter().enumerate() {
        let sequence = event.sequence;
        if sequence != dense_sequence(position)? {
            return Err(TraceIndexError::SequenceNotDense { position, sequence });
        }
        let Some(scope) = scopes.declared.get(event.scope.as_str()) else {
            return Err(TraceIndexError::UnknownEventScope {
                sequence,
                scope: ScalarPreview::of(&event.scope),
            });
        };
        scalar_gate("event.pass", &event.pass)?;
        invocation_gate(&mut next_ordinal, event)?;
        shape_ladder_gate(
            sequence,
            "input",
            &event.input_shape.level,
            &event.input_shape.cardinality,
        )?;
        shape_ladder_gate(
            sequence,
            "output",
            &event.output_shape.level,
            &event.output_shape.cardinality,
        )?;
        event_coherence_gate(event)?;
        if let Some(filename) = &event.snapshot {
            let identity = SnapshotIdentity {
                sequence,
                invocation: event.invocation,
                kind: kind_spelling(&scope.kind),
                pass: &event.pass,
                label: &scope.label,
                artifact: &scope.artifact,
            };
            if let Some(reason) = snapshot_unsafety(filename, &identity) {
                return Err(TraceIndexError::UnsafeSnapshot {
                    sequence,
                    filename: ScalarPreview::of(filename),
                    reason,
                });
            }
            if let Some(first) = snapshots.insert(filename.as_str(), sequence) {
                return Err(TraceIndexError::DuplicateSnapshot {
                    filename: ScalarPreview::of(filename),
                    first,
                    second: sequence,
                });
            }
        }
        if let Some(diagnostic) = &event.diagnostic {
            cap_gate(&DiagnosticSite::EventDiagnostic { sequence }, diagnostic)?;
        }
        if scopes.skipped.contains(event.scope.as_str()) {
            return Err(TraceIndexError::SkippedScopeHasEvents {
                scope: ScalarPreview::of(&event.scope),
                first_event: sequence,
            });
        }
    }
    Ok(())
}

/// `invocation-key`: this event must spend exactly the next ordinal its
/// `(scope, pass)` owes, and the counter advances with checked addition.
fn invocation_gate<'a>(
    next_ordinal: &mut BTreeMap<(&'a str, &'a str), u32>,
    event: &'a PassEvent,
) -> Result<(), TraceIndexError> {
    let key = (event.scope.as_str(), event.pass.as_str());
    let expected = next_ordinal.entry(key).or_insert(0);
    if event.invocation != *expected {
        return Err(TraceIndexError::InvocationNotDense {
            scope: ScalarPreview::of(&event.scope),
            pass: ScalarPreview::of(&event.pass),
            expected: *expected,
            invocation: event.invocation,
        });
    }
    advance_invocation(expected, &event.scope, &event.pass)
}

/// Advance one dense invocation counter. Kept as a small boundary so the
/// epoch ceiling is runnable in a unit test without allocating 2^32 events.
fn advance_invocation(expected: &mut u32, scope: &str, pass: &str) -> Result<(), TraceIndexError> {
    *expected = expected
        .checked_add(1)
        .ok_or_else(|| TraceIndexError::InvocationOverflow {
            scope: ScalarPreview::of(scope),
            pass: ScalarPreview::of(pass),
        })?;
    Ok(())
}

/// The dense sequence a list position must carry. A list longer than
/// `uint32` can address has no dense numbering in this epoch — refused,
/// never silently truncated by an `as` cast that would wrap in release.
fn dense_sequence(position: usize) -> Result<u32, TraceIndexError> {
    u32::try_from(position).map_err(|_| TraceIndexError::SequenceOverflow { position })
}

/// `root-coherence`: the root's terminal word matches what the run did.
///
/// The root status is the COMPILE/LIFECYCLE outcome, and the trace is an
/// observer of it. Two consequences the matrix has to respect, or it
/// starts lying about what the compiler did:
///
/// - A `snapshot-failed` or `snapshot-skipped-budget` event is the
///   observer failing or standing down, not the compile failing. Root
///   `ok` MUST admit them — refusing would make enabling `--trace-compile`
///   able to turn a green run red, which is exactly the property a
///   diagnostic switch must not have. Only `pass-failed` and
///   `verification-failed` are compile failures root `ok` refuses.
/// - A root `failed` may be caused AFTER every pass event succeeded (the
///   StaticWrite / boot-transaction rollback case), so a failed scope or
///   failed event is NOT required as evidence. `failed` + a bounded
///   `failure` + `finished` is the whole obligation.
fn root_pass(index: &CompilerTraceIndex) -> Result<(), TraceIndexError> {
    if let Some(failure) = &index.failure {
        cap_gate(&DiagnosticSite::RunFailure, failure)?;
        if index.status != RunStatus::Failed {
            return Err(TraceIndexError::FailureOutsideFailedRun {
                status: index.status.clone(),
            });
        }
    }
    if index.status != RunStatus::Running && index.finished.is_none() {
        return Err(TraceIndexError::TerminalWithoutFinished {
            status: index.status.clone(),
        });
    }
    match index.status {
        // A partial index says nothing about the run's outcome yet: a
        // pending scope, a failed scope and a failed event are all
        // legal mid-run — the terminal word is written last.
        RunStatus::Running => {}
        RunStatus::Ok => {
            if let Some(scope) = index
                .scopes
                .iter()
                .find(|s| s.status == ScopeStatus::Pending)
            {
                return Err(TraceIndexError::OkWithPendingScope {
                    scope: ScalarPreview::of(&scope.id),
                });
            }
            if let Some(scope) = index
                .scopes
                .iter()
                .find(|s| s.status == ScopeStatus::Failed)
            {
                return Err(TraceIndexError::OkWithFailedScope {
                    scope: ScalarPreview::of(&scope.id),
                });
            }
            if let Some(event) = index.events.iter().find(|e| is_compile_failure(&e.status)) {
                return Err(TraceIndexError::OkWithFailedEvent {
                    sequence: event.sequence,
                    status: event.status.clone(),
                });
            }
        }
        RunStatus::Failed => {
            if index.failure.is_none() {
                return Err(TraceIndexError::FailedRunWithoutFailure);
            }
        }
    }
    Ok(())
}

/// Which event statuses mean the COMPILE failed. `snapshot-failed` and
/// `snapshot-skipped-budget` are deliberately absent: they are trace-side
/// observability, and they never propagate into the run's outcome.
fn is_compile_failure(status: &PassStatus) -> bool {
    matches!(
        status,
        PassStatus::PassFailed | PassStatus::VerificationFailed
    )
}

/// `diagnostic-cap` on one failure/diagnostic text.
fn cap_gate(site: &DiagnosticSite, text: &str) -> Result<(), TraceIndexError> {
    let bytes = text.len();
    if bytes > DIAGNOSTIC_CAP_BYTES {
        return Err(TraceIndexError::DiagnosticOverCap {
            site: site.clone(),
            bytes,
        });
    }
    Ok(())
}

/// The wire spelling of a target — the open vocabulary's verbatim string.
fn target_spelling(target: &ArtifactTarget) -> &str {
    match target {
        ArtifactTarget::StaticMd => "static-md",
        ArtifactTarget::StaticXml => "static-xml",
        ArtifactTarget::Unknown(value) => value.as_str(),
    }
}

/// `shape-ladder`: which level/cardinality pairs the IR admits.
fn shape_ladder_gate(
    sequence: u32,
    which: &'static str,
    level: &IrLevel,
    cardinality: &IrCardinality,
) -> Result<(), TraceIndexError> {
    let legal = match level {
        IrLevel::Source => cardinality == &IrCardinality::Document,
        IrLevel::Document => true,
        IrLevel::Closure | IrLevel::Lane | IrLevel::Emitted => {
            cardinality == &IrCardinality::Artifact
        }
    };
    if legal {
        Ok(())
    } else {
        Err(TraceIndexError::IllegalShape {
            sequence,
            which,
            level: level.clone(),
            cardinality: cardinality.clone(),
        })
    }
}

/// What each status's snapshot/diagnostic/duration members must be.
/// `true` = present, `false` = absent. A pass/verifier failure omits the
/// later durations honestly; `ok` certifies a snapshot and carries all
/// three stages; a budget skip never attempted the encode.
fn event_expectation(status: &PassStatus, field: &EventField) -> bool {
    use EventField as F;
    use PassStatus as S;
    match (status, field) {
        (S::Ok, F::Snapshot)
        | (S::Ok, F::PassMicros)
        | (S::Ok, F::VerifyMicros)
        | (S::Ok, F::EncodeMicros) => true,
        (S::Ok, F::Diagnostic) => false,
        (S::SnapshotSkippedBudget, F::PassMicros) | (S::SnapshotSkippedBudget, F::VerifyMicros) => {
            true
        }
        (S::SnapshotSkippedBudget, F::Snapshot)
        | (S::SnapshotSkippedBudget, F::Diagnostic)
        | (S::SnapshotSkippedBudget, F::EncodeMicros) => false,
        (S::PassFailed, F::PassMicros) | (S::PassFailed, F::Diagnostic) => true,
        (S::PassFailed, F::Snapshot)
        | (S::PassFailed, F::VerifyMicros)
        | (S::PassFailed, F::EncodeMicros) => false,
        (S::VerificationFailed, F::PassMicros)
        | (S::VerificationFailed, F::VerifyMicros)
        | (S::VerificationFailed, F::Diagnostic) => true,
        (S::VerificationFailed, F::Snapshot) | (S::VerificationFailed, F::EncodeMicros) => false,
        (S::SnapshotFailed, F::PassMicros)
        | (S::SnapshotFailed, F::VerifyMicros)
        | (S::SnapshotFailed, F::EncodeMicros)
        | (S::SnapshotFailed, F::Diagnostic) => true,
        (S::SnapshotFailed, F::Snapshot) => false,
    }
}

/// `event-coherence` for one event: the status's member matrix, and the
/// canonical form of every duration it does carry.
fn event_coherence_gate(event: &PassEvent) -> Result<(), TraceIndexError> {
    let durations: [(EventField, &Option<Duration>); 3] = [
        (EventField::PassMicros, &event.pass_micros),
        (EventField::VerifyMicros, &event.verify_micros),
        (EventField::EncodeMicros, &event.encode_micros),
    ];
    let members: [(EventField, bool); 5] = [
        (EventField::Snapshot, event.snapshot.is_some()),
        (EventField::Diagnostic, event.diagnostic.is_some()),
        (EventField::PassMicros, event.pass_micros.is_some()),
        (EventField::VerifyMicros, event.verify_micros.is_some()),
        (EventField::EncodeMicros, event.encode_micros.is_some()),
    ];
    for (field, present) in members {
        let expected = event_expectation(&event.status, &field);
        if present != expected {
            return Err(TraceIndexError::EventIncoherent {
                sequence: event.sequence,
                status: event.status.clone(),
                field,
                expected,
            });
        }
    }
    for (field, duration) in durations {
        if let Some(duration) = duration {
            canonical_gate(
                DurationSite::Event {
                    sequence: event.sequence,
                    field,
                },
                duration,
            )?;
        }
    }
    Ok(())
}
