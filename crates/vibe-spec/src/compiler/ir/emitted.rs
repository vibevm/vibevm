//! Manager-owned emitted bytes and immutable production evidence.

use crate::SpecAddress;

use super::{
    ArtifactContext, ClosureNodeId, ContributionMeta, DocumentAddress, LaneContribution, LaneFrame,
    LinkInputDigest, OriginRename,
};
use crate::compiler::backend::BackendId;
use crate::compiler::pass::PassName;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LaneInputDigest(pub(crate) [u8; 32]);

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum EmissionContributionWitness {
    Normal {
        meta: ContributionMeta,
        seed: ClosureNodeId,
        seed_address: SpecAddress,
        chunk_digest: [u8; 32],
    },
    Simple {
        meta: ContributionMeta,
        address: DocumentAddress,
        chunk_digest: [u8; 32],
    },
    Elided {
        meta: ContributionMeta,
    },
    Hoisted {
        meta: ContributionMeta,
        target: SpecAddress,
    },
}

/// Target-specific semantic material prepared from the Lane before a backend
/// can emit bytes. XML documents are parsed from the Lane's canonical
/// Markdown exactly once, then shared by the renderer and the independent
/// emitted-tape validator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PreparedEmissionTarget {
    Markdown,
    Xml {
        documents: Vec<Option<vibe_specdoc::doc::SpecDoc>>,
    },
    #[cfg(any(test, feature = "test-support"))]
    Custom,
}

/// Manager-owned snapshot captured before the selected backend can run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PreEmissionWitness {
    pub(crate) context: ArtifactContext,
    pub(crate) source_node_count: usize,
    pub(crate) source_link_digest: LinkInputDigest,
    pub(crate) frame: LaneFrame,
    pub(crate) contributions: Vec<LaneContribution>,
    pub(crate) lane_digest: LaneInputDigest,
    pub(crate) emission_witnesses: Vec<EmissionContributionWitness>,
    pub(crate) prepared_target: PreparedEmissionTarget,
}

/// Immutable evidence created by the manager at the selected backend boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmissionProvenance {
    pub(crate) context: ArtifactContext,
    pub(crate) backend: BackendId,
    pub(crate) producer: PassName,
    pub(crate) source_lane_digest: LaneInputDigest,
    pub(crate) renames: Vec<OriginRename>,
    pub(crate) contributions: Vec<EmissionContributionWitness>,
    pub(crate) bytes_digest: [u8; 32],
}

impl EmissionProvenance {
    pub fn context(&self) -> &ArtifactContext {
        &self.context
    }

    pub fn backend_id(&self) -> &str {
        self.backend.as_str()
    }

    pub fn producer(&self) -> &str {
        self.producer.as_str()
    }
}

/// Arbitrary final artifact bytes plus manager-owned immutable provenance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmittedArtifact {
    pub(crate) provenance: EmissionProvenance,
    pub(crate) bytes: Vec<u8>,
}

impl EmittedArtifact {
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }

    pub fn provenance(&self) -> &EmissionProvenance {
        &self.provenance
    }

    #[cfg(test)]
    pub(crate) fn testing(context: ArtifactContext, bytes: Vec<u8>) -> Self {
        Self {
            provenance: EmissionProvenance {
                context,
                backend: BackendId::new("test").unwrap(),
                producer: PassName::new("emit:test").unwrap(),
                source_lane_digest: LaneInputDigest([0; 32]),
                renames: Vec::new(),
                contributions: Vec::new(),
                bytes_digest: [0; 32],
            },
            bytes,
        }
    }

    pub(crate) fn context(&self) -> &ArtifactContext {
        &self.provenance.context
    }
}

pub(crate) type EmittedIr = EmittedArtifact;
