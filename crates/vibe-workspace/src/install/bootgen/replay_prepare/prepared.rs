use std::path::PathBuf;

use vibe_core::manifest::SpecFormat;

use crate::extension_world::OwnerRuntimeId;

use super::{ReplayCandidate, ReplayLane};

pub(crate) struct PreparedBootReplay {
    pub(super) publications: Box<[PreparedOwnerPublication]>,
}

pub(crate) enum PreparedOwnerPublication {
    Unit {
        owner: OwnerRuntimeId,
        boot_dir: PathBuf,
        spec_format: SpecFormat,
        index: Box<[u8]>,
        static_lane: Option<Box<[u8]>>,
    },
    Node {
        owner: OwnerRuntimeId,
        node_dir: PathBuf,
        node_rel: String,
        spec_format: SpecFormat,
        index: Box<[u8]>,
        static_lane: Option<Box<[u8]>>,
    },
}

impl PreparedOwnerPublication {
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "remove when R5.4-REPLAY-PUBLISH consumes prepared owner identity"
        )
    )]
    pub(crate) fn owner(&self) -> &OwnerRuntimeId {
        match self {
            Self::Unit { owner, .. } | Self::Node { owner, .. } => owner,
        }
    }

    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "remove when R5.4-REPLAY-PUBLISH consumes prepared INDEX bytes"
        )
    )]
    pub(crate) fn index(&self) -> &[u8] {
        match self {
            Self::Unit { index, .. } | Self::Node { index, .. } => index,
        }
    }

    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "remove when R5.4-REPLAY-PUBLISH consumes prepared STATIC bytes"
        )
    )]
    pub(crate) fn static_lane(&self) -> Option<&[u8]> {
        match self {
            Self::Unit { static_lane, .. } | Self::Node { static_lane, .. } => {
                static_lane.as_deref()
            }
        }
    }

    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "remove when R5.4-REPLAY-PUBLISH consumes prepared format"
        )
    )]
    pub(crate) fn spec_format(&self) -> SpecFormat {
        match self {
            Self::Unit { spec_format, .. } | Self::Node { spec_format, .. } => *spec_format,
        }
    }

    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "remove when R5.4-REPLAY-PUBLISH consumes prepared target root"
        )
    )]
    pub(crate) fn target_root(&self) -> &std::path::Path {
        match self {
            Self::Unit { boot_dir, .. } => boot_dir,
            Self::Node { node_dir, .. } => node_dir,
        }
    }

    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "remove when R5.4-REPLAY-PUBLISH consumes prepared node relation"
        )
    )]
    pub(crate) fn node_rel(&self) -> Option<&str> {
        match self {
            Self::Unit { .. } => None,
            Self::Node { node_rel, .. } => Some(node_rel),
        }
    }
}

impl PreparedBootReplay {
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "remove when R5.4-REPLAY-PUBLISH consumes prepared owners"
        )
    )]
    pub(crate) fn publications(&self) -> &[PreparedOwnerPublication] {
        &self.publications
    }
}

impl ReplayLane {
    pub(super) fn into_publication(
        self,
        index: Box<[u8]>,
        static_lane: Option<Box<[u8]>>,
    ) -> PreparedOwnerPublication {
        match self.candidate {
            ReplayCandidate::Unit(candidate) => PreparedOwnerPublication::Unit {
                owner: candidate.owner,
                boot_dir: candidate.boot_dir,
                spec_format: candidate.spec_format,
                index,
                static_lane,
            },
            ReplayCandidate::Node(candidate) => PreparedOwnerPublication::Node {
                owner: candidate.owner,
                node_dir: candidate.node_dir,
                node_rel: candidate.rel,
                spec_format: candidate.spec_format,
                index,
                static_lane,
            },
        }
    }
}
