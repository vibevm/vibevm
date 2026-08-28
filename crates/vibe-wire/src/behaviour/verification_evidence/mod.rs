//! The verification-evidence member's relational laws — the
//! hand-written validation cell beside the ONE generated
//! [`VerificationEvidence`] the lifecycle report carries (PROP-054
//! §14.7, `##EVIDENCE-WIRE-AND-SURFACES`; R7.5 P1).
//!
//! JTD owns the FORM (the closed five-value outcome vocabulary, the
//! optional witnesses, the explicit-even-when-empty row lists); the
//! laws a form cannot say are named in the `verification_evidence`
//! fragment's `metadata.x-relational-laws`
//! (`formats/vocabularies.json`) and enforced HERE, in one pure pass
//! over the generated type with typed errors. The two label sets are
//! pinned equal by `tests/verification_evidence_wire.rs`, so an
//! undocumented law and an unimplemented label are both red — the same
//! seam [`crate::behaviour::compile_trace_report`] already carries.
//!
//! Nothing here is a second algorithm for anything a neighbour owns:
//! the 32-hex run-id rule, the `sha256:` spelling, the canonical
//! decimal predicate and the project-relative path grammar all live in
//! [`crate::behaviour::scalars`], shared with the requirements cell,
//! and the diagnostic cap is the trace index's one budget rather than
//! a second one.
//!
//! What this cell does NOT do, on purpose: it does not compute
//! `evidence_id`. P1 validates the SHAPE of an identity and the
//! relations between the members it joins; forging a second digest
//! recipe here would create exactly the reference-implementation split
//! the architecture's §10 rejects. The writer's recipe lands with the
//! writer (P2), and this validator is what it must satisfy.
//!
//! Every value it reads is untrusted — an evidence member is a report
//! on disk or a tool's stdout — so no refusal clones the offending
//! scalar: errors carry a bounded `ScalarPreview` and the true byte
//! length.

use std::collections::BTreeMap;

use crate::behaviour::compiler_trace_index::{DIAGNOSTIC_CAP_BYTES, ScalarPreview};
use crate::behaviour::scalars::{
    has_control_bytes, is_canonical_decimal, is_lowercase_hex, is_sha256, relative_path_defect,
};
use crate::generated::shared::{DigestWitness, EvidenceStatus, VerificationEvidence};

mod errors;
mod rows;

pub use errors::{
    CountDefect, EvidenceError, PathUnsafety, RowKind, RowRef, ShapeDefect, TextUnsafety,
    WitnessHalf,
};
use rows::{Comparison, rows};

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

#[cfg(test)]
#[path = "tests_hardening.rs"]
mod tests_hardening;

/// Every implemented law label, in fragment order. Set-equal to the
/// `verification_evidence` fragment's `x-relational-laws` prefixes by
/// the wire test: a law the validator enforces but the fragment does
/// not name, and a label the fragment names but no code answers, are
/// both red.
pub const IMPLEMENTED_LAWS: &[&str] = &[
    "evidence-identity",
    "run-identity",
    "measurement-identity",
    "witness-shape",
    "comparison-shape",
    "overall-status",
    "row-identity",
    "pattern-safety",
    "path-safety",
    "bounded-identity",
];

/// The evidence wire epoch this validator speaks — the `evidence`
/// member's one legal value today.
pub const EVIDENCE_EPOCH: u32 = 1;

/// Validate one verification-evidence member against every relational
/// law. Pure: the value in, the first broken law out.
pub fn validate(evidence: &VerificationEvidence) -> Result<(), EvidenceError> {
    identity_gate(evidence)?;
    run_gate(evidence)?;
    let mut worst: Option<&EvidenceStatus> = None;
    let mut seen: BTreeMap<(RowKind, &str), usize> = BTreeMap::new();
    for row in rows(evidence) {
        row_gate(&row, &mut seen)?;
        worst = Some(match worst {
            Some(current) if severity(current) >= severity(row.status) => current,
            _ => row.status,
        });
    }
    overall_gate(evidence, worst)
}

/// `evidence-identity`: the epoch this member speaks, and the shape of
/// the id a reader joins claims by.
fn identity_gate(evidence: &VerificationEvidence) -> Result<(), EvidenceError> {
    if evidence.evidence != EVIDENCE_EPOCH {
        return Err(EvidenceError::EvidenceEpoch {
            found: evidence.evidence,
        });
    }
    if !is_sha256(&evidence.evidence_id) {
        return Err(EvidenceError::EvidenceIdShape {
            evidence_id: preview(&evidence.evidence_id),
        });
    }
    Ok(())
}

/// `run-identity`: the run header is the first half of what
/// `##VERIFY-EVIDENCE-IDENTITY` calls the identity tuple, so EVERY
/// member of it is held to a shape — including the ones a reader will
/// print, which is why the chain's phases are checked one by one
/// rather than only counted.
fn run_gate(evidence: &VerificationEvidence) -> Result<(), EvidenceError> {
    let run = &evidence.run;
    if !is_lowercase_hex(&run.run_id, 32) {
        return Err(EvidenceError::RunIdNotLowercaseHex {
            run_id: preview(&run.run_id),
        });
    }
    if run.selected != "."
        && let Some(reason) = relative_path_defect(&run.selected)
    {
        return Err(EvidenceError::UnsafeSelected {
            selected: preview(&run.selected),
            reason,
        });
    }
    run_scalar("selected", &run.selected)?;
    run_scalar("requested", &run.requested)?;
    run_scalar("started", &run.started)?;
    if run.chain.is_empty() {
        return Err(EvidenceError::EmptyChain);
    }
    for (index, phase) in run.chain.iter().enumerate() {
        if let Some(reason) = scalar_defect(phase) {
            return Err(EvidenceError::UnsafeChainPhase {
                index,
                phase: preview(phase),
                reason,
            });
        }
    }
    if !run.chain.contains(&run.requested) {
        return Err(EvidenceError::RequestedOutsideChain {
            requested: preview(&run.requested),
        });
    }
    Ok(())
}

/// One run-header scalar: nonblank, bounded and control-free. The
/// header is what a reader prints first, so a control byte in it is a
/// terminal-rewriting document, not a cosmetic defect.
fn run_scalar(field: &'static str, value: &str) -> Result<(), EvidenceError> {
    match scalar_defect(value) {
        Some(reason) => Err(EvidenceError::UnsafeRunScalar {
            field,
            value: preview(value),
            reason,
        }),
        None => Ok(()),
    }
}

/// The first thing wrong with an identity scalar: blank, over the
/// shared cap, or carrying a byte a reader cannot print.
fn scalar_defect(value: &str) -> Option<TextUnsafety> {
    if value.trim().is_empty() {
        Some(TextUnsafety::Blank)
    } else if value.len() > DIAGNOSTIC_CAP_BYTES {
        Some(TextUnsafety::OverCap)
    } else if has_control_bytes(value) {
        Some(TextUnsafety::ControlByte)
    } else {
        None
    }
}

/// Every per-row law, in the order a reader meets them: who the row is
/// about, whether the scope and path it names are ones this wire may
/// certify, whether its text is usable, who measured it, whether its
/// witnesses are witnesses, and finally what its status may mean.
fn row_gate<'a>(
    row: &Comparison<'a>,
    seen: &mut BTreeMap<(RowKind, &'a str), usize>,
) -> Result<(), EvidenceError> {
    row_identity_gate(row, seen)?;
    path_gate(row)?;
    pattern_gate(row)?;
    bounded_gate(row)?;
    measurement_gate(row)?;
    witness_gate(row)?;
    comparison_gate(row)
}

/// `row-identity`: a nonblank key, and one row per key.
fn row_identity_gate<'a>(
    row: &Comparison<'a>,
    seen: &mut BTreeMap<(RowKind, &'a str), usize>,
) -> Result<(), EvidenceError> {
    if row.key.trim().is_empty() {
        return Err(EvidenceError::RowKeyBlank { row: row.at() });
    }
    if let Some(first) = seen.insert((row.kind, row.key), row.index) {
        return Err(EvidenceError::RowKeyDuplicate {
            row: row.at(),
            first,
        });
    }
    Ok(())
}

/// `path-safety`: an artifact row's `path` is the canonical
/// project-relative path under `run.selected`. Durable state keeps the
/// absolute machine path it needs to reopen the file; the published
/// row carries the portable half, because `C:/Users/<name>/…` in a
/// document an orchestrator may forward is both non-portable and a
/// leak of the operator's home.
fn path_gate(row: &Comparison<'_>) -> Result<(), EvidenceError> {
    let Some(path) = row.path else { return Ok(()) };
    match relative_path_defect(path) {
        Some(reason) => Err(EvidenceError::UnsafeArtifactPath {
            row: row.at(),
            path: preview(path),
            reason,
        }),
        None => Ok(()),
    }
}

/// `pattern-safety`: a declared pattern names a project-relative scope
/// this wire may certify, or it names one it may not.
fn pattern_gate(row: &Comparison<'_>) -> Result<(), EvidenceError> {
    for (index, pattern) in row.patterns.iter().enumerate() {
        if let Some(reason) = relative_path_defect(pattern) {
            return Err(EvidenceError::UnsafePattern {
                row: row.at(),
                index,
                pattern: preview(pattern),
                reason,
            });
        }
    }
    Ok(())
}

/// `bounded-identity`: every identity scalar of the row is nonblank,
/// within the shared cap and control-free; every diagnostic scalar is
/// bounded and control-free. Blankness of a REASON belongs to
/// `comparison-shape` and of an ALGORITHM to `witness-shape`: one
/// defect landing in two laws would make a mutation's verdict depend
/// on check order.
fn bounded_gate(row: &Comparison<'_>) -> Result<(), EvidenceError> {
    for (field, value) in row.identity {
        if let Some(reason) = scalar_defect(value) {
            return Err(EvidenceError::UnsafeScalar {
                row: row.at(),
                field,
                value: preview(value),
                reason,
            });
        }
    }
    let mut diagnostics: Vec<(&'static str, &str)> = Vec::new();
    if let Some(reason) = row.reason_code {
        diagnostics.push(("reason_code", reason));
    }
    for witness in [row.measured, row.observed].into_iter().flatten() {
        diagnostics.push(("algorithm", &witness.algorithm));
        diagnostics.push(("digest", &witness.digest));
    }
    for (field, value) in diagnostics {
        let reason = if value.len() > DIAGNOSTIC_CAP_BYTES {
            Some(TextUnsafety::OverCap)
        } else if has_control_bytes(value) {
            Some(TextUnsafety::ControlByte)
        } else {
            None
        };
        if let Some(reason) = reason {
            return Err(EvidenceError::UnsafeScalar {
                row: row.at(),
                field,
                value: preview(value),
                reason,
            });
        }
    }
    Ok(())
}

/// `measurement-identity`: a measured claim names the run that
/// measured it and the declaration it was measured against.
///
/// The id and the witness are ONE fact, so they are present together
/// or absent together. An id beside no `measured` attributes a
/// measurement the row itself says does not exist — a reader joining
/// evidence by run id would follow it to a claim that was never made;
/// a `measured` beside no id is a measurement nobody can be held to.
/// WHICH statuses may take the absent branch is `comparison-shape`'s
/// (only `unavailable` may carry no `measured`), so this law states
/// the pairing and nothing else.
fn measurement_gate(row: &Comparison<'_>) -> Result<(), EvidenceError> {
    if let Some(fingerprint) = row.declaration_fingerprint
        && !is_sha256(fingerprint)
    {
        return Err(EvidenceError::DeclarationFingerprintShape {
            row: row.at(),
            fingerprint: preview(fingerprint),
        });
    }
    match (row.measured_run_id, row.measured.is_some()) {
        (Some(value), true) => {
            if is_lowercase_hex(value, 32) {
                Ok(())
            } else {
                Err(EvidenceError::MeasuredRunIdNotLowercaseHex {
                    row: row.at(),
                    value: preview(value),
                })
            }
        }
        (Some(value), false) => Err(EvidenceError::MeasuredRunIdOrphaned {
            row: row.at(),
            value: preview(value),
        }),
        (None, true) => Err(EvidenceError::MeasuredRunIdAbsent { row: row.at() }),
        (None, false) => Ok(()),
    }
}

/// `witness-shape`: a witness names its algorithm, carries a sha256
/// digest and a lossless byte count, and carries its counts as ONE
/// pair whose presence the ROW decides.
fn witness_gate(row: &Comparison<'_>) -> Result<(), EvidenceError> {
    for (half, witness) in [
        (WitnessHalf::Measured, row.measured),
        (WitnessHalf::Observed, row.observed),
    ] {
        let Some(witness) = witness else { continue };
        if witness.algorithm.trim().is_empty() {
            return Err(EvidenceError::WitnessAlgorithmBlank {
                row: row.at(),
                half,
            });
        }
        if !is_sha256(&witness.digest) {
            return Err(EvidenceError::WitnessDigestShape {
                row: row.at(),
                half,
                digest: preview(&witness.digest),
            });
        }
        if let Some(bytes) = witness.bytes.as_deref()
            && !is_canonical_decimal(bytes)
        {
            return Err(EvidenceError::NonCanonicalByteCount {
                row: row.at(),
                half,
                value: preview(bytes),
            });
        }
        if let Some(defect) = count_defect(witness, row.counts_expected()) {
            return Err(EvidenceError::WitnessCountShape {
                row: row.at(),
                half,
                defect,
            });
        }
    }
    Ok(())
}

/// The count pair, read from both directions: `files` and `bytes` are
/// present together or absent together, and WHICH it is follows from
/// the row — an input row's witnesses are manifests over a declared
/// file set, an artifact row's are the content itself.
fn count_defect(witness: &DigestWitness, expected: bool) -> Option<CountDefect> {
    match (witness.files.is_some(), witness.bytes.is_some()) {
        (true, false) => Some(CountDefect::BytesMissing),
        (false, true) => Some(CountDefect::FilesMissing),
        (true, true) if !expected => Some(CountDefect::Unexpected),
        (false, false) if expected => Some(CountDefect::Absent),
        _ => None,
    }
}

/// `comparison-shape`: the per-status matrix. Every arm answers the
/// same three questions — which witnesses are present, whether they
/// compare equal, and whether a reason is owed — and the answers are
/// what the five words MEAN.
fn comparison_gate(row: &Comparison<'_>) -> Result<(), EvidenceError> {
    if let Some(reason) = row.reason_code
        && reason.trim().is_empty()
    {
        return Err(row.defect(ShapeDefect::ReasonBlank));
    }
    let equal = matches!((row.measured, row.observed), (Some(left), Some(right)) if left == right);
    match row.status {
        EvidenceStatus::Matched => {
            require(row, row.measured.is_some(), ShapeDefect::MissingMeasured)?;
            require(row, row.observed.is_some(), ShapeDefect::MissingObserved)?;
            require(row, equal, ShapeDefect::UnequalWitnesses)?;
            require(row, row.reason_code.is_none(), ShapeDefect::ReasonPresent)
        }
        EvidenceStatus::Stale => {
            require(row, row.measured.is_some(), ShapeDefect::MissingMeasured)?;
            require(row, row.observed.is_some(), ShapeDefect::MissingObserved)?;
            require(row, !equal, ShapeDefect::EqualWitnesses)
        }
        EvidenceStatus::Missing => {
            require(row, row.measured.is_some(), ShapeDefect::MissingMeasured)?;
            require(row, row.observed.is_none(), ShapeDefect::UnexpectedObserved)?;
            require(row, row.reason_code.is_some(), ShapeDefect::ReasonAbsent)
        }
        EvidenceStatus::Unavailable => {
            require(row, row.measured.is_none(), ShapeDefect::UnexpectedMeasured)?;
            require(row, row.reason_code.is_some(), ShapeDefect::ReasonAbsent)
        }
        EvidenceStatus::Unstable => {
            // Something WAS measured — otherwise the row is
            // `unavailable`, the one honest no-measurement case. What
            // `unstable` refuses is the RE-observation.
            require(row, row.measured.is_some(), ShapeDefect::MissingMeasured)?;
            require(row, row.observed.is_none(), ShapeDefect::UnexpectedObserved)?;
            require(row, row.reason_code.is_some(), ShapeDefect::ReasonAbsent)
        }
    }
}

/// One matrix clause: hold, or name the defect this status could not
/// satisfy.
fn require(row: &Comparison<'_>, held: bool, defect: ShapeDefect) -> Result<(), EvidenceError> {
    if held {
        Ok(())
    } else {
        Err(row.defect(defect))
    }
}

/// `overall-status`: the root never speaks for itself. With no rows it
/// is `unavailable`; with rows it is the worst of them.
fn overall_gate(
    evidence: &VerificationEvidence,
    worst: Option<&EvidenceStatus>,
) -> Result<(), EvidenceError> {
    let expected = worst.cloned().unwrap_or(EvidenceStatus::Unavailable);
    if evidence.status == expected {
        Ok(())
    } else {
        Err(EvidenceError::OverallStatus {
            declared: evidence.status.clone(),
            expected,
        })
    }
}

/// The precedence the root's status is the maximum of:
/// `unstable` > `missing` > `stale` > `unavailable` > `matched`.
fn severity(status: &EvidenceStatus) -> u8 {
    match status {
        EvidenceStatus::Matched => 0,
        EvidenceStatus::Unavailable => 1,
        EvidenceStatus::Stale => 2,
        EvidenceStatus::Missing => 3,
        EvidenceStatus::Unstable => 4,
    }
}

/// One bounded preview — the same refusal discipline the trace index
/// cell uses, applied through its shared type.
fn preview(value: &str) -> ScalarPreview {
    ScalarPreview::of(value)
}
