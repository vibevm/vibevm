//! The closure's typestate cell: READ-ONCE absorption, the link result, and
//! the pending source/embed snapshots — with the witness-order gates.

use std::collections::{BTreeMap, BTreeSet};

use super::super::super::embed_snapshot::EmbedResolutionSnapshot;
use super::super::super::ir::{
    AbsorptionOccurrence, AbsorptionPlan, AbsorptionState, ClosureNodeId, ContributionAbsorption,
    LinkContributionWitness, LinkInputDigest, LinkMarkerKey, LinkOccurrence, LinkResult, LinkState,
};
use super::super::super::source_snapshot::{
    DocumentObservation, ExpansionObservation, SourceResolutionSnapshot,
};
use super::super::address::{
    decode_document_address, decode_spec_address, encode_document_address, encode_spec_address,
};
use super::super::emitted::{digest_hex, parse_digest};
use super::super::lane as lane_fence;
use super::super::tree::{decode_document_ir, encode_document_ir};
use super::super::{IrWireError, decode_mode, encode_mode, narrow, widen, wire};
use super::{apply_package_relation, check_node_id, decode_meta, encode_meta};

pub(super) fn decode_absorption_state(
    value: &wire::AbsorptionState,
    node_count: usize,
) -> Result<AbsorptionState, IrWireError> {
    match value {
        wire::AbsorptionState::Unplanned(_) => Ok(AbsorptionState::Unplanned),
        wire::AbsorptionState::Planned(arm) => Ok(AbsorptionState::Planned(decode_plan(
            &arm.plan, node_count,
        )?)),
        wire::AbsorptionState::Applied(arm) => Ok(AbsorptionState::Applied(decode_plan(
            &arm.plan, node_count,
        )?)),
    }
}

fn decode_plan(
    value: &wire::AbsorptionPlan,
    node_count: usize,
) -> Result<AbsorptionPlan, IrWireError> {
    let mut plan = Vec::with_capacity(value.contributions.len());
    for entry in &value.contributions {
        plan.push(decode_contribution_absorption(entry, node_count)?);
    }
    Ok(AbsorptionPlan {
        mode: decode_mode(&value.mode),
        contributions: plan,
    })
}

fn decode_contribution_absorption(
    value: &wire::ContributionAbsorption,
    node_count: usize,
) -> Result<ContributionAbsorption, IrWireError> {
    match value {
        wire::ContributionAbsorption::Normal(normal) => {
            let meta = decode_meta(&normal.meta)?;
            let seed_address = decode_spec_address(&normal.seed_address)?;
            apply_package_relation("normal", &meta, &seed_address, false)?;
            let seed = narrow("plan seed", normal.seed)?;
            check_node_id("plan seed", seed, node_count)?;
            let mut occurrences = Vec::with_capacity(normal.occurrences.len());
            for occurrence in &normal.occurrences {
                let node = narrow("absorption occurrence node", occurrence.node)?;
                check_node_id("absorption occurrence node", node, node_count)?;
                occurrences.push(AbsorptionOccurrence {
                    node: ClosureNodeId(node),
                    requested_address: decode_spec_address(&occurrence.requested_address)?,
                    absorbed: occurrence.absorbed,
                });
            }
            Ok(ContributionAbsorption::Normal {
                meta,
                seed: ClosureNodeId(seed),
                seed_address,
                occurrences,
            })
        }
        wire::ContributionAbsorption::Simple(simple) => Ok(ContributionAbsorption::Simple {
            meta: decode_meta(&simple.meta)?,
            address: decode_document_address(&simple.address)?,
        }),
        wire::ContributionAbsorption::Elided(elided) => Ok(ContributionAbsorption::Elided {
            meta: decode_meta(&elided.meta)?,
        }),
        wire::ContributionAbsorption::Hoisted(hoisted) => {
            let meta = decode_meta(&hoisted.meta)?;
            let target = decode_spec_address(&hoisted.target)?;
            apply_package_relation("hoisted", &meta, &target, true)?;
            Ok(ContributionAbsorption::Hoisted { meta, target })
        }
    }
}

pub(super) fn decode_link_state(
    value: &wire::LinkState,
    node_count: usize,
) -> Result<LinkState, IrWireError> {
    match value {
        wire::LinkState::Unlinked(_) => Ok(LinkState::Unlinked),
        wire::LinkState::Linked(arm) => Ok(LinkState::Linked(decode_link_result(
            &arm.result,
            node_count,
        )?)),
    }
}

fn decode_link_result(
    value: &wire::LinkResult,
    node_count: usize,
) -> Result<LinkResult, IrWireError> {
    let input_digest = parse_digest("link input digest", &value.input_digest)?;
    let mut witnesses = Vec::with_capacity(value.contributions.len());
    for witness in &value.contributions {
        witnesses.push(decode_link_witness(witness, node_count)?);
    }
    let mut occurrences = Vec::with_capacity(value.occurrences.len());
    for occurrence in &value.occurrences {
        occurrences.push(decode_link_occurrence(occurrence, node_count)?);
    }
    Ok(LinkResult {
        mode: decode_mode(&value.mode),
        input_digest: LinkInputDigest(input_digest),
        contributions: witnesses,
        occurrences,
    })
}

fn decode_link_witness(
    value: &wire::LinkContributionWitness,
    node_count: usize,
) -> Result<LinkContributionWitness, IrWireError> {
    match value {
        wire::LinkContributionWitness::Normal(normal) => {
            let meta = decode_meta(&normal.meta)?;
            let seed_address = decode_spec_address(&normal.seed_address)?;
            apply_package_relation("normal", &meta, &seed_address, false)?;
            let seed = narrow("link witness seed", normal.seed)?;
            check_node_id("link witness seed", seed, node_count)?;
            Ok(LinkContributionWitness::Normal {
                meta,
                seed: ClosureNodeId(seed),
                seed_address,
                occurrence_count: narrow("occurrence count", normal.occurrence_count)?,
            })
        }
        wire::LinkContributionWitness::Simple(simple) => Ok(LinkContributionWitness::Simple {
            meta: decode_meta(&simple.meta)?,
            address: decode_document_address(&simple.address)?,
        }),
        wire::LinkContributionWitness::Elided(elided) => Ok(LinkContributionWitness::Elided {
            meta: decode_meta(&elided.meta)?,
        }),
        wire::LinkContributionWitness::Hoisted(hoisted) => {
            let meta = decode_meta(&hoisted.meta)?;
            let target = decode_spec_address(&hoisted.target)?;
            apply_package_relation("hoisted", &meta, &target, true)?;
            Ok(LinkContributionWitness::Hoisted { meta, target })
        }
    }
}

fn decode_link_occurrence(
    value: &wire::LinkOccurrence,
    node_count: usize,
) -> Result<LinkOccurrence, IrWireError> {
    match value {
        wire::LinkOccurrence::Normal(normal) => {
            let node = narrow("link occurrence node", normal.node)?;
            check_node_id("link occurrence node", node, node_count)?;
            Ok(LinkOccurrence::Normal {
                contribution: narrow("occurrence contribution", normal.contribution)?,
                occurrence: narrow("occurrence index", normal.occurrence)?,
                node: ClosureNodeId(node),
                address: decode_spec_address(&normal.address)?,
                marker: LinkMarkerKey::new(normal.marker.clone()),
                fence_before: lane_fence::decode_fence(&normal.fence_before)?,
                fence_after: lane_fence::decode_fence(&normal.fence_after)?,
                body: normal.body.clone(),
                trailing_newline_required: normal.trailing_newline_required,
            })
        }
        wire::LinkOccurrence::Simple(simple) => Ok(LinkOccurrence::Simple {
            contribution: narrow("occurrence contribution", simple.contribution)?,
            occurrence: narrow("occurrence index", simple.occurrence)?,
            address: decode_document_address(&simple.address)?,
            fence_before: lane_fence::decode_fence(&simple.fence_before)?,
            fence_after: lane_fence::decode_fence(&simple.fence_after)?,
            body: simple.body.clone(),
            trailing_newline_required: simple.trailing_newline_required,
        }),
    }
}

pub(super) fn decode_source_snapshot(
    value: &wire::SourceResolutionSnapshot,
) -> Result<SourceResolutionSnapshot, IrWireError> {
    let mut documents = BTreeMap::new();
    for (key, observation) in &value.documents {
        documents.insert(key.clone(), decode_document_observation(observation)?);
    }
    let mut expansions = BTreeMap::new();
    for (key, expansion) in &value.expansions {
        expansions.insert(key.clone(), decode_expansion_observation(expansion)?);
    }
    Ok(SourceResolutionSnapshot {
        discovery_order: value.discovery_order.clone(),
        documents,
        expansions,
        explicit_use_keys: decode_set("pending_sources", &value.explicit_use_keys)?,
    })
}

pub(super) fn decode_embed_snapshot(
    value: &wire::EmbedResolutionSnapshot,
) -> Result<EmbedResolutionSnapshot, IrWireError> {
    let mut documents = BTreeMap::new();
    for (key, observation) in &value.documents {
        documents.insert(key.clone(), decode_document_observation(observation)?);
    }
    Ok(EmbedResolutionSnapshot {
        discovery_order: value.discovery_order.clone(),
        documents,
        explicit_use_keys: decode_set("pending_embeds", &value.explicit_use_keys)?,
    })
}

/// SET PROJECTION rides the ordered preflight's own validator (gate 11);
/// construction reuses it as a belt and then collects, so there is one
/// grammar rather than two that can drift.
fn decode_set(site: &'static str, values: &[String]) -> Result<BTreeSet<String>, IrWireError> {
    super::super::preflight::check_set(site, values)?;
    Ok(values.iter().cloned().collect())
}

fn decode_document_observation(
    value: &wire::DocumentObservation,
) -> Result<DocumentObservation, IrWireError> {
    match value {
        wire::DocumentObservation::Resolved(resolved) => Ok(DocumentObservation::Resolved(
            decode_document_ir(&resolved.document)?,
        )),
        wire::DocumentObservation::Failed(failed) => Ok(DocumentObservation::Failed {
            requested: decode_spec_address(&failed.requested)?,
            reason: failed.reason.clone(),
        }),
    }
}

fn decode_expansion_observation(
    value: &wire::ExpansionObservation,
) -> Result<ExpansionObservation, IrWireError> {
    match value {
        wire::ExpansionObservation::Resolved(resolved) => {
            let mut targets = Vec::with_capacity(resolved.targets.len());
            for target in &resolved.targets {
                targets.push(decode_spec_address(target)?);
            }
            Ok(ExpansionObservation::Resolved {
                requested: decode_spec_address(&resolved.requested)?,
                targets,
            })
        }
        wire::ExpansionObservation::Failed(failed) => Ok(ExpansionObservation::Failed {
            requested: decode_spec_address(&failed.requested)?,
            reason: failed.reason.clone(),
        }),
    }
}

pub(super) fn encode_absorption_state(
    value: &AbsorptionState,
) -> Result<wire::AbsorptionState, IrWireError> {
    Ok(match value {
        AbsorptionState::Unplanned => {
            wire::AbsorptionState::Unplanned(Box::new(wire::AbsorptionStateUnplanned {}))
        }
        AbsorptionState::Planned(plan) => {
            wire::AbsorptionState::Planned(Box::new(wire::AbsorptionStatePlanned {
                plan: encode_plan(plan)?,
            }))
        }
        AbsorptionState::Applied(plan) => {
            wire::AbsorptionState::Applied(Box::new(wire::AbsorptionStateApplied {
                plan: encode_plan(plan)?,
            }))
        }
    })
}

fn encode_plan(value: &AbsorptionPlan) -> Result<wire::AbsorptionPlan, IrWireError> {
    let mut contributions = Vec::with_capacity(value.contributions.len());
    for entry in &value.contributions {
        contributions.push(match entry {
            ContributionAbsorption::Normal {
                meta,
                seed,
                seed_address,
                occurrences,
            } => {
                let mut wire_occurrences = Vec::with_capacity(occurrences.len());
                for occurrence in occurrences {
                    wire_occurrences.push(wire::AbsorptionOccurrence {
                        node: widen("absorption occurrence node", occurrence.node.0)?,
                        requested_address: encode_spec_address(&occurrence.requested_address),
                        absorbed: occurrence.absorbed,
                    });
                }
                wire::ContributionAbsorption::Normal(Box::new(wire::ContributionAbsorptionNormal {
                    meta: encode_meta(meta),
                    seed: widen("plan seed", seed.0)?,
                    seed_address: encode_spec_address(seed_address),
                    occurrences: wire_occurrences,
                }))
            }
            ContributionAbsorption::Simple { meta, address } => {
                wire::ContributionAbsorption::Simple(Box::new(wire::ContributionAbsorptionSimple {
                    meta: encode_meta(meta),
                    address: encode_document_address(address),
                }))
            }
            ContributionAbsorption::Elided { meta } => {
                wire::ContributionAbsorption::Elided(Box::new(wire::ContributionAbsorptionElided {
                    meta: encode_meta(meta),
                }))
            }
            ContributionAbsorption::Hoisted { meta, target } => {
                wire::ContributionAbsorption::Hoisted(Box::new(
                    wire::ContributionAbsorptionHoisted {
                        meta: encode_meta(meta),
                        target: encode_spec_address(target),
                    },
                ))
            }
        });
    }
    Ok(wire::AbsorptionPlan {
        mode: encode_mode(value.mode),
        contributions,
    })
}

pub(super) fn encode_link_state(value: &LinkState) -> Result<wire::LinkState, IrWireError> {
    match value {
        LinkState::Unlinked => Ok(wire::LinkState::Unlinked(Box::new(
            wire::LinkStateUnlinked {},
        ))),
        LinkState::Linked(result) => {
            let mut contributions = Vec::with_capacity(result.contributions.len());
            for witness in &result.contributions {
                contributions.push(match witness {
                    LinkContributionWitness::Normal {
                        meta,
                        seed,
                        seed_address,
                        occurrence_count,
                    } => wire::LinkContributionWitness::Normal(Box::new(
                        wire::LinkContributionWitnessNormal {
                            meta: encode_meta(meta),
                            seed: widen("link witness seed", seed.0)?,
                            seed_address: encode_spec_address(seed_address),
                            occurrence_count: widen("occurrence count", *occurrence_count)?,
                        },
                    )),
                    LinkContributionWitness::Simple { meta, address } => {
                        wire::LinkContributionWitness::Simple(Box::new(
                            wire::LinkContributionWitnessSimple {
                                meta: encode_meta(meta),
                                address: encode_document_address(address),
                            },
                        ))
                    }
                    LinkContributionWitness::Elided { meta } => {
                        wire::LinkContributionWitness::Elided(Box::new(
                            wire::LinkContributionWitnessElided {
                                meta: encode_meta(meta),
                            },
                        ))
                    }
                    LinkContributionWitness::Hoisted { meta, target } => {
                        wire::LinkContributionWitness::Hoisted(Box::new(
                            wire::LinkContributionWitnessHoisted {
                                meta: encode_meta(meta),
                                target: encode_spec_address(target),
                            },
                        ))
                    }
                });
            }
            let mut occurrences = Vec::with_capacity(result.occurrences.len());
            for occurrence in &result.occurrences {
                occurrences.push(match occurrence {
                    LinkOccurrence::Normal {
                        contribution,
                        occurrence,
                        node,
                        address,
                        marker,
                        fence_before,
                        fence_after,
                        body,
                        trailing_newline_required,
                    } => wire::LinkOccurrence::Normal(Box::new(wire::LinkOccurrenceNormal {
                        contribution: widen("occurrence contribution", *contribution)?,
                        occurrence: widen("occurrence index", *occurrence)?,
                        node: widen("link occurrence node", node.0)?,
                        address: encode_spec_address(address),
                        marker: marker.as_str().to_string(),
                        fence_before: lane_fence::encode_fence(*fence_before)?,
                        fence_after: lane_fence::encode_fence(*fence_after)?,
                        body: body.clone(),
                        trailing_newline_required: *trailing_newline_required,
                    })),
                    LinkOccurrence::Simple {
                        contribution,
                        occurrence,
                        address,
                        fence_before,
                        fence_after,
                        body,
                        trailing_newline_required,
                    } => wire::LinkOccurrence::Simple(Box::new(wire::LinkOccurrenceSimple {
                        contribution: widen("occurrence contribution", *contribution)?,
                        occurrence: widen("occurrence index", *occurrence)?,
                        address: encode_document_address(address),
                        fence_before: lane_fence::encode_fence(*fence_before)?,
                        fence_after: lane_fence::encode_fence(*fence_after)?,
                        body: body.clone(),
                        trailing_newline_required: *trailing_newline_required,
                    })),
                });
            }
            Ok(wire::LinkState::Linked(Box::new(wire::LinkStateLinked {
                result: wire::LinkResult {
                    mode: encode_mode(result.mode),
                    input_digest: digest_hex(&result.input_digest.0),
                    contributions,
                    occurrences,
                },
            })))
        }
    }
}

pub(super) fn encode_source_snapshot(
    value: &SourceResolutionSnapshot,
) -> Result<wire::SourceResolutionSnapshot, IrWireError> {
    let mut documents = BTreeMap::new();
    for (key, observation) in &value.documents {
        documents.insert(key.clone(), encode_document_observation(observation)?);
    }
    let mut expansions = BTreeMap::new();
    for (key, expansion) in &value.expansions {
        expansions.insert(key.clone(), encode_expansion_observation(expansion)?);
    }
    Ok(wire::SourceResolutionSnapshot {
        discovery_order: value.discovery_order.clone(),
        documents,
        expansions,
        explicit_use_keys: value.explicit_use_keys.iter().cloned().collect(),
    })
}

pub(super) fn encode_embed_snapshot(
    value: &EmbedResolutionSnapshot,
) -> Result<wire::EmbedResolutionSnapshot, IrWireError> {
    let mut documents = BTreeMap::new();
    for (key, observation) in &value.documents {
        documents.insert(key.clone(), encode_document_observation(observation)?);
    }
    Ok(wire::EmbedResolutionSnapshot {
        discovery_order: value.discovery_order.clone(),
        documents,
        explicit_use_keys: value.explicit_use_keys.iter().cloned().collect(),
    })
}

fn encode_document_observation(
    value: &DocumentObservation,
) -> Result<wire::DocumentObservation, IrWireError> {
    match value {
        DocumentObservation::Resolved(document) => Ok(wire::DocumentObservation::Resolved(
            Box::new(wire::DocumentObservationResolved {
                document: encode_document_ir(document)?,
            }),
        )),
        DocumentObservation::Failed { requested, reason } => Ok(wire::DocumentObservation::Failed(
            Box::new(wire::DocumentObservationFailed {
                requested: encode_spec_address(requested),
                reason: reason.clone(),
            }),
        )),
    }
}

fn encode_expansion_observation(
    value: &ExpansionObservation,
) -> Result<wire::ExpansionObservation, IrWireError> {
    match value {
        ExpansionObservation::Resolved { requested, targets } => Ok(
            wire::ExpansionObservation::Resolved(Box::new(wire::ExpansionObservationResolved {
                requested: encode_spec_address(requested),
                targets: targets.iter().map(encode_spec_address).collect::<Vec<_>>(),
            })),
        ),
        ExpansionObservation::Failed { requested, reason } => Ok(
            wire::ExpansionObservation::Failed(Box::new(wire::ExpansionObservationFailed {
                requested: encode_spec_address(requested),
                reason: reason.clone(),
            })),
        ),
    }
}
