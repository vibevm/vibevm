//! Immutable pre-pass witnesses and the input/output transition law.
//!
//! Planned/applied alignment alone cannot authenticate the semantic value of an
//! `absorbed` bit: a pass can flip a disposition and adjust the live order to
//! match. The manager therefore derives a witness from the *pass input* and the
//! verifier compares the just-produced value against it. The witness is
//! comparison evidence for one invocation — never a second IR store, never a
//! repair, and never a wire field.

use std::fmt;

use super::super::ir::{
    AbsorptionPlan, AbsorptionState, ArtifactContext, ClosureIr, DocumentAddress, LaneIr,
    LinkInputDigest, OriginRename, QualificationState, SourceFormatId,
};
use super::super::pass::AnyIr;
use super::super::qualify::analyze_absorption;
use super::{VerificationError, address_label};

/// Why a pass output is not a legal transition from its own verified input.
#[derive(Debug, thiserror::Error)]
pub(crate) enum TransitionError {
    #[error(
        "a source/document pass changed the carrier identity: expected `{expected}`, got `{actual}`"
    )]
    Identity { expected: String, actual: String },
    #[error("absorption typestate regressed from {from} to {to}")]
    AbsorptionRegression {
        from: &'static str,
        to: &'static str,
    },
    #[error("absorption skipped planning: an unplanned carrier became applied in one pass")]
    AbsorptionSkippedPlanning,
    #[error(
        "a plan produced from an unplanned carrier does not match the pre-pass semantic analysis of its input"
    )]
    AbsorptionPlanUnauthenticated,
    #[error("an established absorption plan was mutated by a later pass")]
    AbsorptionPlanMutated,
    #[error("qualification typestate regressed from applied to pending")]
    QualificationRegression,
    #[error(
        "a lane transform rewrote the immutable provenance field `{field}`: expected `{expected}`, got `{actual}`"
    )]
    LaneProvenance {
        field: LaneProvenanceField,
        expected: String,
        actual: String,
    },
}

/// Which immutable lane provenance field a lane transform moved.
///
/// The set is the executable spelling of the witness boundary: every member
/// describes what the closure and link stages produced, and the one lane
/// member deliberately absent from it — `contributions` — is the working
/// surface a lane transform may rewrite.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LaneProvenanceField {
    SourceNodeCount,
    SourceLinkDigest,
    FrameGeneratedPath,
    FrameSourceRoot,
    FrameRenames,
    Context,
}

impl LaneProvenanceField {
    /// The field's exact spelling in the lane value, so a refusal names the
    /// thing a reader can go and look at.
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::SourceNodeCount => "source_node_count",
            Self::SourceLinkDigest => "source_link_digest",
            Self::FrameGeneratedPath => "frame.generated_path",
            Self::FrameSourceRoot => "frame.source_root",
            Self::FrameRenames => "frame.renames",
            Self::Context => "context",
        }
    }
}

impl fmt::Display for LaneProvenanceField {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Ephemeral, private comparison evidence derived from one valid pass input.
#[derive(Debug)]
pub(crate) enum VerificationWitness {
    Source {
        address: DocumentAddress,
        format: SourceFormatId,
    },
    Document {
        address: DocumentAddress,
        format: SourceFormatId,
    },
    Documents,
    Closure(ClosureWitness),
    Lane(LaneWitness),
    Emitted,
}

/// The immutable provenance of one lane, copied from the pass INPUT.
///
/// Field by field rather than a whole [`LaneIr`] clone, because the witness IS
/// the statement of what a lane transform may not touch: `context`,
/// `source_node_count`, `source_link_digest` and the three parts of `frame`
/// describe what the closure and link stages produced, while `contributions`
/// is absent on purpose — it is the working surface. A future reader gets the
/// boundary off the type instead of re-deriving the argument.
///
/// `frame` is carried as its three parts rather than as one `LaneFrame`
/// because `frame.renames` flows onward into `EmissionProvenance.renames`: a
/// transform that rewrote them would forge provenance the manager alone owns,
/// so each part is named separately in the refusal.
#[derive(Debug, Clone)]
pub(crate) struct LaneWitness {
    context: ArtifactContext,
    source_node_count: usize,
    source_link_digest: LinkInputDigest,
    generated_path: Option<String>,
    source_root: Option<String>,
    renames: Vec<OriginRename>,
}

/// The pre-pass closure state: the absorption plan a pass may establish (for
/// an unplanned input, the pure analysis of that exact view) plus the
/// qualification state it must not regress from.
#[derive(Debug)]
pub(crate) struct ClosureWitness {
    qualification: QualificationState,
    absorption: AbsorptionWitness,
}

#[derive(Debug)]
enum AbsorptionWitness {
    Unplanned { expected: AbsorptionPlan },
    Planned(AbsorptionPlan),
    Applied(AbsorptionPlan),
}

impl AbsorptionWitness {
    fn rank(&self) -> u8 {
        match self {
            Self::Unplanned { .. } => 0,
            Self::Planned(_) => 1,
            Self::Applied(_) => 2,
        }
    }

    fn plan(&self) -> &AbsorptionPlan {
        match self {
            Self::Unplanned { expected } | Self::Planned(expected) | Self::Applied(expected) => {
                expected
            }
        }
    }
}

fn rank_name(rank: u8) -> &'static str {
    match rank {
        0 => "unplanned",
        1 => "planned",
        _ => "applied",
    }
}

pub(super) fn witness(ir: &AnyIr) -> Result<VerificationWitness, VerificationError> {
    match ir {
        AnyIr::Source(source) => Ok(VerificationWitness::Source {
            address: source.address().clone(),
            format: source.format().clone(),
        }),
        AnyIr::Document(document) => Ok(VerificationWitness::Document {
            address: document.source().address().clone(),
            format: document.source().format().clone(),
        }),
        AnyIr::Documents(_) => Ok(VerificationWitness::Documents),
        AnyIr::Closure(closure) => Ok(VerificationWitness::Closure(ClosureWitness {
            qualification: closure.qualification,
            absorption: match &closure.absorption {
                AbsorptionState::Unplanned => AbsorptionWitness::Unplanned {
                    expected: analyze_absorption(closure).map_err(|source| {
                        VerificationError::AbsorptionAnalyze {
                            source: Box::new(source),
                        }
                    })?,
                },
                AbsorptionState::Planned(plan) => AbsorptionWitness::Planned(plan.clone()),
                AbsorptionState::Applied(plan) => AbsorptionWitness::Applied(plan.clone()),
            },
        })),
        AnyIr::Lane(lane) => Ok(VerificationWitness::Lane(lane_witness(lane))),
        AnyIr::Emitted(_) => Ok(VerificationWitness::Emitted),
    }
}

/// Derive the immutable lane evidence of one valid lane.
///
/// Infallible by construction: every field is copied and nothing is analysed,
/// so the manager-side lane gate that consumes it has no impossible error arm
/// to eliminate by panic.
pub(crate) fn lane_witness(lane: &LaneIr) -> LaneWitness {
    LaneWitness {
        context: lane.context().clone(),
        source_node_count: lane.source_node_count,
        source_link_digest: lane.source_link_digest.clone(),
        generated_path: lane.frame.generated_path.clone(),
        source_root: lane.frame.source_root.clone(),
        renames: lane.frame.renames.clone(),
    }
}

pub(super) fn verify(before: &VerificationWitness, ir: &AnyIr) -> Result<(), VerificationError> {
    match (before, ir) {
        (VerificationWitness::Source { address, format }, AnyIr::Source(actual)) => {
            verify_identity(address, format, actual.address(), actual.format())
        }
        (VerificationWitness::Source { address, format }, AnyIr::Document(actual)) => {
            verify_identity(
                address,
                format,
                actual.source().address(),
                actual.source().format(),
            )
        }
        (VerificationWitness::Document { address, format }, AnyIr::Document(actual)) => {
            verify_identity(
                address,
                format,
                actual.source().address(),
                actual.source().format(),
            )
        }
        (VerificationWitness::Closure(witness), AnyIr::Closure(actual)) => {
            verify_closure_transition(witness, actual)
        }
        (VerificationWitness::Lane(witness), AnyIr::Lane(actual)) => {
            verify_lane_transition(witness, actual).map_err(VerificationError::from)
        }
        _ => Ok(()),
    }
}

/// Authenticate one lane against the evidence of the value that produced it.
///
/// A lane transform owns `contributions` and nothing else. Every other member
/// is provenance: the manager derived it from the closure and link stages, and
/// `frame.renames` in particular flows onward into `EmissionProvenance`, so a
/// rewrite there would forge a record the manager alone authors.
///
/// The order below is diagnostic, never permissive — any difference in any
/// field refuses. The narrow fields come first so a transform that rewrote a
/// frame part in BOTH halves (the only way such a rewrite survives the
/// intrinsic frame/context agreement check) is named by the exact part it
/// moved, and the whole-context comparison stays as the residual that still
/// catches an artifact id, target or compile-mode rewrite.
pub(crate) fn verify_lane_transition(
    before: &LaneWitness,
    actual: &LaneIr,
) -> Result<(), TransitionError> {
    immutable(
        LaneProvenanceField::SourceNodeCount,
        &before.source_node_count,
        &actual.source_node_count,
    )?;
    if before.source_link_digest != actual.source_link_digest {
        return Err(TransitionError::LaneProvenance {
            field: LaneProvenanceField::SourceLinkDigest,
            expected: digest_label(&before.source_link_digest),
            actual: digest_label(&actual.source_link_digest),
        });
    }
    immutable(
        LaneProvenanceField::FrameGeneratedPath,
        &before.generated_path,
        &actual.frame.generated_path,
    )?;
    immutable(
        LaneProvenanceField::FrameSourceRoot,
        &before.source_root,
        &actual.frame.source_root,
    )?;
    immutable(
        LaneProvenanceField::FrameRenames,
        &before.renames,
        &actual.frame.renames,
    )?;
    immutable(
        LaneProvenanceField::Context,
        &before.context,
        actual.context(),
    )
}

/// One provenance field compared, with both values named in the refusal.
fn immutable<T: PartialEq + fmt::Debug>(
    field: LaneProvenanceField,
    expected: &T,
    actual: &T,
) -> Result<(), TransitionError> {
    if expected == actual {
        Ok(())
    } else {
        Err(TransitionError::LaneProvenance {
            field,
            expected: format!("{expected:?}"),
            actual: format!("{actual:?}"),
        })
    }
}

/// A link digest rendered as lowercase hex: a refusal names a digest, never a
/// thirty-two element Rust array literal.
fn digest_label(digest: &LinkInputDigest) -> String {
    let mut label = String::with_capacity(digest.0.len() * 2);
    for byte in digest.0 {
        label.push(hex_digit(byte >> 4));
        label.push(hex_digit(byte & 0x0f));
    }
    label
}

fn hex_digit(nibble: u8) -> char {
    char::from_digit(u32::from(nibble), 16).unwrap_or('?')
}

/// A source-level transform may alter only text; a parse-position lowering may
/// not retarget the document it was handed.
fn verify_identity(
    expected_address: &DocumentAddress,
    expected_format: &SourceFormatId,
    actual_address: &DocumentAddress,
    actual_format: &SourceFormatId,
) -> Result<(), VerificationError> {
    if expected_address == actual_address && expected_format == actual_format {
        Ok(())
    } else {
        Err(TransitionError::Identity {
            expected: format!(
                "{} [{}]",
                address_label(expected_address),
                expected_format.as_str()
            ),
            actual: format!(
                "{} [{}]",
                address_label(actual_address),
                actual_format.as_str()
            ),
        }
        .into())
    }
}

fn verify_closure_transition(
    witness: &ClosureWitness,
    actual: &ClosureIr,
) -> Result<(), VerificationError> {
    let input_applied = matches!(witness.qualification, QualificationState::Applied(_));
    let output_applied = matches!(actual.qualification, QualificationState::Applied(_));
    if input_applied && !output_applied {
        return Err(TransitionError::QualificationRegression.into());
    }

    let input_rank = witness.absorption.rank();
    let output_rank = match &actual.absorption {
        AbsorptionState::Unplanned => 0,
        AbsorptionState::Planned(_) => 1,
        AbsorptionState::Applied(_) => 2,
    };
    if output_rank < input_rank {
        return Err(TransitionError::AbsorptionRegression {
            from: rank_name(input_rank),
            to: rank_name(output_rank),
        }
        .into());
    }
    if output_rank == 2 && input_rank == 0 {
        return Err(TransitionError::AbsorptionSkippedPlanning.into());
    }
    if output_rank == 0 {
        // Still unplanned: pre-qualify content/topology transforms are legal,
        // and the next planning pass derives its own witness from this view.
        return Ok(());
    }
    let expected_plan = witness.absorption.plan();
    let actual_plan = match &actual.absorption {
        AbsorptionState::Planned(plan) | AbsorptionState::Applied(plan) => plan,
        AbsorptionState::Unplanned => unreachable!("output_rank > 0 named a plan"),
    };
    if expected_plan != actual_plan {
        return Err(if input_rank == 0 {
            TransitionError::AbsorptionPlanUnauthenticated
        } else {
            TransitionError::AbsorptionPlanMutated
        }
        .into());
    }
    Ok(())
}
