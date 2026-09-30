//! Pinned I/O for a host-admitted local UserProfile vibe-bin operation.
//!
//! A caller retains the physical-destination cooperative lock for this
//! value's whole lifetime. This cell never opens a root from a pathname and
//! grants nothing from the artifact record or wire binding by itself.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-076#ENVIRONMENT-BOUND-BUILTIN");

use vibe_safefs::{FileIdentity, LockGuard, Pinned, Project, PublishStage, StableFileState};
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
    #[error(
        "bound lock setup may have created {created:?} infrastructure before {phase} refused: {reason}"
    )]
    LockSetupPartial {
        phase: &'static str,
        created: Vec<&'static str>,
        reason: String,
    },
    #[error("declared {0} child no longer names its locked physical directory")]
    ChildDrift(&'static str),
    #[error(
        "bound bin/store or lock infrastructure share one physical directory; distinct logical resource roles would alias (setup may have created {created:?})"
    )]
    AliasedChildRoles { created: Vec<&'static str> },
    #[error("bound vibe-bin resource is outside its admitted bin/store child")]
    UnadmittedResource,
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

    pub(crate) fn matches_resolved(&self, relative: &str, digest: &str, bytes: u64) -> bool {
        self.relative == relative && self.digest == digest && self.bytes == bytes
    }
}

/// The destination capability and its lock are one value. A caller cannot
/// accidentally pass a lock from an unrelated directory to bound I/O.
pub(crate) struct BoundDestinationLock {
    settings: Project,
    bin: Project,
    store: Project,
    bin_identity: FileIdentity,
    store_identity: FileIdentity,
    _guards: Vec<LockGuard>,
}

impl BoundDestinationLock {
    pub(crate) fn acquire(settings: Project) -> Result<Self, BoundVibeBinError> {
        let mut created = Vec::new();
        let root = settings
            .root_dir()
            .map_err(|error| BoundVibeBinError::Destination(format!("{error:#}")))?;
        let (lock_home, made) = root
            .ensure_child_recording(".vibe")
            .map_err(|error| setup_error("root lock home", &created, format!("{error:#}")))?;
        if made {
            created.push(".vibe");
        }
        let mut seen = Vec::new();
        let mut guards = Vec::new();
        let lock_home_identity =
            acquire_child(&lock_home, "settings", &created, &mut seen, &mut guards)?;
        // Fixed semantic order plus nonblocking acquisition: even if one
        // process's bin is another's store, neither waits while holding its
        // first child lock. Equal cross-role physical identities refuse.
        let (bin_pin, made) = root
            .ensure_child_recording(store::BIN_DIR)
            .map_err(|error| setup_error("bin child", &created, format!("{error:#}")))?;
        if made {
            created.push("bin");
        }
        if bin_pin
            .identity()
            .map_err(|error| setup_error("bin identity", &created, format!("{error:#}")))?
            == lock_home_identity
        {
            return Err(BoundVibeBinError::AliasedChildRoles { created });
        }
        let bin_identity = acquire_child(&bin_pin, "bin", &created, &mut seen, &mut guards)?;
        let (store_pin, made) = root
            .ensure_child_recording(store::STORE_DIR)
            .map_err(|error| setup_error("store child", &created, format!("{error:#}")))?;
        if made {
            created.push("store");
        }
        let observed_store = store_pin
            .identity()
            .map_err(|error| setup_error("store identity", &created, format!("{error:#}")))?;
        if observed_store == lock_home_identity || observed_store == bin_identity {
            return Err(BoundVibeBinError::AliasedChildRoles { created });
        }
        let store_identity = acquire_child(&store_pin, "store", &created, &mut seen, &mut guards)?;
        let bin = settings
            .subproject(&[store::BIN_DIR])
            .map_err(|error| setup_error("bin pin", &created, format!("{error:#}")))?;
        let store = settings
            .subproject(&[store::STORE_DIR])
            .map_err(|error| setup_error("store pin", &created, format!("{error:#}")))?;
        if bin
            .root_identity()
            .map_err(|error| setup_error("bin identity", &created, format!("{error:#}")))?
            != bin_identity
        {
            return Err(BoundVibeBinError::ChildDrift("bin"));
        }
        if store
            .root_identity()
            .map_err(|error| setup_error("store identity", &created, format!("{error:#}")))?
            != store_identity
        {
            return Err(BoundVibeBinError::ChildDrift("store"));
        }
        Ok(Self {
            settings,
            bin,
            store,
            bin_identity,
            store_identity,
            _guards: guards,
        })
    }

    pub(crate) fn settings(&self) -> &Project {
        &self.settings
    }

    pub(crate) fn validate_children(&self) -> Result<(), BoundVibeBinError> {
        for (name, wanted) in [
            (store::BIN_DIR, self.bin_identity),
            (store::STORE_DIR, self.store_identity),
        ] {
            let observed = self
                .settings
                .dir_if_present(&[name])
                .map_err(|error| BoundVibeBinError::Destination(format!("{error:#}")))?
                .ok_or(BoundVibeBinError::ChildDrift(name))?;
            if observed
                .identity()
                .map_err(|error| BoundVibeBinError::Destination(format!("{error:#}")))?
                != wanted
            {
                return Err(BoundVibeBinError::ChildDrift(name));
            }
        }
        Ok(())
    }

    fn child<'a>(&'a self, relative: &'a str) -> Result<(&'a Project, &'a str), BoundVibeBinError> {
        self.validate_children()?;
        if let Some(name) = direct_child(relative, "bin/") {
            return Ok((&self.bin, name));
        }
        if let Some(name) = direct_child(relative, "store/") {
            return Ok((&self.store, name));
        }
        Err(BoundVibeBinError::UnadmittedResource)
    }

    fn bin_child<'a>(
        &'a self,
        relative: &'a str,
    ) -> Result<(&'a Project, &'a str), BoundVibeBinError> {
        self.validate_children()?;
        direct_child(relative, "bin/")
            .map(|name| (&self.bin, name))
            .ok_or(BoundVibeBinError::UnadmittedResource)
    }

    fn store_child(&self) -> Result<&Project, BoundVibeBinError> {
        self.validate_children()?;
        Ok(&self.store)
    }
}

const PHYSICAL_LOCK: &str = "bound-vibe-bin-physical.lock";

fn acquire_child(
    pinned: &Pinned,
    phase: &'static str,
    created: &[&'static str],
    seen: &mut Vec<FileIdentity>,
    guards: &mut Vec<LockGuard>,
) -> Result<FileIdentity, BoundVibeBinError> {
    let identity = pinned
        .identity()
        .map_err(|error| setup_error(phase, created, format!("{error:#}")))?;
    if seen.contains(&identity) {
        return Ok(identity);
    }
    let guard = pinned
        .try_lock_file(PHYSICAL_LOCK)
        .map_err(|error| setup_error(phase, created, format!("{error:#}")))?
        .ok_or_else(|| {
            if created.is_empty() {
                BoundVibeBinError::LockBusy
            } else {
                setup_error(
                    phase,
                    created,
                    "another cooperating writer holds this child".into(),
                )
            }
        })?;
    seen.push(identity);
    guards.push(guard);
    Ok(identity)
}

fn setup_error(phase: &'static str, created: &[&'static str], reason: String) -> BoundVibeBinError {
    if created.is_empty() {
        BoundVibeBinError::Destination(reason)
    } else {
        BoundVibeBinError::LockSetupPartial {
            phase,
            created: created.to_vec(),
            reason,
        }
    }
}

fn direct_child<'a>(relative: &'a str, prefix: &str) -> Option<&'a str> {
    let tail = relative.strip_prefix(prefix)?;
    (!tail.is_empty() && !tail.contains('/')).then_some(tail)
}

// Test seam between one held read and its final named-child check. Shipped
// builds have no hook and inspect no ambient process state here.
#[cfg(test)]
thread_local! {
    static AFTER_READ: std::cell::RefCell<Option<Box<dyn FnOnce()>>> =
        std::cell::RefCell::new(None);
}

#[cfg(test)]
pub(super) fn arm_after_read(hook: Option<Box<dyn FnOnce()>>) {
    AFTER_READ.with(|slot| *slot.borrow_mut() = hook);
}

#[cfg(test)]
fn after_read_observation() {
    AFTER_READ.with(|slot| {
        if let Some(hook) = slot.borrow_mut().take() {
            hook();
        }
    });
}

#[cfg(not(test))]
const fn after_read_observation() {}

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
        let filename =
            direct_child(&relative, "store/").ok_or(BoundVibeBinError::UnadmittedResource)?;
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
                self.destination.store_child()?,
                filename,
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
                self.destination.validate_children().map_err(|error| {
                    BoundVibeBinError::PublicationUncertain(format!(
                        "published CAS payload but named child changed: {error}"
                    ))
                })?;
                Ok(PayloadPlacement::Written)
            }
            _ => Err(BoundVibeBinError::PublicationUncertain(
                "published CAS occupant did not reobserve at expected bytes and mode".into(),
            )),
        }
    }

    /// Bounded launcher classification through the same pinned settings root.
    pub(crate) fn classify_launcher(&self, relative: &str) -> Result<Occupant, BoundVibeBinError> {
        let (bin, name) = self.destination.bin_child(relative)?;
        let bytes = bin
            .read_file_bounded(name, 64 * 1024)
            .map_err(|error| BoundVibeBinError::Destination(format!("{error:#}")))?;
        after_read_observation();
        self.destination.validate_children()?;
        Ok(bytes
            .as_deref()
            .map_or(Occupant::Vacant, launcher::classify_bytes))
    }

    /// A missing pointer is absence; a present malformed pointer refuses.
    pub(crate) fn prior_pointer(
        &self,
        relative: &str,
    ) -> Result<Option<String>, BoundVibeBinError> {
        let (bin, name) = self.destination.bin_child(relative)?;
        let bytes = bin
            .read_file_bounded(name, 128)
            .map_err(|error| BoundVibeBinError::Destination(format!("{error:#}")))?;
        after_read_observation();
        self.destination.validate_children()?;
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
        let (bin, name) = self.destination.bin_child(relative)?;
        bin.write_atomic_with_mode(name, bytes, resource_mode(executable))
            .map(|_| ())
            .map_err(|error| {
                let uncertain = error.stage == PublishStage::PossiblyPublished;
                let rendered = format!("{:#}", error.into_report());
                if uncertain {
                    BoundVibeBinError::PublicationUncertain(rendered)
                } else {
                    BoundVibeBinError::Destination(rendered)
                }
            })?;
        self.destination.validate_children().map_err(|error| {
            BoundVibeBinError::PublicationUncertain(format!(
                "published bin resource but child path drifted: {error}"
            ))
        })
    }

    pub(crate) fn state(
        &self,
        relative: &str,
    ) -> Result<Option<StableFileState>, BoundVibeBinError> {
        let (child, name) = self.destination.child(relative)?;
        let observed = child
            .stable_file_state(name)
            .map_err(|error| BoundVibeBinError::Destination(format!("{error:#}")))?;
        after_read_observation();
        self.destination.validate_children()?;
        Ok(observed)
    }

    pub(crate) fn remove_resource(&self, relative: &str) -> Result<bool, BoundVibeBinError> {
        let (bin, name) = self.destination.bin_child(relative)?;
        let removed = bin
            .remove_file(name)
            .map_err(|error| BoundVibeBinError::Destination(format!("{error:#}")))?;
        if removed {
            self.destination.validate_children().map_err(|error| {
                BoundVibeBinError::PublicationUncertain(format!(
                    "removed bin resource but child path drifted: {error}"
                ))
            })?;
        }
        Ok(removed)
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
