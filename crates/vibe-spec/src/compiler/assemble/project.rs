//! Lossless typed projection of one validated whole-artifact LinkResult.

use super::super::ir::{
    ArtifactFrame, ClosureContribution, ClosureIr, DocumentAddress, LaneChunk, LaneContribution,
    LaneFrame, LaneIr, LaneNode, LinkContributionWitness, LinkOccurrence, LinkResult,
};

#[derive(Debug, thiserror::Error)]
pub(crate) enum LaneProjectionError {
    #[error("linked artifact differs from Closure at {field}")]
    ArtifactMismatch { field: &'static str },
    #[error("contribution {contribution} differs between Closure and LinkResult")]
    ContributionMismatch { contribution: usize },
    #[error("linked occurrence {index} is missing for contribution {contribution}")]
    MissingOccurrence { contribution: usize, index: usize },
    #[error("linked occurrence {index} differs at {field}")]
    OccurrenceMismatch { index: usize, field: &'static str },
    #[error("linked occurrence {index} names missing closure node {node}")]
    MissingNode { index: usize, node: usize },
    #[error("linked stream has an unclaimed occurrence at index {index}")]
    TrailingOccurrence { index: usize },
}

pub(super) fn project_lane(
    closure: &ClosureIr,
    link: &LinkResult,
) -> Result<LaneIr, LaneProjectionError> {
    if link.mode != closure.context().mode() {
        return Err(LaneProjectionError::ArtifactMismatch {
            field: "compile mode",
        });
    }
    let mut cursor = OccurrenceCursor::new(&link.occurrences);
    let mut contributions = Vec::with_capacity(link.contributions.len());
    for (contribution, witness) in link.contributions.iter().enumerate() {
        let source = closure
            .contributions
            .get(contribution)
            .ok_or(LaneProjectionError::ContributionMismatch { contribution })?;
        match (witness, source) {
            (
                LinkContributionWitness::Normal {
                    meta,
                    seed,
                    seed_address,
                    occurrence_count,
                },
                ClosureContribution::Normal { .. },
            ) => {
                let mut chunks = Vec::new();
                for occurrence in 0..*occurrence_count {
                    project_normal(closure, &mut cursor, &mut chunks, contribution, occurrence)?;
                }
                contributions.push(LaneContribution::Normal {
                    meta: meta.clone(),
                    seed: *seed,
                    seed_address: seed_address.clone(),
                    chunks,
                });
            }
            (
                LinkContributionWitness::Simple { meta, address },
                ClosureContribution::Simple { document, .. },
            ) => {
                let mut chunks = Vec::new();
                project_simple(
                    &mut cursor,
                    &mut chunks,
                    contribution,
                    address,
                    &document.origin,
                )?;
                contributions.push(LaneContribution::Simple {
                    meta: meta.clone(),
                    address: address.clone(),
                    chunks,
                });
            }
            (LinkContributionWitness::Elided { meta }, ClosureContribution::Elided { .. }) => {
                contributions.push(LaneContribution::Elided { meta: meta.clone() })
            }
            (
                LinkContributionWitness::Hoisted { meta, target },
                ClosureContribution::Hoisted { .. },
            ) => contributions.push(LaneContribution::Hoisted {
                meta: meta.clone(),
                target: target.clone(),
            }),
            _ => return Err(LaneProjectionError::ContributionMismatch { contribution }),
        }
    }
    if cursor.peek().is_some() {
        return Err(LaneProjectionError::TrailingOccurrence {
            index: cursor.index,
        });
    }
    let (generated_path, source_root) = match closure.context().frame() {
        ArtifactFrame::StaticLane {
            generated_path,
            source_root,
        } => (Some(generated_path.clone()), Some(source_root.clone())),
        ArtifactFrame::CompatibilityFragment => (None, None),
    };
    Ok(LaneIr::assembled(
        closure.context().clone(),
        closure.nodes.len(),
        link.input_digest.clone(),
        LaneFrame {
            generated_path,
            source_root,
            renames: closure.renames.clone(),
        },
        contributions,
    ))
}

fn project_normal(
    closure: &ClosureIr,
    cursor: &mut OccurrenceCursor<'_>,
    output: &mut Vec<LaneChunk>,
    contribution: usize,
    occurrence: usize,
) -> Result<(), LaneProjectionError> {
    let index = cursor.index;
    let current = cursor.take(contribution)?;
    let LinkOccurrence::Normal {
        contribution: actual_contribution,
        occurrence: actual_occurrence,
        node,
        address,
        marker,
        fence_before,
        fence_after,
        body,
        trailing_newline_required,
    } = current
    else {
        return Err(LaneProjectionError::OccurrenceMismatch {
            index,
            field: "kind",
        });
    };
    require(*actual_contribution == contribution, index, "contribution")?;
    require(*actual_occurrence == occurrence, index, "occurrence")?;
    let document = closure
        .nodes
        .get(node.0)
        .ok_or(LaneProjectionError::MissingNode {
            index,
            node: node.0,
        })?;
    let DocumentAddress::Spec(document_address) = &document.address else {
        return Err(LaneProjectionError::OccurrenceMismatch {
            index,
            field: "document address kind",
        });
    };
    require(
        document_address.without_pin() == address.without_pin(),
        index,
        "document address",
    )?;
    output.push(LaneChunk::NormalOpen {
        contribution,
        occurrence,
        marker: marker.clone(),
    });
    output.push(LaneChunk::Node(Box::new(LaneNode::Normal {
        contribution,
        occurrence,
        node: *node,
        requested_address: address.clone(),
        origin: document.origin.clone(),
        marker: marker.clone(),
        fence_before: *fence_before,
        fence_after: *fence_after,
        body: body.clone(),
    })));
    if *trailing_newline_required {
        output.push(LaneChunk::ForcedNewline {
            contribution,
            occurrence,
        });
    }
    output.push(LaneChunk::NormalClose {
        contribution,
        occurrence,
        marker: marker.clone(),
    });
    Ok(())
}

fn project_simple(
    cursor: &mut OccurrenceCursor<'_>,
    output: &mut Vec<LaneChunk>,
    contribution: usize,
    address: &DocumentAddress,
    origin: &str,
) -> Result<(), LaneProjectionError> {
    let index = cursor.index;
    let current = cursor.take(contribution)?;
    let LinkOccurrence::Simple {
        contribution: actual_contribution,
        occurrence,
        address: actual_address,
        fence_before,
        fence_after,
        body,
        trailing_newline_required,
    } = current
    else {
        return Err(LaneProjectionError::OccurrenceMismatch {
            index,
            field: "kind",
        });
    };
    require(*actual_contribution == contribution, index, "contribution")?;
    require(*occurrence == 0, index, "occurrence")?;
    require(actual_address == address, index, "address")?;
    output.push(LaneChunk::Node(Box::new(LaneNode::Simple {
        contribution,
        occurrence: 0,
        address: actual_address.clone(),
        origin: origin.to_string(),
        fence_before: *fence_before,
        fence_after: *fence_after,
        body: body.clone(),
    })));
    if *trailing_newline_required {
        output.push(LaneChunk::ForcedNewline {
            contribution,
            occurrence: 0,
        });
    }
    Ok(())
}

fn require(condition: bool, index: usize, field: &'static str) -> Result<(), LaneProjectionError> {
    if condition {
        Ok(())
    } else {
        Err(LaneProjectionError::OccurrenceMismatch { index, field })
    }
}

struct OccurrenceCursor<'a> {
    occurrences: &'a [LinkOccurrence],
    index: usize,
}

impl<'a> OccurrenceCursor<'a> {
    fn new(occurrences: &'a [LinkOccurrence]) -> Self {
        Self {
            occurrences,
            index: 0,
        }
    }

    fn peek(&self) -> Option<&'a LinkOccurrence> {
        self.occurrences.get(self.index)
    }

    fn take(&mut self, contribution: usize) -> Result<&'a LinkOccurrence, LaneProjectionError> {
        let index = self.index;
        let occurrence = self.peek().ok_or(LaneProjectionError::MissingOccurrence {
            contribution,
            index,
        })?;
        self.index += 1;
        Ok(occurrence)
    }
}
