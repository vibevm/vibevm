//! Typed generated occurrence conversion; rendered marker text is never input.

use super::address::{decode_spec_address, encode_spec_address};
use super::emitted::{digest_hex, parse_digest};
use super::{G_SPAN_BOUNDS, IrWireError, gate, narrow, widen, wire};
use crate::embed::occurrences::{EmbedOccurrence, EmbedRegion, EmbedRegionKind, EmbedTape};

pub(super) fn decode(
    value: &Option<wire::EmbedTape>,
    body: &str,
) -> Result<Option<EmbedTape>, IrWireError> {
    value
        .as_ref()
        .map(|value| decode_required(value, body))
        .transpose()
}

pub(super) fn decode_required(
    value: &wire::EmbedTape,
    body: &str,
) -> Result<EmbedTape, IrWireError> {
    let mut occurrences = Vec::with_capacity(value.occurrences.len());
    for row in &value.occurrences {
        let (kind, start, end) = match &row.owner {
            wire::EmbedRegion::Preamble(v) => (EmbedRegionKind::Preamble, v.start, v.end),
            wire::EmbedRegion::Unanchored(v) => (EmbedRegionKind::Unanchored, v.start, v.end),
            wire::EmbedRegion::Unit(v) => (EmbedRegionKind::Unit(v.anchor.clone()), v.start, v.end),
        };
        occurrences.push(EmbedOccurrence {
            path: row
                .path
                .iter()
                .map(|n| narrow("embed path", *n))
                .collect::<Result<_, _>>()?,
            target: decode_spec_address(&row.target)?,
            source: row.source.clone(),
            directive_line: narrow("embed directive line", row.directive_line)?,
            start: narrow("embed start", row.start)?,
            end: narrow("embed end", row.end)?,
            intrinsic_empty: row.intrinsic_empty,
            suppressed: row.suppressed,
            owner: EmbedRegion {
                kind,
                start: narrow("embed owner start", start)?,
                end: narrow("embed owner end", end)?,
            },
        });
    }
    let tape = EmbedTape {
        body_digest: parse_digest("embed body", &value.body_digest)?,
        occurrences,
    };
    tape.validate(body)
        .map_err(|error| gate(G_SPAN_BOUNDS, error.to_string()))?;
    Ok(tape)
}

pub(super) fn encode(value: &Option<EmbedTape>) -> Result<Option<wire::EmbedTape>, IrWireError> {
    value.as_ref().map(encode_required).transpose()
}

pub(super) fn encode_required(value: &EmbedTape) -> Result<wire::EmbedTape, IrWireError> {
    let mut occurrences = Vec::with_capacity(value.occurrences.len());
    for row in &value.occurrences {
        let start = widen("embed owner start", row.owner.start)?;
        let end = widen("embed owner end", row.owner.end)?;
        let owner = match &row.owner.kind {
            EmbedRegionKind::Preamble => {
                wire::EmbedRegion::Preamble(Box::new(wire::EmbedRegionPreamble { start, end }))
            }
            EmbedRegionKind::Unanchored => {
                wire::EmbedRegion::Unanchored(Box::new(wire::EmbedRegionUnanchored { start, end }))
            }
            EmbedRegionKind::Unit(anchor) => {
                wire::EmbedRegion::Unit(Box::new(wire::EmbedRegionUnit {
                    start,
                    end,
                    anchor: anchor.clone(),
                }))
            }
        };
        occurrences.push(wire::EmbedOccurrence {
            path: row
                .path
                .iter()
                .map(|n| widen("embed path", *n))
                .collect::<Result<_, _>>()?,
            target: encode_spec_address(&row.target),
            source: row.source.clone(),
            directive_line: widen("embed directive line", row.directive_line)?,
            start: widen("embed start", row.start)?,
            end: widen("embed end", row.end)?,
            intrinsic_empty: row.intrinsic_empty,
            suppressed: row.suppressed,
            owner,
        });
    }
    Ok(wire::EmbedTape {
        body_digest: digest_hex(&value.body_digest),
        occurrences,
    })
}
