//! Gates 7–10 as WHOLE-CARRIER phases: every arena index in the carrier is
//! in range before any forest is walked, every forest is proved before any
//! span is judged, and so on. Running tree A's gates 7–10 before tree B's
//! gate 7 would be the schema's order per tree, not per carrier — the point
//! of the phase schedule is that the FIRST failure a hostile carrier gets is
//! its lowest-numbered fault, wherever in the carrier it lives.

use crate::compiler::wire::tree::{
    check_anchor_coherence, check_arena_bounds, check_forest, check_span_bounds,
};
use crate::compiler::wire::{G_ARENA_BOUNDS, IrWireError, gate, narrow, wire};

use super::inventory::trees;
use super::{closure, lane, plan_of};

/// Gate 7: every index in the carrier — tree arenas, the closure graph, and
/// the lane's view of its source closure — before anything follows one.
pub(super) fn bounds(ir: &wire::Ir) -> Result<(), IrWireError> {
    for tree in trees(ir) {
        check_arena_bounds(tree)?;
    }
    if let Some(value) = closure(ir) {
        closure_bounds(value)?;
    }
    if let Some(value) = lane(ir) {
        lane_bounds(value)?;
    }
    if let Some(value) = emitted_seeds(ir) {
        value?;
    }
    Ok(())
}

/// Gate 8: every forest, iteratively and with a step bound.
pub(super) fn forests(ir: &wire::Ir) -> Result<(), IrWireError> {
    for tree in trees(ir) {
        check_forest(tree)?;
    }
    Ok(())
}

/// Gate 9: every span, before any slicing.
pub(super) fn spans(ir: &wire::Ir) -> Result<(), IrWireError> {
    for tree in trees(ir) {
        check_span_bounds(tree)?;
    }
    Ok(())
}

/// Gate 10: every anchor index and duplicate record.
pub(super) fn anchors(ir: &wire::Ir) -> Result<(), IrWireError> {
    for tree in trees(ir) {
        check_anchor_coherence(tree)?;
    }
    Ok(())
}

fn in_arena(site: &'static str, index: u32, len: usize) -> Result<(), IrWireError> {
    let index = narrow(site, index)?;
    if index >= len {
        return Err(gate(
            G_ARENA_BOUNDS,
            format!("{site} names node {index} outside the graph of {len} nodes"),
        ));
    }
    Ok(())
}

fn closure_bounds(value: &wire::ClosureIr) -> Result<(), IrWireError> {
    let len = value.nodes.len();
    for edge in &value.edges {
        in_arena("edge source", edge.from, len)?;
        in_arena("edge target", edge.to, len)?;
    }
    for contribution in &value.contributions {
        if let wire::ClosureContribution::Normal(inner) = contribution {
            in_arena("normal seed", inner.seed, len)?;
            for occurrence in &inner.emission_order {
                in_arena("emission occurrence node", occurrence.node, len)?;
            }
        }
    }
    if let Some(plan) = plan_of(value) {
        for contribution in &plan.contributions {
            if let wire::ContributionAbsorption::Normal(inner) = contribution {
                in_arena("plan seed", inner.seed, len)?;
                for occurrence in &inner.occurrences {
                    in_arena("absorption occurrence node", occurrence.node, len)?;
                }
            }
        }
    }
    if let wire::LinkState::Linked(arm) = &value.link {
        for witness in &arm.result.contributions {
            if let wire::LinkContributionWitness::Normal(inner) = witness {
                in_arena("link witness seed", inner.seed, len)?;
                narrow("occurrence count", inner.occurrence_count)?;
            }
        }
        for occurrence in &arm.result.occurrences {
            match occurrence {
                wire::LinkOccurrence::Normal(inner) => {
                    in_arena("link occurrence node", inner.node, len)?;
                    narrow("occurrence contribution", inner.contribution)?;
                    narrow("occurrence index", inner.occurrence)?;
                }
                wire::LinkOccurrence::Simple(inner) => {
                    narrow("occurrence contribution", inner.contribution)?;
                    narrow("occurrence index", inner.occurrence)?;
                }
            }
        }
    }
    Ok(())
}

fn lane_bounds(value: &wire::LaneIr) -> Result<(), IrWireError> {
    let len = narrow("lane source node count", value.source_node_count)?;
    for contribution in &value.contributions {
        let chunks = match contribution {
            wire::LaneContribution::Normal(inner) => {
                in_arena("lane seed", inner.seed, len)?;
                &inner.chunks
            }
            wire::LaneContribution::Simple(inner) => &inner.chunks,
            wire::LaneContribution::Elided(_) | wire::LaneContribution::Hoisted(_) => continue,
        };
        for chunk in chunks {
            if let wire::LaneChunk::Node(inner) = chunk
                && let wire::LaneNode::Normal(node) = &inner.node
            {
                in_arena("lane node", node.node, len)?;
            }
        }
    }
    Ok(())
}

/// The emitted carrier holds no arena, so its witness seeds owe only the
/// checked u32→usize narrowing every index owes.
fn emitted_seeds(ir: &wire::Ir) -> Option<Result<(), IrWireError>> {
    let emitted = super::emitted(ir)?;
    for witness in &emitted.provenance.contributions {
        if let wire::EmissionContributionWitness::Normal(inner) = witness
            && let Err(error) = narrow("witness seed", inner.seed)
        {
            return Some(Err(error));
        }
    }
    Some(Ok(()))
}
