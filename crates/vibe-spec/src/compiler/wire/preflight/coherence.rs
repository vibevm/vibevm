//! The context-tuple phase (gate 3) and the origin-relation phase (gate 4),
//! each over EVERY context and every normal/hoisted contribution in the
//! carrier, before any digest, address, arena or construction gate runs.

use crate::compiler::ir::{ContributionMeta, validate_package_relation};
use crate::compiler::wire::{G_ORIGIN_PACKAGE, G_SCALAR_IDS, IrWireError, gate};

use super::super::address::carried_spec_address;
use super::super::bounded::{display, preview};
use super::super::closure::decode_context;
use super::{closure, emitted, lane, wire};

/// Gate 3: every `ArtifactContext` is one `ArtifactContext::new` row.
pub(super) fn contexts(ir: &wire::Ir) -> Result<(), IrWireError> {
    if let Some(closure) = closure(ir) {
        decode_context(&closure.context)?;
    }
    if let Some(lane) = lane(ir) {
        decode_context(&lane.context)?;
    }
    if let Some(emitted) = emitted(ir) {
        decode_context(&emitted.provenance.context)?;
    }
    Ok(())
}

/// Gate 4: every normal/hoisted contribution's origin coordinate equals its
/// target package coordinate, and a hoisted target is an unversioned whole
/// document.
pub(super) fn origins(ir: &wire::Ir) -> Result<(), IrWireError> {
    if let Some(closure) = closure(ir) {
        for contribution in &closure.contributions {
            match contribution {
                wire::ClosureContribution::Normal(inner) => {
                    pair("normal", &inner.meta, &inner.seed_address, false)?;
                }
                wire::ClosureContribution::Hoisted(inner) => {
                    pair("hoisted", &inner.meta, &inner.target, true)?;
                }
                _ => {}
            }
        }
        let plan = match &closure.absorption {
            wire::AbsorptionState::Planned(arm) => Some(&arm.plan),
            wire::AbsorptionState::Applied(arm) => Some(&arm.plan),
            wire::AbsorptionState::Unplanned(_) => None,
        };
        if let Some(plan) = plan {
            for contribution in &plan.contributions {
                match contribution {
                    wire::ContributionAbsorption::Normal(inner) => {
                        pair("normal", &inner.meta, &inner.seed_address, false)?;
                    }
                    wire::ContributionAbsorption::Hoisted(inner) => {
                        pair("hoisted", &inner.meta, &inner.target, true)?;
                    }
                    _ => {}
                }
            }
        }
        if let wire::LinkState::Linked(arm) = &closure.link {
            for witness in &arm.result.contributions {
                match witness {
                    wire::LinkContributionWitness::Normal(inner) => {
                        pair("normal", &inner.meta, &inner.seed_address, false)?;
                    }
                    wire::LinkContributionWitness::Hoisted(inner) => {
                        pair("hoisted", &inner.meta, &inner.target, true)?;
                    }
                    _ => {}
                }
            }
        }
    }
    if let Some(lane) = lane(ir) {
        for contribution in &lane.contributions {
            match contribution {
                wire::LaneContribution::Normal(inner) => {
                    pair("normal", &inner.meta, &inner.seed_address, false)?;
                }
                wire::LaneContribution::Hoisted(inner) => {
                    pair("hoisted", &inner.meta, &inner.target, true)?;
                }
                _ => {}
            }
        }
    }
    if let Some(emitted) = emitted(ir) {
        for witness in &emitted.provenance.contributions {
            match witness {
                wire::EmissionContributionWitness::Normal(inner) => {
                    pair("normal", &inner.meta, &inner.seed_address, false)?;
                }
                wire::EmissionContributionWitness::Hoisted(inner) => {
                    pair("hoisted", &inner.meta, &inner.target, true)?;
                }
                _ => {}
            }
        }
    }
    Ok(())
}

fn pair(
    kind: &'static str,
    meta: &wire::ContributionMeta,
    target: &wire::SpecAddress,
    whole_unversioned: bool,
) -> Result<(), IrWireError> {
    let origin = meta.origin.clone();
    let target = carried_spec_address(target);
    let meta = ContributionMeta::new(origin, meta.path.clone()).map_err(|source| {
        gate(
            G_SCALAR_IDS,
            format!("contribution meta: {}", display(source)),
        )
    })?;
    validate_package_relation(kind, &meta.origin, &target, whole_unversioned).map_err(|source| {
        gate(
            G_ORIGIN_PACKAGE,
            format!(
                "{kind} contribution ({}) contradicts its target: {}",
                preview(&meta.origin),
                display(source)
            ),
        )
    })
}
