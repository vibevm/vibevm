//! The lane analyzer report's scalar and relational laws — the
//! hand-written validation cell beside the ONE generated
//! [`ExtensionsAnalyze`] document `vibe extensions analyze` prints (R4.3,
//! the packages-2026-09 architecture §9).
//!
//! JTD owns the FORM (the closed target/contribution-kind/stage
//! vocabularies, the four-arm provider and lane-identity one-ofs, the
//! required-nullable delta and estimator members); this cell owns what a
//! form cannot say about itself: the epoch constant, the command
//! identity, the canonical unsigned-decimal byte-count law, the
//! per-artifact reconciliation of contributions + frame against the
//! emitted total, the stage-labelled coupling of the two delta members,
//! the emitted-delta chain that must terminate at the artifact's own
//! total, and the estimator coupling that keeps a token estimate
//! honest. Every predicate that is not this report's own is REUSED from
//! [`crate::behaviour::scalars`] — one grammar, every wire.
//!
//! The report is read back through the generated reader on every
//! production path (the CLI validates before printing), so the values
//! here are treated as untrusted wire: no refusal clones the offending
//! scalar, and every preview rides the shared bounded
//! [`ScalarPreview`].
//!
//! One conversion helper completes the cell: [`spell_bytes`] is the
//! single spelling of a byte count a PRODUCER reaches for, so the
//! canonical-decimal law cannot drift between the writer and the reader
//! that judges it.

use crate::behaviour::compiler_trace_index::ScalarPreview;
use crate::behaviour::scalars::{has_control_bytes, is_canonical_decimal};
use crate::generated::extensions_analyze::{
    ArtifactRow, ContributionKind, ContributionRow, DeltaRow, ExtensionsAnalyze, LaneIdentity,
    Stage,
};

mod errors;

pub use errors::ExtensionsAnalyzeError;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

/// The report epoch this validator speaks — the `schema` member's one
/// legal value today.
pub const REPORT_EPOCH: u32 = 1;

/// The command identity this format belongs to. A document produced by
/// any other verb is not this format's report, however well-formed its
/// members are.
pub const COMMAND: &str = "extensions-analyze";

/// The artifact-id spellings the two builtin static targets own. For a
/// builtin static lane the artifact id IS the target's backend spelling
/// (`ArtifactContext::new` enforces the same equality at plan
/// construction); restating it here keeps a hand-authored row from
/// naming an id its own target disclaims.
const STATIC_MD: &str = "static-md";
const STATIC_XML: &str = "static-xml";

/// Validate one analyzer report against every scalar and relational law.
/// Pure: the value in, the first broken law out.
pub fn validate(report: &ExtensionsAnalyze) -> Result<(), ExtensionsAnalyzeError> {
    if report.schema != REPORT_EPOCH {
        return Err(ExtensionsAnalyzeError::SchemaEpoch {
            found: report.schema,
        });
    }
    if report.command != COMMAND {
        return Err(ExtensionsAnalyzeError::CommandIdentity {
            command: preview(&report.command),
        });
    }
    for (index, artifact) in report.artifacts.iter().enumerate() {
        validate_artifact(index, artifact)?;
    }
    Ok(())
}

/// Spell one byte count in the wire's single canonical form. The
/// producer half of the byte-count law: a count that went through this
/// function always passes [`is_canonical_decimal`], so the writer and
/// the reader that judges it cannot drift apart.
#[must_use]
pub fn spell_bytes(count: u128) -> String {
    count.to_string()
}

fn validate_artifact(index: usize, artifact: &ArtifactRow) -> Result<(), ExtensionsAnalyzeError> {
    let lane = index.try_into().unwrap_or(u32::MAX);
    let expected_id = match artifact.target {
        crate::generated::extensions_analyze::ArtifactTarget::StaticMd => STATIC_MD,
        crate::generated::extensions_analyze::ArtifactTarget::StaticXml => STATIC_XML,
    };
    if artifact.artifact_id != expected_id {
        return Err(ExtensionsAnalyzeError::ArtifactTargetMismatch {
            lane,
            artifact_id: preview(&artifact.artifact_id),
        });
    }
    validate_lane_identity(lane, artifact)?;
    count_gate(lane, "total_emitted_bytes", &artifact.total_emitted_bytes)?;
    count_gate(lane, "frame_overhead_bytes", &artifact.frame_overhead_bytes)?;
    estimator_gate(lane, artifact)?;
    let mut contributed: u128 = 0;
    let mut occurrences: u128 = 0;
    for (seat, row) in artifact.contributions.iter().enumerate() {
        contribution_gate(lane, seat, row)?;
        contributed += parse_count(&row.bytes);
        occurrences += u128::from(row.occurrences);
    }
    // Row 13's first half: the contributions plus the frame ARE the
    // artifact — nothing else exists to hold bytes.
    let total = parse_count(&artifact.total_emitted_bytes);
    if contributed + parse_count(&artifact.frame_overhead_bytes) != total {
        return Err(ExtensionsAnalyzeError::TotalsDoNotReconcile {
            lane,
            contributions: contributed,
            frame_overhead: artifact.frame_overhead_bytes.clone(),
            total: artifact.total_emitted_bytes.clone(),
        });
    }
    let occurrences = u32::try_from(occurrences).unwrap_or(u32::MAX);
    if occurrences != artifact.occurrence_count {
        return Err(ExtensionsAnalyzeError::OccurrenceCountMismatch {
            lane,
            declared: artifact.occurrence_count,
            contributions: occurrences,
        });
    }
    deltas_gate(lane, artifact, &artifact.total_emitted_bytes)
}

/// The lane identity's node spelling: a portable rel path (`.` for the
/// root), forward-slashed and control-free. The unit arm's components
/// are non-blank and control-free.
fn validate_lane_identity(lane: u32, artifact: &ArtifactRow) -> Result<(), ExtensionsAnalyzeError> {
    match &artifact.lane {
        LaneIdentity::Node(node) => text_gate(
            lane,
            "lane.node.node_rel",
            &node.node_rel,
            TextLaw::ForwardSlashed,
        ),
        LaneIdentity::Unit(unit) => {
            text_gate(lane, "lane.unit.group", &unit.group, TextLaw::Plain)?;
            text_gate(lane, "lane.unit.name", &unit.name, TextLaw::Plain)
        }
    }
}

/// One contribution row: byte count, free-text members, and the
/// occurrence grammar (an `elided` or `hoisted` row brackets no
/// occurrences; a `simple` one brackets exactly one).
fn contribution_gate(
    lane: u32,
    seat: usize,
    row: &ContributionRow,
) -> Result<(), ExtensionsAnalyzeError> {
    let seat = seat.try_into().unwrap_or(u32::MAX);
    count_gate(lane, &format!("contributions[{seat}].bytes"), &row.bytes)?;
    text_gate(
        lane,
        &format!("contributions[{seat}].origin"),
        &row.origin,
        TextLaw::Plain,
    )?;
    text_gate(
        lane,
        &format!("contributions[{seat}].path"),
        &row.path,
        TextLaw::ForwardSlashed,
    )?;
    let lawful = match row.kind {
        ContributionKind::Elided | ContributionKind::Hoisted => row.occurrences == 0,
        ContributionKind::Simple => row.occurrences == 1,
        ContributionKind::Normal => true,
    };
    if !lawful {
        return Err(ExtensionsAnalyzeError::OccurrenceGrammar {
            lane,
            seat,
            kind: kind_text(&row.kind).to_string(),
            occurrences: row.occurrences,
        });
    }
    Ok(())
}

/// The stage law and the chain law. The two delta members are labelled
/// apart and never conflated: a `lane` row carries the lane pair with a
/// null artifact pair, an `emitted` row the mirror image. Emitted rows
/// then chain — each `before` is the previous `after`, and the LAST
/// `after` is the artifact's total, so the deltas reconcile against the
/// same bytes the contributions reconciled against (row 13's second
/// half).
fn deltas_gate(
    lane: u32,
    artifact: &ArtifactRow,
    total: &str,
) -> Result<(), ExtensionsAnalyzeError> {
    let mut emitted_after: Option<&str> = None;
    for (seat, row) in artifact.deltas.iter().enumerate() {
        delta_gate(lane, seat, row, &mut emitted_after)?;
    }
    if let Some(after) = emitted_after
        && after != total
    {
        return Err(ExtensionsAnalyzeError::DeltaChainDoesNotReachTotal {
            lane,
            last_after: after.to_string(),
            total: total.to_string(),
        });
    }
    Ok(())
}

fn delta_gate<'a>(
    lane: u32,
    seat: usize,
    row: &'a DeltaRow,
    emitted_after: &mut Option<&'a str>,
) -> Result<(), ExtensionsAnalyzeError> {
    let seat = seat.try_into().unwrap_or(u32::MAX);
    text_gate(
        lane,
        &format!("deltas[{seat}].pass"),
        &row.pass,
        TextLaw::Plain,
    )?;
    let (lane_pair, artifact_pair) = (&row.lane_byte_delta, &row.artifact_byte_delta);
    let coherent = match row.stage {
        Stage::Lane => lane_pair.is_some() && artifact_pair.is_none(),
        Stage::Emitted => artifact_pair.is_some() && lane_pair.is_none(),
    };
    if !coherent {
        return Err(ExtensionsAnalyzeError::StageMemberMismatch {
            lane,
            seat,
            stage: stage_text(&row.stage).to_string(),
        });
    }
    if let Some(pair) = lane_pair {
        count_gate(
            lane,
            &format!("deltas[{seat}].lane_byte_delta.before"),
            &pair.before,
        )?;
        count_gate(
            lane,
            &format!("deltas[{seat}].lane_byte_delta.after"),
            &pair.after,
        )?;
    }
    if let Some(pair) = artifact_pair {
        count_gate(
            lane,
            &format!("deltas[{seat}].artifact_byte_delta.before"),
            &pair.before,
        )?;
        count_gate(
            lane,
            &format!("deltas[{seat}].artifact_byte_delta.after"),
            &pair.after,
        )?;
        if let Some(previous) = *emitted_after
            && previous != pair.before
        {
            return Err(ExtensionsAnalyzeError::DeltaChainBroken {
                lane,
                seat,
                previous_after: previous.to_string(),
                before: pair.before.clone(),
            });
        }
        *emitted_after = Some(&pair.after);
    }
    Ok(())
}

/// The estimator coupling: an estimate without a named estimator, or a
/// named estimator without an estimate, is refused. This atom ships no
/// estimator, so every report the CLI produces spells both members null
/// — and the corpus negative pins the refused form.
fn estimator_gate(lane: u32, artifact: &ArtifactRow) -> Result<(), ExtensionsAnalyzeError> {
    match (&artifact.token_estimate, &artifact.estimator_id) {
        (None, None) => Ok(()),
        (Some(_), Some(id)) => text_gate(lane, "estimator_id", id, TextLaw::Plain),
        (estimate, id) => Err(ExtensionsAnalyzeError::EstimatorCoupling {
            lane,
            estimate_is_some: estimate.is_some(),
            estimator_id: id.as_deref().map(preview),
        }),
    }
}

/// Which free-text law a member answers to.
#[derive(Clone, Copy)]
enum TextLaw {
    /// Non-blank once trimmed, no CR/LF/NUL.
    Plain,
    /// The plain law plus the forward-slash separator spelling — a
    /// backslash is not a separator to any path a reader joins on.
    ForwardSlashed,
}

fn text_gate(
    lane: u32,
    member: &str,
    value: &str,
    law: TextLaw,
) -> Result<(), ExtensionsAnalyzeError> {
    if value.trim().is_empty() || has_control_bytes(value) {
        return Err(ExtensionsAnalyzeError::UnsafeScalar {
            lane,
            member: member.to_string(),
            value: preview(value),
        });
    }
    if matches!(law, TextLaw::ForwardSlashed) && value.contains('\\') {
        return Err(ExtensionsAnalyzeError::BackslashedPath {
            lane,
            member: member.to_string(),
            value: preview(value),
        });
    }
    Ok(())
}

/// The byte-count law on one member: a CANONICAL unsigned decimal
/// string, exactly as the shared `digest_witness` fragment spells its
/// own `bytes`.
fn count_gate(lane: u32, member: &str, value: &str) -> Result<(), ExtensionsAnalyzeError> {
    if !is_canonical_decimal(value) {
        return Err(ExtensionsAnalyzeError::ByteCountNotCanonical {
            lane,
            member: member.to_string(),
            value: preview(value),
        });
    }
    Ok(())
}

/// Parse a byte count the cell has already judged, or is about to judge,
/// canonical. Saturates on any other value rather than panicking, so no
/// caller can turn a wire string into a crash; every value that passed
/// [`count_gate`] parses exactly.
fn parse_count(value: &str) -> u128 {
    if is_canonical_decimal(value) {
        value.parse().unwrap_or(u128::MAX)
    } else {
        u128::MAX
    }
}

fn stage_text(stage: &Stage) -> &'static str {
    match stage {
        Stage::Lane => "lane",
        Stage::Emitted => "emitted",
    }
}

fn kind_text(kind: &ContributionKind) -> &'static str {
    match kind {
        ContributionKind::Normal => "normal",
        ContributionKind::Simple => "simple",
        ContributionKind::Elided => "elided",
        ContributionKind::Hoisted => "hoisted",
    }
}

fn preview(value: &str) -> ScalarPreview {
    ScalarPreview::of(value)
}
