//! Independent Closure + validated Link + supplied Lane relational proof.

use super::super::ir::{
    ArtifactFrame, ClosureContribution, ClosureIr, DocumentAddress, LaneContribution, LaneIr,
    LinkContributionWitness, LinkResult, LinkState,
};
use super::super::link::{LinkPassError, validate_linked};
use super::validate::{LaneValidationError, validate_lane};

mod contributions;
use contributions::{validate_normal, validate_simple};

#[derive(Debug, thiserror::Error)]
pub(crate) enum LaneTransitionError {
    #[error("linked source is invalid: {0}")]
    InvalidLink(#[source] Box<LinkPassError>),
    #[error("lane is intrinsically invalid: {0}")]
    InvalidLane(#[source] Box<LaneValidationError>),
    #[error("assembled lane differs at {field}")]
    ArtifactField { field: &'static str },
    #[error("{carrier} is missing contribution {contribution}")]
    MissingContribution {
        carrier: &'static str,
        contribution: usize,
    },
    #[error("contribution {contribution} differs at {field}")]
    ContributionField {
        contribution: usize,
        field: &'static str,
    },
    #[error("linked occurrence {index} is missing for contribution {contribution}")]
    MissingOccurrence { contribution: usize, index: usize },
    #[error("linked occurrence {index} is {actual}; expected {expected}")]
    UnexpectedOccurrence {
        index: usize,
        expected: &'static str,
        actual: &'static str,
    },
    #[error("linked occurrence {index} differs at {field}")]
    OccurrenceField { index: usize, field: &'static str },
    #[error("linked occurrence {index} names missing closure node {node}")]
    MissingNode { index: usize, node: usize },
    #[error("linked occurrence {index} closure node {node} is not a spec document")]
    NonSpecNode { index: usize, node: usize },
    #[error("lane contribution {contribution} chunk {chunk} is missing; expected {expected}")]
    MissingChunk {
        contribution: usize,
        chunk: usize,
        expected: &'static str,
    },
    #[error("lane contribution {contribution} chunk {chunk} is {actual}; expected {expected}")]
    UnexpectedChunk {
        contribution: usize,
        chunk: usize,
        expected: &'static str,
        actual: &'static str,
    },
    #[error("lane contribution {contribution} chunk {chunk} differs at {field}")]
    ChunkField {
        contribution: usize,
        chunk: usize,
        field: &'static str,
    },
    #[error("lane contribution {contribution} has trailing chunk {chunk}")]
    TrailingChunk { contribution: usize, chunk: usize },
    #[error("linked artifact has trailing occurrence {index}")]
    TrailingOccurrence { index: usize },
    #[error("linked state disappeared after validation")]
    MissingLinkedState,
}

/// Validate the exact relational postcondition without invoking the projector.
pub(crate) fn validate_assembled_transition(
    closure: &ClosureIr,
    lane: &LaneIr,
) -> Result<(), LaneTransitionError> {
    validate_linked(closure).map_err(|error| LaneTransitionError::InvalidLink(Box::new(error)))?;
    validate_lane(lane).map_err(|error| LaneTransitionError::InvalidLane(Box::new(error)))?;
    let LinkState::Linked(link) = &closure.link else {
        return Err(LaneTransitionError::MissingLinkedState);
    };
    validate_artifact_fields(closure, link, lane)?;

    let count = closure
        .contributions
        .len()
        .max(link.contributions.len())
        .max(lane.contributions.len());
    let mut occurrence_index = 0;
    for contribution in 0..count {
        let source = closure.contributions.get(contribution).ok_or(
            LaneTransitionError::MissingContribution {
                carrier: "Closure",
                contribution,
            },
        )?;
        let witness = link.contributions.get(contribution).ok_or(
            LaneTransitionError::MissingContribution {
                carrier: "LinkResult",
                contribution,
            },
        )?;
        let assembled = lane.contributions.get(contribution).ok_or(
            LaneTransitionError::MissingContribution {
                carrier: "Lane",
                contribution,
            },
        )?;
        validate_contribution(
            closure,
            link,
            contribution,
            source,
            witness,
            assembled,
            &mut occurrence_index,
        )?;
    }
    if occurrence_index != link.occurrences.len() {
        return Err(LaneTransitionError::TrailingOccurrence {
            index: occurrence_index,
        });
    }
    Ok(())
}

fn validate_artifact_fields(
    closure: &ClosureIr,
    link: &LinkResult,
    lane: &LaneIr,
) -> Result<(), LaneTransitionError> {
    artifact(closure.context() == lane.context(), "artifact context")?;
    artifact(link.mode == closure.context().mode(), "compile mode")?;
    artifact(
        closure.nodes.len() == lane.source_node_count,
        "source node count",
    )?;
    artifact(
        link.input_digest == lane.source_link_digest,
        "source link digest",
    )?;
    artifact(closure.renames == lane.frame.renames, "ordered renames")?;
    match closure.context().frame() {
        ArtifactFrame::CompatibilityFragment => {
            artifact(lane.frame.generated_path.is_none(), "generated path")?;
            artifact(lane.frame.source_root.is_none(), "source root")
        }
        ArtifactFrame::StaticLane {
            generated_path,
            source_root,
        } => {
            artifact(
                lane.frame.generated_path.as_ref() == Some(generated_path),
                "generated path",
            )?;
            artifact(
                lane.frame.source_root.as_ref() == Some(source_root),
                "source root",
            )
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn validate_contribution(
    closure: &ClosureIr,
    link: &LinkResult,
    contribution: usize,
    source: &ClosureContribution,
    witness: &LinkContributionWitness,
    assembled: &LaneContribution,
    occurrence_index: &mut usize,
) -> Result<(), LaneTransitionError> {
    match (source, witness, assembled) {
        (
            ClosureContribution::Normal {
                meta,
                seed,
                seed_address,
                emission_order,
            },
            LinkContributionWitness::Normal {
                meta: link_meta,
                seed: link_seed,
                seed_address: link_seed_address,
                occurrence_count,
            },
            LaneContribution::Normal {
                meta: lane_meta,
                seed: lane_seed,
                seed_address: lane_seed_address,
                chunks,
            },
        ) => {
            contribution_field(meta == link_meta && meta == lane_meta, contribution, "meta")?;
            contribution_field(seed == link_seed && seed == lane_seed, contribution, "seed")?;
            contribution_field(
                seed_address == link_seed_address && seed_address == lane_seed_address,
                contribution,
                "exact seed address",
            )?;
            contribution_field(
                *occurrence_count == emission_order.len(),
                contribution,
                "occurrence count",
            )?;
            validate_seed(closure, contribution, seed.0, seed_address)?;
            validate_normal(
                closure,
                link,
                contribution,
                emission_order,
                chunks,
                occurrence_index,
            )
        }
        (
            ClosureContribution::Simple { meta, document },
            LinkContributionWitness::Simple {
                meta: link_meta,
                address: link_address,
            },
            LaneContribution::Simple {
                meta: lane_meta,
                address: lane_address,
                chunks,
            },
        ) => {
            contribution_field(meta == link_meta && meta == lane_meta, contribution, "meta")?;
            contribution_field(
                document.address == *link_address && document.address == *lane_address,
                contribution,
                "simple address",
            )?;
            validate_simple(link, contribution, document, chunks, occurrence_index)
        }
        (
            ClosureContribution::Elided { meta },
            LinkContributionWitness::Elided { meta: link_meta },
            LaneContribution::Elided { meta: lane_meta },
        ) => contribution_field(meta == link_meta && meta == lane_meta, contribution, "meta"),
        (
            ClosureContribution::Hoisted { meta, target },
            LinkContributionWitness::Hoisted {
                meta: link_meta,
                target: link_target,
            },
            LaneContribution::Hoisted {
                meta: lane_meta,
                target: lane_target,
            },
        ) => {
            contribution_field(meta == link_meta && meta == lane_meta, contribution, "meta")?;
            contribution_field(
                target == link_target && target == lane_target,
                contribution,
                "hoisted target",
            )
        }
        _ => Err(LaneTransitionError::ContributionField {
            contribution,
            field: "kind",
        }),
    }
}

fn validate_seed(
    closure: &ClosureIr,
    contribution: usize,
    node: usize,
    requested: &crate::SpecAddress,
) -> Result<(), LaneTransitionError> {
    let document = closure
        .nodes
        .get(node)
        .ok_or(LaneTransitionError::MissingNode {
            index: contribution,
            node,
        })?;
    let DocumentAddress::Spec(address) = &document.address else {
        return Err(LaneTransitionError::NonSpecNode {
            index: contribution,
            node,
        });
    };
    contribution_field(
        address.without_pin() == requested.without_pin(),
        contribution,
        "seed node address",
    )
}

fn artifact(condition: bool, field: &'static str) -> Result<(), LaneTransitionError> {
    if condition {
        Ok(())
    } else {
        Err(LaneTransitionError::ArtifactField { field })
    }
}

fn contribution_field(
    condition: bool,
    contribution: usize,
    field: &'static str,
) -> Result<(), LaneTransitionError> {
    if condition {
        Ok(())
    } else {
        Err(LaneTransitionError::ContributionField {
            contribution,
            field,
        })
    }
}
