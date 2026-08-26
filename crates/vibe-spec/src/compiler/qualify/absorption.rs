//! Immutable READ-ONCE analysis over the post-embed, pre-qualification view.

use crate::SpecAddress;

use super::QualifyPassError;
use crate::compiler::ir::{
    AbsorptionOccurrence, AbsorptionPlan, ClosureContribution, ClosureIr, ContributionAbsorption,
    ContributionMeta, DocumentAddress,
};

struct Candidate {
    address: SpecAddress,
    text: String,
}

pub(super) fn analyze(closure: &ClosureIr) -> Result<AbsorptionPlan, QualifyPassError> {
    let mut contributions = Vec::with_capacity(closure.contributions.len());

    for (contribution_index, contribution) in closure.contributions.iter().enumerate() {
        match contribution {
            ClosureContribution::Normal {
                meta,
                seed,
                emission_order,
            } => {
                let mut candidates = Vec::with_capacity(emission_order.len());
                for (occurrence, node_id) in emission_order.iter().copied().enumerate() {
                    let node =
                        closure
                            .nodes
                            .get(node_id.0)
                            .ok_or(QualifyPassError::InvalidNodeId {
                                contribution: contribution_index,
                                occurrence,
                                node: node_id.0,
                            })?;
                    let DocumentAddress::Spec(address) = &node.address else {
                        return Err(QualifyPassError::NonSpecGraphNode {
                            contribution: contribution_index,
                            occurrence,
                        });
                    };
                    candidates.push(Candidate {
                        address: address.clone(),
                        text: node.tree.text(node.tree.root()),
                    });
                }

                let dispositions: Vec<bool> = candidates
                    .iter()
                    .enumerate()
                    .map(|(i, node)| {
                        candidates.iter().enumerate().any(|(j, other)| {
                            i != j
                                && node.address.authority == other.address.authority
                                && node.address.doc_path == other.address.doc_path
                                && ((node.text.len() < other.text.len()
                                    && other.text.contains(node.text.as_str()))
                                    || (node.text == other.text && j < i))
                        })
                    })
                    .collect();
                let occurrences = emission_order
                    .iter()
                    .copied()
                    .zip(dispositions)
                    .map(|(node, absorbed)| AbsorptionOccurrence { node, absorbed })
                    .collect();
                contributions.push(ContributionAbsorption::Normal {
                    meta: meta.clone(),
                    seed: *seed,
                    occurrences,
                });
            }
            ClosureContribution::Simple { meta, document } => {
                contributions.push(ContributionAbsorption::Simple {
                    meta: meta.clone(),
                    address: document.address.clone(),
                });
            }
        }
    }

    Ok(AbsorptionPlan { contributions })
}

/// Validate occurrence alignment before either qualify or the legacy tail
/// consumes the plan. A node-id set cannot represent this invariant.
pub(super) fn validate(plan: &AbsorptionPlan, closure: &ClosureIr) -> Result<(), QualifyPassError> {
    if plan.contributions.len() != closure.contributions.len() {
        return Err(QualifyPassError::AbsorptionAlignment {
            contribution: None,
            expected: closure.contributions.len(),
            actual: plan.contributions.len(),
        });
    }

    for (index, (contribution, disposition)) in closure
        .contributions
        .iter()
        .zip(&plan.contributions)
        .enumerate()
    {
        match (contribution, disposition) {
            (
                ClosureContribution::Normal {
                    meta,
                    seed,
                    emission_order,
                },
                ContributionAbsorption::Normal {
                    meta: expected_meta,
                    seed: expected_seed,
                    occurrences,
                },
            ) => {
                validate_meta(index, expected_meta, meta)?;
                if expected_seed != seed {
                    return Err(QualifyPassError::AbsorptionSeed {
                        contribution: index,
                        expected: expected_seed.0,
                        actual: seed.0,
                    });
                }
                if occurrences.len() != emission_order.len() {
                    return Err(QualifyPassError::AbsorptionAlignment {
                        contribution: Some(index),
                        expected: emission_order.len(),
                        actual: occurrences.len(),
                    });
                }
                for (occurrence, (current, expected)) in
                    emission_order.iter().zip(occurrences).enumerate()
                {
                    if expected.node != *current {
                        return Err(QualifyPassError::AbsorptionOccurrence {
                            contribution: index,
                            occurrence,
                            expected: expected.node.0,
                            actual: current.0,
                        });
                    }
                    if closure.nodes.get(current.0).is_none() {
                        return Err(QualifyPassError::InvalidNodeId {
                            contribution: index,
                            occurrence,
                            node: current.0,
                        });
                    }
                }
            }
            (
                ClosureContribution::Simple { meta, document },
                ContributionAbsorption::Simple {
                    meta: expected_meta,
                    address: expected_address,
                },
            ) => {
                validate_meta(index, expected_meta, meta)?;
                if expected_address != &document.address {
                    return Err(QualifyPassError::AbsorptionContributionIdentity {
                        contribution: index,
                        expected: format!(
                            "{}:{}:{expected_address:?}",
                            expected_meta.origin, expected_meta.path
                        ),
                        actual: format!("{}:{}:{:?}", meta.origin, meta.path, document.address),
                    });
                }
            }
            _ => {
                return Err(QualifyPassError::AbsorptionKind {
                    contribution: index,
                });
            }
        }
    }

    Ok(())
}

fn validate_meta(
    contribution: usize,
    expected: &ContributionMeta,
    actual: &ContributionMeta,
) -> Result<(), QualifyPassError> {
    if expected == actual {
        return Ok(());
    }
    let identity = |meta: &ContributionMeta| format!("{}:{}", meta.origin, meta.path);
    Err(QualifyPassError::AbsorptionContributionIdentity {
        contribution,
        expected: identity(expected),
        actual: identity(actual),
    })
}
