//! The named whole-artifact READ-ONCE absorption pass.
//!
//! Qualify has already judged exact contribution occurrences from immutable
//! post-embed, pre-rewrite text. Absorb applies only that identity-bound plan:
//! normal emission orders become their live ordered projection, while the
//! graph, document payloads, simple contributions and rename audit stay whole.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#IR-REFACTOR");

use crate::SpecAddress;

use super::ir::{
    AbsorptionState, ClosureContribution, ClosureIr, ContributionAbsorption, ContributionMeta,
    DocumentAddress, QualificationState,
};
use super::pass::{Pass, PassName};
use super::qualify::{QualifyPassError, validate_planned_absorption};

pub(crate) const ABSORB_PASS_NAME: &str = "absorb";

pub(crate) struct AbsorbPass {
    name: PassName,
}

impl AbsorbPass {
    pub(crate) fn new() -> Self {
        Self {
            name: PassName::new(ABSORB_PASS_NAME)
                .expect("the static built-in absorb pass name is non-blank"),
        }
    }
}

impl Pass for AbsorbPass {
    type Input = ClosureIr;
    type Output = ClosureIr;
    type Error = AbsorbPassError;

    fn name(&self) -> &PassName {
        &self.name
    }

    fn run(&self, input: ClosureIr) -> Result<ClosureIr, AbsorbPassError> {
        #[cfg(test)]
        ABSORB_INVOCATIONS.with(|count| count.set(count.get() + 1));
        absorb_closure(input)
    }
}

#[cfg(test)]
std::thread_local! {
    static ABSORB_INVOCATIONS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
pub(crate) fn reset_absorb_invocations() {
    ABSORB_INVOCATIONS.with(|count| count.set(0));
}

#[cfg(test)]
pub(crate) fn absorb_invocations() -> usize {
    ABSORB_INVOCATIONS.with(std::cell::Cell::get)
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum AbsorbPassError {
    #[error("absorb requires applied qualification state")]
    QualificationPending,
    #[error("absorb requires named merge and embed to consume their pending state")]
    PendingEarlierPass,
    #[error("absorb requires qualify's planned absorption state")]
    Unplanned,
    #[error("absorb cannot apply an already applied absorption plan")]
    AlreadyApplied,
    #[error("applied absorption verifier requires applied state")]
    AppliedStateRequired,
    #[error("absorb received invalid qualification state: {0}")]
    InvalidPlan(#[source] QualifyPassError),
    #[error(
        "applied absorption is misaligned{suffix}: expected {expected} entries, got {actual}",
        suffix = contribution.map(|index| format!(" at contribution {index}")).unwrap_or_default()
    )]
    AppliedAlignment {
        contribution: Option<usize>,
        expected: usize,
        actual: usize,
    },
    #[error(
        "applied absorption contribution {contribution} identity changed: expected `{expected}`, got `{actual}`"
    )]
    AppliedContributionIdentity {
        contribution: usize,
        expected: String,
        actual: String,
    },
    #[error("applied absorption mode changed: expected {expected:?}, got {actual:?}")]
    AppliedMode {
        expected: super::ir::StaticCompileMode,
        actual: super::ir::StaticCompileMode,
    },
    #[error(
        "applied absorption contribution {contribution} seed changed: expected node {expected}, got {actual}"
    )]
    AppliedSeed {
        contribution: usize,
        expected: usize,
        actual: usize,
    },
    #[error("applied absorption contribution {contribution} seed names missing node {node}")]
    AppliedMissingSeedNode { contribution: usize, node: usize },
    #[error(
        "applied absorption contribution {contribution} seed node {node} is not a spec document"
    )]
    AppliedNonSpecSeedNode { contribution: usize, node: usize },
    #[error(
        "applied absorption contribution {contribution} seed node {node} address changed: expected {expected:?}, got {actual:?}"
    )]
    AppliedSeedAddress {
        contribution: usize,
        node: usize,
        expected: Box<SpecAddress>,
        actual: Box<SpecAddress>,
    },
    #[error(
        "applied absorption contribution {contribution} live occurrence {occurrence} changed: expected node {expected}, got {actual}"
    )]
    AppliedOccurrence {
        contribution: usize,
        occurrence: usize,
        expected: usize,
        actual: usize,
    },
    #[error("applied absorption plan kind differs from contribution {contribution}")]
    AppliedKind { contribution: usize },
    #[error(
        "applied absorption contribution {contribution} occurrence {occurrence} names missing closure node {node}"
    )]
    AppliedMissingNode {
        contribution: usize,
        occurrence: usize,
        node: usize,
    },
    #[error(
        "applied absorption contribution {contribution} occurrence {occurrence} node {node} is not a spec document"
    )]
    AppliedNonSpecNode {
        contribution: usize,
        occurrence: usize,
        node: usize,
    },
    #[error(
        "applied absorption contribution {contribution} occurrence {occurrence} node {node} address changed: expected {expected:?}, got {actual:?}"
    )]
    AppliedOccurrenceAddress {
        contribution: usize,
        occurrence: usize,
        node: usize,
        expected: Box<SpecAddress>,
        actual: Box<SpecAddress>,
    },
}

struct Projection {
    contribution: usize,
    emission_order: Vec<super::ir::ClosureNodeId>,
}

fn absorb_closure(input: ClosureIr) -> Result<ClosureIr, AbsorbPassError> {
    if !matches!(input.qualification, QualificationState::Applied(_)) {
        return Err(AbsorbPassError::QualificationPending);
    }
    if input.pending_sources.is_some() || input.pending_embeds.is_some() {
        return Err(AbsorbPassError::PendingEarlierPass);
    }
    let plan = match &input.absorption {
        AbsorptionState::Unplanned => return Err(AbsorbPassError::Unplanned),
        AbsorptionState::Applied(_) => return Err(AbsorbPassError::AlreadyApplied),
        AbsorptionState::Planned(plan) => plan,
    };
    validate_planned_absorption(plan, &input).map_err(AbsorbPassError::InvalidPlan)?;

    let mut projections = Vec::new();
    for (contribution, (current, disposition)) in input
        .contributions
        .iter()
        .zip(&plan.contributions)
        .enumerate()
    {
        match (current, disposition) {
            (
                ClosureContribution::Normal { .. },
                ContributionAbsorption::Normal { occurrences, .. },
            ) => projections.push(Projection {
                contribution,
                emission_order: occurrences
                    .iter()
                    .filter(|occurrence| !occurrence.absorbed)
                    .map(|occurrence| occurrence.node)
                    .collect(),
            }),
            (ClosureContribution::Simple { .. }, ContributionAbsorption::Simple { .. }) => {}
            _ => unreachable!("planned absorption validation checked contribution kinds"),
        }
    }

    let mut output = input;
    for projection in projections {
        let ClosureContribution::Normal { emission_order, .. } =
            &mut output.contributions[projection.contribution]
        else {
            unreachable!("the staged normal contribution kept its identity")
        };
        *emission_order = projection.emission_order;
    }
    let plan = match std::mem::replace(&mut output.absorption, AbsorptionState::Unplanned) {
        AbsorptionState::Planned(plan) => plan,
        _ => unreachable!("the validated planned state stayed stable during finalization"),
    };
    output.absorption = AbsorptionState::Applied(plan);
    validate_applied_absorption(&output)?;
    Ok(output)
}

/// Verify that every normal order is the exact live projection of the applied
/// mode/address-bound plan. R3.3 can lift this function into verify-each
/// unchanged without recomputing containment from transformed body text.
pub(crate) fn validate_applied_absorption(closure: &ClosureIr) -> Result<(), AbsorbPassError> {
    let actual_mode = match closure.qualification {
        QualificationState::Applied(mode) => mode,
        QualificationState::Pending(_) => return Err(AbsorbPassError::QualificationPending),
    };
    if closure.pending_sources.is_some() || closure.pending_embeds.is_some() {
        return Err(AbsorbPassError::PendingEarlierPass);
    }
    let plan = match &closure.absorption {
        AbsorptionState::Applied(plan) => plan,
        AbsorptionState::Unplanned | AbsorptionState::Planned(_) => {
            return Err(AbsorbPassError::AppliedStateRequired);
        }
    };
    if plan.mode != actual_mode {
        return Err(AbsorbPassError::AppliedMode {
            expected: plan.mode,
            actual: actual_mode,
        });
    }
    if plan.contributions.len() != closure.contributions.len() {
        return Err(AbsorbPassError::AppliedAlignment {
            contribution: None,
            expected: plan.contributions.len(),
            actual: closure.contributions.len(),
        });
    }

    for (index, (current, disposition)) in closure
        .contributions
        .iter()
        .zip(&plan.contributions)
        .enumerate()
    {
        match (current, disposition) {
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
                if seed != expected_seed {
                    return Err(AbsorbPassError::AppliedSeed {
                        contribution: index,
                        expected: expected_seed.0,
                        actual: seed.0,
                    });
                }
                let seed_node =
                    closure
                        .nodes
                        .get(seed.0)
                        .ok_or(AbsorbPassError::AppliedMissingSeedNode {
                            contribution: index,
                            node: seed.0,
                        })?;
                let DocumentAddress::Spec(actual_seed_address) = &seed_node.address else {
                    return Err(AbsorbPassError::AppliedNonSpecSeedNode {
                        contribution: index,
                        node: seed.0,
                    });
                };
                if expected_seed_address != actual_seed_address {
                    return Err(AbsorbPassError::AppliedSeedAddress {
                        contribution: index,
                        node: seed.0,
                        expected: Box::new(expected_seed_address.clone()),
                        actual: Box::new(actual_seed_address.clone()),
                    });
                }
                for (occurrence, planned) in occurrences.iter().enumerate() {
                    let current_node = closure.nodes.get(planned.node.0).ok_or(
                        AbsorbPassError::AppliedMissingNode {
                            contribution: index,
                            occurrence,
                            node: planned.node.0,
                        },
                    )?;
                    let DocumentAddress::Spec(actual_address) = &current_node.address else {
                        return Err(AbsorbPassError::AppliedNonSpecNode {
                            contribution: index,
                            occurrence,
                            node: planned.node.0,
                        });
                    };
                    if planned.address != *actual_address {
                        return Err(AbsorbPassError::AppliedOccurrenceAddress {
                            contribution: index,
                            occurrence,
                            node: planned.node.0,
                            expected: Box::new(planned.address.clone()),
                            actual: Box::new(actual_address.clone()),
                        });
                    }
                }
                let expected: Vec<_> = occurrences
                    .iter()
                    .filter(|occurrence| !occurrence.absorbed)
                    .map(|occurrence| occurrence.node)
                    .collect();
                if expected.len() != emission_order.len() {
                    return Err(AbsorbPassError::AppliedAlignment {
                        contribution: Some(index),
                        expected: expected.len(),
                        actual: emission_order.len(),
                    });
                }
                for (occurrence, (expected, actual)) in
                    expected.iter().zip(emission_order).enumerate()
                {
                    if expected != actual {
                        return Err(AbsorbPassError::AppliedOccurrence {
                            contribution: index,
                            occurrence,
                            expected: expected.0,
                            actual: actual.0,
                        });
                    }
                }
            }
            (
                ClosureContribution::Simple { meta, document },
                ContributionAbsorption::Simple {
                    meta: expected_meta,
                    address,
                },
            ) => {
                validate_meta(index, expected_meta, meta)?;
                if address != &document.address {
                    return Err(AbsorbPassError::AppliedContributionIdentity {
                        contribution: index,
                        expected: contribution_identity(expected_meta, Some(address)),
                        actual: contribution_identity(meta, Some(&document.address)),
                    });
                }
            }
            _ => {
                return Err(AbsorbPassError::AppliedKind {
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
) -> Result<(), AbsorbPassError> {
    if expected == actual {
        return Ok(());
    }
    Err(AbsorbPassError::AppliedContributionIdentity {
        contribution,
        expected: contribution_identity(expected, None),
        actual: contribution_identity(actual, None),
    })
}

fn contribution_identity(meta: &ContributionMeta, address: Option<&DocumentAddress>) -> String {
    match address {
        Some(address) => format!("{}:{}:{address:?}", meta.origin, meta.path),
        None => format!("{}:{}", meta.origin, meta.path),
    }
}

#[cfg(test)]
#[path = "absorb/tests.rs"]
mod tests;

#[cfg(test)]
#[path = "absorb/domain_tests.rs"]
mod domain_tests;
