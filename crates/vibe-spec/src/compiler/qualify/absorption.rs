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
                seed_address,
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
                let DocumentAddress::Spec(node_seed_address) = &seed_node.address else {
                    return Err(QualifyPassError::NonSpecSeedGraphNode {
                        contribution: contribution_index,
                        node: seed.0,
                    });
                };
                debug_assert_eq!(node_seed_address.without_pin(), seed_address.without_pin());
                let mut candidates = Vec::with_capacity(emission_order.len());
                for (occurrence, current) in emission_order.iter().enumerate() {
                    let node = closure.nodes.get(current.node.0).ok_or(
                        QualifyPassError::InvalidNodeId {
                            contribution: contribution_index,
                            occurrence,
                            node: current.node.0,
                        },
                    )?;
                    let DocumentAddress::Spec(address) = &node.address else {
                        return Err(QualifyPassError::NonSpecGraphNode {
                            contribution: contribution_index,
                            occurrence,
                        });
                    };
                    debug_assert_eq!(
                        address.without_pin(),
                        current.requested_address.without_pin()
                    );
                    candidates.push(Candidate {
                        address: current.requested_address.clone(),
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
                    .zip(&candidates)
                    .zip(dispositions)
                    .map(|((current, candidate), absorbed)| AbsorptionOccurrence {
                        node: current.node,
                        requested_address: candidate.address.clone(),
                        absorbed,
                    })
                    .collect();
                contributions.push(ContributionAbsorption::Normal {
                    meta: meta.clone(),
                    seed: *seed,
                    seed_address: seed_address.clone(),
                    occurrences,
                });
            }
            ClosureContribution::Simple { meta, document } => {
                contributions.push(ContributionAbsorption::Simple {
                    meta: meta.clone(),
                    address: document.address.clone(),
                });
            }
            ClosureContribution::Elided { meta } => {
                contributions.push(ContributionAbsorption::Elided { meta: meta.clone() });
            }
            ClosureContribution::Hoisted { meta, target } => {
                contributions.push(ContributionAbsorption::Hoisted {
                    meta: meta.clone(),
                    target: target.clone(),
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
                    seed_address,
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
                let DocumentAddress::Spec(node_seed_address) = &seed_node.address else {
                    return Err(QualifyPassError::NonSpecSeedGraphNode {
                        contribution: index,
                        node: seed.0,
                    });
                };
                if node_seed_address.without_pin() != seed_address.without_pin() {
                    return Err(QualifyPassError::AbsorptionSeedAddress {
                        contribution: index,
                        node: seed.0,
                        expected: Box::new(seed_address.clone()),
                        actual: Box::new(node_seed_address.clone()),
                    });
                }
                if expected_seed_address != seed_address {
                    return Err(QualifyPassError::AbsorptionSeedAddress {
                        contribution: index,
                        node: seed.0,
                        expected: Box::new(expected_seed_address.clone()),
                        actual: Box::new(seed_address.clone()),
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
                    if expected.node != current.node {
                        return Err(QualifyPassError::AbsorptionOccurrence {
                            contribution: index,
                            occurrence,
                            expected: expected.node.0,
                            actual: current.node.0,
                        });
                    }
                    let current_node = closure.nodes.get(current.node.0).ok_or(
                        QualifyPassError::InvalidNodeId {
                            contribution: index,
                            occurrence,
                            node: current.node.0,
                        },
                    )?;
                    let DocumentAddress::Spec(actual_address) = &current_node.address else {
                        return Err(QualifyPassError::NonSpecGraphNode {
                            contribution: index,
                            occurrence,
                        });
                    };
                    if actual_address.without_pin() != current.requested_address.without_pin() {
                        return Err(QualifyPassError::AbsorptionOccurrenceAddress {
                            contribution: index,
                            occurrence,
                            node: current.node.0,
                            expected: Box::new(current.requested_address.clone()),
                            actual: Box::new(actual_address.clone()),
                        });
                    }
                    if expected.requested_address != current.requested_address {
                        return Err(QualifyPassError::AbsorptionOccurrenceAddress {
                            contribution: index,
                            occurrence,
                            node: current.node.0,
                            expected: Box::new(expected.requested_address.clone()),
                            actual: Box::new(current.requested_address.clone()),
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
            (
                ClosureContribution::Elided { meta },
                ContributionAbsorption::Elided {
                    meta: expected_meta,
                },
            ) => validate_meta(index, expected_meta, meta)?,
            (
                ClosureContribution::Hoisted { meta, target },
                ContributionAbsorption::Hoisted {
                    meta: expected_meta,
                    target: expected_target,
                },
            ) => {
                validate_meta(index, expected_meta, meta)?;
                if expected_target != target {
                    return Err(QualifyPassError::AbsorptionContributionIdentity {
                        contribution: index,
                        expected: format!(
                            "{}:{}:{expected_target}",
                            expected_meta.origin, expected_meta.path
                        ),
                        actual: format!("{}:{}:{target}", meta.origin, meta.path),
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
