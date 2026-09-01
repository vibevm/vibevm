//! Consume-only publication of one fully prepared replay.

use std::fmt;

use crate::WorkspaceError;
use crate::boot_artifacts::transaction::{
    self, ArtifactWrite, DetailedTransactionFailure, TransactionFailureDisposition,
};
use crate::extension_world::OwnerRuntimeId;

use super::replay_prepare::{
    PreparedBootReplay, PreparedOwnerKind, PreparedOwnerParts, PreparedOwnerPublication,
};

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct BootReplayOwner {
    owner: OwnerRuntimeId,
    kind: PreparedOwnerKind,
}

impl BootReplayOwner {
    #[must_use]
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "remove when R5.4-INSTALL consumes replay owner identity"
        )
    )]
    pub(crate) const fn owner(&self) -> &OwnerRuntimeId {
        &self.owner
    }

    #[must_use]
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "remove when R5.4-INSTALL consumes replay owner kind"
        )
    )]
    pub(crate) const fn kind(&self) -> PreparedOwnerKind {
        self.kind
    }
}

#[derive(Debug)]
pub(crate) struct PublishedBootReplay {
    committed: Box<[BootReplayOwner]>,
}

impl PublishedBootReplay {
    #[must_use]
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "remove when R5.4-INSTALL consumes committed replay owners"
        )
    )]
    pub(crate) const fn committed(&self) -> &[BootReplayOwner] {
        &self.committed
    }
}

#[derive(Debug)]
pub(crate) struct BootReplayPublishFailure {
    source: Box<WorkspaceError>,
    committed_before: Box<[BootReplayOwner]>,
    failed_owner: BootReplayOwner,
    disposition: TransactionFailureDisposition,
    untouched: Box<[BootReplayOwner]>,
}

impl BootReplayPublishFailure {
    #[must_use]
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "remove when R5.4-INSTALL reports replay source")
    )]
    pub(crate) fn source_error(&self) -> &WorkspaceError {
        self.source.as_ref()
    }

    #[must_use]
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "remove when R5.4-INSTALL reports committed replay prefix"
        )
    )]
    pub(crate) const fn committed_before(&self) -> &[BootReplayOwner] {
        &self.committed_before
    }

    #[must_use]
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "remove when R5.4-INSTALL reports failed replay owner"
        )
    )]
    pub(crate) const fn failed_owner(&self) -> &BootReplayOwner {
        &self.failed_owner
    }

    #[must_use]
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "remove when R5.4-INSTALL reports replay disposition"
        )
    )]
    pub(crate) const fn disposition(&self) -> TransactionFailureDisposition {
        self.disposition
    }

    #[must_use]
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "remove when R5.4-INSTALL reports untouched replay suffix"
        )
    )]
    pub(crate) const fn untouched(&self) -> &[BootReplayOwner] {
        &self.untouched
    }
}

impl fmt::Display for BootReplayPublishFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "boot replay publication failed for {}: {}",
            self.failed_owner.owner, self.source
        )
    }
}

impl std::error::Error for BootReplayPublishFailure {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.source.as_ref())
    }
}

#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "remove when R5.4-INSTALL invokes replay publication"
    )
)]
pub(crate) fn publish_boot_replay(
    prepared: PreparedBootReplay,
) -> Result<PublishedBootReplay, BootReplayPublishFailure> {
    publish_boot_replay_using(prepared, transaction::write_production_detailed)
}

#[cfg(test)]
pub(crate) fn publish_boot_replay_with_faults(
    prepared: PreparedBootReplay,
    faults: &impl transaction::FaultInjector,
) -> Result<PublishedBootReplay, BootReplayPublishFailure> {
    publish_boot_replay_using(prepared, |write| {
        transaction::write_with_faults_detailed(write, faults)
    })
}

fn publish_boot_replay_using(
    prepared: PreparedBootReplay,
    mut publish: impl FnMut(ArtifactWrite<'_>) -> Result<(), DetailedTransactionFailure>,
) -> Result<PublishedBootReplay, BootReplayPublishFailure> {
    let mut publications = prepared.into_publications().into_vec().into_iter();
    let mut committed = Vec::new();
    while let Some(publication) = publications.next() {
        let parts = publication.into_parts();
        let current = owner_identity(&parts);
        let result = publish(ArtifactWrite {
            index_path: &parts.index_path,
            index_bytes: &parts.index,
            static_path: &parts.static_path,
            static_bytes: parts.static_lane.as_deref(),
            stale_path: &parts.stale_path,
        });
        if let Err(failure) = result {
            let disposition = failure.disposition();
            let untouched = publications
                .map(PreparedOwnerPublication::into_parts)
                .map(|parts| owner_identity(&parts))
                .collect::<Vec<_>>()
                .into_boxed_slice();
            return Err(BootReplayPublishFailure {
                source: Box::new(failure.into_source()),
                committed_before: committed.into_boxed_slice(),
                failed_owner: current,
                disposition,
                untouched,
            });
        }
        committed.push(current);
    }
    Ok(PublishedBootReplay {
        committed: committed.into_boxed_slice(),
    })
}

fn owner_identity(parts: &PreparedOwnerParts) -> BootReplayOwner {
    BootReplayOwner {
        owner: parts.owner.clone(),
        kind: parts.kind,
    }
}

#[cfg(test)]
#[path = "replay_publish/tests.rs"]
mod tests;
