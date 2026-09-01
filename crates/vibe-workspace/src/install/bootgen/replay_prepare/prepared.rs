#[cfg(test)]
use std::path::Path;
use std::path::PathBuf;

use crate::extension_world::OwnerRuntimeId;

use super::{ReplayCandidate, ReplayLane};

pub(crate) struct PreparedBootReplay {
    pub(super) publications: Box<[PreparedOwnerPublication]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PreparedOwnerKind {
    Unit,
    Node,
}

pub(crate) struct PreparedOwnerPublication {
    owner: OwnerRuntimeId,
    kind: PreparedOwnerKind,
    index_path: PathBuf,
    static_path: PathBuf,
    stale_path: PathBuf,
    index: Box<[u8]>,
    static_lane: Option<Box<[u8]>>,
}

pub(crate) struct PreparedOwnerParts {
    pub(crate) owner: OwnerRuntimeId,
    pub(crate) kind: PreparedOwnerKind,
    pub(crate) index_path: PathBuf,
    pub(crate) static_path: PathBuf,
    pub(crate) stale_path: PathBuf,
    pub(crate) index: Box<[u8]>,
    pub(crate) static_lane: Option<Box<[u8]>>,
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
    pub(crate) fn into_parts(self) -> PreparedOwnerParts {
        PreparedOwnerParts {
            owner: self.owner,
            kind: self.kind,
            index_path: self.index_path,
            static_path: self.static_path,
            stale_path: self.stale_path,
            index: self.index,
            static_lane: self.static_lane,
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
        }
    }
}
