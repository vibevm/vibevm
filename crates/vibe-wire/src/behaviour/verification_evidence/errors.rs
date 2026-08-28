//! The typed refusals of the verification-evidence member's relational
//! laws. The same refusal discipline the trace index and the report
//! trace member carry: an evidence document is read from disk or a
//! tool's stdout, so no variant here clones a wire string — every
//! untrusted scalar rides a bounded [`ScalarPreview`] (shared with the
//! index cell, one type, not a third preview), and every index, member
//! name and row reference is bounded by construction.

use crate::behaviour::compiler_trace_index::ScalarPreview;
use crate::generated::shared::EvidenceStatus;

/// Why a project-relative path, glob or selected node failed its
/// spelling law. The GRAMMAR is shared with the requirements cell
/// ([`crate::behaviour::scalars`]); only the refusal that wraps it is
/// this cell's own.
pub use crate::behaviour::scalars::RelativePathDefect as PathUnsafety;

/// Which row list a refusal is about. The two generated row types
/// carry the same comparison half, so a refusal must say which side of
/// the document it walked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RowKind {
    /// An `inputs[]` row — a declared-input manifest comparison.
    Input,
    /// An `artifacts[]` row — a produced-artifact content comparison.
    Artifact,
}

impl RowKind {
    /// The wire spelling of the list — what a refusal quotes so a
    /// reader can find the row.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            RowKind::Input => "inputs",
            RowKind::Artifact => "artifacts",
        }
    }

    /// The member a row of this list is keyed by.
    #[must_use]
    pub fn key_member(self) -> &'static str {
        match self {
            RowKind::Input => "execution",
            RowKind::Artifact => "id",
        }
    }
}

/// A bounded reference to one comparison row: which list, which
/// position, and — previewed — which key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RowRef {
    pub kind: RowKind,
    pub index: usize,
    pub key: ScalarPreview,
}

impl std::fmt::Display for RowRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}[{}] ({} {})",
            self.kind.as_str(),
            self.index,
            self.kind.key_member(),
            self.key
        )
    }
}

/// Which half of a comparison a witness refusal is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WitnessHalf {
    /// The witness durable state recorded when the work ran.
    Measured,
    /// The witness recomputed at verify.
    Observed,
}

impl WitnessHalf {
    /// The wire member name.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            WitnessHalf::Measured => "measured",
            WitnessHalf::Observed => "observed",
        }
    }
}

/// What is wrong with a witness's `files`/`bytes` pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CountDefect {
    /// `files` without `bytes` — half a claim about scope.
    BytesMissing,
    /// `bytes` without `files` — the other half.
    FilesMissing,
    /// A manifest row's witness carries no counts at all.
    Absent,
    /// An artifact row's witness carries counts it cannot have
    /// measured — a file's witness IS its bytes.
    Unexpected,
}

impl CountDefect {
    fn phrase(self) -> &'static str {
        match self {
            CountDefect::BytesMissing => {
                "carries `files` without `bytes`; the count pair is one claim, not two numbers"
            }
            CountDefect::FilesMissing => {
                "carries `bytes` without `files`; the count pair is one claim, not two numbers"
            }
            CountDefect::Absent => {
                "carries no counts, but an input manifest must say how much of its declared \
                 scope it covered"
            }
            CountDefect::Unexpected => {
                "carries counts, but an artifact witness IS its content and counts nothing"
            }
        }
    }
}

/// Which clause of the `comparison-shape` matrix a row failed. Typed
/// rather than stringly so a mutation test can assert the exact arm.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShapeDefect {
    /// The status claims a measurement the row does not carry.
    MissingMeasured,
    /// The status claims an observation the row does not carry.
    MissingObserved,
    /// `unavailable` carries a measurement; then the measurement is
    /// not what was unavailable.
    UnexpectedMeasured,
    /// `missing`/`unstable` carries an observation; then nothing was
    /// missing and nothing moved.
    UnexpectedObserved,
    /// `stale` carries two witnesses that compare EQUAL — a mismatch
    /// that is not a mismatch.
    EqualWitnesses,
    /// `matched` carries two witnesses that differ — the one word that
    /// is a pass, granted to two different claims.
    UnequalWitnesses,
    /// `matched` carries a reason code; a pass explains nothing.
    ReasonPresent,
    /// A status that owes a reason carries none. A refusal that says
    /// nothing is not a refusal.
    ReasonAbsent,
    /// A reason code is present but empty or whitespace-only — the
    /// LENGTH of an explanation with none of its meaning.
    ReasonBlank,
}

impl ShapeDefect {
    /// The sentence this defect reads as, after the row and status.
    fn phrase(self) -> &'static str {
        match self {
            ShapeDefect::MissingMeasured => "carries no `measured` witness",
            ShapeDefect::MissingObserved => "carries no `observed` witness",
            ShapeDefect::UnexpectedMeasured => {
                "carries a `measured` witness, so the measurement is not what was unavailable"
            }
            ShapeDefect::UnexpectedObserved => {
                "carries an `observed` witness, so nothing was missing and nothing moved"
            }
            ShapeDefect::EqualWitnesses => {
                "carries two witnesses that compare equal; a mismatch that matches is not stale"
            }
            ShapeDefect::UnequalWitnesses => {
                "carries two witnesses that differ; `matched` is the only pass and it compares equal"
            }
            ShapeDefect::ReasonPresent => "carries a reason code; a pass explains nothing",
            ShapeDefect::ReasonAbsent => "carries no reason code, and this status owes one",
            ShapeDefect::ReasonBlank => {
                "carries a blank reason code; an empty or whitespace-only string is not a reason"
            }
        }
    }
}

/// Why an identity or diagnostic scalar failed its text law.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextUnsafety {
    /// Empty or whitespace-only where a name is owed.
    Blank,
    /// Over the shared diagnostic cap.
    OverCap,
    /// CR, LF or NUL inside a value a reader will print.
    ControlByte,
}

impl TextUnsafety {
    fn phrase(self) -> &'static str {
        match self {
            TextUnsafety::Blank => "is blank; an unnamed identity is an unjoinable claim",
            TextUnsafety::OverCap => "is over the shared diagnostic cap",
            TextUnsafety::ControlByte => "carries CR, LF or NUL",
        }
    }
}

/// One broken relational law, with the context needed to name the
/// offender. Typed end to end — no stringly `detail` — so a test can
/// assert the exact family a mutation lands in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvidenceError {
    /// `evidence-identity` — the member speaks an epoch this reader
    /// does not.
    EvidenceEpoch { found: u32 },
    /// `evidence-identity` — `evidence_id` is not `sha256:` + 64
    /// lowercase hex.
    EvidenceIdShape { evidence_id: ScalarPreview },
    /// `run-identity` — `run.run_id` is not exactly 32 lowercase hex.
    RunIdNotLowercaseHex { run_id: ScalarPreview },
    /// `run-identity` — `run.selected` is neither `.` nor a safe
    /// forward-slashed workspace-relative path.
    UnsafeSelected {
        selected: ScalarPreview,
        reason: PathUnsafety,
    },
    /// `run-identity` — a run header scalar is blank, over the cap, or
    /// carries a byte a reader cannot print.
    UnsafeRunScalar {
        field: &'static str,
        value: ScalarPreview,
        reason: TextUnsafety,
    },
    /// `run-identity` — the chain is empty; a run with no chain is
    /// not a run.
    EmptyChain,
    /// `run-identity` — a chain entry is blank, over the cap or
    /// control-carrying.
    UnsafeChainPhase {
        index: usize,
        phase: ScalarPreview,
        reason: TextUnsafety,
    },
    /// `run-identity` — the requested phase is not in its own chain.
    RequestedOutsideChain { requested: ScalarPreview },
    /// `measurement-identity` — a present `measured_run_id` is not
    /// exactly 32 lowercase hex.
    MeasuredRunIdNotLowercaseHex { row: RowRef, value: ScalarPreview },
    /// `measurement-identity` — a row carries a `measured` witness
    /// but cannot name the run that took it.
    MeasuredRunIdAbsent { row: RowRef },
    /// `measurement-identity` — a row names the run that measured it
    /// while carrying no `measured` witness. The id attributes a
    /// measurement the row itself says does not exist.
    MeasuredRunIdOrphaned { row: RowRef, value: ScalarPreview },
    /// `measurement-identity` — an input row's declaration
    /// fingerprint is not `sha256:` + 64 lowercase hex.
    DeclarationFingerprintShape {
        row: RowRef,
        fingerprint: ScalarPreview,
    },
    /// `witness-shape` — a witness names no algorithm.
    WitnessAlgorithmBlank { row: RowRef, half: WitnessHalf },
    /// `witness-shape` — a witness digest is not `sha256:` + 64
    /// lowercase hex.
    WitnessDigestShape {
        row: RowRef,
        half: WitnessHalf,
        digest: ScalarPreview,
    },
    /// `witness-shape` — a byte count is not a canonical unsigned
    /// decimal string; the one member meant to be lossless is not.
    NonCanonicalByteCount {
        row: RowRef,
        half: WitnessHalf,
        value: ScalarPreview,
    },
    /// `witness-shape` — the `files`/`bytes` pair does not match what
    /// the row's kind requires.
    WitnessCountShape {
        row: RowRef,
        half: WitnessHalf,
        defect: CountDefect,
    },
    /// `comparison-shape` — the row's status cannot mean what the
    /// row's members say.
    ComparisonShape {
        row: RowRef,
        status: EvidenceStatus,
        defect: ShapeDefect,
    },
    /// `overall-status` — the root's status is not the worst row's
    /// (or, with no rows, not `unavailable`).
    OverallStatus {
        declared: EvidenceStatus,
        expected: EvidenceStatus,
    },
    /// `row-identity` — a row's key is blank.
    RowKeyBlank { row: RowRef },
    /// `row-identity` — one key got two rows; a reader would have two
    /// answers to one question and no rule for choosing.
    RowKeyDuplicate { row: RowRef, first: usize },
    /// `pattern-safety` — a declared pattern names a scope this wire
    /// may not certify.
    UnsafePattern {
        row: RowRef,
        index: usize,
        pattern: ScalarPreview,
        reason: PathUnsafety,
    },
    /// `path-safety` — an artifact path is not the canonical
    /// project-relative path under `run.selected`.
    UnsafeArtifactPath {
        row: RowRef,
        path: ScalarPreview,
        reason: PathUnsafety,
    },
    /// `bounded-identity` — a scalar is blank, over the cap, or
    /// carries a byte a reader cannot safely print or join on.
    UnsafeScalar {
        row: RowRef,
        field: &'static str,
        value: ScalarPreview,
        reason: TextUnsafety,
    },
}

impl EvidenceError {
    /// The implemented-law label this violation witnesses — the join
    /// key the wire-corpus parity test reads.
    #[must_use]
    pub fn law(&self) -> &'static str {
        use EvidenceError as E;
        match self {
            E::EvidenceEpoch { .. } | E::EvidenceIdShape { .. } => "evidence-identity",
            E::RunIdNotLowercaseHex { .. }
            | E::UnsafeSelected { .. }
            | E::UnsafeRunScalar { .. }
            | E::EmptyChain
            | E::UnsafeChainPhase { .. }
            | E::RequestedOutsideChain { .. } => "run-identity",
            E::MeasuredRunIdNotLowercaseHex { .. }
            | E::MeasuredRunIdAbsent { .. }
            | E::MeasuredRunIdOrphaned { .. }
            | E::DeclarationFingerprintShape { .. } => "measurement-identity",
            E::WitnessAlgorithmBlank { .. }
            | E::WitnessDigestShape { .. }
            | E::NonCanonicalByteCount { .. }
            | E::WitnessCountShape { .. } => "witness-shape",
            E::ComparisonShape { .. } => "comparison-shape",
            E::OverallStatus { .. } => "overall-status",
            E::RowKeyBlank { .. } | E::RowKeyDuplicate { .. } => "row-identity",
            E::UnsafePattern { .. } => "pattern-safety",
            E::UnsafeArtifactPath { .. } => "path-safety",
            E::UnsafeScalar { .. } => "bounded-identity",
        }
    }
}

impl std::error::Error for EvidenceError {}

/// The wire spelling of an outcome — the closed enum carries no
/// `Display` of its own, and a refusal quoting the exact wire word
/// beats one that names the Rust variant.
fn status_spelling(status: &EvidenceStatus) -> &'static str {
    match status {
        EvidenceStatus::Matched => "matched",
        EvidenceStatus::Stale => "stale",
        EvidenceStatus::Missing => "missing",
        EvidenceStatus::Unavailable => "unavailable",
        EvidenceStatus::Unstable => "unstable",
    }
}

impl std::fmt::Display for EvidenceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use EvidenceError as E;
        match self {
            E::EvidenceEpoch { found } => write!(
                f,
                "evidence = {found}; this reader speaks epoch {}",
                super::EVIDENCE_EPOCH
            ),
            E::EvidenceIdShape { evidence_id } => write!(
                f,
                "evidence_id {evidence_id} is not `sha256:` followed by 64 lowercase hex characters"
            ),
            E::RunIdNotLowercaseHex { run_id } => write!(
                f,
                "run.run_id {run_id} is not exactly 32 lowercase hex characters"
            ),
            E::UnsafeSelected { selected, reason } => write!(
                f,
                "run.selected {selected} {} — it is `.` or a project-relative forward-slashed path",
                reason.phrase()
            ),
            E::UnsafeRunScalar {
                field,
                value,
                reason,
            } => write!(f, "run.{field} {value} {}", reason.phrase()),
            E::EmptyChain => write!(f, "run.chain is empty; a run with no chain is not a run"),
            E::UnsafeChainPhase {
                index,
                phase,
                reason,
            } => write!(f, "run.chain[{index}] {phase} {}", reason.phrase()),
            E::RequestedOutsideChain { requested } => write!(
                f,
                "run.requested {requested} is not one of run.chain; that names a run which never ran"
            ),
            E::MeasuredRunIdNotLowercaseHex { row, value } => write!(
                f,
                "{row}: measured_run_id {value} is not exactly 32 lowercase hex characters"
            ),
            E::MeasuredRunIdAbsent { row } => write!(
                f,
                "{row}: a `measured` witness with no measured_run_id; a measurement nobody can be \
                 held to is not evidence"
            ),
            E::MeasuredRunIdOrphaned { row, value } => write!(
                f,
                "{row}: measured_run_id {value} with no `measured` witness; the id attributes a \
                 measurement this row says does not exist"
            ),
            E::DeclarationFingerprintShape { row, fingerprint } => write!(
                f,
                "{row}: declaration_fingerprint {fingerprint} is not `sha256:` followed by 64 \
                 lowercase hex characters"
            ),
            E::WitnessAlgorithmBlank { row, half } => write!(
                f,
                "{row}: the `{}` witness names no algorithm; the framing is part of the witness",
                half.as_str()
            ),
            E::WitnessDigestShape { row, half, digest } => write!(
                f,
                "{row}: the `{}` witness digest {digest} is not `sha256:` followed by 64 lowercase \
                 hex characters",
                half.as_str()
            ),
            E::NonCanonicalByteCount { row, half, value } => write!(
                f,
                "{row}: the `{}` witness carries bytes = {value}, which is not a canonical unsigned \
                 decimal string (nonempty ASCII digits, no leading zero unless the value is 0)",
                half.as_str()
            ),
            E::WitnessCountShape { row, half, defect } => write!(
                f,
                "{row}: the `{}` witness {}",
                half.as_str(),
                defect.phrase()
            ),
            E::ComparisonShape {
                row,
                status,
                defect,
            } => write!(
                f,
                "{row}: status `{}` {}",
                status_spelling(status),
                defect.phrase()
            ),
            E::OverallStatus { declared, expected } => write!(
                f,
                "the overall status is `{}` but the rows say `{}`; the root is the worst row, \
                 and with no rows at all it is `unavailable`",
                status_spelling(declared),
                status_spelling(expected)
            ),
            E::RowKeyBlank { row } => write!(
                f,
                "{row}: the row's key is blank; an unnamed row is an unjoinable claim"
            ),
            E::RowKeyDuplicate { row, first } => write!(
                f,
                "{row}: this key already had row {first}; one identity gets one answer"
            ),
            E::UnsafePattern {
                row,
                index,
                pattern,
                reason,
            } => write!(f, "{row}: patterns[{index}] {pattern} {}", reason.phrase()),
            E::UnsafeArtifactPath { row, path, reason } => write!(
                f,
                "{row}: path {path} {} — an evidence row carries the project-relative path under \
                 run.selected, never the absolute machine path durable state keeps",
                reason.phrase()
            ),
            E::UnsafeScalar {
                row,
                field,
                value,
                reason,
            } => write!(f, "{row}: `{field}` {value} {}", reason.phrase()),
        }
    }
}
