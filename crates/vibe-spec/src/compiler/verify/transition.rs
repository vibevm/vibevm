//! Immutable pre-pass witnesses and the input/output transition law.
//!
//! Planned/applied alignment alone cannot authenticate the semantic value of an
//! `absorbed` bit: a pass can flip a disposition and adjust the live order to
//! match. The manager therefore derives a witness from the *pass input* and the
//! verifier compares the just-produced value against it. The witness is
//! comparison evidence for one invocation — never a second IR store, never a
//! repair, and never a wire field.

use super::super::ir::{
    AbsorptionPlan, AbsorptionState, ClosureIr, DocumentAddress, QualificationState, SourceFormatId,
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
    Lane,
    Emitted,
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
        AnyIr::Lane(_) => Ok(VerificationWitness::Lane),
        AnyIr::Emitted(_) => Ok(VerificationWitness::Emitted),
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
        _ => Ok(()),
    }
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
