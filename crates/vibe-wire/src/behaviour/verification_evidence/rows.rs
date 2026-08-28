//! The two generated row types, seen as one comparison.
//!
//! `inputs[]` and `artifacts[]` carry the SAME comparison half under
//! different identity halves, and JTD has no way to say so. Rather
//! than write every law twice, the validator walks this borrowed view:
//! the identity half a refusal names, the witness half every matrix
//! clause reads, and the one thing the two rows genuinely differ on —
//! whether their witnesses are counted manifests or uncounted content
//! digests.

use crate::behaviour::compiler_trace_index::ScalarPreview;
use crate::generated::shared::{
    ArtifactWitness, DigestWitness, EvidenceStatus, InputMeasurement, VerificationEvidence,
};

use super::errors::{EvidenceError, RowKind, RowRef, ShapeDefect};

/// One comparison row, borrowed.
pub(super) struct Comparison<'a> {
    pub(super) kind: RowKind,
    pub(super) index: usize,
    pub(super) key: &'a str,
    /// The `(wire member, text)` pairs the bounded-identity law reads.
    /// The first is always the row's key, which `row-identity` has
    /// already proven nonblank by the time this is walked.
    pub(super) identity: [(&'static str, &'a str); 3],
    /// Declared input patterns — empty for an artifact row, which
    /// declares no scope of its own.
    pub(super) patterns: &'a [String],
    /// The declaration this row was measured against — input rows only.
    pub(super) declaration_fingerprint: Option<&'a str>,
    /// The project-relative artifact path — artifact rows only.
    pub(super) path: Option<&'a str>,
    pub(super) status: &'a EvidenceStatus,
    pub(super) measured_run_id: Option<&'a str>,
    pub(super) measured: Option<&'a DigestWitness>,
    pub(super) observed: Option<&'a DigestWitness>,
    pub(super) reason_code: Option<&'a str>,
}

impl<'a> Comparison<'a> {
    fn of_input(index: usize, input: &'a InputMeasurement) -> Self {
        Comparison {
            kind: RowKind::Input,
            index,
            key: &input.execution,
            identity: [
                ("execution", &input.execution),
                ("phase", &input.phase),
                ("declaration_fingerprint", &input.declaration_fingerprint),
            ],
            patterns: &input.patterns,
            declaration_fingerprint: Some(&input.declaration_fingerprint),
            path: None,
            status: &input.status,
            measured_run_id: input.measured_run_id.as_deref(),
            measured: input.measured.as_ref(),
            observed: input.observed.as_ref(),
            reason_code: input.reason_code.as_deref(),
        }
    }

    fn of_artifact(index: usize, artifact: &'a ArtifactWitness) -> Self {
        Comparison {
            kind: RowKind::Artifact,
            index,
            key: &artifact.id,
            identity: [
                ("id", &artifact.id),
                ("kind", &artifact.kind),
                ("path", &artifact.path),
            ],
            patterns: &[],
            declaration_fingerprint: None,
            path: Some(&artifact.path),
            status: &artifact.status,
            measured_run_id: artifact.measured_run_id.as_deref(),
            measured: artifact.measured.as_ref(),
            observed: artifact.observed.as_ref(),
            reason_code: artifact.reason_code.as_deref(),
        }
    }

    /// Whether this row's witnesses are COUNTED manifests. An input
    /// row's witness covers a declared file set and must say how much
    /// of it it covered; an artifact row's witness is the content
    /// itself and counts nothing. The rule is the row's, never the
    /// writer's, which is what stops a counted and an uncounted claim
    /// from ever comparing equal.
    pub(super) fn counts_expected(&self) -> bool {
        matches!(self.kind, RowKind::Input)
    }

    /// The bounded reference a refusal names this row by.
    pub(super) fn at(&self) -> RowRef {
        RowRef {
            kind: self.kind,
            index: self.index,
            key: ScalarPreview::of(self.key),
        }
    }

    /// One `comparison-shape` refusal for this row's status.
    pub(super) fn defect(&self, defect: ShapeDefect) -> EvidenceError {
        EvidenceError::ComparisonShape {
            row: self.at(),
            status: self.status.clone(),
            defect,
        }
    }
}

/// Both row lists as one sequence of comparisons — inputs first, then
/// artifacts, each in wire order.
pub(super) fn rows(evidence: &VerificationEvidence) -> impl Iterator<Item = Comparison<'_>> {
    evidence
        .inputs
        .iter()
        .enumerate()
        .map(|(index, input)| Comparison::of_input(index, input))
        .chain(
            evidence
                .artifacts
                .iter()
                .enumerate()
                .map(|(index, artifact)| Comparison::of_artifact(index, artifact)),
        )
}
