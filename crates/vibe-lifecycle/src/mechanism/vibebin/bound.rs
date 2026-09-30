//! Pinned I/O for a host-admitted local UserProfile vibe-bin operation.
//!
//! A caller retains the physical-destination cooperative lock for this
//! value's whole lifetime. This cell never opens a root from a pathname and
//! grants nothing from the artifact record or wire binding by itself.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-076#ENVIRONMENT-BOUND-BUILTIN");

use vibe_safefs::{LockGuard, Project, PublishStage, StableFileState};
use vibe_wire::generated::artifact_record::{
    ArtifactKind, ArtifactRecord, ArtifactShape, DigestAlgorithm, RelativeRoot,
};

use super::launcher::{self, LauncherFlavour, Occupant};
use super::store::{self, PayloadPlacement};

#[derive(Debug, thiserror::Error)]
pub(crate) enum BoundVibeBinError {
    #[error("bound vibe-bin requires a project-root executable file artifact")]
    UnsupportedArtifact,
    #[error("bound artifact record is invalid: {0}")]
    InvalidRecord(String),
    #[error("project-root artifact is absent or unreadable: {0}")]
    Source(String),
    #[error("project-root artifact differs from its recorded SHA-256 or byte length")]
    SourceChanged,
    #[error("content-addressed payload exists with different bytes or mode")]
    CasMismatch,
    #[error("bound destination observation or operation refused: {0}")]
    Destination(String),
    #[error(
        "bound destination may already hold a published byte; inspect its retained operation before retry: {0}"
    )]
    PublicationUncertain(String),
    #[error("bound pointer exists but does not carry a valid payload digest")]
    InvalidPointer,
    #[error("the pinned physical destination already has a cooperating writer")]
    LockBusy,
}

/// The source coordinates the engine's validated artifact record named.
/// Its byte count is the owner's current measured result; the copy rechecks
/// both length and SHA-256 through one held source handle before any write.
pub(crate) struct BoundProjectArtifact {
    relative: String,
    digest: String,
    bytes: u64,
}

impl BoundProjectArtifact {
    pub(crate) fn from_record(
        record: &ArtifactRecord,
        measured_bytes: u64,
    ) -> Result<Self, BoundVibeBinError> {
        vibe_wire::behaviour::artifact_record::validate(record)
            .map_err(|error| BoundVibeBinError::InvalidRecord(error.to_string()))?;
        if record.path_relative.root != RelativeRoot::Project
            || record.shape != ArtifactShape::File
            || record.kind != ArtifactKind::Executable
            || record.digest.algorithm != DigestAlgorithm::Sha256
        {
            return Err(BoundVibeBinError::UnsupportedArtifact);
        }
        Ok(Self {
            relative: record.path_relative.path.clone(),
            digest: record.digest.value.clone(),
            bytes: measured_bytes,
        })
    }
}

/// The destination capability and its lock are one value. A caller cannot
/// accidentally pass a lock from an unrelated directory to bound I/O.
pub(crate) struct BoundDestinationLock {
    settings: Project,
    _guard: LockGuard,
}

impl BoundDestinationLock {
    pub(crate) fn acquire(settings: Project) -> Result<Self, BoundVibeBinError> {
        let lock_home = settings
            .dir(&[".vibe"], true)
            .map_err(|error| BoundVibeBinError::Destination(format!("{error:#}")))?;
        let guard = lock_home
            .try_lock_file("bound-vibe-bin-physical.lock")
            .map_err(|error| BoundVibeBinError::Destination(format!("{error:#}")))?
            .ok_or(BoundVibeBinError::LockBusy)?;
        Ok(Self {
            settings,
            _guard: guard,
        })
    }

    pub(crate) fn settings(&self) -> &Project {
        &self.settings
    }
}

/// Exactly the two roots the host pinned. The destination lock stays live
/// through the last observation because this store borrows its owner.
pub(crate) struct BoundVibeBinStore<'a> {
    project: &'a Project,
    destination: &'a BoundDestinationLock,
}

impl<'a> BoundVibeBinStore<'a> {
    pub(crate) fn new(project: &'a Project, destination: &'a BoundDestinationLock) -> Self {
        Self {
            project,
            destination,
        }
    }

    fn settings(&self) -> &Project {
        self.destination.settings()
    }

    /// Reprove source, then publish one immutable payload only when absent.
    /// Under a cooperating-writer lock, a matching occupant is a true no-op
    /// and a third state is never repaired by overwriting it.
    pub(crate) fn place_payload(
        &self,
        artifact: &BoundProjectArtifact,
        flavour: LauncherFlavour,
    ) -> Result<PayloadPlacement, BoundVibeBinError> {
        let source = self
            .project
            .stable_file_state(&artifact.relative)
            .map_err(|error| BoundVibeBinError::Source(format!("{error:#}")))?
            .ok_or_else(|| BoundVibeBinError::Source("recorded artifact is absent".into()))?;
        if source.sha256 != artifact.digest || source.bytes != artifact.bytes {
            return Err(BoundVibeBinError::SourceChanged);
        }
        let relative = store::payload_relative(flavour, &artifact.digest);
        let mode = executable_mode(flavour);
        if let Some(observed) = self.state(&relative)? {
            if matches_payload(&observed, artifact, mode) {
                return Ok(PayloadPlacement::AlreadyPresent);
            }
            return Err(BoundVibeBinError::CasMismatch);
        }
        self.project
            .copy_stable_file_to_expected(
                &artifact.relative,
                self.settings(),
                &relative,
                mode,
                &artifact.digest,
                artifact.bytes,
            )
            .map_err(|error| {
                let uncertain = error.stage == PublishStage::PossiblyPublished;
                let rendered = format!("{:#}", error.into_report());
                if uncertain {
                    BoundVibeBinError::PublicationUncertain(rendered)
                } else {
                    BoundVibeBinError::Destination(rendered)
                }
            })?;
        let visible = self.state(&relative).map_err(|error| {
            BoundVibeBinError::PublicationUncertain(format!(
                "published CAS occupant could not be reobserved: {error}"
            ))
        })?;
        match visible {
            Some(observed) if matches_payload(&observed, artifact, mode) => {
                Ok(PayloadPlacement::Written)
            }
            _ => Err(BoundVibeBinError::PublicationUncertain(
                "published CAS occupant did not reobserve at expected bytes and mode".into(),
            )),
        }
    }

    /// Bounded launcher classification through the same pinned settings root.
    pub(crate) fn classify_launcher(&self, relative: &str) -> Result<Occupant, BoundVibeBinError> {
        let bytes = self
            .settings()
            .read_file_bounded(relative, 64 * 1024)
            .map_err(|error| BoundVibeBinError::Destination(format!("{error:#}")))?;
        Ok(bytes
            .as_deref()
            .map_or(Occupant::Vacant, launcher::classify_bytes))
    }

    /// A missing pointer is absence; a present malformed pointer refuses.
    pub(crate) fn prior_pointer(
        &self,
        relative: &str,
    ) -> Result<Option<String>, BoundVibeBinError> {
        let bytes = self
            .settings()
            .read_file_bounded(relative, 128)
            .map_err(|error| BoundVibeBinError::Destination(format!("{error:#}")))?;
        match bytes {
            None => Ok(None),
            Some(bytes) => launcher::pointer_digest(&bytes)
                .map(Some)
                .ok_or(BoundVibeBinError::InvalidPointer),
        }
    }

    /// Launcher and pointer publication never reopen settings by pathname.
    pub(crate) fn write_resource(
        &self,
        relative: &str,
        bytes: &[u8],
        executable: bool,
    ) -> Result<(), BoundVibeBinError> {
        self.settings()
            .write_atomic_with_mode(relative, bytes, resource_mode(executable))
            .map(|_| ())
            .map_err(|error| {
                let uncertain = error.stage == PublishStage::PossiblyPublished;
                let rendered = format!("{:#}", error.into_report());
                if uncertain {
                    BoundVibeBinError::PublicationUncertain(rendered)
                } else {
                    BoundVibeBinError::Destination(rendered)
                }
            })
    }

    pub(crate) fn state(
        &self,
        relative: &str,
    ) -> Result<Option<StableFileState>, BoundVibeBinError> {
        self.settings()
            .stable_file_state(relative)
            .map_err(|error| BoundVibeBinError::Destination(format!("{error:#}")))
    }

    pub(crate) fn remove_resource(&self, relative: &str) -> Result<bool, BoundVibeBinError> {
        self.settings()
            .remove_file(relative)
            .map_err(|error| BoundVibeBinError::Destination(format!("{error:#}")))
    }
}

fn matches_payload(
    observed: &StableFileState,
    artifact: &BoundProjectArtifact,
    mode: Option<u32>,
) -> bool {
    observed.sha256 == artifact.digest
        && observed.bytes == artifact.bytes
        && mode.is_none_or(|mode| observed.unix_mode == Some(mode))
}

#[cfg(unix)]
fn executable_mode(flavour: LauncherFlavour) -> Option<u32> {
    flavour.needs_executable_bit().then_some(0o755)
}
#[cfg(not(unix))]
fn executable_mode(_flavour: LauncherFlavour) -> Option<u32> {
    None
}

#[cfg(unix)]
fn resource_mode(executable: bool) -> Option<u32> {
    executable.then_some(0o755)
}
#[cfg(not(unix))]
fn resource_mode(_executable: bool) -> Option<u32> {
    None
}

#[cfg(test)]
#[path = "bound_tests.rs"]
mod tests;
