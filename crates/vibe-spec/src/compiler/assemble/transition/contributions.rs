//! Independent occurrence and chunk consumption for the relational proof.

use super::super::super::ir::{
    ClosureDocument, ClosureIr, ClosureOccurrence, DocumentAddress, LaneChunk, LaneNode,
    LinkMarkerKey, LinkOccurrence, LinkResult,
};
use super::LaneTransitionError;

pub(super) fn validate_normal(
    closure: &ClosureIr,
    link: &LinkResult,
    contribution: usize,
    emission_order: &[ClosureOccurrence],
    chunks: &[LaneChunk],
    occurrence_index: &mut usize,
) -> Result<(), LaneTransitionError> {
    let mut chunk_index = 0;
    for (occurrence, expected) in emission_order.iter().enumerate() {
        let flat = *occurrence_index;
        let linked = link
            .occurrences
            .get(flat)
            .ok_or(LaneTransitionError::MissingOccurrence {
                contribution,
                index: flat,
            })?;
        let LinkOccurrence::Normal {
            contribution: link_contribution,
            occurrence: link_occurrence,
            node,
            address,
            marker,
            fence_before,
            fence_after,
            body,
            trailing_newline_required,
        } = linked
        else {
            return Err(unexpected_occurrence(flat, "normal", linked));
        };
        occurrence_field(*link_contribution == contribution, flat, "contribution")?;
        occurrence_field(*link_occurrence == occurrence, flat, "occurrence")?;
        occurrence_field(*node == expected.node, flat, "node")?;
        occurrence_field(
            *address == expected.requested_address,
            flat,
            "exact request",
        )?;
        occurrence_field(
            *marker == LinkMarkerKey::from_address(&expected.requested_address),
            flat,
            "marker key",
        )?;
        occurrence_field(
            *trailing_newline_required != body.ends_with('\n'),
            flat,
            "newline",
        )?;
        let document = closure
            .nodes
            .get(node.0)
            .ok_or(LaneTransitionError::MissingNode {
                index: flat,
                node: node.0,
            })?;
        let DocumentAddress::Spec(document_address) = &document.address else {
            return Err(LaneTransitionError::NonSpecNode {
                index: flat,
                node: node.0,
            });
        };
        occurrence_field(
            document_address.without_pin() == address.without_pin(),
            flat,
            "document address",
        )?;

        let open_at = chunk_index;
        let open = take_chunk(chunks, contribution, &mut chunk_index, "normal-open")?;
        let LaneChunk::NormalOpen {
            contribution: lane_contribution,
            occurrence: lane_occurrence,
            marker: lane_marker,
        } = open
        else {
            return Err(unexpected_chunk(contribution, open_at, "normal-open", open));
        };
        chunk_field(
            *lane_contribution == contribution,
            contribution,
            open_at,
            "contribution",
        )?;
        chunk_field(
            *lane_occurrence == occurrence,
            contribution,
            open_at,
            "occurrence",
        )?;
        chunk_field(lane_marker == marker, contribution, open_at, "marker")?;

        let node_at = chunk_index;
        let lane_chunk = take_chunk(chunks, contribution, &mut chunk_index, "normal node")?;
        let LaneChunk::Node(lane_node) = lane_chunk else {
            return Err(unexpected_chunk(
                contribution,
                node_at,
                "normal node",
                lane_chunk,
            ));
        };
        let LaneNode::Normal {
            contribution: lane_contribution,
            occurrence: lane_occurrence,
            node: lane_node_id,
            requested_address,
            origin,
            marker: lane_marker,
            fence_before: lane_before,
            fence_after: lane_after,
            body: lane_body,
        } = lane_node.as_ref()
        else {
            return Err(unexpected_chunk(
                contribution,
                node_at,
                "normal node",
                lane_chunk,
            ));
        };
        chunk_field(
            *lane_contribution == contribution,
            contribution,
            node_at,
            "contribution",
        )?;
        chunk_field(
            *lane_occurrence == occurrence,
            contribution,
            node_at,
            "occurrence",
        )?;
        chunk_field(lane_node_id == node, contribution, node_at, "node")?;
        chunk_field(
            requested_address == address,
            contribution,
            node_at,
            "exact request",
        )?;
        chunk_field(origin == &document.origin, contribution, node_at, "origin")?;
        chunk_field(lane_marker == marker, contribution, node_at, "marker")?;
        chunk_field(
            lane_before == fence_before,
            contribution,
            node_at,
            "fence before",
        )?;
        chunk_field(
            lane_after == fence_after,
            contribution,
            node_at,
            "fence after",
        )?;
        chunk_field(lane_body == body, contribution, node_at, "body")?;

        if *trailing_newline_required {
            validate_newline(chunks, contribution, occurrence, &mut chunk_index)?;
        }
        let close_at = chunk_index;
        let close = take_chunk(chunks, contribution, &mut chunk_index, "normal-close")?;
        let LaneChunk::NormalClose {
            contribution: lane_contribution,
            occurrence: lane_occurrence,
            marker: lane_marker,
        } = close
        else {
            return Err(unexpected_chunk(
                contribution,
                close_at,
                "normal-close",
                close,
            ));
        };
        chunk_field(
            *lane_contribution == contribution,
            contribution,
            close_at,
            "contribution",
        )?;
        chunk_field(
            *lane_occurrence == occurrence,
            contribution,
            close_at,
            "occurrence",
        )?;
        chunk_field(lane_marker == marker, contribution, close_at, "marker")?;
        *occurrence_index += 1;
    }
    no_trailing_chunks(contribution, chunks, chunk_index)
}

pub(super) fn validate_simple(
    link: &LinkResult,
    contribution: usize,
    document: &ClosureDocument,
    chunks: &[LaneChunk],
    occurrence_index: &mut usize,
) -> Result<(), LaneTransitionError> {
    let flat = *occurrence_index;
    let linked = link
        .occurrences
        .get(flat)
        .ok_or(LaneTransitionError::MissingOccurrence {
            contribution,
            index: flat,
        })?;
    let LinkOccurrence::Simple {
        contribution: link_contribution,
        occurrence,
        address,
        fence_before,
        fence_after,
        body,
        trailing_newline_required,
    } = linked
    else {
        return Err(unexpected_occurrence(flat, "simple", linked));
    };
    occurrence_field(*link_contribution == contribution, flat, "contribution")?;
    occurrence_field(*occurrence == 0, flat, "occurrence")?;
    occurrence_field(*address == document.address, flat, "address")?;
    occurrence_field(
        *trailing_newline_required != body.ends_with('\n'),
        flat,
        "newline",
    )?;

    let mut chunk_index = 0;
    let lane_chunk = take_chunk(chunks, contribution, &mut chunk_index, "simple node")?;
    let LaneChunk::Node(lane_node) = lane_chunk else {
        return Err(unexpected_chunk(contribution, 0, "simple node", lane_chunk));
    };
    let LaneNode::Simple {
        contribution: lane_contribution,
        occurrence: lane_occurrence,
        address: lane_address,
        origin,
        fence_before: lane_before,
        fence_after: lane_after,
        body: lane_body,
    } = lane_node.as_ref()
    else {
        return Err(unexpected_chunk(contribution, 0, "simple node", lane_chunk));
    };
    chunk_field(
        *lane_contribution == contribution,
        contribution,
        0,
        "contribution",
    )?;
    chunk_field(*lane_occurrence == 0, contribution, 0, "occurrence")?;
    chunk_field(lane_address == address, contribution, 0, "address")?;
    chunk_field(origin == &document.origin, contribution, 0, "origin")?;
    chunk_field(lane_before == fence_before, contribution, 0, "fence before")?;
    chunk_field(lane_after == fence_after, contribution, 0, "fence after")?;
    chunk_field(lane_body == body, contribution, 0, "body")?;
    if *trailing_newline_required {
        validate_newline(chunks, contribution, 0, &mut chunk_index)?;
    }
    *occurrence_index += 1;
    no_trailing_chunks(contribution, chunks, chunk_index)
}

fn validate_newline(
    chunks: &[LaneChunk],
    contribution: usize,
    occurrence: usize,
    index: &mut usize,
) -> Result<(), LaneTransitionError> {
    let at = *index;
    let newline = take_chunk(chunks, contribution, index, "forced-newline")?;
    let LaneChunk::ForcedNewline {
        contribution: lane_contribution,
        occurrence: lane_occurrence,
    } = newline
    else {
        return Err(unexpected_chunk(
            contribution,
            at,
            "forced-newline",
            newline,
        ));
    };
    chunk_field(
        *lane_contribution == contribution,
        contribution,
        at,
        "contribution",
    )?;
    chunk_field(
        *lane_occurrence == occurrence,
        contribution,
        at,
        "occurrence",
    )
}

fn take_chunk<'a>(
    chunks: &'a [LaneChunk],
    contribution: usize,
    index: &mut usize,
    expected: &'static str,
) -> Result<&'a LaneChunk, LaneTransitionError> {
    let chunk = chunks
        .get(*index)
        .ok_or(LaneTransitionError::MissingChunk {
            contribution,
            chunk: *index,
            expected,
        })?;
    *index += 1;
    Ok(chunk)
}

fn no_trailing_chunks(
    contribution: usize,
    chunks: &[LaneChunk],
    index: usize,
) -> Result<(), LaneTransitionError> {
    if index == chunks.len() {
        Ok(())
    } else {
        Err(LaneTransitionError::TrailingChunk {
            contribution,
            chunk: index,
        })
    }
}

fn unexpected_occurrence(
    index: usize,
    expected: &'static str,
    actual: &LinkOccurrence,
) -> LaneTransitionError {
    LaneTransitionError::UnexpectedOccurrence {
        index,
        expected,
        actual: match actual {
            LinkOccurrence::Normal { .. } => "normal",
            LinkOccurrence::Simple { .. } => "simple",
        },
    }
}

fn unexpected_chunk(
    contribution: usize,
    chunk: usize,
    expected: &'static str,
    actual: &LaneChunk,
) -> LaneTransitionError {
    LaneTransitionError::UnexpectedChunk {
        contribution,
        chunk,
        expected,
        actual: match actual {
            LaneChunk::NormalOpen { .. } => "normal-open",
            LaneChunk::Node(node) => match node.as_ref() {
                LaneNode::Normal { .. } => "normal node",
                LaneNode::Simple { .. } => "simple node",
            },
            LaneChunk::ForcedNewline { .. } => "forced-newline",
            LaneChunk::NormalClose { .. } => "normal-close",
        },
    }
}

fn occurrence_field(
    condition: bool,
    index: usize,
    field: &'static str,
) -> Result<(), LaneTransitionError> {
    if condition {
        Ok(())
    } else {
        Err(LaneTransitionError::OccurrenceField { index, field })
    }
}

fn chunk_field(
    condition: bool,
    contribution: usize,
    chunk: usize,
    field: &'static str,
) -> Result<(), LaneTransitionError> {
    if condition {
        Ok(())
    } else {
        Err(LaneTransitionError::ChunkField {
            contribution,
            chunk,
            field,
        })
    }
}
