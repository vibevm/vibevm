//! Immutable READ-ONCE analysis over the post-embed, pre-qualification view.

use crate::SpecAddress;

use super::QualifyPassError;
use crate::compiler::ir::{
    AbsorptionOccurrence, AbsorptionPlan, ClosureContribution, ClosureIr, ContributionAbsorption,
    ContributionMeta, DocumentAddress, QualificationState,
};

struct Candidate {
    address: SpecAddress,
    text: String,
}

pub(super) fn analyze(closure: &ClosureIr) -> Result<AbsorptionPlan, QualifyPassError> {
    let mode = match closure.qualification {
        QualificationState::Pending(mode) | QualificationState::Applied(mode) => mode,
    };
    let mut contributions = Vec::with_capacity(closure.contributions.len());

    for (contribution_index, contribution) in closure.contributions.iter().enumerate() {
        match contribution {
            ClosureContribution::Normal {
                meta,
                seed,
                emission_order,
            } => {
                let seed_node =
                    closure
                        .nodes
                        .get(seed.0)
                        .ok_or(QualifyPassError::InvalidSeedNodeId {
                            contribution: contribution_index,
                            node: seed.0,
                        })?;
                let DocumentAddress::Spec(seed_address) = &seed_node.address else {
                    return Err(QualifyPassError::NonSpecSeedGraphNode {
                        contribution: contribution_index,
                        node: seed.0,
                    });
                };
                let seed_address = seed_address.clone();
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
                    .zip(&candidates)
                    .zip(dispositions)
                    .map(|((node, candidate), absorbed)| AbsorptionOccurrence {
                        node,
                        address: candidate.address.clone(),
                        absorbed,
                    })
                    .collect();
                contributions.push(ContributionAbsorption::Normal {
                    meta: meta.clone(),
                    seed: *seed,
                    seed_address,
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

    Ok(AbsorptionPlan {
        mode,
        contributions,
    })
}

/// Validate occurrence alignment before either qualify or the legacy tail
/// consumes the plan. A node-id set cannot represent this invariant.
pub(super) fn validate(plan: &AbsorptionPlan, closure: &ClosureIr) -> Result<(), QualifyPassError> {
    let actual_mode = match closure.qualification {
        QualificationState::Pending(mode) | QualificationState::Applied(mode) => mode,
    };
    if plan.mode != actual_mode {
        return Err(QualifyPassError::AbsorptionMode {
            expected: plan.mode,
            actual: actual_mode,
        });
    }
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
                    seed_address: expected_seed_address,
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
                let seed_node =
                    closure
                        .nodes
                        .get(seed.0)
                        .ok_or(QualifyPassError::InvalidSeedNodeId {
                            contribution: index,
                            node: seed.0,
                        })?;
                let DocumentAddress::Spec(actual_seed_address) = &seed_node.address else {
                    return Err(QualifyPassError::NonSpecSeedGraphNode {
                        contribution: index,
                        node: seed.0,
                    });
                };
                if expected_seed_address != actual_seed_address {
                    return Err(QualifyPassError::AbsorptionSeedAddress {
                        contribution: index,
                        node: seed.0,
                        expected: Box::new(expected_seed_address.clone()),
                        actual: Box::new(actual_seed_address.clone()),
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
                    let current_node =
                        closure
                            .nodes
                            .get(current.0)
                            .ok_or(QualifyPassError::InvalidNodeId {
                                contribution: index,
                                occurrence,
                                node: current.0,
                            })?;
                    let DocumentAddress::Spec(actual_address) = &current_node.address else {
                        return Err(QualifyPassError::NonSpecGraphNode {
                            contribution: index,
                            occurrence,
                        });
                    };
                    if expected.address != *actual_address {
                        return Err(QualifyPassError::AbsorptionOccurrenceAddress {
                            contribution: index,
                            occurrence,
                            node: current.0,
                            expected: Box::new(expected.address.clone()),
                            actual: Box::new(actual_address.clone()),
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
