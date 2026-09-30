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
use vibe_wire::behaviour::deploy_records::validate_intent;
use vibe_wire::generated::deploy_intent::DeployIntent;
use vibe_wire::generated::deploy_receipt::DeployReceipt;

use super::error::DeployError;
use super::model::DEPLOY_STATE_DIR;
use super::sidecar::LockResources;
use super::state::{
    DeploymentHome, INTENT_FILE, RECEIPT_FILE, checked_receipt, read_record, rendered,
};

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

    /// Inspect the state home below an already-held settings capability.
    /// An absent home stays absent and creates nothing. A replacement between
    /// the first no-create observation and the retained subproject refuses.
    pub(crate) fn open_bound(settings: &Project) -> Result<Self, DeployError> {
        let root = settings.root_path().join(DEPLOY_STATE_DIR);
        let components = super::state::bound_state_components();
        let observed =
            settings
                .dir_if_present(&components)
                .map_err(|error| DeployError::StateHome {
                    path: rendered(&root),
                    reason: format!("{error:#}"),
                })?;
        let Some(observed) = observed else {
            return Ok(Self { project: None });
        };
        let project = settings.subproject(&components).map_err(|error| {
            DeployError::BoundStateHomeChanged {
                path: rendered(&root),
                reason: format!("{error:#}"),
            }
        })?;
        let before = observed
            .identity()
            .map_err(|error| DeployError::BoundStateHomeChanged {
                path: rendered(&root),
                reason: format!("{error:#}"),
            })?;
        let held = project
            .root_identity()
            .map_err(|error| DeployError::BoundStateHomeChanged {
                path: rendered(&root),
                reason: format!("{error:#}"),
            })?;
        if before != held {
            return Err(DeployError::BoundStateHomeChanged {
                path: rendered(&root),
                reason: "state home changed between observation and held open".into(),
            });
        }
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
        let receipt = checked_receipt(receipt)?;
        home.ensure_binding(RECEIPT_FILE, receipt.operation_binding.as_ref())?;
        Ok(Some(receipt))
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

    /// One deployment's unretired durable intent, or `None` when it retired
    /// — including when the whole state home is absent.
    ///
    /// Validated through the SAME wire validator and the SAME error
    /// mapping [`DeployState::read_intent`](super::state::DeployState::read_intent)
    /// uses, so a planner and an apply can never disagree about what a
    /// state home holds: the value a provider's `plan` consults as
    /// settlement-reachability evidence is byte-for-byte the journal the
    /// transaction later settles. This read is still the no-create half —
    /// an intent is only ever WRITTEN by an apply, and reading one here
    /// creates nothing.
    pub(crate) fn read_intent(
        &self,
        home: &DeploymentHome,
    ) -> Result<Option<DeployIntent>, DeployError> {
        let Some(project) = self.project.as_ref() else {
            return Ok(None);
        };
        let Some(intent) = read_record::<DeployIntent>(project, &home.member(INTENT_FILE))? else {
            return Ok(None);
        };
        validate_intent(&intent).map_err(|error| DeployError::RecordInvalid {
            record: INTENT_FILE,
            reason: error.to_string(),
        })?;
        home.ensure_binding(INTENT_FILE, intent.operation_binding.as_ref())?;
        Ok(Some(intent))
    }
}
