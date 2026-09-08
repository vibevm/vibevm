//! `vibe-safefs` implementation of the transaction mutation boundary.
//!
//! The adapter deliberately keeps the strong safefs ownership objects alive
//! for the duration of one process.  It never adopts a pre-existing candidate
//! or quarantine merely because its name has the expected spelling.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-056#IMPL-E");

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use sha2::{Digest as _, Sha256};
use vibe_safefs::{
    CleanupIntent as SafefsCleanupIntent, CleanupPreparation as SafefsCleanupPreparation,
    DirectoryDurability, EntryIdentity, EntryState, EntryStateKind, ExistingTreeEntryLease,
    OwnedDirectory, OwnedDirectoryCreateError, OwnedDirectoryIdentity, OwnedTreeCleanupError,
    OwnedTreeCleanupProgress, OwnedTreeObservation as SafefsTreeObservation, OwnedTreePublishError,
    Pinned, Project as SafefsProject, PublishedPendingVerification, RenameError,
    ReopenOwnedDirectoryError, TreeEntry as SafefsTreeEntry, TreeManifest as SafefsTreeManifest,
};

use super::{
    Digest, ExclusiveTreeCreation, ExportEntry, ExportPayload, ExportPlan, ExportTreeSlot,
    FileState, InPlacePlan, Journal, Location, MutationKind, MutationStep, OwnedEntrySeal,
    OwnedRootObservation, OwnedTreeCleanupCompletion, OwnedTreeCleanupIntent,
    OwnedTreeCleanupPreparation, OwnedTreeObservation, OwnedTreeSeal, PathState, PreparedMode,
    PreparedTransaction, SealedObservation, TransactionError, TransactionFilesystem, TreeEntry,
    TreeEntryKind, TreeManifest,
};

/// Production filesystem adapter for one pinned prepared scrape.
///
/// Construct it from the prepared value before giving it to [`super::Engine`].
/// The project identity is rechecked at construction and before every mutation
/// family is entered.  One adapter instance must not be shared by concurrent
/// engines.
#[derive(Debug)]
pub struct SafefsTransactionFilesystem {
    project: SafefsProject,
    project_root: PathBuf,
    project_identity_token: String,
    /// Strong ownership state, indexed by the transaction id encoded in the
    /// engine-generated sibling name.
    live: BTreeMap<String, LiveOwnedTree>,
}

#[derive(Debug)]
struct LiveOwnedTree {
    name: String,
    namespace_name: String,
    owner: String,
    parent_path: PathBuf,
    identity: OwnedDirectoryIdentity,
    manifest: SafefsTreeManifest,
    recovery_stage_path: Option<String>,
    state: LiveTreeState,
}

#[derive(Debug)]
enum LiveTreeState {
    Owned {
        directory: OwnedDirectory,
        lease: ExistingTreeEntryLease,
    },
    /// The owned directory handle remains pinned while descendant leases are
    /// deliberately dropped for one journal-authorized mutation.
    OwnedMutable {
        directory: OwnedDirectory,
    },
    Published(PublishedPendingVerification),
    /// A safefs `PossiblyMoved` result is never guessed into either namespace.
    PossiblyMoved(String),
}

impl SafefsTransactionFilesystem {
    /// Open and identity-bind the project used by a prepared transaction.
    pub fn for_prepared(prepared: &PreparedTransaction) -> Result<Self, TransactionError> {
        Self::open(
            Path::new(&prepared.project_display_root),
            &prepared.project_identity_token,
        )
    }

    /// Open an explicitly trusted project root and require its prepared opaque
    /// identity token.  Non-Windows hosts refuse before any mutation.
    pub fn open(
        project_root: &Path,
        expected_identity_token: &str,
    ) -> Result<Self, TransactionError> {
        ensure_supported()?;
        let project = SafefsProject::open(project_root).map_err(fs_error("opening project"))?;
        let actual = project
            .identity_token()
            .map_err(fs_error("sealing project identity"))?;
        if actual != expected_identity_token {
            return Err(TransactionError::ThirdState(
                "project root identity differs from the prepared transaction".to_owned(),
            ));
        }
        Ok(Self {
            project,
            project_root: project_root.to_path_buf(),
            project_identity_token: actual,
            live: BTreeMap::new(),
        })
    }

    /// Rebind the journaled owned tree after process restart.  The journal's
    /// opaque root identity and complete identity-bearing manifest are
    /// reconstructed through safefs' validating constructors before either
    /// namespace is touched.
    pub fn rebind_from_journal(&mut self, journal: &Journal) -> Result<(), TransactionError> {
        self.require_project()?;
        let Some(owner) = journal.owned_tree_token.as_deref() else {
            return Ok(());
        };
        match &journal.execution {
            PreparedMode::Export(plan) => {
                let Some(candidate_name) = journal.candidate_name.as_deref() else {
                    return Ok(());
                };
                let key = Self::live_key(candidate_name, ".vibe-scrape-candidate-")?;
                if self.live.contains_key(&key) {
                    return Ok(());
                }
                let parent = self.output_parent(plan)?;
                let root = parent
                    .root_dir()
                    .map_err(fs_error("pinning output parent"))?;
                let candidate_present = child_directory_present(&root, candidate_name)?;
                let output_present = child_directory_present(&root, &plan.output_name)?;
                let namespace_name = match (candidate_present, output_present) {
                    (false, false) => return Ok(()),
                    (true, false) => candidate_name,
                    (false, true) => plan.output_name.as_str(),
                    (true, true) => {
                        return Err(TransactionError::ThirdState(
                            "candidate and output are both occupied during recovery".to_owned(),
                        ));
                    }
                };
                let seal = journal.owned_tree_seal.as_ref().ok_or_else(|| {
                    TransactionError::ThirdState(
                        "owned export exists before its first durable identity seal; automatic adoption is forbidden"
                            .to_owned(),
                    )
                })?;
                let authorized_stage = authorized_owned_stage_in_seal(journal, owner, seal)?;
                let (identity, persisted_manifest) =
                    safefs_seal(seal, authorized_stage.as_deref())?;
                let reopened = root
                    .reopen_owned_child_by_identity(namespace_name, owner, &identity)
                    .map_err(map_reopen)?;
                let (directory, lease) = reopened.into_parts();
                let manifest = lease.manifest().clone();
                let recovery_stage_path =
                    recovery_owned_stage_path(journal, owner, &model_manifest(&manifest))?;
                let logical_manifest =
                    model_manifest_without_stage(&manifest, recovery_stage_path.as_deref());
                validate_rebound_manifest(
                    journal,
                    namespace_name,
                    &model_manifest(&persisted_manifest),
                    &logical_manifest,
                )?;
                self.live.insert(
                    key,
                    LiveOwnedTree {
                        name: candidate_name.to_owned(),
                        namespace_name: namespace_name.to_owned(),
                        owner: owner.to_owned(),
                        parent_path: parent.root_path().to_path_buf(),
                        identity,
                        manifest,
                        recovery_stage_path,
                        state: LiveTreeState::Owned { directory, lease },
                    },
                );
            }
            PreparedMode::InPlace(plan) => {
                let Some(quarantine_name) = journal.quarantine_name.as_deref() else {
                    return Ok(());
                };
                let key = Self::live_key(quarantine_name, ".vibe-scrape-quarantine-")?;
                if self.live.contains_key(&key) {
                    return Ok(());
                }
                let parent = self.quarantine_parent(plan)?;
                let root = parent
                    .root_dir()
                    .map_err(fs_error("pinning quarantine parent"))?;
                if !child_directory_present(&root, quarantine_name)? {
                    return Ok(());
                }
                let seal = journal.owned_tree_seal.as_ref().ok_or_else(|| {
                    TransactionError::ThirdState(
                        "owned quarantine exists before its first durable identity seal; automatic adoption is forbidden"
                            .to_owned(),
                    )
                })?;
                let authorized_stage = authorized_owned_stage_in_seal(journal, owner, seal)?;
                let (identity, persisted_manifest) =
                    safefs_seal(seal, authorized_stage.as_deref())?;
                let reopened = root
                    .reopen_owned_child_by_identity(quarantine_name, owner, &identity)
                    .map_err(map_reopen)?;
                let (directory, lease) = reopened.into_parts();
                let manifest = lease.manifest().clone();
                let recovery_stage_path =
                    recovery_owned_stage_path(journal, owner, &model_manifest(&manifest))?;
                let logical_manifest =
                    model_manifest_without_stage(&manifest, recovery_stage_path.as_deref());
                validate_rebound_manifest(
                    journal,
                    quarantine_name,
                    &model_manifest(&persisted_manifest),
                    &logical_manifest,
                )?;
                self.live.insert(
                    key,
                    LiveOwnedTree {
                        name: quarantine_name.to_owned(),
                        namespace_name: quarantine_name.to_owned(),
                        owner: owner.to_owned(),
                        parent_path: parent.root_path().to_path_buf(),
                        identity,
                        manifest,
                        recovery_stage_path,
                        state: LiveTreeState::Owned { directory, lease },
                    },
                );
            }
        }
        Ok(())
    }

    fn require_project(&self) -> Result<(), TransactionError> {
        ensure_supported()?;
        let actual = self
            .project
            .identity_token()
            .map_err(fs_error("rechecking project identity"))?;
        if actual == self.project_identity_token {
            Ok(())
        } else {
            Err(TransactionError::ThirdState(
                "project capability changed identity".to_owned(),
            ))
        }
    }

    fn output_parent(&self, plan: &ExportPlan) -> Result<SafefsProject, TransactionError> {
        let output = Path::new(&plan.output_display_path);
        if output.file_name().and_then(|name| name.to_str()) != Some(plan.output_name.as_str()) {
            return Err(TransactionError::InvalidPrepared(
                "export output name disagrees with its display path".to_owned(),
            ));
        }
        let parent_path = output.parent().ok_or_else(|| {
            TransactionError::InvalidPrepared("export output has no parent".to_owned())
        })?;
        let parent = SafefsProject::open(parent_path).map_err(fs_error("opening output parent"))?;
        require_project_token(&parent, &plan.output_parent_identity, "output parent")?;
        Ok(parent)
    }

    fn quarantine_parent(&self, plan: &InPlacePlan) -> Result<SafefsProject, TransactionError> {
        let parent_path = self.project_root.parent().ok_or_else(|| {
            TransactionError::InvalidPrepared("project root has no quarantine parent".to_owned())
        })?;
        let parent =
            SafefsProject::open(parent_path).map_err(fs_error("opening quarantine parent"))?;
        require_project_token(
            &parent,
            &plan.quarantine_parent_identity,
            "quarantine parent",
        )?;
        Ok(parent)
    }

    fn live_key(name: &str, prefix: &str) -> Result<String, TransactionError> {
        let transaction = name.strip_prefix(prefix).ok_or_else(|| {
            TransactionError::InvalidPrepared(format!(
                "owned sibling `{name}` does not use prefix `{prefix}`"
            ))
        })?;
        if transaction.is_empty() || !transaction.bytes().all(|byte| byte.is_ascii_alphanumeric()) {
            return Err(TransactionError::InvalidPrepared(format!(
                "owned sibling `{name}` has an invalid transaction id"
            )));
        }
        Ok(transaction.to_owned())
    }

    fn owned_mut(
        &mut self,
        key: &str,
        name: &str,
        owner: &str,
    ) -> Result<&mut LiveOwnedTree, TransactionError> {
        let live = self.live.get_mut(key).ok_or_else(|| {
            TransactionError::ThirdState(format!(
                "no live safefs ownership handle exists for `{name}`"
            ))
        })?;
        if live.name != name || live.owner != owner {
            return Err(TransactionError::ThirdState(format!(
                "live ownership for `{name}` is bound to different durable evidence"
            )));
        }
        Ok(live)
    }

    fn refresh_owned(live: &mut LiveOwnedTree) -> Result<(), TransactionError> {
        let state = std::mem::replace(
            &mut live.state,
            LiveTreeState::PossiblyMoved("owned-tree reseal was interrupted".to_owned()),
        );
        let directory = match state {
            LiveTreeState::Owned { directory, lease } => {
                drop(lease);
                directory
            }
            LiveTreeState::OwnedMutable { directory } => directory,
            other => {
                live.state = other;
                return Err(TransactionError::ThirdState(format!(
                    "owned tree `{}` no longer has its creation handle",
                    live.name
                )));
            }
        };
        let lease = directory
            .lease_existing_entries()
            .map_err(fs_error("leasing owned tree after mutation"))?;
        live.identity = lease.identity().clone();
        live.manifest = lease.manifest().clone();
        live.recovery_stage_path = None;
        live.state = LiveTreeState::Owned { directory, lease };
        Ok(())
    }

    fn begin_owned_mutation(live: &mut LiveOwnedTree) -> Result<Pinned, TransactionError> {
        let state = std::mem::replace(
            &mut live.state,
            LiveTreeState::PossiblyMoved("owned-tree mutation was interrupted".to_owned()),
        );
        let directory = match state {
            LiveTreeState::Owned { directory, lease } => {
                drop(lease);
                directory
            }
            LiveTreeState::OwnedMutable { directory } => directory,
            other => {
                live.state = other;
                return Err(TransactionError::ThirdState(format!(
                    "owned tree `{}` is not in mutable owned state",
                    live.name
                )));
            }
        };
        let root = directory
            .directory()
            .map_err(fs_error("retaining mutable owned-tree capability"))?;
        live.state = LiveTreeState::OwnedMutable { directory };
        Ok(root)
    }

    fn quarantine_directory(
        &mut self,
        key: &str,
        name: &str,
        owner: &str,
    ) -> Result<Pinned, TransactionError> {
        let live = self.owned_mut(key, name, owner)?;
        match &live.state {
            LiveTreeState::Owned { directory, .. } => directory
                .directory()
                .map_err(fs_error("retaining quarantine capability")),
            LiveTreeState::OwnedMutable { directory } => directory
                .directory()
                .map_err(fs_error("retaining mutable quarantine capability")),
            LiveTreeState::PossiblyMoved(detail) => {
                Err(TransactionError::ThirdState(detail.clone()))
            }
            _ => Err(TransactionError::ThirdState(
                "quarantine is not at its owned sibling name".to_owned(),
            )),
        }
    }

    fn quarantine_directory_for_mutation(
        &mut self,
        key: &str,
        name: &str,
        owner: &str,
    ) -> Result<Pinned, TransactionError> {
        let live = self.owned_mut(key, name, owner)?;
        Self::begin_owned_mutation(live)
    }
}

include!("safefs/transition.rs");
include!("safefs/trait_export.rs");
include!("safefs/trait_in_place.rs");

impl TransactionFilesystem for SafefsTransactionFilesystem {
    safefs_transaction_filesystem_export_methods!();
    safefs_transaction_filesystem_in_place_methods!();
}

impl SafefsTransactionFilesystem {
    safefs_transaction_transition_methods!();
}

include!("safefs/ownership_durability.rs");
include!("safefs/observation_rebound.rs");
include!("safefs/model_topology.rs");
include!("safefs/filesystem_ops.rs");

#[cfg(all(test, windows))]
mod tests {
    include!("safefs/tests_shared_export.rs");
    include!("safefs/tests_in_place.rs");
    include!("safefs/tests_restart_recovery.rs");
}
