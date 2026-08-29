//! The typed refusals of the lane analyzer report's laws. The same
//! refusal discipline the trace and record cells carry: a report is
//! read back through the generated reader, so no variant here clones a
//! whole wire string — every untrusted scalar rides a bounded
//! [`ScalarPreview`] (shared with the trace index cell, one type, not a
//! second preview), and every member name is spelled at the refusal
//! site, which the caller already holds as data.

use crate::behaviour::compiler_trace_index::ScalarPreview;

/// One broken law of the analyzer report, with the context needed to
/// name the offender. Typed end to end — no stringly `detail` — so a
/// test can assert the exact family a mutation lands in. `lane` is the
/// offending artifact row's index; `seat`, where present, is the row's
/// index inside `contributions` or `deltas`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExtensionsAnalyzeError {
    /// `schema` is not this validator's epoch — a newer report must fail
    /// loudly, not parse into a wrong meaning.
    SchemaEpoch { found: u32 },
    /// `command` is not this format's verb.
    CommandIdentity { command: ScalarPreview },
    /// `artifact_id` is not the id its own `target` owns — for a builtin
    /// static lane the two are one identity, and a row naming them
    /// differently describes nothing the compiler can produce.
    ArtifactTargetMismatch {
        lane: u32,
        artifact_id: ScalarPreview,
    },
    /// A byte count is not a canonical unsigned decimal string.
    ByteCountNotCanonical {
        lane: u32,
        member: String,
        value: ScalarPreview,
    },
    /// A free-text member is blank or carries CR, LF or NUL.
    UnsafeScalar {
        lane: u32,
        member: String,
        value: ScalarPreview,
    },
    /// A path member carries a backslash; the wire spelling is forward
    /// slashes.
    BackslashedPath {
        lane: u32,
        member: String,
        value: ScalarPreview,
    },
    /// A contribution's occurrence count contradicts its kind (an
    /// `elided`/`hoisted` row brackets none; a `simple` one exactly
    /// one).
    OccurrenceGrammar {
        lane: u32,
        seat: u32,
        kind: String,
        occurrences: u32,
    },
    /// Row 13's first half: the contributions plus the frame do not sum
    /// to the artifact's emitted total.
    TotalsDoNotReconcile {
        lane: u32,
        contributions: u128,
        frame_overhead: String,
        total: String,
    },
    /// The artifact's declared occurrence count is not the sum of its
    /// contribution occurrences.
    OccurrenceCountMismatch {
        lane: u32,
        declared: u32,
        contributions: u32,
    },
    /// The stage law: a delta row's two members are labelled apart, and
    /// the row must carry exactly the pair its stage measures.
    StageMemberMismatch { lane: u32, seat: u32, stage: String },
    /// An emitted delta's `before` is not the previous emitted delta's
    /// `after` — the chain of artifact bytes is discontinuous.
    DeltaChainBroken {
        lane: u32,
        seat: u32,
        previous_after: String,
        before: String,
    },
    /// The last emitted delta's `after` is not the artifact's
    /// `total_emitted_bytes`.
    DeltaChainDoesNotReachTotal {
        lane: u32,
        last_after: String,
        total: String,
    },
    /// The estimator law: an estimate and its estimator are present
    /// exactly together. A number with no named method is not an
    /// estimate.
    EstimatorCoupling {
        lane: u32,
        estimate_is_some: bool,
        estimator_id: Option<ScalarPreview>,
    },
}

impl std::error::Error for ExtensionsAnalyzeError {}

impl std::fmt::Display for ExtensionsAnalyzeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use ExtensionsAnalyzeError as E;
        match self {
            E::SchemaEpoch { found } => write!(
                f,
                "schema = {found} is not the extensions-analyze epoch {epoch}",
                epoch = super::REPORT_EPOCH
            ),
            E::CommandIdentity { command } => write!(
                f,
                "command {command} is not `{expected}`; this format belongs to one verb",
                expected = super::COMMAND
            ),
            E::ArtifactTargetMismatch { lane, artifact_id } => write!(
                f,
                "artifact {lane}: artifact_id {artifact_id} is not the id its target owns — a \
                 builtin static lane's id IS its target's backend spelling"
            ),
            E::ByteCountNotCanonical {
                lane,
                member,
                value,
            } => write!(
                f,
                "artifact {lane}: {member} {value} is not a canonical unsigned decimal string \
                 (digits only, no sign, no leading zero except `0`)"
            ),
            E::UnsafeScalar {
                lane,
                member,
                value,
            } => write!(
                f,
                "artifact {lane}: {member} {value} is empty, whitespace-only or carries CR, LF \
                 or NUL"
            ),
            E::BackslashedPath {
                lane,
                member,
                value,
            } => write!(
                f,
                "artifact {lane}: {member} {value} contains a backslash; the wire spelling is \
                 forward slashes"
            ),
            E::OccurrenceGrammar {
                lane,
                seat,
                kind,
                occurrences,
            } => write!(
                f,
                "artifact {lane}: contributions[{seat}] is `{kind}` but brackets {occurrences} \
                 occurrence(s); an `elided`/`hoisted` row brackets none and a `simple` one \
                 exactly one"
            ),
            E::TotalsDoNotReconcile {
                lane,
                contributions,
                frame_overhead,
                total,
            } => write!(
                f,
                "artifact {lane}: contributions ({contributions}) + frame_overhead \
                 ({frame_overhead}) do not equal total_emitted_bytes ({total})"
            ),
            E::OccurrenceCountMismatch {
                lane,
                declared,
                contributions,
            } => write!(
                f,
                "artifact {lane}: occurrence_count is {declared} but the contributions sum to \
                 {contributions}"
            ),
            E::StageMemberMismatch { lane, seat, stage } => write!(
                f,
                "artifact {lane}: deltas[{seat}] is a `{stage}`-stage row whose delta members \
                 are not the pair that stage measures; lane-byte delta and artifact-byte delta \
                 are different members and never stand in for each other"
            ),
            E::DeltaChainBroken {
                lane,
                seat,
                previous_after,
                before,
            } => write!(
                f,
                "artifact {lane}: deltas[{seat}] begins at {before} bytes but the previous \
                 emitted pass left {previous_after}"
            ),
            E::DeltaChainDoesNotReachTotal {
                lane,
                last_after,
                total,
            } => write!(
                f,
                "artifact {lane}: the last emitted delta ends at {last_after} bytes, not the \
                 artifact's total_emitted_bytes ({total})"
            ),
            E::EstimatorCoupling {
                lane,
                estimate_is_some,
                estimator_id,
            } => write!(
                f,
                "artifact {lane}: token_estimate and estimator_id are present exactly together \
                 (estimate {} a value, estimator_id {})",
                if *estimate_is_some {
                    "carries"
                } else {
                    "lacks"
                },
                estimator_id
                    .as_ref()
                    .map(ToString::to_string)
                    .unwrap_or_else(|| "is null".to_string())
            ),
        }
    }
}
