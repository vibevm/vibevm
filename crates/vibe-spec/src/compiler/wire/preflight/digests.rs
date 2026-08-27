//! The digest/base64 phase (gate 5): every digest spelling in the carrier is
//! 64 lowercase hex characters, and the emitted `bytes_b64` is a canonical
//! padded STANDARD spelling — checked BEFORE any allocation from the decoded
//! length and before any arena/forest/span gate.

use crate::compiler::wire::IrWireError;

use super::super::emitted::{check_canonical_base64, parse_digest};
use super::{closure, emitted, lane, wire};

pub(super) fn run(ir: &wire::Ir) -> Result<(), IrWireError> {
    if let Some(closure) = closure(ir)
        && let wire::LinkState::Linked(arm) = &closure.link
    {
        parse_digest("link input digest", &arm.result.input_digest)?;
    }
    if let Some(lane) = lane(ir) {
        parse_digest("lane source_link_digest", &lane.source_link_digest)?;
    }
    if let Some(emitted) = emitted(ir) {
        let provenance = &emitted.provenance;
        parse_digest("source lane digest", &provenance.source_lane_digest)?;
        parse_digest("bytes digest", &provenance.bytes_digest)?;
        for witness in &provenance.contributions {
            match witness {
                wire::EmissionContributionWitness::Normal(inner) => {
                    parse_digest("chunk digest", &inner.chunk_digest)?;
                }
                wire::EmissionContributionWitness::Simple(inner) => {
                    parse_digest("chunk digest", &inner.chunk_digest)?;
                }
                _ => {}
            }
        }
        check_canonical_base64(&emitted.bytes_b64)?;
    }
    Ok(())
}
