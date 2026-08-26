//! Backend-neutral result of cross-node short-link resolution.

use crate::SpecAddress;

use super::{ClosureNodeId, ContributionMeta, DocumentAddress, StaticCompileMode};

/// Exact digest of the semantic input used by the named link pass.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LinkInputDigest(pub(crate) [u8; 32]);

/// The Markdown fence state at one linked occurrence boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LinkFenceSnapshot {
    Closed,
    Open { delimiter: char, run: usize },
}

impl LinkFenceSnapshot {
    /// The same boundary state as the Markdown fence machine sees it, so a
    /// consumer of the lane can resume scanning a body exactly where link left
    /// off instead of assuming every body starts outside a fence.
    pub(crate) fn markdown(&self) -> crate::doctree::FenceSnapshot {
        match *self {
            Self::Closed => crate::doctree::FenceSnapshot::Closed,
            Self::Open { delimiter, run } => crate::doctree::FenceSnapshot::Open { delimiter, run },
        }
    }
}

/// Typed reversible-marker identity; concrete comment bytes belong to emit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LinkMarkerKey(String);

impl LinkMarkerKey {
    pub(crate) fn from_address(address: &SpecAddress) -> Self {
        Self(address.without_pin())
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

/// Exact top-level identity consumed by link, including empty contributions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum LinkContributionWitness {
    Normal {
        meta: ContributionMeta,
        seed: ClosureNodeId,
        seed_address: SpecAddress,
        occurrence_count: usize,
    },
    Simple {
        meta: ContributionMeta,
        address: DocumentAddress,
    },
    Elided {
        meta: ContributionMeta,
    },
    Hoisted {
        meta: ContributionMeta,
        target: SpecAddress,
    },
}

/// One resolved occurrence. It carries semantics, never backend marker bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum LinkOccurrence {
    Normal {
        contribution: usize,
        occurrence: usize,
        node: ClosureNodeId,
        address: SpecAddress,
        marker: LinkMarkerKey,
        fence_before: LinkFenceSnapshot,
        fence_after: LinkFenceSnapshot,
        body: String,
        trailing_newline_required: bool,
    },
    Simple {
        contribution: usize,
        occurrence: usize,
        address: DocumentAddress,
        fence_before: LinkFenceSnapshot,
        fence_after: LinkFenceSnapshot,
        body: String,
        trailing_newline_required: bool,
    },
}

/// Canonical result of linking one whole Closure/artifact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LinkResult {
    pub(crate) mode: StaticCompileMode,
    pub(crate) input_digest: LinkInputDigest,
    pub(crate) contributions: Vec<LinkContributionWitness>,
    pub(crate) occurrences: Vec<LinkOccurrence>,
}

/// Runtime typestate of the occurrence-sensitive link transform.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum LinkState {
    Unlinked,
    Linked(LinkResult),
}
