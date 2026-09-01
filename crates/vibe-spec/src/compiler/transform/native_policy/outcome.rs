//! Public managed-compile outcome type state.

use std::fmt;

use crate::compiler::ir::EmittedArtifact;

use super::{CompilerInvocationReceipts, CompilerPendingSet};

/// Stable status word of one managed compiler-native result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompilerNativeStatus {
    Ready,
    Pending,
}

/// A managed compile either produced publishable bytes or a provisional value
/// that only a later in-crate finalizer may consume.
pub enum CompilerNativeOutcome {
    Ready(CompilerReadyArtifact),
    Pending(CompilerPendingArtifact),
}

impl CompilerNativeOutcome {
    #[must_use]
    pub const fn status(&self) -> CompilerNativeStatus {
        match self {
            Self::Ready(_) => CompilerNativeStatus::Ready,
            Self::Pending(_) => CompilerNativeStatus::Pending,
        }
    }

    #[must_use]
    pub const fn as_ready(&self) -> Option<&CompilerReadyArtifact> {
        match self {
            Self::Ready(ready) => Some(ready),
            Self::Pending(_) => None,
        }
    }

    #[must_use]
    pub const fn as_pending(&self) -> Option<&CompilerPendingArtifact> {
        match self {
            Self::Ready(_) => None,
            Self::Pending(pending) => Some(pending),
        }
    }

    pub(crate) fn ready(artifact: EmittedArtifact, receipts: CompilerInvocationReceipts) -> Self {
        Self::Ready(CompilerReadyArtifact { artifact, receipts })
    }

    pub(crate) fn pending(artifact: EmittedArtifact, pending: CompilerPendingSet) -> Self {
        Self::Pending(CompilerPendingArtifact { artifact, pending })
    }
}

impl fmt::Debug for CompilerNativeOutcome {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ready(ready) => ready.fmt(formatter),
            Self::Pending(pending) => pending.fmt(formatter),
        }
    }
}

/// Publishable managed output and replay receipts.
pub struct CompilerReadyArtifact {
    artifact: EmittedArtifact,
    receipts: CompilerInvocationReceipts,
}

impl CompilerReadyArtifact {
    #[must_use]
    pub const fn artifact(&self) -> &EmittedArtifact {
        &self.artifact
    }

    #[must_use]
    pub const fn receipts(&self) -> &CompilerInvocationReceipts {
        &self.receipts
    }

    #[must_use]
    pub fn into_artifact(self) -> EmittedArtifact {
        self.artifact
    }

    /// Consume the Ready result without discarding replay receipts.
    #[must_use]
    pub fn into_parts(self) -> (EmittedArtifact, CompilerInvocationReceipts) {
        (self.artifact, self.receipts)
    }
}

impl fmt::Debug for CompilerReadyArtifact {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CompilerReadyArtifact")
            .field("receipts", &self.receipts)
            .finish_non_exhaustive()
    }
}

/// Non-publishable compiler output. Bytes and provenance remain private until
/// WORKSPACE's later finalizer rebuilds the two transform headers.
pub struct CompilerPendingArtifact {
    artifact: EmittedArtifact,
    pending: CompilerPendingSet,
}

impl CompilerPendingArtifact {
    #[must_use]
    pub const fn status(&self) -> CompilerNativeStatus {
        CompilerNativeStatus::Pending
    }

    #[must_use]
    pub const fn pending(&self) -> &CompilerPendingSet {
        &self.pending
    }

    /// Consume a pending outcome into replay identity only.
    ///
    /// The provisional artifact is deliberately dropped here. It is neither
    /// returned nor projected, so callers can feed the non-reusable set into
    /// [`super::CompilerNativePolicy::resolve`] without gaining publishable
    /// bytes, a digest or provenance.
    #[must_use]
    pub fn into_pending_set(self) -> CompilerPendingSet {
        self.pending
    }

    pub(crate) fn into_parts(self) -> (EmittedArtifact, CompilerPendingSet) {
        (self.artifact, self.pending)
    }

    #[cfg(test)]
    pub(crate) const fn artifact_for_test(&self) -> &EmittedArtifact {
        &self.artifact
    }
}

impl fmt::Debug for CompilerPendingArtifact {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CompilerPendingArtifact")
            .field("status", &self.status())
            .field("pending", &self.pending)
            .finish()
    }
}
