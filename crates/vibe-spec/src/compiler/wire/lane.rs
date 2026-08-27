//! Lane conversion: one complete artifact after linking, plus the fence
//! snapshot codec the link/lane vocabulary shares.

use super::super::ir::{
    ClosureNodeId, LaneChunk, LaneContribution, LaneFrame, LaneIr, LaneNode, LinkFenceSnapshot,
    LinkInputDigest, LinkMarkerKey, OriginRename,
};
use super::address::{
    decode_document_address, decode_spec_address, encode_document_address, encode_spec_address,
};
use super::closure::{
    apply_package_relation, decode_context, decode_meta, encode_context, encode_meta,
};
use super::emitted::{digest_hex, parse_digest};
use super::{
    G_ADDRESS_REPARSE, G_ARENA_BOUNDS, IrWireError, gate, narrow, require_scalar, widen, wire,
};

pub(super) fn decode_lane(value: &wire::LaneIr) -> Result<LaneIr, IrWireError> {
    let context = decode_context(&value.context)?;
    let source_node_count = narrow("lane source_node_count", value.source_node_count)?;
    let source_link_digest = parse_digest("lane source_link_digest", &value.source_link_digest)?;
    let frame = LaneFrame {
        generated_path: value.frame.generated_path.clone(),
        source_root: value.frame.source_root.clone(),
        renames: value
            .frame
            .renames
            .iter()
            .map(decode_origin_rename)
            .collect(),
    };
    let mut contributions = Vec::with_capacity(value.contributions.len());
    for contribution in &value.contributions {
        contributions.push(decode_lane_contribution(contribution, source_node_count)?);
    }
    Ok(LaneIr::assembled(
        context,
        source_node_count,
        LinkInputDigest(source_link_digest),
        frame,
        contributions,
    ))
}

fn decode_lane_contribution(
    value: &wire::LaneContribution,
    source_node_count: usize,
) -> Result<LaneContribution, IrWireError> {
    match value {
        wire::LaneContribution::Normal(normal) => {
            let meta = decode_meta(&normal.meta)?;
            let seed_address = decode_spec_address(&normal.seed_address)?;
            apply_package_relation("normal", &meta, &seed_address, false)?;
            let seed = narrow("lane seed", normal.seed)?;
            if seed >= source_node_count {
                return Err(gate(
                    G_ARENA_BOUNDS,
                    format!(
                        "lane seed names node {seed} of a {source_node_count}-node source closure"
                    ),
                ));
            }
            let mut chunks = Vec::with_capacity(normal.chunks.len());
            for chunk in &normal.chunks {
                chunks.push(decode_chunk(chunk, source_node_count)?);
            }
            Ok(LaneContribution::Normal {
                meta,
                seed: ClosureNodeId(seed),
                seed_address,
                chunks,
            })
        }
        wire::LaneContribution::Simple(simple) => {
            let mut chunks = Vec::with_capacity(simple.chunks.len());
            for chunk in &simple.chunks {
                chunks.push(decode_chunk(chunk, source_node_count)?);
            }
            Ok(LaneContribution::Simple {
                meta: decode_meta(&simple.meta)?,
                address: decode_document_address(&simple.address)?,
                chunks,
            })
        }
        wire::LaneContribution::Elided(elided) => Ok(LaneContribution::Elided {
            meta: decode_meta(&elided.meta)?,
        }),
        wire::LaneContribution::Hoisted(hoisted) => {
            let meta = decode_meta(&hoisted.meta)?;
            let target = decode_spec_address(&hoisted.target)?;
            apply_package_relation("hoisted", &meta, &target, true)?;
            Ok(LaneContribution::Hoisted { meta, target })
        }
    }
}

fn decode_chunk(
    value: &wire::LaneChunk,
    source_node_count: usize,
) -> Result<LaneChunk, IrWireError> {
    Ok(match value {
        wire::LaneChunk::NormalOpen(open) => LaneChunk::NormalOpen {
            contribution: narrow("chunk contribution", open.contribution)?,
            occurrence: narrow("chunk occurrence", open.occurrence)?,
            marker: LinkMarkerKey::new(open.marker.clone()),
        },
        wire::LaneChunk::Node(node) => {
            LaneChunk::Node(Box::new(decode_lane_node(&node.node, source_node_count)?))
        }
        wire::LaneChunk::ForcedNewline(newline) => LaneChunk::ForcedNewline {
            contribution: narrow("chunk contribution", newline.contribution)?,
            occurrence: narrow("chunk occurrence", newline.occurrence)?,
        },
        wire::LaneChunk::NormalClose(close) => LaneChunk::NormalClose {
            contribution: narrow("chunk contribution", close.contribution)?,
            occurrence: narrow("chunk occurrence", close.occurrence)?,
            marker: LinkMarkerKey::new(close.marker.clone()),
        },
    })
}

fn decode_lane_node(
    value: &wire::LaneNode,
    source_node_count: usize,
) -> Result<LaneNode, IrWireError> {
    match value {
        wire::LaneNode::Normal(normal) => {
            require_scalar("lane node origin", &normal.origin)?;
            let node = narrow("lane node", normal.node)?;
            if node >= source_node_count {
                return Err(gate(
                    G_ARENA_BOUNDS,
                    format!("lane node {node} indexes a {source_node_count}-node source closure"),
                ));
            }
            Ok(LaneNode::Normal {
                contribution: narrow("lane node contribution", normal.contribution)?,
                occurrence: narrow("lane node occurrence", normal.occurrence)?,
                node: ClosureNodeId(node),
                requested_address: decode_spec_address(&normal.requested_address)?,
                origin: normal.origin.clone(),
                marker: LinkMarkerKey::new(normal.marker.clone()),
                fence_before: decode_fence(&normal.fence_before)?,
                fence_after: decode_fence(&normal.fence_after)?,
                body: normal.body.clone(),
            })
        }
        wire::LaneNode::Simple(simple) => {
            require_scalar("lane node origin", &simple.origin)?;
            Ok(LaneNode::Simple {
                contribution: narrow("lane node contribution", simple.contribution)?,
                occurrence: narrow("lane node occurrence", simple.occurrence)?,
                address: decode_document_address(&simple.address)?,
                origin: simple.origin.clone(),
                fence_before: decode_fence(&simple.fence_before)?,
                fence_after: decode_fence(&simple.fence_after)?,
                body: simple.body.clone(),
            })
        }
    }
}

/// A fence snapshot is carried as `closed` or `open` with a delimiter that
/// must be exactly one fence character.
pub(super) fn decode_fence(value: &wire::FenceSnapshot) -> Result<LinkFenceSnapshot, IrWireError> {
    match value {
        wire::FenceSnapshot::Closed(_) => Ok(LinkFenceSnapshot::Closed),
        wire::FenceSnapshot::Open(open) => {
            let mut chars = open.delimiter.chars();
            let Some(delimiter) = chars.next() else {
                return Err(gate(
                    G_ADDRESS_REPARSE,
                    "a fence delimiter must be exactly one character",
                ));
            };
            if chars.next().is_some() || (delimiter != '`' && delimiter != '~') {
                return Err(gate(
                    G_ADDRESS_REPARSE,
                    format!(
                        "a fence delimiter must be exactly one fence character, got `{}`",
                        open.delimiter
                    ),
                ));
            }
            Ok(LinkFenceSnapshot::Open {
                delimiter,
                run: narrow("fence run", open.run)?,
            })
        }
    }
}

pub(super) fn encode_fence(value: LinkFenceSnapshot) -> Result<wire::FenceSnapshot, IrWireError> {
    Ok(match value {
        LinkFenceSnapshot::Closed => {
            wire::FenceSnapshot::Closed(Box::new(wire::FenceSnapshotClosed {}))
        }
        LinkFenceSnapshot::Open { delimiter, run } => {
            wire::FenceSnapshot::Open(Box::new(wire::FenceSnapshotOpen {
                delimiter: delimiter.to_string(),
                run: widen("fence run", run)?,
            }))
        }
    })
}

fn decode_origin_rename(value: &wire::OriginRename) -> OriginRename {
    OriginRename {
        origin: value.origin.clone(),
        rename: crate::RenameEntry {
            original: value.rename.original.clone(),
            qualified: value.rename.qualified.clone(),
        },
    }
}

fn encode_origin_rename(value: &OriginRename) -> wire::OriginRename {
    wire::OriginRename {
        origin: value.origin.clone(),
        rename: wire::RenameEntry {
            original: value.rename.original.clone(),
            qualified: value.rename.qualified.clone(),
        },
    }
}

pub(super) fn encode_lane(value: &LaneIr) -> Result<wire::LaneIr, IrWireError> {
    let mut contributions = Vec::with_capacity(value.contributions.len());
    for contribution in &value.contributions {
        contributions.push(encode_lane_contribution(contribution)?);
    }
    Ok(wire::LaneIr {
        context: encode_context(value.context())?,
        source_node_count: widen("lane source_node_count", value.source_node_count)?,
        source_link_digest: digest_hex(&value.source_link_digest.0),
        frame: wire::LaneFrame {
            generated_path: value.frame.generated_path.clone(),
            source_root: value.frame.source_root.clone(),
            renames: value
                .frame
                .renames
                .iter()
                .map(encode_origin_rename)
                .collect(),
        },
        contributions,
    })
}

fn encode_lane_contribution(
    value: &LaneContribution,
) -> Result<wire::LaneContribution, IrWireError> {
    Ok(match value {
        LaneContribution::Normal {
            meta,
            seed,
            seed_address,
            chunks,
        } => {
            let mut wire_chunks = Vec::with_capacity(chunks.len());
            for chunk in chunks {
                wire_chunks.push(encode_chunk(chunk)?);
            }
            wire::LaneContribution::Normal(Box::new(wire::LaneContributionNormal {
                meta: encode_meta(meta),
                seed: widen("lane seed", seed.0)?,
                seed_address: encode_spec_address(seed_address),
                chunks: wire_chunks,
            }))
        }
        LaneContribution::Simple {
            meta,
            address,
            chunks,
        } => {
            let mut wire_chunks = Vec::with_capacity(chunks.len());
            for chunk in chunks {
                wire_chunks.push(encode_chunk(chunk)?);
            }
            wire::LaneContribution::Simple(Box::new(wire::LaneContributionSimple {
                meta: encode_meta(meta),
                address: encode_document_address(address),
                chunks: wire_chunks,
            }))
        }
        LaneContribution::Elided { meta } => {
            wire::LaneContribution::Elided(Box::new(wire::LaneContributionElided {
                meta: encode_meta(meta),
            }))
        }
        LaneContribution::Hoisted { meta, target } => {
            wire::LaneContribution::Hoisted(Box::new(wire::LaneContributionHoisted {
                meta: encode_meta(meta),
                target: encode_spec_address(target),
            }))
        }
    })
}

fn encode_chunk(value: &LaneChunk) -> Result<wire::LaneChunk, IrWireError> {
    Ok(match value {
        LaneChunk::NormalOpen {
            contribution,
            occurrence,
            marker,
        } => wire::LaneChunk::NormalOpen(Box::new(wire::LaneChunkNormalOpen {
            contribution: widen("chunk contribution", *contribution)?,
            occurrence: widen("chunk occurrence", *occurrence)?,
            marker: marker.as_str().to_string(),
        })),
        LaneChunk::Node(node) => wire::LaneChunk::Node(Box::new(wire::LaneChunkNode {
            node: encode_lane_node(node)?,
        })),
        LaneChunk::ForcedNewline {
            contribution,
            occurrence,
        } => wire::LaneChunk::ForcedNewline(Box::new(wire::LaneChunkForcedNewline {
            contribution: widen("chunk contribution", *contribution)?,
            occurrence: widen("chunk occurrence", *occurrence)?,
        })),
        LaneChunk::NormalClose {
            contribution,
            occurrence,
            marker,
        } => wire::LaneChunk::NormalClose(Box::new(wire::LaneChunkNormalClose {
            contribution: widen("chunk contribution", *contribution)?,
            occurrence: widen("chunk occurrence", *occurrence)?,
            marker: marker.as_str().to_string(),
        })),
    })
}

fn encode_lane_node(value: &LaneNode) -> Result<wire::LaneNode, IrWireError> {
    Ok(match value {
        LaneNode::Normal {
            contribution,
            occurrence,
            node,
            requested_address,
            origin,
            marker,
            fence_before,
            fence_after,
            body,
        } => wire::LaneNode::Normal(Box::new(wire::LaneNodeNormal {
            contribution: widen("lane node contribution", *contribution)?,
            occurrence: widen("lane node occurrence", *occurrence)?,
            node: widen("lane node", node.0)?,
            requested_address: encode_spec_address(requested_address),
            origin: origin.clone(),
            marker: marker.as_str().to_string(),
            fence_before: encode_fence(*fence_before)?,
            fence_after: encode_fence(*fence_after)?,
            body: body.clone(),
        })),
        LaneNode::Simple {
            contribution,
            occurrence,
            address,
            origin,
            fence_before,
            fence_after,
            body,
        } => wire::LaneNode::Simple(Box::new(wire::LaneNodeSimple {
            contribution: widen("lane node contribution", *contribution)?,
            occurrence: widen("lane node occurrence", *occurrence)?,
            address: encode_document_address(address),
            origin: origin.clone(),
            fence_before: encode_fence(*fence_before)?,
            fence_after: encode_fence(*fence_after)?,
            body: body.clone(),
        })),
    })
}
