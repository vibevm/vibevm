#[cfg(test)]
use std::path::Path;
use std::path::PathBuf;

use crate::extension_world::OwnerRuntimeId;
use crate::system_prompt::PromptPublication;

use super::{ReplayCandidate, ReplayLane};

pub(crate) struct PreparedBootReplay {
    pub(super) publications: Box<[PreparedOwnerPublication]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PreparedOwnerKind {
    Unit,
    Node,
}

/// One node lane's prompt publication, together with the facts its managed
/// block selector needs. A unit lane has none: a package slot publishes no
/// prefix (PROP-076 `#SYS-TRANSACTION-ROLES`).
pub(crate) struct NodePromptPublication {
    /// What this generation publishes for the node's prompt.
    pub(crate) publication: PromptPublication,
    /// The node selector this publication runs inside its own transaction,
    /// **already built** at preparation time: the publisher is consume-only
    /// and derives nothing, which is a property its own source test pins.
    pub(crate) selectors: NodeSelectorFn,
}

/// One prepared node selector, owned.
pub(crate) type NodeSelectorFn = Box<dyn Fn(&str) -> Result<(), crate::WorkspaceError>>;

pub(crate) struct PreparedOwnerPublication {
    owner: OwnerRuntimeId,
    kind: PreparedOwnerKind,
    index_path: PathBuf,
    static_path: PathBuf,
    stale_path: PathBuf,
    index: Box<[u8]>,
    static_lane: Option<Box<[u8]>>,
    node_guards: Vec<std::sync::Arc<crate::install::bootgen::PreparedNodeBoot>>,
    node_prompt: Option<NodePromptPublication>,
}

pub(crate) struct PreparedOwnerParts {
    pub(crate) owner: OwnerRuntimeId,
    pub(crate) kind: PreparedOwnerKind,
    pub(crate) index_path: PathBuf,
    pub(crate) static_path: PathBuf,
    pub(crate) stale_path: PathBuf,
    pub(crate) index: Box<[u8]>,
    pub(crate) static_lane: Option<Box<[u8]>>,
    pub(crate) node_guards: Vec<std::sync::Arc<crate::install::bootgen::PreparedNodeBoot>>,
    pub(crate) node_prompt: Option<NodePromptPublication>,
}

impl PreparedBootReplay {
    pub(crate) fn into_publications(self) -> Box<[PreparedOwnerPublication]> {
        self.publications
    }

    #[cfg(test)]
    pub(crate) fn publications(&self) -> &[PreparedOwnerPublication] {
        &self.publications
    }

    #[cfg(test)]
    pub(crate) fn from_test(publications: Vec<PreparedOwnerPublication>) -> Self {
        Self {
            publications: publications.into_boxed_slice(),
        }
    }
}

impl PreparedOwnerPublication {
    pub(super) fn with_node_guards(
        mut self,
        guards: Vec<std::sync::Arc<crate::install::bootgen::PreparedNodeBoot>>,
    ) -> Self {
        self.node_guards = guards;
        self
    }

    pub(super) fn with_node_prompt(mut self, prompt: Option<NodePromptPublication>) -> Self {
        self.node_prompt = prompt;
        self
    }
    pub(crate) fn into_parts(self) -> PreparedOwnerParts {
        PreparedOwnerParts {
            owner: self.owner,
            kind: self.kind,
            index_path: self.index_path,
            static_path: self.static_path,
            stale_path: self.stale_path,
            index: self.index,
            static_lane: self.static_lane,
            node_guards: self.node_guards,
            node_prompt: self.node_prompt,
        }
    }

    #[cfg(test)]
    pub(crate) fn for_test(
        owner: OwnerRuntimeId,
        kind: PreparedOwnerKind,
        index_path: PathBuf,
        static_path: PathBuf,
        stale_path: PathBuf,
        index: impl Into<Box<[u8]>>,
        static_lane: Option<Box<[u8]>>,
    ) -> Self {
        Self {
            owner,
            kind,
            index_path,
            static_path,
            stale_path,
            index: index.into(),
            static_lane,
            node_guards: Vec::new(),
            node_prompt: None,
        }
    }

    #[cfg(test)]
    pub(crate) const fn owner(&self) -> &OwnerRuntimeId {
        &self.owner
    }

    #[cfg(test)]
    pub(crate) const fn kind(&self) -> PreparedOwnerKind {
        self.kind
    }

    #[cfg(test)]
    pub(crate) fn index_path(&self) -> &Path {
        &self.index_path
    }

    #[cfg(test)]
    pub(crate) fn static_path(&self) -> &Path {
        &self.static_path
    }

    #[cfg(test)]
    pub(crate) fn stale_path(&self) -> &Path {
        &self.stale_path
    }

    #[cfg(test)]
    pub(crate) fn index(&self) -> &[u8] {
        &self.index
    }

    #[cfg(test)]
    pub(crate) fn static_lane(&self) -> Option<&[u8]> {
        self.static_lane.as_deref()
    }
}

impl ReplayLane {
    pub(super) fn into_publication(
        self,
        index: Box<[u8]>,
        static_lane: Option<Box<[u8]>>,
    ) -> PreparedOwnerPublication {
        let (owner, kind, index_path, static_path, stale_path) = match self.candidate {
            ReplayCandidate::Unit(candidate) => (
                candidate.owner,
                PreparedOwnerKind::Unit,
                candidate.index_path,
                candidate.static_path,
                candidate.stale_path,
            ),
            ReplayCandidate::Node(candidate) => (
                candidate.owner,
                PreparedOwnerKind::Node,
                candidate.index_path,
                candidate.static_path,
                candidate.stale_path,
            ),
        };
        PreparedOwnerPublication {
            owner,
            kind,
            index_path,
            static_path,
            stale_path,
            index,
            static_lane,
            node_guards: Vec::new(),
            node_prompt: None,
        }
    }
}
