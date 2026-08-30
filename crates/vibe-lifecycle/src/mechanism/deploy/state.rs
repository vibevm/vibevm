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
//!    <home>/<deployment-id>/intent.json       the §7.2 durable intent
//!    <home>/<deployment-id>/checkpoints.json  the apply checkpoint ledger
//!    <home>/<deployment-id>/receipt.json      the §7.2 finalized receipt
//!    <home>/<deployment-id>/staging/          engine-owned staging scratch
//!    <home>/.vibe/<destination-id>.lock       the per-destination locks
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
use crate::mechanism::MechanismError;

/// The intent journal's file name inside a deployment's own directory.
const INTENT_FILE: &str = "intent.json";
/// The checkpoint ledger's file name.
const CHECKPOINT_FILE: &str = "checkpoints.json";
/// The finalized receipt's file name.
const RECEIPT_FILE: &str = "receipt.json";
/// The engine-owned staging directory's name.
const STAGING_DIR: &str = "staging";

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
    fn member(&self, name: &str) -> String {
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
        validate_receipt(&receipt).map_err(|error| DeployError::RecordInvalid {
            record: RECEIPT_FILE,
            reason: error.to_string(),
        })?;
        Ok(Some(receipt))
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
        match std::fs::symlink_metadata(&staging) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(refuse(
                    "a link occupies the staging path; the engine never removes a tree through a \
                     link"
                        .to_owned(),
                ));
            }
            Ok(metadata) if metadata.is_dir() => {
                std::fs::remove_dir_all(&staging).map_err(|error| refuse(error.to_string()))?;
            }
            Ok(_) => {
                std::fs::remove_file(&staging).map_err(|error| refuse(error.to_string()))?;
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(refuse(error.to_string())),
        }
        std::fs::create_dir_all(&staging).map_err(|error| refuse(error.to_string()))?;
        Ok(staging)
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
            validate_receipt(&receipt).map_err(|error| DeployError::RecordInvalid {
                record: RECEIPT_FILE,
                reason: error.to_string(),
            })?;
            rows.push((id, receipt));
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
        let Some(bytes) =
            self.project
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

/// The engine's checkpoint sink, as a provider sees it.
///
/// A provider can say "this operation completed" and can say nothing else:
/// it cannot read the ledger back, cannot rewrite it, and cannot decide
/// where it lives. Every call publishes the ledger atomically before it
/// returns, because a checkpoint that is only in memory is not a
/// checkpoint.
#[derive(Debug)]
pub(crate) struct CheckpointLedger<'a> {
    state: &'a DeployState,
    home: &'a DeploymentHome,
    record: CheckpointRecord,
}

impl<'a> CheckpointLedger<'a> {
    /// Open a ledger for one plan, adopting whatever an interrupted apply
    /// of the SAME plan already completed.
    pub(crate) fn open(
        state: &'a DeployState,
        home: &'a DeploymentHome,
        plan_hash: &str,
    ) -> Result<Self, DeployError> {
        let record = state
            .read_checkpoints(home, plan_hash)?
            .unwrap_or_else(|| CheckpointRecord {
                schema: CHECKPOINT_EPOCH,
                plan_hash: plan_hash.to_owned(),
                completed: Vec::new(),
            });
        Ok(Self {
            state,
            home,
            record,
        })
    }

    /// Record that one completed operation completed.
    ///
    /// The provider-facing half of the ledger. Its callers name the
    /// operation, not necessarily a receipted resource: the vibe-bin
    /// provider checkpoints its content-addressed payload write under the
    /// payload's own store identity even though §7.1.0 ruling 4 keeps the
    /// payload out of the receipt's OWNED set. §7.2 asks apply to
    /// "checkpoint completed operations", and the payload write is one.
    pub(crate) fn completed(&mut self, resource: &str) -> Result<(), MechanismError> {
        if self.record.completed.iter().any(|done| done == resource) {
            return Ok(());
        }
        self.record.completed.push(resource.to_owned());
        self.state
            .write_checkpoints(self.home, &self.record)
            .map_err(|error| MechanismError::DeployCheckpoint {
                resource: resource.to_owned(),
                reason: error.to_string(),
            })
    }
}

/// The stable id of one destination — the lock file's name stem.
fn digest_of(resource: &str) -> String {
    let mut hash = Sha256::new();
    hash.update(resource.as_bytes());
    format!("{:x}", hash.finalize())
}

/// One path in the forward-slashed spelling every refusal quotes.
fn rendered(path: &Path) -> String {
    path.display().to_string().replace('\\', "/")
}
