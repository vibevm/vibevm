//! The NO-CREATE read view over an existing deployment state home —
//! §6.3.1.5's "Read-only planning truly creates nothing".
//!
//! > "Receipt/sidecar inspection uses a no-create state view;
//! > `DeployState::open` remains apply-only. The same prior receipt value
//! > reaches provider plan in both `--plan` and preapply."
//!
//! Its own cell because "creates nothing" is a property of a whole type, and
//! a type that is separate is a property a reader can check. [`DeployState`]
//! next door creates its root on purpose — a deployment's first act is
//! writing an intent, and a home that had to exist beforehand would make the
//! first deploy on a machine fail for a reason no manifest can fix. That
//! reasoning is exactly wrong for a planner, which is why the two are two
//! values rather than one value with a flag: a flag can be passed wrongly,
//! and a missing capability cannot.
//!
//! An ABSENT root is a VALUE here, not a fault: nothing was ever deployed on
//! this machine, so there is no receipt and no sidecar, and reporting that
//! is the honest answer. A root that is present and unusable — a link, a
//! file, malformed JSON — still refuses through the same typed state errors
//! [`DeployState`] raises, because a planner that quietly reported "no prior
//! ownership" over a broken state home would promise a deployment apply is
//! required to refuse.
//!
//! [`DeployState`]: super::state::DeployState

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS");

use std::path::Path;

use vibe_safefs::Project;
use vibe_wire::generated::deploy_receipt::DeployReceipt;

use super::error::DeployError;
use super::sidecar::LockResources;
use super::state::{DeploymentHome, RECEIPT_FILE, checked_receipt, read_record, rendered};

/// A read-only capability over a deployment state home that may not exist.
///
/// `None` is "the root is not there", which is why the field is an option
/// rather than the type being one: every read below answers `Ok(None)`
/// against it, and no caller has to branch on whether a machine has ever
/// deployed anything.
#[derive(Debug)]
pub(crate) struct DeployStateView {
    project: Option<Project>,
}

impl DeployStateView {
    /// Pin an EXISTING state home, or record that there is none.
    ///
    /// The metadata probe is deliberately `symlink_metadata`: a link at the
    /// state home is refused rather than followed, the same posture the
    /// staging directory and the receipt walk already take, because a state
    /// home is not a publication surface.
    pub(crate) fn open(root: &Path) -> Result<Self, DeployError> {
        let refuse = |reason: String| DeployError::StateHome {
            path: rendered(root),
            reason,
        };
        match std::fs::symlink_metadata(root) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self { project: None });
            }
            Err(error) => return Err(refuse(error.to_string())),
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(refuse(
                    "a link occupies the deployment state home; a read view never follows one"
                        .to_owned(),
                ));
            }
            Ok(metadata) if !metadata.is_dir() => {
                return Err(refuse(
                    "the deployment state home is not a directory".to_owned(),
                ));
            }
            Ok(_) => {}
        }
        let project = Project::open(root).map_err(|error| refuse(format!("{error:#}")))?;
        Ok(Self {
            project: Some(project),
        })
    }

    /// One deployment's finalised receipt, or `None` when there is none —
    /// including when the whole state home is absent.
    pub(crate) fn read_receipt(
        &self,
        home: &DeploymentHome,
    ) -> Result<Option<DeployReceipt>, DeployError> {
        let Some(project) = self.project.as_ref() else {
            return Ok(None);
        };
        let Some(receipt) = read_record::<DeployReceipt>(project, &home.member(RECEIPT_FILE))?
        else {
            return Ok(None);
        };
        checked_receipt(receipt).map(Some)
    }

    /// One deployment's durable lock sidecar, or `None` when there is none.
    ///
    /// Unread by the planner today and read by it the moment a reference
    /// owner ships: the view answers both questions §6.3.1.5 names
    /// ("receipt/sidecar inspection uses a no-create state view") from the
    /// same capability, so the two can never disagree about what a state
    /// home holds.
    #[allow(
        dead_code,
        reason = "§6.3.1.5's sidecar half of the no-create view; the shipped reader is the first \
                  reference-owning client provider, and the law is proven at this seam today"
    )]
    pub(crate) fn read_lock_resources(
        &self,
        home: &DeploymentHome,
    ) -> Result<Option<LockResources>, DeployError> {
        let Some(project) = self.project.as_ref() else {
            return Ok(None);
        };
        let Some(record) = read_record::<LockResources>(
            project,
            &home.member(super::sidecar::LOCK_RESOURCES_FILE),
        )?
        else {
            return Ok(None);
        };
        record.validate()?;
        Ok(Some(record))
    }
}
