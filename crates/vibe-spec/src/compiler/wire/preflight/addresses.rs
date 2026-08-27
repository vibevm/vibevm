//! The address/fence phase (gate 6): every `spec://` address in the carrier
//! re-parses to the fields carried beside it, and every fence snapshot
//! carries a delimiter of exactly one fence character.
//!
//! The address set is the shared [`inventory`] walk — the SAME one gate 2's
//! scalar phase rides — so the two phases can never visit different sites.

use crate::compiler::wire::IrWireError;

use super::super::address::decode_spec_address;
use super::super::lane::decode_fence;
use super::inventory::{AddressVisitor, addresses};
use super::{closure, lane, wire};

pub(super) fn run(ir: &wire::Ir) -> Result<(), IrWireError> {
    addresses(ir, &mut Reparse)?;
    fences(ir)
}

/// Gate 6's address clause: `raw` re-parses to the carried fields.
struct Reparse;

impl AddressVisitor for Reparse {
    fn spec(&mut self, value: &wire::SpecAddress) -> Result<(), IrWireError> {
        decode_spec_address(value).map(|_| ())
    }
}

/// Gate 6's fence clause. Fences are not addresses, so they ride their own
/// walk — every occurrence and lane node boundary the carrier holds.
fn fences(ir: &wire::Ir) -> Result<(), IrWireError> {
    if let Some(value) = closure(ir)
        && let wire::LinkState::Linked(arm) = &value.link
    {
        for occurrence in &arm.result.occurrences {
            let (before, after) = match occurrence {
                wire::LinkOccurrence::Normal(inner) => (&inner.fence_before, &inner.fence_after),
                wire::LinkOccurrence::Simple(inner) => (&inner.fence_before, &inner.fence_after),
            };
            decode_fence(before)?;
            decode_fence(after)?;
        }
    }
    if let Some(value) = lane(ir) {
        for contribution in &value.contributions {
            let chunks = match contribution {
                wire::LaneContribution::Normal(inner) => &inner.chunks,
                wire::LaneContribution::Simple(inner) => &inner.chunks,
                wire::LaneContribution::Elided(_) | wire::LaneContribution::Hoisted(_) => continue,
            };
            for chunk in chunks {
                if let wire::LaneChunk::Node(inner) = chunk {
                    let (before, after) = match &inner.node {
                        wire::LaneNode::Normal(node) => (&node.fence_before, &node.fence_after),
                        wire::LaneNode::Simple(node) => (&node.fence_before, &node.fence_after),
                    };
                    decode_fence(before)?;
                    decode_fence(after)?;
                }
            }
        }
    }
    Ok(())
}
