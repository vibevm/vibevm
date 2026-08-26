//! Intrinsic Lane validation, independent of Closure and the production projector.

use crate::doctree::{FenceSnapshot, FenceTracker};

use super::super::ir::{
    ArtifactFrame, DocumentAddress, LaneChunk, LaneContribution, LaneIr, LaneNode,
    LinkFenceSnapshot,
};

#[derive(Debug, thiserror::Error)]
pub(crate) enum LaneValidationError {
    #[error("lane frame does not match its immutable artifact context")]
    FrameContext,
    #[error("lane contribution {contribution} has unsafe {field}")]
    UnsafeProvenance {
        contribution: usize,
        field: &'static str,
    },
    #[error("lane contribution {contribution} chunk {chunk} is missing; expected {expected}")]
    MissingChunk {
        contribution: usize,
        chunk: usize,
        expected: &'static str,
    },
    #[error("lane contribution {contribution} chunk {chunk} is {actual}; expected {expected}")]
    UnexpectedChunk {
        contribution: usize,
        chunk: usize,
        expected: &'static str,
        actual: &'static str,
    },
    #[error("lane contribution {contribution} chunk {chunk} differs at {field}")]
    ChunkMismatch {
        contribution: usize,
        chunk: usize,
        field: &'static str,
    },
}

/// What the intrinsic walk observed beyond pass/fail: the fence state each
/// top-level contribution leaves behind, in contribution order.
///
/// The intrinsic validator owns *shape*; whether an open boundary is legal is a
/// target policy the inter-pass verifier owns. Handing the summary out keeps
/// both laws on the single [`FenceTracker`] walk below — no second scan, and no
/// parallel grammar — while leaving `validate_lane`'s verdict exactly what it
/// was before R3.3.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LaneShape {
    pub(crate) closing_fences: Vec<LinkFenceSnapshot>,
}

/// Validate a Lane using only the Lane value and its immutable context.
///
/// This is the production verdict: `AssemblePass`, `EmitPass` and the assemble
/// transition all run it, so it must accept exactly what it accepted before
/// R3.3. It deliberately drops the shape summary.
pub(crate) fn validate_lane(lane: &LaneIr) -> Result<(), LaneValidationError> {
    validate_shape(lane).map(|_| ())
}

/// The same intrinsic walk, returning its [`LaneShape`] summary for the
/// inter-pass verifier.
pub(crate) fn validate_shape(lane: &LaneIr) -> Result<LaneShape, LaneValidationError> {
    validate_frame(lane)?;
    let mut closing_fences = Vec::with_capacity(lane.contributions.len());
    for (contribution, entry) in lane.contributions.iter().enumerate() {
        validate_meta(contribution, entry)?;
        closing_fences.push(match entry {
            LaneContribution::Normal { seed, chunks, .. } => {
                require(
                    seed.0 < lane.source_node_count,
                    contribution,
                    0,
                    "seed node bounds",
                )?;
                validate_normal(contribution, chunks, lane.source_node_count)?
            }
            LaneContribution::Simple {
                address, chunks, ..
            } => validate_simple(contribution, address, chunks)?,
            // Neither emits bytes, so neither can leave a fence open.
            LaneContribution::Elided { .. } | LaneContribution::Hoisted { .. } => {
                LinkFenceSnapshot::Closed
            }
        });
    }
    Ok(LaneShape { closing_fences })
}

fn validate_frame(lane: &LaneIr) -> Result<(), LaneValidationError> {
    let matches = match lane.context().frame() {
        ArtifactFrame::CompatibilityFragment => {
            lane.frame.generated_path.is_none() && lane.frame.source_root.is_none()
        }
        ArtifactFrame::StaticLane {
            generated_path,
            source_root,
        } => {
            lane.frame.generated_path.as_ref() == Some(generated_path)
                && lane.frame.source_root.as_ref() == Some(source_root)
        }
    };
    if matches {
        Ok(())
    } else {
        Err(LaneValidationError::FrameContext)
    }
}

fn validate_meta(contribution: usize, entry: &LaneContribution) -> Result<(), LaneValidationError> {
    let meta = match entry {
        LaneContribution::Normal { meta, .. }
        | LaneContribution::Simple { meta, .. }
        | LaneContribution::Elided { meta }
        | LaneContribution::Hoisted { meta, .. } => meta,
    };
    for (field, value) in [
        ("origin", meta.origin.as_str()),
        ("path", meta.path.as_str()),
    ] {
        if value.trim().is_empty() || value.contains(['\n', '\r', '\0']) {
            return Err(LaneValidationError::UnsafeProvenance {
                contribution,
                field,
            });
        }
    }
    Ok(())
}

/// Returns the fence state this contribution leaves behind — the very tracker
/// the occurrence-by-occurrence `fence_before`/`fence_after` checks already run,
/// never a second scan with a parallel grammar.
fn validate_normal(
    contribution: usize,
    chunks: &[LaneChunk],
    source_node_count: usize,
) -> Result<LinkFenceSnapshot, LaneValidationError> {
    let mut cursor = LaneCursor::new(contribution, chunks);
    let mut occurrence = 0;
    let mut fence = FenceTracker::default();
    while cursor.peek().is_some() {
        let open_at = cursor.index;
        let open = cursor.take("normal-open")?;
        let LaneChunk::NormalOpen {
            contribution: open_contribution,
            occurrence: open_occurrence,
            marker: open_marker,
        } = open
        else {
            return Err(cursor.unexpected(open_at, "normal-open", open));
        };
        require(
            *open_contribution == contribution,
            contribution,
            open_at,
            "open contribution",
        )?;
        require(
            *open_occurrence == occurrence,
            contribution,
            open_at,
            "open occurrence",
        )?;

        let node_at = cursor.index;
        let node_chunk = cursor.take("normal node")?;
        let LaneChunk::Node(node) = node_chunk else {
            return Err(cursor.unexpected(node_at, "normal node", node_chunk));
        };
        let LaneNode::Normal {
            contribution: node_contribution,
            occurrence: node_occurrence,
            node,
            requested_address,
            origin,
            marker,
            fence_before,
            fence_after,
            body,
            ..
        } = node.as_ref()
        else {
            return Err(cursor.unexpected(node_at, "normal node", node_chunk));
        };
        require(
            *node_contribution == contribution,
            contribution,
            node_at,
            "node contribution",
        )?;
        require(
            *node_occurrence == occurrence,
            contribution,
            node_at,
            "node occurrence",
        )?;
        require(
            node.0 < source_node_count,
            contribution,
            node_at,
            "node bounds",
        )?;
        require(safe_text(origin), contribution, node_at, "node origin")?;
        let expected_key = requested_address.without_pin();
        require(
            marker.as_str() == expected_key,
            contribution,
            node_at,
            "marker key",
        )?;
        require(open_marker == marker, contribution, open_at, "open marker")?;
        require(
            *fence_before == fence_snapshot(fence.snapshot()),
            contribution,
            node_at,
            "fence before",
        )?;
        advance_fence(&mut fence, body);
        require(
            *fence_after == fence_snapshot(fence.snapshot()),
            contribution,
            node_at,
            "fence after",
        )?;

        validate_newline(&mut cursor, occurrence, !body.ends_with('\n'))?;
        let close_at = cursor.index;
        let close = cursor.take("normal-close")?;
        let LaneChunk::NormalClose {
            contribution: close_contribution,
            occurrence: close_occurrence,
            marker: close_marker,
        } = close
        else {
            return Err(cursor.unexpected(close_at, "normal-close", close));
        };
        require(
            *close_contribution == contribution,
            contribution,
            close_at,
            "close contribution",
        )?;
        require(
            *close_occurrence == occurrence,
            contribution,
            close_at,
            "close occurrence",
        )?;
        require(
            close_marker == marker,
            contribution,
            close_at,
            "close marker",
        )?;
        occurrence += 1;
    }
    Ok(fence_snapshot(fence.snapshot()))
}

/// Returns the fence state this contribution leaves behind, from its own
/// tracker — see [`validate_normal`].
fn validate_simple(
    contribution: usize,
    address: &DocumentAddress,
    chunks: &[LaneChunk],
) -> Result<LinkFenceSnapshot, LaneValidationError> {
    let DocumentAddress::StaticEntry {
        origin: address_origin,
        path: address_path,
    } = address
    else {
        return Err(LaneValidationError::ChunkMismatch {
            contribution,
            chunk: 0,
            field: "simple address kind",
        });
    };
    require(
        safe_text(address_origin) && safe_text(address_path),
        contribution,
        0,
        "simple address",
    )?;
    let mut cursor = LaneCursor::new(contribution, chunks);
    let node_chunk = cursor.take("simple node")?;
    let LaneChunk::Node(node) = node_chunk else {
        return Err(cursor.unexpected(0, "simple node", node_chunk));
    };
    let LaneNode::Simple {
        contribution: actual_contribution,
        occurrence,
        address: actual_address,
        origin,
        fence_before,
        fence_after,
        body,
        ..
    } = node.as_ref()
    else {
        return Err(cursor.unexpected(0, "simple node", node_chunk));
    };
    require(
        *actual_contribution == contribution,
        contribution,
        0,
        "contribution",
    )?;
    require(*occurrence == 0, contribution, 0, "occurrence")?;
    require(actual_address == address, contribution, 0, "address")?;
    require(safe_text(origin), contribution, 0, "node origin")?;
    let mut fence = FenceTracker::default();
    require(
        *fence_before == fence_snapshot(fence.snapshot()),
        contribution,
        0,
        "fence before",
    )?;
    advance_fence(&mut fence, body);
    require(
        *fence_after == fence_snapshot(fence.snapshot()),
        contribution,
        0,
        "fence after",
    )?;
    validate_newline(&mut cursor, 0, !body.ends_with('\n'))?;
    if let Some(chunk) = cursor.peek() {
        return Err(cursor.unexpected(cursor.index, "end of simple contribution", chunk));
    }
    Ok(fence_snapshot(fence.snapshot()))
}

fn validate_newline(
    cursor: &mut LaneCursor<'_>,
    occurrence: usize,
    required: bool,
) -> Result<(), LaneValidationError> {
    let present = matches!(cursor.peek(), Some(LaneChunk::ForcedNewline { .. }));
    if required != present {
        return Err(LaneValidationError::UnexpectedChunk {
            contribution: cursor.contribution,
            chunk: cursor.index,
            expected: if required {
                "forced-newline"
            } else {
                "next node or close"
            },
            actual: cursor.peek().map_or("end of contribution", chunk_kind),
        });
    }
    if present {
        let at = cursor.index;
        let chunk = cursor.take("forced-newline")?;
        let LaneChunk::ForcedNewline {
            contribution,
            occurrence: actual_occurrence,
        } = chunk
        else {
            unreachable!("presence check accepted only ForcedNewline")
        };
        require(
            *contribution == cursor.contribution,
            cursor.contribution,
            at,
            "newline contribution",
        )?;
        require(
            *actual_occurrence == occurrence,
            cursor.contribution,
            at,
            "newline occurrence",
        )?;
    }
    Ok(())
}

fn advance_fence(fence: &mut FenceTracker, text: &str) {
    for line in text.split('\n') {
        fence.classify(line);
    }
}

fn safe_text(value: &str) -> bool {
    !value.trim().is_empty() && !value.contains(['\n', '\r', '\0'])
}

fn fence_snapshot(snapshot: FenceSnapshot) -> LinkFenceSnapshot {
    match snapshot {
        FenceSnapshot::Closed => LinkFenceSnapshot::Closed,
        FenceSnapshot::Open { delimiter, run } => LinkFenceSnapshot::Open { delimiter, run },
    }
}

fn require(
    condition: bool,
    contribution: usize,
    chunk: usize,
    field: &'static str,
) -> Result<(), LaneValidationError> {
    if condition {
        Ok(())
    } else {
        Err(LaneValidationError::ChunkMismatch {
            contribution,
            chunk,
            field,
        })
    }
}

struct LaneCursor<'a> {
    contribution: usize,
    chunks: &'a [LaneChunk],
    index: usize,
}

impl<'a> LaneCursor<'a> {
    fn new(contribution: usize, chunks: &'a [LaneChunk]) -> Self {
        Self {
            contribution,
            chunks,
            index: 0,
        }
    }

    fn peek(&self) -> Option<&'a LaneChunk> {
        self.chunks.get(self.index)
    }

    fn take(&mut self, expected: &'static str) -> Result<&'a LaneChunk, LaneValidationError> {
        let chunk = self.peek().ok_or(LaneValidationError::MissingChunk {
            contribution: self.contribution,
            chunk: self.index,
            expected,
        })?;
        self.index += 1;
        Ok(chunk)
    }

    fn unexpected(
        &self,
        chunk: usize,
        expected: &'static str,
        actual: &LaneChunk,
    ) -> LaneValidationError {
        LaneValidationError::UnexpectedChunk {
            contribution: self.contribution,
            chunk,
            expected,
            actual: chunk_kind(actual),
        }
    }
}

fn chunk_kind(chunk: &LaneChunk) -> &'static str {
    match chunk {
        LaneChunk::NormalOpen { .. } => "normal-open",
        LaneChunk::Node(node) => match node.as_ref() {
            LaneNode::Normal { .. } => "normal node",
            LaneNode::Simple { .. } => "simple node",
        },
        LaneChunk::ForcedNewline { .. } => "forced-newline",
        LaneChunk::NormalClose { .. } => "normal-close",
    }
}
