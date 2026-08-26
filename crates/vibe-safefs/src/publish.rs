//! What a publication attempt did, and how far it got.
//!
//! A caller that applies several files needs more than "it failed": it needs to
//! know whether the failing row is *definitely* not on disk, or *might* be. The
//! rename is the boundary — everything before it is provably invisible, the
//! rename itself and everything after it may already be visible — so the stage
//! is part of the error type rather than something a caller infers from prose.
//!
//! The other observable a naive "nothing was written" claim misses is
//! **directories**. Creating `docs/nested/` to hold one output is a real,
//! visible mutation even when the file never lands. Every publication therefore
//! reports the directories it created, so a caller can name them or remove
//! exactly the ones this invocation proved it made — never a directory it
//! merely found.

use std::path::PathBuf;

use specmark::spec;

/// How far a publication got before it failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#REPLY-SHAPE")]
pub enum PublishStage {
    /// The rename had not been attempted. The destination file is definitely
    /// unchanged; any staging file has been removed.
    BeforePublication,
    /// The rename was attempted and may have taken effect, or it took effect
    /// and the visible result then failed verification. The destination file
    /// must be treated as possibly replaced.
    PossiblyPublished,
}

/// One failed publication, with the fact a caller cannot re-derive.
#[derive(Debug)]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#REPLY-SHAPE")]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#REPLY-SHAPE")]
pub struct PublishError {
    pub stage: PublishStage,
    /// Directories this invocation created before failing — observable state
    /// even when no file landed.
    pub created_directories: Vec<PathBuf>,
    pub source: anyhow::Error,
}

impl PublishError {
    /// Flatten into an ordinary error chain for a caller that does not
    /// distinguish the stages, keeping both facts it could not re-derive: how
    /// far the publication got, and which directories it created.
    #[must_use]
    pub fn into_report(self) -> anyhow::Error {
        let created = self.created_display();
        let stage = match self.stage {
            PublishStage::BeforePublication => {
                "failed before publication (the destination is unchanged)".to_string()
            }
            PublishStage::PossiblyPublished => "failed after the rename was attempted \
                 (the destination may already hold the new bytes)"
                .to_string(),
        };
        let context = if created.is_empty() {
            stage
        } else {
            format!("{stage}; this run created {}", created.join(", "))
        };
        self.source.context(context)
    }

    pub(crate) fn before(created: Vec<PathBuf>, source: anyhow::Error) -> Self {
        Self {
            stage: PublishStage::BeforePublication,
            created_directories: created,
            source,
        }
    }

    pub(crate) fn possibly(created: Vec<PathBuf>, source: anyhow::Error) -> Self {
        Self {
            stage: PublishStage::PossiblyPublished,
            created_directories: created,
            source,
        }
    }

    /// The created directories as forward-slashed display paths.
    #[must_use]
    pub fn created_display(&self) -> Vec<String> {
        self.created_directories
            .iter()
            .map(|path| path.to_string_lossy().replace('\\', "/"))
            .collect()
    }
}

impl std::fmt::Display for PublishError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:#}", self.source)
    }
}

impl std::error::Error for PublishError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source.source()
    }
}

/// A completed publication: the file is verified on disk, and these are the
/// directories that had to be created to hold it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#REPLY-SHAPE")]
pub struct Published {
    pub created_directories: Vec<PathBuf>,
}
