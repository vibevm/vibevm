use super::super::digest::StableDigest;
use super::super::ir::{
    ArtifactFrame, ArtifactTarget, DocumentAddress, LaneChunk, LaneContribution, LaneInputDigest,
    LaneIr, LaneNode, LinkFenceSnapshot, StaticCompileMode,
};

const LANE_DOMAIN: &[u8] = b"vibe-spec/lane-input/v1";
const CHUNK_DOMAIN: &[u8] = b"vibe-spec/lane-chunks/v1";
const BYTES_DOMAIN: &[u8] = b"vibe-spec/emitted-bytes/v1";

pub(super) fn lane_digest(lane: &LaneIr) -> LaneInputDigest {
    let mut digest = StableDigest::new(LANE_DOMAIN);
    digest.field(lane.context().artifact().as_str().as_bytes());
    digest.byte(target_byte(&lane.context().target()));
    digest.byte(mode_byte(lane.context().mode()));
    match lane.context().frame() {
        ArtifactFrame::CompatibilityFragment => digest.byte(0),
        ArtifactFrame::StaticLane {
            generated_path,
            source_root,
        } => {
            digest.byte(1);
            digest.field(generated_path.as_bytes());
            digest.field(source_root.as_bytes());
        }
    }
    digest.usize(lane.source_node_count);
    digest.field(&lane.source_link_digest.0);
    match (&lane.frame.generated_path, &lane.frame.source_root) {
        (Some(path), Some(root)) => {
            digest.byte(1);
            digest.field(path.as_bytes());
            digest.field(root.as_bytes());
        }
        (path, root) => {
            digest.byte(0);
            if let Some(path) = path {
                digest.field(path.as_bytes());
            }
            if let Some(root) = root {
                digest.field(root.as_bytes());
            }
        }
    }
    digest.usize(lane.frame.renames.len());
    for rename in &lane.frame.renames {
        digest.field(rename.origin.as_bytes());
        digest.field(rename.rename.original.as_bytes());
        digest.field(rename.rename.qualified.as_bytes());
    }
    digest.usize(lane.contributions.len());
    for contribution in &lane.contributions {
        hash_contribution(&mut digest, contribution);
    }
    LaneInputDigest(digest.finish())
}

pub(super) fn chunks_digest(chunks: &[LaneChunk]) -> [u8; 32] {
    let mut digest = StableDigest::new(CHUNK_DOMAIN);
    digest.usize(chunks.len());
    for chunk in chunks {
        hash_chunk(&mut digest, chunk);
    }
    digest.finish()
}

pub(crate) fn bytes_digest(bytes: &[u8]) -> [u8; 32] {
    let mut digest = StableDigest::new(BYTES_DOMAIN);
    digest.field(bytes);
    digest.finish()
}

fn hash_contribution(digest: &mut StableDigest, contribution: &LaneContribution) {
    match contribution {
        LaneContribution::Normal {
            meta,
            seed,
            seed_address,
            chunks,
        } => {
            digest.byte(0);
            hash_meta(digest, meta);
            digest.usize(seed.0);
            digest.field(seed_address.to_string().as_bytes());
            digest.field(&chunks_digest(chunks));
        }
        LaneContribution::Simple {
            meta,
            address,
            chunks,
        } => {
            digest.byte(1);
            hash_meta(digest, meta);
            hash_document_address(digest, address);
            digest.field(&chunks_digest(chunks));
        }
        LaneContribution::Elided { meta } => {
            digest.byte(2);
            hash_meta(digest, meta);
        }
        LaneContribution::Hoisted { meta, target } => {
            digest.byte(3);
            hash_meta(digest, meta);
            digest.field(target.to_string().as_bytes());
        }
    }
}

fn hash_chunk(digest: &mut StableDigest, chunk: &LaneChunk) {
    match chunk {
        LaneChunk::NormalOpen {
            contribution,
            occurrence,
            marker,
        } => {
            digest.byte(0);
            digest.usize(*contribution);
            digest.usize(*occurrence);
            digest.field(marker.as_str().as_bytes());
        }
        LaneChunk::Node(node) => match node.as_ref() {
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
            } => {
                digest.byte(1);
                digest.usize(*contribution);
                digest.usize(*occurrence);
                digest.usize(node.0);
                digest.field(requested_address.to_string().as_bytes());
                digest.field(origin.as_bytes());
                digest.field(marker.as_str().as_bytes());
                hash_fence(digest, fence_before);
                hash_fence(digest, fence_after);
                digest.field(body.as_bytes());
            }
            LaneNode::Simple {
                contribution,
                occurrence,
                address,
                origin,
                fence_before,
                fence_after,
                body,
            } => {
                digest.byte(2);
                digest.usize(*contribution);
                digest.usize(*occurrence);
                hash_document_address(digest, address);
                digest.field(origin.as_bytes());
                hash_fence(digest, fence_before);
                hash_fence(digest, fence_after);
                digest.field(body.as_bytes());
            }
        },
        LaneChunk::ForcedNewline {
            contribution,
            occurrence,
        } => {
            digest.byte(3);
            digest.usize(*contribution);
            digest.usize(*occurrence);
        }
        LaneChunk::NormalClose {
            contribution,
            occurrence,
            marker,
        } => {
            digest.byte(4);
            digest.usize(*contribution);
            digest.usize(*occurrence);
            digest.field(marker.as_str().as_bytes());
        }
    }
}

fn hash_meta(digest: &mut StableDigest, meta: &super::super::ir::ContributionMeta) {
    digest.field(meta.origin.as_bytes());
    digest.field(meta.path.as_bytes());
}

fn hash_document_address(digest: &mut StableDigest, address: &DocumentAddress) {
    match address {
        DocumentAddress::Spec(address) => {
            digest.byte(0);
            digest.field(address.to_string().as_bytes());
        }
        DocumentAddress::StaticEntry { origin, path } => {
            digest.byte(1);
            digest.field(origin.as_bytes());
            digest.field(path.as_bytes());
        }
    }
}

fn hash_fence(digest: &mut StableDigest, fence: &LinkFenceSnapshot) {
    match fence {
        LinkFenceSnapshot::Closed => digest.byte(0),
        LinkFenceSnapshot::Open { delimiter, run } => {
            digest.byte(1);
            digest.u32(*delimiter as u32);
            digest.usize(*run);
        }
    }
}

fn target_byte(target: &ArtifactTarget) -> u8 {
    if target.is_static_markdown() {
        0
    } else if target.is_static_xml() {
        1
    } else {
        2
    }
}

fn mode_byte(mode: StaticCompileMode) -> u8 {
    match mode {
        StaticCompileMode::Plain => 0,
        StaticCompileMode::QualifyPerNode => 1,
    }
}
