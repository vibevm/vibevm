//! Backend-neutral lane carrier produced by the named assemble lowering.

use crate::SpecAddress;

use super::{
    ArtifactContext, ClosureNodeId, ContributionMeta, DocumentAddress, LinkFenceSnapshot,
    LinkInputDigest, LinkMarkerKey, OriginRename,
};

/// Semantic frame data shared by every concrete lane backend.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LaneFrame {
    pub(crate) generated_path: Option<String>,
    pub(crate) source_root: Option<String>,
    pub(crate) renames: Vec<OriginRename>,
}

/// One top-level contribution in exact effective-boot order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum LaneContribution {
    Normal {
        meta: ContributionMeta,
        seed: ClosureNodeId,
        seed_address: SpecAddress,
        chunks: Vec<LaneChunk>,
    },
    Simple {
        meta: ContributionMeta,
        address: DocumentAddress,
        chunks: Vec<LaneChunk>,
    },
    Elided {
        meta: ContributionMeta,
    },
    Hoisted {
        meta: ContributionMeta,
        target: SpecAddress,
    },
}

/// Backend-neutral framing and body chunks for one contribution.
///
/// Open/close and forced-newline semantics are closed variants. Concrete
/// comment or newline bytes remain the selected emit backend's responsibility.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum LaneChunk {
    NormalOpen {
        contribution: usize,
        occurrence: usize,
        marker: LinkMarkerKey,
    },
    Node(Box<LaneNode>),
    ForcedNewline {
        contribution: usize,
        occurrence: usize,
    },
    NormalClose {
        contribution: usize,
        occurrence: usize,
        marker: LinkMarkerKey,
    },
}

/// One exact linked occurrence. Normal and simple marker policy is closed by type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum LaneNode {
    Normal {
        contribution: usize,
        occurrence: usize,
        node: ClosureNodeId,
        requested_address: SpecAddress,
        origin: String,
        marker: LinkMarkerKey,
        fence_before: LinkFenceSnapshot,
        fence_after: LinkFenceSnapshot,
        body: String,
    },
    Simple {
        contribution: usize,
        occurrence: usize,
        address: DocumentAddress,
        origin: String,
        fence_before: LinkFenceSnapshot,
        fence_after: LinkFenceSnapshot,
        body: String,
    },
}

/// One complete artifact after linking and before backend serialisation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LaneIr {
    context: ArtifactContext,
    pub(crate) source_node_count: usize,
    pub(crate) source_link_digest: LinkInputDigest,
    pub(crate) frame: LaneFrame,
    pub(crate) contributions: Vec<LaneContribution>,
}

impl LaneIr {
    pub(crate) fn assembled(
        context: ArtifactContext,
        source_node_count: usize,
        source_link_digest: LinkInputDigest,
        frame: LaneFrame,
        contributions: Vec<LaneContribution>,
    ) -> Self {
        Self {
            context,
            source_node_count,
            source_link_digest,
            frame,
            contributions,
        }
    }

    pub(crate) fn context(&self) -> &ArtifactContext {
        &self.context
    }
}
