//! The engine-owned deployment state home — §7.0.3's "`state/deployments/`
//! under the `vibe_core::settings` directory".
//!
//! Three decisions shape this cell and none of them is a provider's:
//!
//! 1. **The home is a PARAMETER, never a read.** Nothing here calls
//!    `settings_dir()`. The command layer resolves the settings directory
//!    once and hands the absolute root down, exactly as it hands the build
//!    and package roots down. That is what makes the operator's real home
//!    unreachable from a unit test *by construction* rather than by an
//!    environment variable a test could forget to set.
//! 2. **The layout inside the home is engine-owned and disclosed here**
//!    (§3.2 gives the engine state persistence; §7.0.3 says the layout is
//!    the implementation's to choose and to disclose):
//!
//!    ```text
//!    <home>/<deployment-id>/intent.json         the §7.2 durable intent
//!    <home>/<deployment-id>/inverse.json        an in-progress saga inverse
//!    <home>/<deployment-id>/checkpoints.json    the apply checkpoint ledger
//!    <home>/<deployment-id>/lock-resources.json the §6.3.1.2 lock sidecar
//!    <home>/<deployment-id>/receipt.json        the §7.2 finalized receipt
//!    <home>/<deployment-id>/staging/            engine-owned staging scratch
//!    <home>/.vibe/deployment-<deployment-id>.lock the state lock
//!    <home>/.vibe/<destination-id>.lock         the per-destination locks
//!    ```
//!
//!    `<deployment-id>` is the SHA-256 of `project\0package\0target\0`, and
//!    not a rendered identity: a project identity is arbitrary text while a
//!    directory name is not, and a name that had to be escaped would be a
//!    second grammar nobody asked for. The receipt inside carries the
//!    readable identity, which is what `vibe deployments` prints.
//!    `<destination-id>` is the SHA-256 of the resource identity, so the
//!    lock really is per DESTINATION and two deployments that touch one
//!    resource contend on one file.
//!
//!    The `.vibe/` component under the home is not a project marker: it is
//!    where `vibe_safefs::Project::lock` puts its lock files, and reusing
//!    that audited primitive — with its post-lock identity recheck — is
//!    worth more than a prettier path. This cell adds no second lock
//!    implementation.
//! 3. **Every durable byte is published atomically through the capability.**
//!    The intent must be on disk *before* the first external write (§7.2),
//!    and "atomically" there is not a figure of speech.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS");

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use vibe_safefs::{LockGuard, Project};
use vibe_wire::behaviour::deploy_records::{validate_intent, validate_receipt};
use vibe_wire::generated::deploy_intent::DeployIntent;
use vibe_wire::generated::deploy_receipt::DeployReceipt;

use super::error::DeployError;
use super::sidecar::{LOCK_RESOURCES_FILE, LockResources};

mod inverse;
pub(crate) use inverse::InverseRecord;

// The provider-facing checkpoint sink lives in its own cell and is
// re-exported here, so every existing `state::CheckpointLedger` use site
// keeps one spelling: the split is a responsibility seam, not a rename.
pub(crate) use super::ledger::CheckpointLedger;

/// The intent journal's file name inside a deployment's own directory.
pub(super) const INTENT_FILE: &str = "intent.json";
/// The checkpoint ledger's file name.
const CHECKPOINT_FILE: &str = "checkpoints.json";
/// The finalized receipt's file name.
pub(super) const RECEIPT_FILE: &str = "receipt.json";
/// The engine-owned staging directory's name.
const STAGING_DIR: &str = "staging";
/// The stable per-deployment state lock's file-name prefix.
///
/// Prefixed rather than bare so it cannot collide with a per-destination
/// lock, whose name is a bare 64-hex digest of a resource identity — the
/// deployment id is 64 hex too, and two different locks sharing one name
/// would silently serialise a deployment against a destination.
const DEPLOYMENT_LOCK_PREFIX: &str = "deployment-";

/// The checkpoint ledger's schema epoch.
pub(crate) const CHECKPOINT_EPOCH: u32 = 1;

/// The durable record of which planned operations apply already completed.
///
/// §7.2: "Apply checkpoints completed operations without storing secrets."
/// It is an ENGINE-owned sidecar with its own schema epoch rather than a
/// member of the intent journal, because the intent's wire shape is frozen
/// (A2, `deny_unknown_fields`) and a checkpoint is not a plan: rewriting
/// the planned-resource list to mean "done" would destroy the exact set
/// recovery compares against.
///
/// `plan_hash` ties the ledger to the intent it belongs to, so a ledger
/// left by an earlier plan cannot be read as progress against this one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CheckpointRecord {
    pub(crate) schema: u32,
    pub(crate) plan_hash: String,
    /// Resource identities whose operation completed, in completion order.
    pub(crate) completed: Vec<String>,
}

/// One deployment's own directory inside the state home.
///
/// Cheap to build (it computes one digest and joins paths) and it opens
/// nothing: a value that named a live capability would make "does this
/// deployment exist" a question with a filesystem answer.
#[derive(Debug, Clone)]
pub(crate) struct DeploymentHome {
    /// The absolute state-home root, as the command layer resolved it.
    root: PathBuf,
    /// The 64-hex deployment id — this deployment's directory name.
    id: String,
}

impl DeploymentHome {
    /// The deployment identity of one project/package/target triple.
    pub(crate) fn new(root: &Path, project: &str, package: Option<&str>, target: &str) -> Self {
        let mut hash = Sha256::new();
        hash.update(project.as_bytes());
        hash.update(b"\x00");
        hash.update(package.unwrap_or("").as_bytes());
        hash.update(b"\x00");
        hash.update(target.as_bytes());
        hash.update(b"\x00");
        Self {
            root: root.to_path_buf(),
            id: format!("{:x}", hash.finalize()),
        }
    }

    /// The deployment id — the directory name, and the join key a reader
    /// of the state home sees.
    pub(crate) fn id(&self) -> &str {
        &self.id
    }

    /// The absolute directory of this deployment.
    pub(crate) fn directory(&self) -> PathBuf {
        self.root.join(&self.id)
    }

    /// The engine-owned staging directory this deployment's apply uses.
    pub(crate) fn staging(&self) -> PathBuf {
        self.directory().join(STAGING_DIR)
    }

    /// The home-relative, forward-slashed spelling of one member file.
    pub(super) fn member(&self, name: &str) -> String {
        format!("{}/{name}", self.id)
    }
}

/// A live capability over the state home, plus the small set of durable
/// operations §7.2 needs. One value so no call site opens a second
/// capability on the same root.
#[derive(Debug)]
pub(crate) struct DeployState {
    project: Project,
    root: PathBuf,
}

impl DeployState {
    /// Create the state home if it is absent, then pin it.
    ///
    /// Creation is the engine's: a deployment's very first act is writing
    /// an intent, and a home that had to exist beforehand would make the
    /// first deploy on a machine fail for a reason no manifest can fix.
    pub(crate) fn open(root: &Path) -> Result<Self, DeployError> {
        std::fs::create_dir_all(root).map_err(|error| DeployError::StateHome {
            path: rendered(root),
            reason: error.to_string(),
        })?;
        let project = Project::open(root).map_err(|error| DeployError::StateHome {
            path: rendered(root),
            reason: format!("{error:#}"),
        })?;
        Ok(Self {
            project,
            root: root.to_path_buf(),
        })
    }

    /// Take the per-destination lock over every resource one plan touches,
    /// in a deterministic order.
    ///
    /// §7.2 asks for "a per-destination lock", and this is literally that:
    /// one lock file per PHYSICAL destination. The acquisition order is the
    /// sorted lock name, which is a TOTAL order over every destination this
    /// engine can name — so two concurrent deployments whose resource sets
    /// overlap can queue but can never deadlock, whatever order their
    /// plans were authored in.
    ///
    /// The lock name is taken over the shared
    /// [`path_identity_key`](vibe_safefs::path_identity_key), never over the
    /// raw spelling, and that is a correctness requirement rather than a
    /// tidiness one. §6.3.0.10's pre-apply judgement already treats
    /// `Shared.json` and `shared.json` as ONE physical destination and
    /// admits two reference owners of it; a lock keyed on the raw bytes
    /// would then hand those two participants two DIFFERENT lock files and
    /// let them edit one document concurrently — the exact race the shared
    /// lock exists to prevent. One identity law, one lock.
    ///
    /// The exact spelling is retained everywhere it is read by a human or
    /// recorded: receipts, intents and refusals all quote what the provider
    /// declared. Only this file NAME is normalised.
    pub(crate) fn lock_destinations(
        &self,
        resources: &[String],
    ) -> Result<Vec<LockGuard>, DeployError> {
        let mut names: Vec<String> = resources
            .iter()
            .map(|resource| {
                format!(
                    "{}.lock",
                    digest_of(&vibe_safefs::path_identity_key(resource))
                )
            })
            .collect();
        names.sort_unstable();
        names.dedup();
        let mut held = Vec::with_capacity(names.len());
        for name in names {
            let guard = self
                .project
                .lock(&name)
                .map_err(|error| DeployError::DestinationLock {
                    path: rendered(&self.root.join(&name)),
                    reason: format!("{error:#}"),
                })?;
            held.push(guard);
        }
        Ok(held)
    }

    /// Take the STABLE per-deployment state lock — §6.3.1.3's "One stable
    /// deployment lock serialises sidecar/state transitions".
    ///
    /// > "Apply, recovery, saga rollback and undeploy take the
    /// > deployment-id lock, then the union of current, committed and
    /// > pending destination locks in canonical order."
    ///
    /// It is keyed on the deployment ID rather than on any resource, so it
    /// is the same lock in every generation and for every plan: the
    /// sidecar, the intent and the receipt are one deployment's records, and
    /// a lock that moved with the plan would leave two runs of the same
    /// deployment free to interleave their read-modify-write pairs over
    /// them. Taken FIRST, always, so the acquisition order over the two
    /// families is total and no pair of runs can deadlock.
    pub(crate) fn lock_deployment(&self, home: &DeploymentHome) -> Result<LockGuard, DeployError> {
        let name = format!("{DEPLOYMENT_LOCK_PREFIX}{}.lock", home.id());
        self.project
            .lock(&name)
            .map_err(|error| DeployError::DeploymentLock {
                path: rendered(&self.root.join(&name)),
                reason: format!("{error:#}"),
            })
    }

    /// Read one deployment's durable lock sidecar, or `None` when it has
    /// none — §6.3.1.2's record, validated before a caller may act on it.
    pub(crate) fn read_lock_resources(
        &self,
        home: &DeploymentHome,
    ) -> Result<Option<LockResources>, DeployError> {
        let Some(record) = self.read::<LockResources>(&home.member(LOCK_RESOURCES_FILE))? else {
            return Ok(None);
        };
        record.validate()?;
        Ok(Some(record))
    }

    /// Publish one deployment's lock sidecar atomically, then READ IT BACK
    /// and validate what is really on disk.
    ///
    /// §6.3.1.2 makes the pending binding durable before the intent and
    /// therefore before the first external write — which is a promise about
    /// the bytes on the disk, not about the value in this process. So the
    /// record is proven twice: once as a value, and once as whatever a later
    /// run would actually read. A publication that encoded, wrote and
    /// returned would be trusting exactly the step the law exists to cover.
    pub(crate) fn write_lock_resources(
        &self,
        home: &DeploymentHome,
        record: &LockResources,
    ) -> Result<(), DeployError> {
        record.validate()?;
        self.publish(&home.member(LOCK_RESOURCES_FILE), record)?;
        let read_back = self.read_lock_resources(home)?;
        if read_back.as_ref() != Some(record) {
            return Err(DeployError::RecordInvalid {
                record: LOCK_RESOURCES_FILE,
                reason: "the published record did not read back as the record that was written"
                    .to_owned(),
            });
        }
        Ok(())
    }

    /// Publish one deployment's intent journal atomically.
    ///
    /// Validated through the A2 behaviour cell BEFORE any byte reaches the
    /// filesystem, for the reason the artifact-record cell states: an
    /// invalid record is a defect in its producer, and the honest place to
    /// find it is the producer's own refusal.
    pub(crate) fn write_intent(
        &self,
        home: &DeploymentHome,
        intent: &DeployIntent,
    ) -> Result<(), DeployError> {
        validate_intent(intent).map_err(|error| DeployError::RecordInvalid {
            record: INTENT_FILE,
            reason: error.to_string(),
        })?;
        self.publish(&home.member(INTENT_FILE), intent)
    }

    /// Read back one deployment's intent, or `None` when it retired.
    pub(crate) fn read_intent(
        &self,
        home: &DeploymentHome,
    ) -> Result<Option<DeployIntent>, DeployError> {
        let Some(intent) = self.read::<DeployIntent>(&home.member(INTENT_FILE))? else {
            return Ok(None);
        };
        validate_intent(&intent).map_err(|error| DeployError::RecordInvalid {
            record: INTENT_FILE,
            reason: error.to_string(),
        })?;
        Ok(Some(intent))
    }

    /// Retire one deployment's intent — §7.2's last step.
    pub(crate) fn retire_intent(&self, home: &DeploymentHome) -> Result<(), DeployError> {
        self.remove(&home.member(INTENT_FILE))?;
        // The ledger belongs to the intent, not to the receipt: leaving it
        // behind would let the next plan's recovery read a completed set
        // that describes a deployment that already finished.
        self.remove(&home.member(CHECKPOINT_FILE))
    }

    /// Publish one deployment's receipt atomically.
    pub(crate) fn write_receipt(
        &self,
        home: &DeploymentHome,
        receipt: &DeployReceipt,
    ) -> Result<(), DeployError> {
        validate_receipt(receipt).map_err(|error| DeployError::RecordInvalid {
            record: RECEIPT_FILE,
            reason: error.to_string(),
        })?;
        self.publish(&home.member(RECEIPT_FILE), receipt)
    }

    /// Read back one deployment's receipt, or `None` when it never applied.
    pub(crate) fn read_receipt(
        &self,
        home: &DeploymentHome,
    ) -> Result<Option<DeployReceipt>, DeployError> {
        let Some(receipt) = self.read::<DeployReceipt>(&home.member(RECEIPT_FILE))? else {
            return Ok(None);
        };
        checked_receipt(receipt).map(Some)
    }

    /// Read one deployment's checkpoint ledger for a given plan, or `None`
    /// when there is none for THAT plan.
    pub(crate) fn read_checkpoints(
        &self,
        home: &DeploymentHome,
        plan_hash: &str,
    ) -> Result<Option<CheckpointRecord>, DeployError> {
        let Some(record) = self.read::<CheckpointRecord>(&home.member(CHECKPOINT_FILE))? else {
            return Ok(None);
        };
        if record.schema != CHECKPOINT_EPOCH {
            return Err(DeployError::RecordInvalid {
                record: CHECKPOINT_FILE,
                reason: format!(
                    "schema epoch {} is not the {CHECKPOINT_EPOCH} this engine writes",
                    record.schema
                ),
            });
        }
        if record.plan_hash != plan_hash {
            return Ok(None);
        }
        Ok(Some(record))
    }

    /// Publish a checkpoint ledger atomically.
    pub(crate) fn write_checkpoints(
        &self,
        home: &DeploymentHome,
        record: &CheckpointRecord,
    ) -> Result<(), DeployError> {
        self.publish(&home.member(CHECKPOINT_FILE), record)
    }

    /// Empty and recreate one deployment's staging directory.
    ///
    /// The same law the package executor's output directory has, and for
    /// the same reason: a leftover file from an interrupted apply that
    /// entered a fresh staging tree would be published as this run's work.
    pub(crate) fn prepare_staging(&self, home: &DeploymentHome) -> Result<PathBuf, DeployError> {
        let staging = home.staging();
        let refuse = |reason: String| DeployError::StateHome {
            path: rendered(&staging),
            reason,
        };
        let relative = home.member(STAGING_DIR);
        let keep_rollback = self.read_inverse(home)?.is_some();
        if self.read_intent(home)?.is_some() || keep_rollback {
            self.project
                .dir(&[home.id(), STAGING_DIR], true)
                .map_err(|error| refuse(format!("{error:#}")))?;
        } else {
            self.project
                .reset_dir(&relative)
                .map_err(|error| refuse(format!("{error:#}")))?;
        }
        Ok(staging)
    }

    /// Clear rollback/staging bytes after the whole selected deploy finished.
    /// A live intent deliberately keeps them for recovery instead.
    pub(crate) fn cleanup_staging(&self, home: &DeploymentHome) -> Result<(), DeployError> {
        let staging = home.staging();
        if self.read_intent(home)?.is_some() || self.read_inverse(home)?.is_some() {
            return Ok(());
        }
        self.project
            .reset_dir(&home.member(STAGING_DIR))
            .map(|_| ())
            .map_err(|error| DeployError::StateHome {
                path: rendered(&staging),
                reason: format!("{error:#}"),
            })
    }

    /// Every receipt this state home holds, in deployment-id order.
    ///
    /// The walk is the engine's own `std::fs` read of a directory it owns,
    /// with links refused rather than followed — the same posture the
    /// containment cell states for build artifacts, and for the same
    /// reason: a state home is not a publication surface.
    pub(crate) fn receipts(&self) -> Result<Vec<(String, DeployReceipt)>, DeployError> {
        let listing = match std::fs::read_dir(&self.root) {
            Ok(listing) => listing,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => {
                return Err(DeployError::StateHome {
                    path: rendered(&self.root),
                    reason: error.to_string(),
                });
            }
        };
        let mut ids: Vec<String> = Vec::new();
        for entry in listing {
            let entry = entry.map_err(|error| DeployError::StateHome {
                path: rendered(&self.root),
                reason: error.to_string(),
            })?;
            let metadata = entry.metadata().map_err(|error| DeployError::StateHome {
                path: rendered(&entry.path()),
                reason: error.to_string(),
            })?;
            if !metadata.is_dir() {
                continue;
            }
            let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
                continue;
            };
            // The lock directory is infrastructure, not a deployment.
            if name.starts_with('.') {
                continue;
            }
            ids.push(name);
        }
        ids.sort_unstable();
        let mut rows = Vec::with_capacity(ids.len());
        for id in ids {
            let relative = format!("{id}/{RECEIPT_FILE}");
            let Some(receipt) = self.read::<DeployReceipt>(&relative)? else {
                continue;
            };
            rows.push((id, checked_receipt(receipt)?));
        }
        Ok(rows)
    }

    /// Encode and publish one JSON record atomically.
    fn publish<T: Serialize>(&self, relative: &str, value: &T) -> Result<(), DeployError> {
        let bytes = serde_json::to_vec_pretty(value).map_err(|error| DeployError::StateWrite {
            path: relative.to_owned(),
            reason: error.to_string(),
        })?;
        self.project
            .write_atomic(relative, &bytes)
            .map_err(|error| DeployError::StateWrite {
                path: relative.to_owned(),
                reason: format!("{:#}", error.into_report()),
            })?;
        Ok(())
    }

    /// Read and decode one JSON record, or `None` when it is not there.
    fn read<T: for<'de> Deserialize<'de>>(&self, relative: &str) -> Result<Option<T>, DeployError> {
        read_record(&self.project, relative)
    }

    /// Remove one state file; absence is success.
    fn remove(&self, relative: &str) -> Result<(), DeployError> {
        let root = self
            .project
            .root_dir()
            .map_err(|error| DeployError::StateWrite {
                path: relative.to_owned(),
                reason: format!("{error:#}"),
            })?;
        self.project
            .remove_file_in(&root, relative)
            .map_err(|error| DeployError::StateWrite {
                path: relative.to_owned(),
                reason: format!("{error:#}"),
            })?;
        Ok(())
    }
}

/// Read and decode one JSON record through a pinned capability, or `None`
/// when it is not there.
///
/// A free function rather than a method because BOTH state capabilities read
/// the same records: the creating [`DeployState`] and the no-create
/// [`DeployStateView`](super::view::DeployStateView). One decoder, one
/// refusal wording, no chance of a planner and an apply disagreeing about
/// what a state home holds.
pub(super) fn read_record<T: for<'de> Deserialize<'de>>(
    project: &Project,
    relative: &str,
) -> Result<Option<T>, DeployError> {
    let Some(bytes) = project
        .read_file(relative)
        .map_err(|error| DeployError::StateRead {
            path: relative.to_owned(),
            reason: format!("{error:#}"),
        })?
    else {
        return Ok(None);
    };
    let value = serde_json::from_slice(&bytes).map_err(|error| DeployError::StateRead {
        path: relative.to_owned(),
        reason: error.to_string(),
    })?;
    Ok(Some(value))
}

/// One decoded receipt, held to the A2 behaviour cell's laws before any
/// reader may act on it.
pub(super) fn checked_receipt(receipt: DeployReceipt) -> Result<DeployReceipt, DeployError> {
    validate_receipt(&receipt).map_err(|error| DeployError::RecordInvalid {
        record: RECEIPT_FILE,
        reason: error.to_string(),
    })?;
    Ok(receipt)
}

/// The stable id of one destination — the lock file's name stem.
fn digest_of(resource: &str) -> String {
    let mut hash = Sha256::new();
    hash.update(resource.as_bytes());
    format!("{:x}", hash.finalize())
}

/// One path in the forward-slashed spelling every refusal quotes.
pub(super) fn rendered(path: &Path) -> String {
    path.display().to_string().replace('\\', "/")
}
