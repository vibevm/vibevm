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

impl TransactionFilesystem for SafefsTransactionFilesystem {
    fn rebind_owned_tree(&mut self, journal: &Journal) -> Result<(), TransactionError> {
        self.rebind_from_journal(journal)
    }

    fn owned_tree_seal(
        &mut self,
        name: &str,
        ownership_token: &str,
    ) -> Result<OwnedTreeSeal, TransactionError> {
        self.require_project()?;
        let live = self
            .live
            .values()
            .find(|live| live.name == name && live.owner == ownership_token)
            .ok_or_else(|| {
                TransactionError::ThirdState(format!(
                    "no live safefs ownership handle exists for `{name}`"
                ))
            })?;
        Ok(owned_tree_seal(&live.identity, &live.manifest))
    }

    fn create_export_candidate(
        &mut self,
        plan: &ExportPlan,
        candidate_name: &str,
        ownership_token: &str,
    ) -> Result<ExclusiveTreeCreation, TransactionError> {
        self.require_project()?;
        let key = Self::live_key(candidate_name, ".vibe-scrape-candidate-")?;
        if self.live.contains_key(&key) {
            return Err(TransactionError::ThirdState(
                "transaction id already owns a live safefs tree".to_owned(),
            ));
        }
        let output = Path::new(&plan.output_display_path);
        let absent = SafefsProject::pin_absent_path(output)
            .map_err(|error| TransactionError::OutputRace(format!("{error:#}")))?;
        if absent.identity_token() != plan.output_identity {
            return Err(TransactionError::OutputRace(
                "output slot identity changed since planning".to_owned(),
            ));
        }
        let parent = self.output_parent(plan)?;
        let parent_root = parent
            .root_dir()
            .map_err(fs_error("pinning output parent"))?;
        match parent_root.create_owned_child_exclusive(candidate_name, ownership_token) {
            Ok(directory) => {
                if !namespace_checkpoint_satisfied(directory.parent_durability()) {
                    let detail = format!(
                        "candidate parent did not durably sync: {:?}",
                        directory.parent_durability()
                    );
                    let lease = directory
                        .lease_existing_entries()
                        .map_err(fs_error("leasing non-durable candidate"))?;
                    self.live.insert(
                        key,
                        live_owned(
                            candidate_name,
                            ownership_token,
                            parent.root_path(),
                            directory,
                            lease,
                        ),
                    );
                    return Ok(ExclusiveTreeCreation::CreatedNotReopened { detail });
                }
                let lease = directory
                    .lease_existing_entries()
                    .map_err(fs_error("leasing created candidate"))?;
                self.live.insert(
                    key,
                    live_owned(
                        candidate_name,
                        ownership_token,
                        parent.root_path(),
                        directory,
                        lease,
                    ),
                );
                Ok(ExclusiveTreeCreation::Owned)
            }
            Err(error) => Ok(map_create(error)),
        }
    }

    fn apply_export_entry(
        &mut self,
        _plan: &ExportPlan,
        candidate_name: &str,
        ownership_token: &str,
        entry: &ExportEntry,
        prepared_after: Option<&[u8]>,
    ) -> Result<(), TransactionError> {
        self.require_project()?;
        let key = Self::live_key(candidate_name, ".vibe-scrape-candidate-")?;
        let root = {
            let live = self.owned_mut(&key, candidate_name, ownership_token)?;
            if live.namespace_name != candidate_name {
                return Err(TransactionError::ThirdState(
                    "export entry cannot be applied after candidate publication".to_owned(),
                ));
            }
            Self::begin_owned_mutation(live)?
        };

        match entry.kind {
            TreeEntryKind::Directory => {
                if entry.payload.is_some() {
                    return Err(TransactionError::InvalidPrepared(
                        "export directory carries a payload".to_owned(),
                    ));
                }
                create_directory_exact(&self.project, &root, &entry.target_path, entry.mode)?;
            }
            TreeEntryKind::File => {
                let bytes = match (&entry.payload, prepared_after) {
                    (
                        Some(ExportPayload::Source {
                            source_path,
                            before,
                        }),
                        None,
                    ) => read_sealed_file(&self.project, source_path, before)?,
                    (Some(ExportPayload::PreparedAfter { .. }), Some(bytes)) => bytes.to_vec(),
                    _ => {
                        return Err(TransactionError::InvalidPrepared(format!(
                            "export payload for `{}` does not match its prepared bytes",
                            entry.target_path
                        )));
                    }
                };
                let desired = PathState::File(FileState {
                    sha256: digest_bytes(&bytes),
                    bytes: bytes.len() as u64,
                    mode: entry.mode,
                });
                if state_matches(&self.project, &root, &entry.target_path, &desired)? {
                    let live = self.owned_mut(&key, candidate_name, ownership_token)?;
                    return Self::refresh_owned(live);
                }
                require_absent(&root, &entry.target_path)?;
                let write = self
                    .project
                    .write_atomic_transactional_in_with_mode(
                        &root,
                        &entry.target_path,
                        &bytes,
                        entry.mode,
                        &transaction_stage_name(
                            ownership_token,
                            &format!("export:{}", entry.target_path),
                            &entry.target_path,
                        ),
                    )
                    .map_err(|error| TransactionError::Filesystem(format!("{error:#}")))?;
                require_durable_write(&write, &entry.target_path)?;
            }
        }
        let live = self.owned_mut(&key, candidate_name, ownership_token)?;
        Self::refresh_owned(live)
    }

    fn observe_export_tree(
        &mut self,
        plan: &ExportPlan,
        slot: ExportTreeSlot,
        candidate_name: &str,
        ownership_token: &str,
    ) -> Result<OwnedTreeObservation, TransactionError> {
        self.require_project()?;
        let key = Self::live_key(candidate_name, ".vibe-scrape-candidate-")?;
        let parent = self.output_parent(plan)?;
        let root = parent
            .root_dir()
            .map_err(fs_error("pinning output parent"))?;
        let requested_name = match slot {
            ExportTreeSlot::Candidate => candidate_name,
            ExportTreeSlot::Output => &plan.output_name,
        };

        let Some(live) = self.live.get_mut(&key) else {
            return observe_unowned_name(&root, requested_name);
        };
        if live.name != candidate_name || live.owner != ownership_token {
            return Ok(OwnedTreeObservation::Third {
                detail: "owned export evidence changed".to_owned(),
            });
        }
        if live.namespace_name != requested_name {
            return observe_expected_absence(&root, requested_name);
        }
        if matches!(live.state, LiveTreeState::OwnedMutable { .. }) {
            Self::refresh_owned(live)?;
        }
        match &mut live.state {
            LiveTreeState::Published(pending) => map_safefs_observation(
                pending
                    .reobserve_published(&live.identity, &live.manifest)
                    .map_err(fs_error("reobserving published output"))?,
            ),
            LiveTreeState::Owned { directory, .. } => {
                let recovery_stage_path = live.recovery_stage_path.clone();
                let lease = if let Some(stage_path) = recovery_stage_path.as_deref() {
                    directory.lease_existing_entries_with_transaction_stage(stage_path)
                } else {
                    directory.lease_existing_entries()
                }
                .map_err(fs_error("leasing export tree for observation"))?;
                let expected = lease.manifest().clone();
                let identity = lease.identity().clone();
                let observed = root
                    .observe_owned_tree(
                        requested_name,
                        ownership_token,
                        &identity,
                        &expected,
                        &lease,
                    )
                    .map_err(fs_error("observing export tree"))?;
                live.identity = identity;
                live.manifest = expected;
                if let LiveTreeState::Owned {
                    lease: held_lease, ..
                } = &mut live.state
                {
                    *held_lease = lease;
                }
                strip_owned_observation_stage(
                    map_safefs_observation(observed)?,
                    recovery_stage_path.as_deref(),
                )
            }
            LiveTreeState::PossiblyMoved(detail) => Ok(OwnedTreeObservation::Third {
                detail: detail.clone(),
            }),
            LiveTreeState::OwnedMutable { .. } => unreachable!("mutable state was resealed"),
        }
    }

    fn publish_export_noreplace(
        &mut self,
        plan: &ExportPlan,
        candidate_name: &str,
        ownership_token: &str,
    ) -> Result<(), TransactionError> {
        self.require_project()?;
        let key = Self::live_key(candidate_name, ".vibe-scrape-candidate-")?;
        let parent = self.output_parent(plan)?;
        let destination = parent
            .root_dir()
            .map_err(fs_error("pinning output parent"))?;
        let live = self.live.remove(&key).ok_or_else(|| {
            TransactionError::ThirdState("candidate has no live ownership handle".to_owned())
        })?;
        if live.name != candidate_name || live.owner != ownership_token {
            self.live.insert(key, live);
            return Err(TransactionError::ThirdState(
                "candidate ownership evidence changed".to_owned(),
            ));
        }
        let LiveOwnedTree {
            name,
            namespace_name,
            owner,
            parent_path,
            identity: prior_identity,
            manifest: prior_manifest,
            recovery_stage_path,
            state,
        } = live;
        let (directory, original_lease) = match state {
            LiveTreeState::Owned { directory, lease } => (directory, lease),
            state => {
                self.live.insert(
                    key,
                    LiveOwnedTree {
                        name,
                        namespace_name,
                        owner,
                        parent_path,
                        identity: prior_identity,
                        manifest: prior_manifest,
                        recovery_stage_path,
                        state,
                    },
                );
                return Err(TransactionError::ThirdState(
                    "candidate is not in publishable owned state".to_owned(),
                ));
            }
        };
        drop(original_lease);
        let publish_lease = directory
            .lease_existing_entries()
            .map_err(fs_error("sealing candidate for publication"))?;
        let manifest = publish_lease.manifest().clone();
        let identity = publish_lease.identity().clone();
        if model_manifest(&manifest) != plan.final_manifest {
            self.live.insert(
                key,
                LiveOwnedTree {
                    name,
                    namespace_name,
                    owner,
                    parent_path,
                    state: LiveTreeState::Owned {
                        directory,
                        lease: publish_lease,
                    },
                    manifest,
                    identity,
                    recovery_stage_path: None,
                },
            );
            return Err(TransactionError::ThirdState(
                "candidate differs from the sealed final manifest".to_owned(),
            ));
        }
        match directory.publish_noreplace_to(
            &destination,
            &plan.output_name,
            ownership_token,
            &manifest,
            publish_lease,
        ) {
            Ok(pending) => {
                let source_parent = pending.source_parent;
                let destination_parent = pending.destination_parent;
                self.live.insert(
                    key,
                    LiveOwnedTree {
                        name,
                        namespace_name: plan.output_name.clone(),
                        owner,
                        parent_path,
                        state: LiveTreeState::Published(pending),
                        manifest,
                        identity,
                        recovery_stage_path: None,
                    },
                );
                require_namespace_checkpoint(source_parent, "candidate parent after publish")?;
                require_namespace_checkpoint(destination_parent, "output parent after publish")?;
                Ok(())
            }
            Err(OwnedTreePublishError::Occupied { path }) => {
                let state = reopen_owned_state(
                    &destination,
                    candidate_name,
                    ownership_token,
                    &identity,
                    &manifest,
                )?;
                self.live.insert(
                    key,
                    LiveOwnedTree {
                        name,
                        namespace_name,
                        owner,
                        parent_path,
                        state,
                        manifest,
                        identity,
                        recovery_stage_path: None,
                    },
                );
                Err(TransactionError::OutputRace(format!(
                    "`{}` is occupied",
                    path.display()
                )))
            }
            Err(OwnedTreePublishError::Unsupported) => {
                let state = reopen_owned_state(
                    &destination,
                    candidate_name,
                    ownership_token,
                    &identity,
                    &manifest,
                )?;
                self.live.insert(
                    key,
                    LiveOwnedTree {
                        name,
                        namespace_name,
                        owner,
                        parent_path,
                        state,
                        manifest,
                        identity,
                        recovery_stage_path: None,
                    },
                );
                Err(TransactionError::AtomicNoReplaceUnsupported)
            }
            Err(OwnedTreePublishError::BeforeMove { detail }) => {
                let state = reopen_owned_state(
                    &destination,
                    candidate_name,
                    ownership_token,
                    &identity,
                    &manifest,
                )?;
                self.live.insert(
                    key,
                    LiveOwnedTree {
                        name,
                        namespace_name,
                        owner,
                        parent_path,
                        state,
                        manifest,
                        identity,
                        recovery_stage_path: None,
                    },
                );
                Err(TransactionError::ThirdState(detail))
            }
            Err(OwnedTreePublishError::PossiblyMoved { detail, .. }) => {
                self.live.insert(
                    key,
                    LiveOwnedTree {
                        name,
                        namespace_name,
                        owner,
                        parent_path,
                        state: LiveTreeState::PossiblyMoved(detail.clone()),
                        manifest,
                        identity,
                        recovery_stage_path: None,
                    },
                );
                Err(TransactionError::ThirdState(detail))
            }
        }
    }

    fn unpublish_export(
        &mut self,
        plan: &ExportPlan,
        candidate_name: &str,
        ownership_token: &str,
    ) -> Result<(), TransactionError> {
        self.require_project()?;
        let key = Self::live_key(candidate_name, ".vibe-scrape-candidate-")?;
        let parent = self.output_parent(plan)?;
        let root = parent
            .root_dir()
            .map_err(fs_error("pinning output parent"))?;
        let live = self.owned_mut(&key, candidate_name, ownership_token)?;
        if live.namespace_name != plan.output_name {
            return Err(TransactionError::ThirdState(
                "export output is not at its published namespace".to_owned(),
            ));
        }
        if matches!(live.state, LiveTreeState::OwnedMutable { .. }) {
            Self::refresh_owned(live)?;
        }
        let observed = match &live.state {
            LiveTreeState::Published(pending) => pending
                .reobserve_published(&live.identity, &live.manifest)
                .map_err(fs_error("reobserving output before rollback"))?,
            LiveTreeState::Owned { lease, .. } => root
                .observe_owned_tree(
                    &plan.output_name,
                    ownership_token,
                    &live.identity,
                    &live.manifest,
                    lease,
                )
                .map_err(fs_error("reobserving rebound output before rollback"))?,
            LiveTreeState::PossiblyMoved(detail) => {
                return Err(TransactionError::ThirdState(detail.clone()));
            }
            LiveTreeState::OwnedMutable { .. } => unreachable!("mutable state was resealed"),
        };
        match observed {
            SafefsTreeObservation::MatchesAtObservation(_) => {}
            SafefsTreeObservation::Absent => {
                return Err(TransactionError::ThirdState(
                    "published output disappeared before rollback".to_owned(),
                ));
            }
            SafefsTreeObservation::Third { detail } => {
                return Err(TransactionError::ThirdState(detail));
            }
        }
        let expected = root
            .inspect_child_state(&plan.output_name)
            .map_err(fs_error("sealing output root for rollback"))?
            .ok_or_else(|| TransactionError::ThirdState("published output is absent".to_owned()))?;
        let identity = live.identity.clone();
        let manifest = live.manifest.clone();
        let prior = std::mem::replace(
            &mut live.state,
            LiveTreeState::PossiblyMoved("unpublish state transition incomplete".to_owned()),
        );
        if matches!(
            &prior,
            LiveTreeState::OwnedMutable { .. } | LiveTreeState::PossiblyMoved(_)
        ) {
            unreachable!("unpublish state was checked and resealed");
        }
        // Close every root/descendant lease before asking Windows for DELETE
        // access to the exact source root.
        drop(prior);
        let durability = root
            .rename_child_noreplace_to_durable(&root, &plan.output_name, candidate_name, &expected)
            .map_err(map_rename)?;
        require_namespace_checkpoint(durability, "output parent after unpublish")?;
        live.namespace_name = candidate_name.to_owned();
        live.state =
            reopen_owned_state(&root, candidate_name, ownership_token, &identity, &manifest)?;
        Ok(())
    }

    fn prepare_owned_tree_cleanup(
        &mut self,
        journal: &Journal,
        name: &str,
        ownership_token: &str,
        seal: &OwnedTreeSeal,
        completed: &[String],
    ) -> Result<OwnedTreeCleanupPreparation, TransactionError> {
        self.require_project()?;
        let (key, parent) = self.cleanup_parent(journal, name)?;
        self.release_cleanup_handles(&key, name, ownership_token, seal)?;
        let root = parent
            .root_dir()
            .map_err(fs_error("pinning owned-tree cleanup parent"))?;
        let authorized_stage = authorized_owned_stage_in_seal(journal, ownership_token, seal)?;
        let (identity, manifest) = safefs_seal(seal, authorized_stage.as_deref())?;
        let progress = OwnedTreeCleanupProgress::from_completed(completed.to_vec())
            .map_err(fs_error("validating cleanup progress"))?;
        match root
            .prepare_owned_tree_cleanup_next(name, ownership_token, &identity, &manifest, &progress)
            .map_err(map_cleanup)?
        {
            SafefsCleanupPreparation::Complete => {
                self.live.remove(&key);
                Ok(OwnedTreeCleanupPreparation::Complete)
            }
            SafefsCleanupPreparation::Intent(intent) => Ok(OwnedTreeCleanupPreparation::Intent(
                model_cleanup_intent(&intent),
            )),
        }
    }

    fn execute_owned_tree_cleanup(
        &mut self,
        journal: &Journal,
        name: &str,
        ownership_token: &str,
        seal: &OwnedTreeSeal,
        completed: &[String],
        intent: &OwnedTreeCleanupIntent,
    ) -> Result<OwnedTreeCleanupCompletion, TransactionError> {
        self.require_project()?;
        let (key, parent) = self.cleanup_parent(journal, name)?;
        self.release_cleanup_handles(&key, name, ownership_token, seal)?;
        let root = parent
            .root_dir()
            .map_err(fs_error("pinning owned-tree cleanup parent"))?;
        let authorized_stage = authorized_owned_stage_in_seal(journal, ownership_token, seal)?;
        let (identity, manifest) = safefs_seal(seal, authorized_stage.as_deref())?;
        let progress = OwnedTreeCleanupProgress::from_completed(completed.to_vec())
            .map_err(fs_error("validating cleanup progress"))?;
        let safefs_intent = safefs_cleanup_intent(intent)?;
        let completion = root
            .execute_owned_tree_cleanup_intent(
                name,
                ownership_token,
                &identity,
                &manifest,
                &progress,
                &safefs_intent,
            )
            .map_err(map_cleanup)?;
        require_namespace_checkpoint(
            completion.durability(),
            &format!("cleanup parent for `{}`", completion.path),
        )?;
        if intent.root {
            self.live.remove(&key);
        }
        Ok(OwnedTreeCleanupCompletion {
            progress_key: completion.progress_key,
            recovered_after_syscall: completion.recovered_after_syscall,
        })
    }

    fn create_quarantine(
        &mut self,
        plan: &InPlacePlan,
        quarantine_name: &str,
        ownership_token: &str,
    ) -> Result<ExclusiveTreeCreation, TransactionError> {
        self.require_project()?;
        let key = Self::live_key(quarantine_name, ".vibe-scrape-quarantine-")?;
        if self.live.contains_key(&key) {
            return Err(TransactionError::ThirdState(
                "transaction id already owns a live safefs tree".to_owned(),
            ));
        }
        let parent = self.quarantine_parent(plan)?;
        let root = parent
            .root_dir()
            .map_err(fs_error("pinning quarantine parent"))?;
        if !root
            .same_filesystem(
                &self
                    .project
                    .root_dir()
                    .map_err(fs_error("pinning project"))?,
            )
            .map_err(fs_error("proving quarantine volume"))?
        {
            return Err(TransactionError::Filesystem(
                "quarantine and project are on different filesystems".to_owned(),
            ));
        }
        match root.create_owned_child_exclusive(quarantine_name, ownership_token) {
            Ok(directory) => {
                let durable = directory.parent_durability();
                let quarantine_root = directory
                    .directory()
                    .map_err(fs_error("retaining quarantine for topology setup"))?;
                for path in quarantine_topology(plan) {
                    create_directory_exact(&self.project, &quarantine_root, &path, None)?;
                }
                let lease = directory
                    .lease_existing_entries()
                    .map_err(fs_error("leasing created quarantine"))?;
                self.live.insert(
                    key,
                    live_owned(
                        quarantine_name,
                        ownership_token,
                        parent.root_path(),
                        directory,
                        lease,
                    ),
                );
                if namespace_checkpoint_satisfied(durable) {
                    Ok(ExclusiveTreeCreation::Owned)
                } else {
                    Ok(ExclusiveTreeCreation::CreatedNotReopened {
                        detail: format!("quarantine parent did not durably sync: {durable:?}"),
                    })
                }
            }
            Err(error) => Ok(map_create(error)),
        }
    }

    fn observe_step(
        &mut self,
        plan: &InPlacePlan,
        quarantine_name: &str,
        ownership_token: &str,
        step: &MutationStep,
    ) -> Result<SealedObservation, TransactionError> {
        self.require_project()?;
        let key = Self::live_key(quarantine_name, ".vibe-scrape-quarantine-")?;
        let quarantine = self.quarantine_directory(&key, quarantine_name, ownership_token)?;
        if step.kind == MutationKind::ContractDeleteLast {
            let project = self.project.root_dir().map_err(fs_error(
                "pinning project for contract-boundary observation",
            ))?;
            let project_tree = model_tree_at(&self.project, &project)?;
            let quarantine_before = step
                .transitions
                .iter()
                .filter(|transition| transition.location == Location::Quarantine)
                .all(|transition| {
                    self.transition_matches(&quarantine, transition, false)
                        .unwrap_or(false)
                });
            let quarantine_after = step
                .transitions
                .iter()
                .filter(|transition| transition.location == Location::Quarantine)
                .all(|transition| {
                    self.transition_matches(&quarantine, transition, true)
                        .unwrap_or(false)
                });
            return Ok(
                match (
                    project_tree == plan.pre_contract_tree && quarantine_before,
                    project_tree == plan.post_contract_tree && quarantine_after,
                ) {
                    (true, false) => SealedObservation::Before,
                    (false, true) => SealedObservation::After,
                    _ => SealedObservation::Third {
                        detail: format!(
                            "contract step `{}` matches neither complete pre-contract nor post-contract tree",
                            step.id
                        ),
                    },
                },
            );
        }
        if step.kind == MutationKind::ContractAncestorTreePark {
            let project = self
                .project
                .root_dir()
                .map_err(fs_error("pinning project for contract-cleanup observation"))?;
            let project_tree = model_tree_at(&self.project, &project)?;
            let quarantine_before = step
                .transitions
                .iter()
                .filter(|transition| transition.location == Location::Quarantine)
                .all(|transition| {
                    self.transition_matches(&quarantine, transition, false)
                        .unwrap_or(false)
                });
            let quarantine_after = step
                .transitions
                .iter()
                .filter(|transition| transition.location == Location::Quarantine)
                .all(|transition| {
                    self.transition_matches(&quarantine, transition, true)
                        .unwrap_or(false)
                });
            return Ok(
                match (
                    project_tree == plan.post_contract_tree && quarantine_before,
                    project_tree == plan.after_tree && quarantine_after,
                ) {
                    (true, false) => SealedObservation::Before,
                    (false, true) => SealedObservation::After,
                    _ => SealedObservation::Third {
                        detail: format!(
                            "contract cleanup `{}` matches neither post-contract nor final tree",
                            step.id
                        ),
                    },
                },
            );
        }
        let before = step.transitions.iter().all(|transition| {
            self.transition_matches(&quarantine, transition, false)
                .unwrap_or(false)
        });
        let after = step.transitions.iter().all(|transition| {
            self.transition_matches(&quarantine, transition, true)
                .unwrap_or(false)
        });
        let supplemental = supplemental_observation(&self.project, &quarantine, step)?;
        match (before, after, supplemental) {
            (true, false, Supplemental::Before | Supplemental::Either) => {
                Ok(SealedObservation::Before)
            }
            (false, true, Supplemental::After | Supplemental::Either) => {
                Ok(SealedObservation::After)
            }
            (true, true, Supplemental::After) => Ok(SealedObservation::After),
            (true, true, _) if step.transitions.is_empty() => Ok(SealedObservation::After),
            _ => Ok(SealedObservation::Third {
                detail: format!("step `{}` matches neither complete sealed side", step.id),
            }),
        }
    }

    fn observe_quarantine_root(
        &mut self,
        plan: &InPlacePlan,
        quarantine_name: &str,
        ownership_token: &str,
    ) -> Result<OwnedRootObservation, TransactionError> {
        self.require_project()?;
        let key = Self::live_key(quarantine_name, ".vibe-scrape-quarantine-")?;
        let parent = self.quarantine_parent(plan)?;
        let root = parent
            .root_dir()
            .map_err(fs_error("pinning quarantine parent"))?;
        let Some(live) = self.live.get_mut(&key) else {
            return match root.open_child_checked(quarantine_name) {
                Ok(None) => Ok(OwnedRootObservation::Absent),
                Ok(Some(_)) => Ok(OwnedRootObservation::Third {
                    detail: "quarantine exists without a live ownership seal".to_owned(),
                }),
                Err(error) => Ok(OwnedRootObservation::Third {
                    detail: format!("quarantine root cannot be opened no-follow: {error:#}"),
                }),
            };
        };
        if live.name != quarantine_name || live.owner != ownership_token {
            return Ok(OwnedRootObservation::Third {
                detail: "quarantine ownership evidence changed".to_owned(),
            });
        }
        if matches!(live.state, LiveTreeState::OwnedMutable { .. }) {
            Self::refresh_owned(live)?;
        }
        match &live.state {
            LiveTreeState::Owned { lease, .. } => {
                match root
                    .observe_owned_tree(
                        quarantine_name,
                        ownership_token,
                        &live.identity,
                        &live.manifest,
                        lease,
                    )
                    .map_err(fs_error("observing quarantine root"))?
                {
                    SafefsTreeObservation::Absent => Ok(OwnedRootObservation::Absent),
                    SafefsTreeObservation::MatchesAtObservation(_) => {
                        Ok(OwnedRootObservation::ExactOwned)
                    }
                    SafefsTreeObservation::Third { detail } => {
                        Ok(OwnedRootObservation::Third { detail })
                    }
                }
            }
            LiveTreeState::PossiblyMoved(detail) => Ok(OwnedRootObservation::Third {
                detail: detail.clone(),
            }),
            LiveTreeState::Published(_) => Ok(OwnedRootObservation::Third {
                detail: "quarantine unexpectedly entered published state".to_owned(),
            }),
            LiveTreeState::OwnedMutable { .. } => unreachable!("mutable state was resealed"),
        }
    }

    fn apply_step(
        &mut self,
        _plan: &InPlacePlan,
        quarantine_name: &str,
        ownership_token: &str,
        step: &MutationStep,
        prepared_after: Option<&[u8]>,
    ) -> Result<(), TransactionError> {
        self.require_project()?;
        let key = Self::live_key(quarantine_name, ".vibe-scrape-quarantine-")?;
        let quarantine =
            self.quarantine_directory_for_mutation(&key, quarantine_name, ownership_token)?;
        self.apply_or_rollback_step(&quarantine, ownership_token, step, prepared_after, false)?;
        let live = self.owned_mut(&key, quarantine_name, ownership_token)?;
        Self::refresh_owned(live)
    }

    fn rollback_step(
        &mut self,
        _plan: &InPlacePlan,
        quarantine_name: &str,
        ownership_token: &str,
        step: &MutationStep,
    ) -> Result<(), TransactionError> {
        self.require_project()?;
        let key = Self::live_key(quarantine_name, ".vibe-scrape-quarantine-")?;
        let quarantine =
            self.quarantine_directory_for_mutation(&key, quarantine_name, ownership_token)?;
        self.apply_or_rollback_step(&quarantine, ownership_token, step, None, true)?;
        let live = self.owned_mut(&key, quarantine_name, ownership_token)?;
        Self::refresh_owned(live)
    }

    fn cleanup_unpublished_step_stage(
        &mut self,
        _plan: &InPlacePlan,
        quarantine_name: &str,
        ownership_token: &str,
        step: &MutationStep,
    ) -> Result<(), TransactionError> {
        self.require_project()?;
        match step.kind {
            MutationKind::AtomicRewrite => {
                let transition = one_at(step, Location::Project)?;
                let PathState::File(after) = &transition.after else {
                    return invalid_step(step, "rewrite after state is not a file");
                };
                let root = self
                    .project
                    .root_dir()
                    .map_err(fs_error("pinning project for stage cleanup"))?;
                remove_transaction_stage(
                    &root,
                    &transition.path,
                    &transaction_stage_name(
                        ownership_token,
                        &format!("apply:{}", step.id),
                        &transition.path,
                    ),
                    after,
                )
            }
            MutationKind::CaptureBeforeImage => {
                let transition = one_at(step, Location::Quarantine)?;
                let PathState::File(after) = &transition.after else {
                    return invalid_step(step, "capture after state is not a file");
                };
                let key = Self::live_key(quarantine_name, ".vibe-scrape-quarantine-")?;
                let root =
                    self.quarantine_directory_for_mutation(&key, quarantine_name, ownership_token)?;
                let result = remove_transaction_stage(
                    &root,
                    &transition.path,
                    &transaction_stage_name(
                        ownership_token,
                        &format!("apply:{}", step.id),
                        &transition.path,
                    ),
                    after,
                );
                let live = self.owned_mut(&key, quarantine_name, ownership_token)?;
                Self::refresh_owned(live)?;
                result
            }
            _ => Ok(()),
        }
    }
}

impl SafefsTransactionFilesystem {
    fn transition_matches(
        &self,
        quarantine: &Pinned,
        transition: &super::PathTransition,
        after: bool,
    ) -> Result<bool, TransactionError> {
        let project_root = self
            .project
            .root_dir()
            .map_err(fs_error("pinning project for observation"))?;
        let root = match transition.location {
            Location::Project => &project_root,
            Location::Quarantine => quarantine,
        };
        state_matches(
            &self.project,
            root,
            &transition.path,
            if after {
                &transition.after
            } else {
                &transition.before
            },
        )
    }

    fn apply_or_rollback_step(
        &self,
        quarantine: &Pinned,
        ownership_token: &str,
        step: &MutationStep,
        prepared_after: Option<&[u8]>,
        rollback: bool,
    ) -> Result<(), TransactionError> {
        let project = self
            .project
            .root_dir()
            .map_err(fs_error("pinning project for mutation"))?;
        match step.kind {
            MutationKind::CaptureBeforeImage => {
                let project_transition = one_at(step, Location::Project)?;
                let quarantine_transition = one_at(step, Location::Quarantine)?;
                let PathState::File(before) = &project_transition.before else {
                    return invalid_step(step, "capture source is not a file");
                };
                if rollback {
                    remove_file_exact(
                        &self.project,
                        quarantine,
                        &quarantine_transition.path,
                        before,
                    )
                } else {
                    let bytes = read_sealed_file_at(
                        &self.project,
                        &project,
                        &project_transition.path,
                        before,
                    )?;
                    require_absent(quarantine, &quarantine_transition.path)?;
                    durable_write(
                        &self.project,
                        quarantine,
                        &quarantine_transition.path,
                        &bytes,
                        before.mode,
                        &transaction_stage_name(
                            ownership_token,
                            &format!("apply:{}", step.id),
                            &quarantine_transition.path,
                        ),
                    )
                }
            }
            MutationKind::AtomicRewrite => {
                let transition = one_at(step, Location::Project)?;
                let (PathState::File(before), PathState::File(after)) =
                    (&transition.before, &transition.after)
                else {
                    return invalid_step(step, "rewrite transition is not file-to-file");
                };
                if rollback {
                    if !state_matches(&self.project, &project, &transition.path, &transition.after)?
                    {
                        return third_step(step, "rewrite target is not its sealed after state");
                    }
                    let backup_path = format!("before/{}", transition.path);
                    let bytes =
                        read_sealed_file_at(&self.project, quarantine, &backup_path, before)?;
                    durable_write(
                        &self.project,
                        &project,
                        &transition.path,
                        &bytes,
                        before.mode,
                        &transaction_stage_name(
                            ownership_token,
                            &format!("rollback:{}", step.id),
                            &transition.path,
                        ),
                    )
                } else {
                    if !state_matches(
                        &self.project,
                        &project,
                        &transition.path,
                        &transition.before,
                    )? {
                        return third_step(step, "rewrite source is not its sealed before state");
                    }
                    let bytes = prepared_after.ok_or_else(|| {
                        TransactionError::InvalidPrepared(format!(
                            "rewrite `{}` has no prepared-after payload",
                            step.id
                        ))
                    })?;
                    if digest_bytes(bytes) != after.sha256 || bytes.len() as u64 != after.bytes {
                        return invalid_step(step, "prepared-after payload differs from the plan");
                    }
                    durable_write(
                        &self.project,
                        &project,
                        &transition.path,
                        bytes,
                        after.mode,
                        &transaction_stage_name(
                            ownership_token,
                            &format!("apply:{}", step.id),
                            &transition.path,
                        ),
                    )
                }
            }
            MutationKind::Relocate => {
                let (source, destination) = move_pair(step)?;
                if rollback {
                    rename_exact(
                        &self.project,
                        &project,
                        &destination.path,
                        &project,
                        &source.path,
                        &source.before,
                        true,
                    )
                } else {
                    rename_exact(
                        &self.project,
                        &project,
                        &source.path,
                        &project,
                        &destination.path,
                        &source.before,
                        false,
                    )
                }
            }
            MutationKind::ContractAncestorTreePark => {
                let source = step
                    .transitions
                    .iter()
                    .find(|transition| transition.location == Location::Project)
                    .ok_or_else(|| invalid_step_error(step, "missing project ancestor tree"))?;
                let destination = step
                    .transitions
                    .iter()
                    .find(|transition| transition.location == Location::Quarantine)
                    .ok_or_else(|| invalid_step_error(step, "missing parked ancestor tree"))?;
                if rollback {
                    rename_exact(
                        &self.project,
                        quarantine,
                        &destination.path,
                        &project,
                        &source.path,
                        &source.before,
                        false,
                    )
                } else {
                    rename_exact(
                        &self.project,
                        &project,
                        &source.path,
                        quarantine,
                        &destination.path,
                        &source.before,
                        false,
                    )
                }
            }
            MutationKind::QuarantineFile | MutationKind::ContractDeleteLast => {
                let source = step
                    .transitions
                    .iter()
                    .find(|transition| {
                        transition.location == Location::Project
                            && matches!(transition.before, PathState::File(_))
                            && transition.after == PathState::Absent
                    })
                    .ok_or_else(|| invalid_step_error(step, "missing project file move"))?;
                let destination = step
                    .transitions
                    .iter()
                    .find(|transition| {
                        transition.location == Location::Quarantine
                            && transition.before == PathState::Absent
                            && transition.after == source.before
                    })
                    .ok_or_else(|| invalid_step_error(step, "missing quarantine file move"))?;
                let mut contract_ancestors = step
                    .transitions
                    .iter()
                    .filter(|transition| {
                        step.kind == MutationKind::ContractDeleteLast
                            && transition.location == Location::Project
                            && matches!(transition.before, PathState::EmptyDirectory { .. })
                    })
                    .collect::<Vec<_>>();
                if rollback {
                    // Recreate the sealed shallow-to-deep ancestor chain while
                    // every directory is still empty. Only then restore the
                    // contract without an implicit parent-creation shortcut.
                    contract_ancestors.sort_by_key(|transition| path_depth(&transition.path));
                    for transition in &contract_ancestors {
                        let PathState::EmptyDirectory { mode } = transition.before else {
                            unreachable!()
                        };
                        create_directory_exact(&self.project, &project, &transition.path, mode)?;
                    }
                    rename_exact(
                        &self.project,
                        quarantine,
                        &destination.path,
                        &project,
                        &source.path,
                        &source.before,
                        false,
                    )?;
                } else {
                    rename_exact(
                        &self.project,
                        &project,
                        &source.path,
                        quarantine,
                        &destination.path,
                        &source.before,
                        true,
                    )?;
                }
                if step.kind == MutationKind::ContractDeleteLast && !rollback {
                    contract_ancestors
                        .sort_by_key(|transition| std::cmp::Reverse(path_depth(&transition.path)));
                    for transition in contract_ancestors {
                        let PathState::EmptyDirectory { mode } = &transition.before else {
                            unreachable!()
                        };
                        remove_empty_directory_exact(
                            &self.project,
                            &project,
                            &transition.path,
                            *mode,
                        )?;
                    }
                }
                Ok(())
            }
            MutationKind::PruneEmptyDirectory => {
                let transition = one_at(step, Location::Project)?;
                let parked = parked_directory(step);
                if rollback {
                    rename_exact(
                        &self.project,
                        quarantine,
                        &parked,
                        &project,
                        &transition.path,
                        &transition.before,
                        true,
                    )
                } else {
                    rename_exact(
                        &self.project,
                        &project,
                        &transition.path,
                        quarantine,
                        &parked,
                        &transition.before,
                        true,
                    )
                }
            }
            MutationKind::CreateRelocationParent => {
                let transition = one_at(step, Location::Project)?;
                let PathState::EmptyDirectory { mode } = &transition.after else {
                    return invalid_step(step, "created relocation parent is not empty-directory");
                };
                if rollback {
                    remove_empty_directory_exact(&self.project, &project, &transition.path, *mode)
                } else {
                    create_directory_exact(&self.project, &project, &transition.path, *mode)
                }
            }
            MutationKind::ContractExternalPreserve => {
                if prepared_after.is_some() {
                    invalid_step(step, "external preservation received mutation bytes")
                } else {
                    Ok(())
                }
            }
        }
    }

    fn cleanup_parent(
        &self,
        journal: &Journal,
        name: &str,
    ) -> Result<(String, SafefsProject), TransactionError> {
        match &journal.execution {
            PreparedMode::Export(plan) => {
                if journal.candidate_name.as_deref() != Some(name) {
                    return Err(TransactionError::Store(
                        "cleanup name differs from the journaled export candidate".to_owned(),
                    ));
                }
                Ok((
                    Self::live_key(name, ".vibe-scrape-candidate-")?,
                    self.output_parent(plan)?,
                ))
            }
            PreparedMode::InPlace(plan) => {
                if journal.quarantine_name.as_deref() != Some(name) {
                    return Err(TransactionError::Store(
                        "cleanup name differs from the journaled quarantine".to_owned(),
                    ));
                }
                Ok((
                    Self::live_key(name, ".vibe-scrape-quarantine-")?,
                    self.quarantine_parent(plan)?,
                ))
            }
        }
    }

    fn release_cleanup_handles(
        &mut self,
        key: &str,
        name: &str,
        owner: &str,
        seal: &OwnedTreeSeal,
    ) -> Result<(), TransactionError> {
        let Some(live) = self.live.get_mut(key) else {
            // Root absence is meaningful only to the safefs active-intent
            // executor; it will reject every other shape below this seam.
            return Ok(());
        };
        if live.name != name || live.owner != owner {
            return Err(TransactionError::ThirdState(
                "live cleanup ownership differs from journal evidence".to_owned(),
            ));
        }
        if live.identity.as_str() != seal.directory_identity {
            return Err(TransactionError::ThirdState(
                "live cleanup root differs from the journaled cleanup identity".to_owned(),
            ));
        }
        // A restart after one or more cleanup syscalls necessarily holds a
        // reduced live manifest. The manifest-bound safefs prepare/execute
        // calls below compare it against the exact canonical completed prefix
        // (and active intent); repeating the original full digest here would
        // make legitimate restart recovery impossible.
        if matches!(live.state, LiveTreeState::Published(_)) {
            return Err(TransactionError::ThirdState(
                "published product cannot be cleanup payload".to_owned(),
            ));
        }
        if !matches!(live.state, LiveTreeState::PossiblyMoved(_)) {
            live.state = LiveTreeState::PossiblyMoved(
                "owned-tree cleanup is controlled by the durable entry WAL".to_owned(),
            );
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Supplemental {
    Before,
    After,
    Either,
}

fn supplemental_observation(
    project: &SafefsProject,
    quarantine: &Pinned,
    step: &MutationStep,
) -> Result<Supplemental, TransactionError> {
    if step.kind != MutationKind::PruneEmptyDirectory {
        return Ok(Supplemental::Either);
    }
    let transition = one_at(step, Location::Project)?;
    let parked = parked_directory(step);
    let before = state_matches(project, quarantine, &parked, &PathState::Absent)?;
    let after = state_matches(project, quarantine, &parked, &transition.before)?;
    match (before, after) {
        (true, false) => Ok(Supplemental::Before),
        (false, true) => Ok(Supplemental::After),
        _ => Err(TransactionError::ThirdState(format!(
            "pruned directory parking state for `{}` is ambiguous",
            step.id
        ))),
    }
}

fn live_owned(
    name: &str,
    owner: &str,
    parent_path: &Path,
    directory: OwnedDirectory,
    lease: ExistingTreeEntryLease,
) -> LiveOwnedTree {
    let identity = lease.identity().clone();
    let manifest = lease.manifest().clone();
    LiveOwnedTree {
        name: name.to_owned(),
        namespace_name: name.to_owned(),
        owner: owner.to_owned(),
        parent_path: parent_path.to_path_buf(),
        identity,
        manifest,
        recovery_stage_path: None,
        state: LiveTreeState::Owned { directory, lease },
    }
}

fn reopen_owned_state(
    parent: &Pinned,
    name: &str,
    owner: &str,
    identity: &OwnedDirectoryIdentity,
    manifest: &SafefsTreeManifest,
) -> Result<LiveTreeState, TransactionError> {
    let reopened = parent
        .reopen_owned_child(name, owner, identity, manifest)
        .map_err(map_reopen)?;
    let (directory, lease) = reopened.into_parts();
    Ok(LiveTreeState::Owned { directory, lease })
}

fn owned_tree_seal(
    identity: &OwnedDirectoryIdentity,
    manifest: &SafefsTreeManifest,
) -> OwnedTreeSeal {
    OwnedTreeSeal {
        directory_identity: identity.as_str().to_owned(),
        manifest_digest: manifest.digest.clone(),
        entries: manifest
            .entries
            .iter()
            .map(|entry| OwnedEntrySeal {
                path: entry.path.clone(),
                kind: match entry.state.kind {
                    EntryStateKind::File => TreeEntryKind::File,
                    EntryStateKind::Directory => TreeEntryKind::Directory,
                },
                sha256: entry.state.sha256.as_deref().map(model_digest),
                bytes: entry.state.bytes,
                mode: entry.state.unix_mode,
                identity: entry.state.identity.as_str().to_owned(),
            })
            .collect(),
    }
}

fn safefs_seal(
    seal: &OwnedTreeSeal,
    authorized_stage_path: Option<&str>,
) -> Result<(OwnedDirectoryIdentity, SafefsTreeManifest), TransactionError> {
    let identity = OwnedDirectoryIdentity::from_token(&seal.directory_identity)
        .map_err(fs_error("validating persisted owned-directory identity"))?;
    let entries = seal
        .entries
        .iter()
        .map(|entry| {
            let state = EntryState {
                kind: match entry.kind {
                    TreeEntryKind::File => EntryStateKind::File,
                    TreeEntryKind::Directory => EntryStateKind::Directory,
                },
                sha256: entry.sha256.as_ref().map(|digest| {
                    digest
                        .0
                        .strip_prefix("sha256:")
                        .unwrap_or(&digest.0)
                        .to_owned()
                }),
                bytes: entry.bytes,
                unix_mode: entry.mode,
                identity: EntryIdentity::from_token(&entry.identity)
                    .map_err(fs_error("validating persisted entry identity"))?,
            };
            Ok(SafefsTreeEntry {
                path: entry.path.clone(),
                state,
            })
        })
        .collect::<Result<Vec<_>, TransactionError>>()?;
    let manifest = match authorized_stage_path {
        Some(stage_path) => SafefsTreeManifest::from_persisted_with_transaction_stage(
            seal.manifest_digest.clone(),
            entries,
            stage_path,
        ),
        None => SafefsTreeManifest::from_persisted(seal.manifest_digest.clone(), entries),
    }
    .map_err(fs_error("validating persisted owned-tree manifest"))?;
    Ok((identity, manifest))
}

fn child_directory_present(parent: &Pinned, name: &str) -> Result<bool, TransactionError> {
    match parent
        .inspect_child_state(name)
        .map_err(fs_error("inspecting journaled owned-tree name"))?
    {
        None => Ok(false),
        Some(state) if state.kind == EntryStateKind::Directory => Ok(true),
        Some(_) => Err(TransactionError::ThirdState(format!(
            "journaled owned-tree name `{name}` is occupied by a non-directory"
        ))),
    }
}

fn map_reopen(error: ReopenOwnedDirectoryError) -> TransactionError {
    match error {
        ReopenOwnedDirectoryError::InvalidPersisted(error) => {
            TransactionError::Store(format!("invalid journaled owned-tree seal: {error:#}"))
        }
        ReopenOwnedDirectoryError::Third { detail } => TransactionError::ThirdState(detail),
        ReopenOwnedDirectoryError::Io(error) => {
            TransactionError::Filesystem(format!("rebinding journaled owned tree: {error:#}"))
        }
        ReopenOwnedDirectoryError::Unsupported => TransactionError::AtomicNoReplaceUnsupported,
    }
}

fn map_create(error: OwnedDirectoryCreateError) -> ExclusiveTreeCreation {
    match error {
        OwnedDirectoryCreateError::NotCreated(error) => ExclusiveTreeCreation::NotCreated {
            detail: format!("{error:#}"),
        },
        OwnedDirectoryCreateError::CreatedButUnsealed { path, source } => {
            ExclusiveTreeCreation::CreatedNotReopened {
                detail: format!(
                    "created `{}` but could not retain its ownership seal: {source:#}",
                    path.display()
                ),
            }
        }
        OwnedDirectoryCreateError::Unsupported => ExclusiveTreeCreation::NotCreated {
            detail: TransactionError::AtomicNoReplaceUnsupported.to_string(),
        },
    }
}

fn require_project_token(
    project: &SafefsProject,
    expected: &str,
    label: &str,
) -> Result<(), TransactionError> {
    let actual = project
        .identity_token()
        .map_err(|error| TransactionError::Filesystem(format!("sealing {label}: {error:#}")))?;
    if actual == expected {
        Ok(())
    } else {
        Err(TransactionError::ThirdState(format!(
            "{label} identity changed since planning"
        )))
    }
}

#[cfg(windows)]
fn ensure_supported() -> Result<(), TransactionError> {
    Ok(())
}

#[cfg(not(windows))]
fn ensure_supported() -> Result<(), TransactionError> {
    Err(TransactionError::AtomicNoReplaceUnsupported)
}

fn fs_error(context: &'static str) -> impl FnOnce(anyhow::Error) -> TransactionError {
    move |error| TransactionError::Filesystem(format!("{context}: {error:#}"))
}

fn require_namespace_checkpoint(
    durability: DirectoryDurability,
    label: &str,
) -> Result<(), TransactionError> {
    if namespace_checkpoint_satisfied(durability) {
        Ok(())
    } else {
        Err(TransactionError::Filesystem(format!(
            "{label} did not provide durable metadata sync: {durability:?}"
        )))
    }
}

fn namespace_checkpoint_satisfied(durability: DirectoryDurability) -> bool {
    matches!(
        durability,
        DirectoryDurability::Synced | DirectoryDurability::JournalRecoverable
    ) || cfg!(windows) && matches!(durability, DirectoryDurability::Unsupported(_))
}

fn require_durable_write(
    write: &vibe_safefs::DurableWrite,
    path: &str,
) -> Result<(), TransactionError> {
    if !write.file_synced {
        return Err(TransactionError::Filesystem(format!(
            "`{path}` data was not synced"
        )));
    }
    require_namespace_checkpoint(write.parent, &format!("parent of `{path}`"))?;
    for sync in &write.directory_syncs {
        require_namespace_checkpoint(
            sync.durability,
            &format!("created-directory parent `{}`", sync.directory.display()),
        )?;
    }
    Ok(())
}

fn durable_write(
    project: &SafefsProject,
    root: &Pinned,
    path: &str,
    bytes: &[u8],
    mode: Option<u32>,
    stage_name: &str,
) -> Result<(), TransactionError> {
    let write = project
        .write_atomic_transactional_in_with_mode(root, path, bytes, mode, stage_name)
        .map_err(|error| TransactionError::Filesystem(format!("writing `{path}`: {error:#}")))?;
    require_durable_write(&write, path)
}

fn remove_transaction_stage(
    root: &Pinned,
    target_path: &str,
    stage_name: &str,
    expected: &FileState,
) -> Result<(), TransactionError> {
    let Some((parent, _)) = holder(root, target_path, false)? else {
        return Err(TransactionError::ThirdState(format!(
            "transaction stage parent for `{target_path}` is absent"
        )));
    };
    let Some(actual) = parent
        .inspect_transaction_stage_state(stage_name)
        .map_err(fs_error("inspecting deterministic transaction stage"))?
    else {
        return Ok(());
    };
    if actual.kind != EntryStateKind::File
        || actual.sha256.as_deref().map(model_digest).as_ref() != Some(&expected.sha256)
        || actual.bytes != Some(expected.bytes)
        || actual.unix_mode != expected.mode
    {
        return Err(TransactionError::ThirdState(format!(
            "transaction stage `{stage_name}` differs from its durable intent"
        )));
    }
    let durability = parent
        .remove_child_expected(stage_name, &actual)
        .map_err(map_cleanup)?;
    require_namespace_checkpoint(durability, "transaction stage parent")
}

pub(super) fn transaction_stage_name(owner: &str, operation: &str, path: &str) -> String {
    let mut hash = Sha256::new();
    hash.update(b"vibe-scrape-file-stage-e1\0");
    hash.update(owner.as_bytes());
    hash.update(b"\0");
    hash.update(operation.as_bytes());
    hash.update(b"\0");
    hash.update(path.as_bytes());
    let digest = format!("{:x}", hash.finalize());
    format!(".vibe-stage-tx-{}", &digest[..32])
}

fn map_cleanup(error: OwnedTreeCleanupError) -> TransactionError {
    match error {
        OwnedTreeCleanupError::Third { detail } => TransactionError::ThirdState(detail),
        OwnedTreeCleanupError::Unsupported => TransactionError::AtomicNoReplaceUnsupported,
        OwnedTreeCleanupError::Io(error) => {
            TransactionError::Filesystem(format!("owned-tree cleanup: {error:#}"))
        }
    }
}

fn map_rename(error: RenameError) -> TransactionError {
    match error {
        RenameError::SourceChanged { path, detail } => TransactionError::ThirdState(format!(
            "rename source `{}` changed: {detail}",
            path.display()
        )),
        RenameError::Occupied { path } => TransactionError::ThirdState(format!(
            "rename destination `{}` is occupied",
            path.display()
        )),
        RenameError::PossiblyMoved {
            source,
            destination,
            detail,
        } => TransactionError::ThirdState(format!(
            "rename `{}` -> `{}` possibly moved a third state: {detail}",
            source.display(),
            destination.display()
        )),
        RenameError::CrossFilesystem => TransactionError::Filesystem(
            "capability-relative rename crossed filesystem boundaries".to_owned(),
        ),
        RenameError::Unsupported => TransactionError::AtomicNoReplaceUnsupported,
        RenameError::Failed(error) => {
            TransactionError::Filesystem(format!("capability-relative rename: {error:#}"))
        }
    }
}

fn observe_unowned_name(
    parent: &Pinned,
    name: &str,
) -> Result<OwnedTreeObservation, TransactionError> {
    match parent.open_child_checked(name) {
        Ok(None) => Ok(OwnedTreeObservation::Absent),
        Ok(Some(_)) => Ok(OwnedTreeObservation::Third {
            detail: format!("`{name}` exists without a live safefs ownership handle"),
        }),
        Err(error) => Ok(OwnedTreeObservation::Third {
            detail: format!("`{name}` cannot be opened no-follow: {error:#}"),
        }),
    }
}

fn observe_expected_absence(
    parent: &Pinned,
    name: &str,
) -> Result<OwnedTreeObservation, TransactionError> {
    match parent.open_child_checked(name) {
        Ok(None) => Ok(OwnedTreeObservation::Absent),
        Ok(Some(_)) => Ok(OwnedTreeObservation::Third {
            detail: format!("unexpected directory `{name}` occupies the other export slot"),
        }),
        Err(error) => Ok(OwnedTreeObservation::Third {
            detail: format!("other export slot `{name}` is unsafe: {error:#}"),
        }),
    }
}

fn map_safefs_observation(
    observation: SafefsTreeObservation,
) -> Result<OwnedTreeObservation, TransactionError> {
    Ok(match observation {
        SafefsTreeObservation::Absent => OwnedTreeObservation::Absent,
        SafefsTreeObservation::MatchesAtObservation(manifest) => {
            OwnedTreeObservation::Exact(model_manifest(&manifest))
        }
        SafefsTreeObservation::Third { detail } => OwnedTreeObservation::Third { detail },
    })
}

fn model_manifest(manifest: &SafefsTreeManifest) -> TreeManifest {
    let entries = manifest
        .entries
        .iter()
        .map(|entry| TreeEntry {
            path: entry.path.clone(),
            kind: match entry.state.kind {
                vibe_safefs::EntryStateKind::File => TreeEntryKind::File,
                vibe_safefs::EntryStateKind::Directory => TreeEntryKind::Directory,
            },
            sha256: entry.state.sha256.as_deref().map(model_digest),
            bytes: entry.state.bytes,
            mode: entry.state.unix_mode,
        })
        .collect::<Vec<_>>();
    transaction_manifest(entries)
}

fn model_manifest_without_stage(
    manifest: &SafefsTreeManifest,
    stage_path: Option<&str>,
) -> TreeManifest {
    let mut model = model_manifest(manifest);
    if let Some(stage_path) = stage_path {
        model.entries.retain(|entry| entry.path != stage_path);
        model = transaction_manifest(model.entries);
    }
    model
}

fn strip_owned_observation_stage(
    observation: OwnedTreeObservation,
    stage_path: Option<&str>,
) -> Result<OwnedTreeObservation, TransactionError> {
    Ok(match (observation, stage_path) {
        (OwnedTreeObservation::Exact(mut manifest), Some(stage_path)) => {
            manifest.entries.retain(|entry| entry.path != stage_path);
            OwnedTreeObservation::Exact(transaction_manifest(manifest.entries))
        }
        (observation, _) => observation,
    })
}

fn recovery_owned_stage_path(
    journal: &Journal,
    owner: &str,
    actual: &TreeManifest,
) -> Result<Option<String>, TransactionError> {
    let Some((stage_path, expected)) = expected_owned_stage(journal, owner) else {
        return Ok(None);
    };
    let Some(stage) = actual.entries.iter().find(|entry| entry.path == stage_path) else {
        return Ok(None);
    };
    if stage.kind != TreeEntryKind::File
        || stage.sha256 != expected.sha256
        || stage.bytes != expected.bytes
        || stage.mode != expected.mode
    {
        return Err(TransactionError::ThirdState(format!(
            "transaction stage `{stage_path}` differs from its durable intent"
        )));
    }
    Ok(Some(stage_path))
}

fn authorized_owned_stage_in_seal(
    journal: &Journal,
    owner: &str,
    seal: &OwnedTreeSeal,
) -> Result<Option<String>, TransactionError> {
    let Some((stage_path, expected)) = expected_owned_stage(journal, owner) else {
        return Ok(None);
    };
    let Some(stage) = seal.entries.iter().find(|entry| entry.path == stage_path) else {
        return Ok(None);
    };
    if stage.kind != TreeEntryKind::File
        || stage.sha256 != expected.sha256
        || stage.bytes != expected.bytes
        || stage.mode != expected.mode
    {
        return Err(TransactionError::ThirdState(format!(
            "journaled transaction stage `{stage_path}` differs from its durable intent"
        )));
    }
    Ok(Some(stage_path))
}

fn expected_owned_stage(journal: &Journal, owner: &str) -> Option<(String, TreeEntry)> {
    match &journal.execution {
        PreparedMode::Export(plan) => journal.active_step.and_then(|index| {
            let entry = plan.entries.get(index)?;
            let final_entry = plan
                .final_manifest
                .entries
                .iter()
                .find(|candidate| candidate.path == entry.target_path)?;
            (entry.kind == TreeEntryKind::File).then(|| {
                let name = transaction_stage_name(
                    owner,
                    &format!("export:{}", entry.target_path),
                    &entry.target_path,
                );
                (
                    stage_relative_path(&entry.target_path, &name),
                    final_entry.clone(),
                )
            })
        }),
        PreparedMode::InPlace(plan) => journal.active_step.and_then(|index| {
            let step = if index < plan.steps.len() {
                &plan.steps[index]
            } else if index == plan.steps.len() {
                &plan.contract_step
            } else {
                return None;
            };
            if step.kind != MutationKind::CaptureBeforeImage {
                return None;
            }
            let transition = step.transitions.iter().find(|transition| {
                transition.location == Location::Quarantine
                    && matches!(transition.after, PathState::File(_))
            })?;
            let PathState::File(file) = &transition.after else {
                return None;
            };
            let name =
                transaction_stage_name(owner, &format!("apply:{}", step.id), &transition.path);
            Some((
                stage_relative_path(&transition.path, &name),
                TreeEntry {
                    path: stage_relative_path(&transition.path, &name),
                    kind: TreeEntryKind::File,
                    sha256: Some(file.sha256.clone()),
                    bytes: Some(file.bytes),
                    mode: file.mode,
                },
            ))
        }),
    }
}

fn stage_relative_path(target: &str, stage_name: &str) -> String {
    target.rsplit_once('/').map_or_else(
        || stage_name.to_owned(),
        |(parent, _)| format!("{parent}/{stage_name}"),
    )
}

fn transaction_manifest(entries: Vec<TreeEntry>) -> TreeManifest {
    super::logical_tree_manifest(entries)
}

fn model_cleanup_intent(intent: &SafefsCleanupIntent) -> OwnedTreeCleanupIntent {
    OwnedTreeCleanupIntent {
        intent_token: intent.intent_token.clone(),
        progress_key: intent.progress_key.clone(),
        path: intent.path.clone(),
        expected: OwnedEntrySeal {
            path: intent.path.clone(),
            kind: match intent.expected.kind {
                EntryStateKind::File => TreeEntryKind::File,
                EntryStateKind::Directory => TreeEntryKind::Directory,
            },
            sha256: intent.expected.sha256.as_deref().map(model_digest),
            bytes: intent.expected.bytes,
            mode: intent.expected.unix_mode,
            identity: intent.expected.identity.as_str().to_owned(),
        },
        root: intent.root,
    }
}

fn safefs_cleanup_intent(
    intent: &OwnedTreeCleanupIntent,
) -> Result<SafefsCleanupIntent, TransactionError> {
    if intent.expected.path != intent.path {
        return Err(TransactionError::Store(
            "cleanup intent path differs from its expected-state seal".to_owned(),
        ));
    }
    Ok(SafefsCleanupIntent {
        intent_token: intent.intent_token.clone(),
        progress_key: intent.progress_key.clone(),
        path: intent.path.clone(),
        expected: EntryState {
            kind: match intent.expected.kind {
                TreeEntryKind::File => EntryStateKind::File,
                TreeEntryKind::Directory => EntryStateKind::Directory,
            },
            sha256: intent.expected.sha256.as_ref().map(|digest| {
                digest
                    .0
                    .strip_prefix("sha256:")
                    .unwrap_or(&digest.0)
                    .to_owned()
            }),
            bytes: intent.expected.bytes,
            unix_mode: intent.expected.mode,
            identity: EntryIdentity::from_token(&intent.expected.identity)
                .map_err(fs_error("validating cleanup entry identity"))?,
        },
        root: intent.root,
    })
}

fn validate_rebound_manifest(
    journal: &Journal,
    namespace_name: &str,
    persisted: &TreeManifest,
    actual: &TreeManifest,
) -> Result<(), TransactionError> {
    if let Some(cleanup) = &journal.cleanup_wal {
        if cleanup.name != namespace_name
            || cleanup.directory_identity
                != journal
                    .owned_tree_seal
                    .as_ref()
                    .map(|seal| seal.directory_identity.as_str())
                    .unwrap_or_default()
            || cleanup.manifest_digest
                != journal
                    .owned_tree_seal
                    .as_ref()
                    .map(|seal| seal.manifest_digest.as_str())
                    .unwrap_or_default()
        {
            return Err(TransactionError::Store(
                "cleanup WAL is not bound to the journaled owned-tree seal".to_owned(),
            ));
        }
        let completed = cleanup
            .completed
            .iter()
            .cloned()
            .collect::<std::collections::BTreeSet<_>>();
        let remaining = persisted
            .entries
            .iter()
            .filter(|entry| !completed.contains(&cleanup_key(entry)))
            .cloned()
            .collect::<Vec<_>>();
        let before = transaction_manifest(remaining.clone());
        if actual == &before {
            return Ok(());
        }
        if let Some(active) = &cleanup.active {
            let after = transaction_manifest(
                remaining
                    .into_iter()
                    .filter(|entry| active.root || entry.path != active.path)
                    .collect(),
            );
            if !active.root && actual == &after {
                return Ok(());
            }
        }
        return Err(TransactionError::ThirdState(
            "rebound owned tree is neither side of the journaled cleanup intent".to_owned(),
        ));
    }
    if actual == persisted {
        return Ok(());
    }
    match &journal.execution {
        PreparedMode::Export(plan) => {
            let mut allowed = vec![transaction_manifest(
                plan.final_manifest
                    .entries
                    .iter()
                    .take(
                        journal
                            .completed_steps
                            .min(plan.final_manifest.entries.len()),
                    )
                    .cloned()
                    .collect(),
            )];
            if let Some(active) = journal.active_step {
                allowed.push(transaction_manifest(
                    plan.final_manifest
                        .entries
                        .iter()
                        .take((active + 1).min(plan.final_manifest.entries.len()))
                        .cloned()
                        .collect(),
                ));
            }
            if allowed.contains(actual) {
                Ok(())
            } else {
                Err(TransactionError::ThirdState(
                    "rebound export tree is outside the durable prefix intent".to_owned(),
                ))
            }
        }
        PreparedMode::InPlace(plan) => {
            let Some(active_index) = journal.active_step else {
                return Err(TransactionError::ThirdState(
                    "rebound quarantine changed without an active mutation intent".to_owned(),
                ));
            };
            let step = if active_index < plan.steps.len() {
                &plan.steps[active_index]
            } else if active_index == plan.steps.len() {
                &plan.contract_step
            } else {
                return Err(TransactionError::Store(
                    "active in-place step exceeds the journaled plan".to_owned(),
                ));
            };
            let status = journal
                .mutation_progress
                .iter()
                .find(|progress| progress.id == step.id)
                .map(|progress| progress.status)
                .ok_or_else(|| {
                    TransactionError::Store("active step has no progress row".to_owned())
                })?;
            let after = rebound_in_place_manifest(persisted, step, status)?;
            if actual == &after {
                Ok(())
            } else {
                Err(TransactionError::ThirdState(
                    "rebound quarantine is neither side of the active mutation intent".to_owned(),
                ))
            }
        }
    }
}

fn cleanup_key(entry: &TreeEntry) -> String {
    format!(
        "{}:{}",
        match entry.kind {
            TreeEntryKind::File => "file",
            TreeEntryKind::Directory => "directory",
        },
        entry.path
    )
}

fn rebound_in_place_manifest(
    persisted: &TreeManifest,
    step: &MutationStep,
    status: super::MutationStatus,
) -> Result<TreeManifest, TransactionError> {
    let apply = match status {
        super::MutationStatus::ApplyIntent => true,
        super::MutationStatus::RollbackIntent => false,
        _ => {
            return Err(TransactionError::Store(
                "changed rebound tree lacks an apply/rollback intent".to_owned(),
            ));
        }
    };
    let mut entries = persisted
        .entries
        .iter()
        .cloned()
        .map(|entry| (entry.path.clone(), entry))
        .collect::<BTreeMap<_, _>>();
    for transition in step
        .transitions
        .iter()
        .filter(|transition| transition.location == Location::Quarantine)
    {
        set_flat_state(
            &mut entries,
            &transition.path,
            if apply {
                &transition.after
            } else {
                &transition.before
            },
        );
    }
    if step.kind == MutationKind::PruneEmptyDirectory {
        let path = parked_directory(step);
        if apply {
            let transition = one_at(step, Location::Project)?;
            set_flat_state(&mut entries, &path, &transition.before);
        } else {
            set_flat_state(&mut entries, &path, &PathState::Absent);
        }
    }
    Ok(transaction_manifest(entries.into_values().collect()))
}

fn set_flat_state(entries: &mut BTreeMap<String, TreeEntry>, path: &str, state: &PathState) {
    entries.retain(|candidate, _| candidate != path && !candidate.starts_with(&format!("{path}/")));
    match state {
        PathState::Absent => {}
        PathState::File(file) => {
            entries.insert(
                path.to_owned(),
                TreeEntry {
                    path: path.to_owned(),
                    kind: TreeEntryKind::File,
                    sha256: Some(file.sha256.clone()),
                    bytes: Some(file.bytes),
                    mode: file.mode,
                },
            );
        }
        PathState::EmptyDirectory { mode } => {
            entries.insert(
                path.to_owned(),
                TreeEntry {
                    path: path.to_owned(),
                    kind: TreeEntryKind::Directory,
                    sha256: None,
                    bytes: None,
                    mode: *mode,
                },
            );
        }
        PathState::Tree(tree) => {
            entries.insert(
                path.to_owned(),
                TreeEntry {
                    path: path.to_owned(),
                    kind: TreeEntryKind::Directory,
                    sha256: None,
                    bytes: None,
                    mode: tree.root_mode,
                },
            );
            for child in &tree.descendants {
                let child_path = format!("{path}/{}", child.relative_path);
                entries.insert(
                    child_path.clone(),
                    TreeEntry {
                        path: child_path,
                        kind: child.kind,
                        sha256: child.sha256.clone(),
                        bytes: child.bytes,
                        mode: child.mode,
                    },
                );
            }
        }
    }
}

fn model_digest(value: &str) -> Digest {
    if value.starts_with("sha256:") {
        Digest(value.to_owned())
    } else {
        Digest(format!("sha256:{value}"))
    }
}

fn digest_bytes(bytes: &[u8]) -> Digest {
    Digest(format!("sha256:{:x}", Sha256::digest(bytes)))
}

fn one_at(
    step: &MutationStep,
    location: Location,
) -> Result<&super::PathTransition, TransactionError> {
    let mut found = step
        .transitions
        .iter()
        .filter(|transition| transition.location == location);
    let answer = found
        .next()
        .ok_or_else(|| invalid_step_error(step, "required transition is absent"))?;
    if found.next().is_some() {
        return invalid_step(step, "required transition is not unique");
    }
    Ok(answer)
}

fn move_pair(
    step: &MutationStep,
) -> Result<(&super::PathTransition, &super::PathTransition), TransactionError> {
    let source = step
        .transitions
        .iter()
        .find(|transition| {
            transition.before != PathState::Absent && transition.after == PathState::Absent
        })
        .ok_or_else(|| invalid_step_error(step, "move source is absent"))?;
    let destination = step
        .transitions
        .iter()
        .find(|transition| {
            transition.before == PathState::Absent && transition.after == source.before
        })
        .ok_or_else(|| invalid_step_error(step, "move destination is absent"))?;
    Ok((source, destination))
}

fn invalid_step<T>(step: &MutationStep, detail: &str) -> Result<T, TransactionError> {
    Err(invalid_step_error(step, detail))
}

fn invalid_step_error(step: &MutationStep, detail: &str) -> TransactionError {
    TransactionError::InvalidPrepared(format!("step `{}`: {detail}", step.id))
}

fn third_step<T>(step: &MutationStep, detail: &str) -> Result<T, TransactionError> {
    Err(TransactionError::ThirdState(format!(
        "step `{}`: {detail}",
        step.id
    )))
}

fn parked_directory(step: &MutationStep) -> String {
    format!("pruned/{}", step.id)
}

fn quarantine_topology(plan: &InPlacePlan) -> Vec<String> {
    let mut directories = BTreeSet::new();
    let mut add_parents = |path: &str| {
        let components = path.split('/').collect::<Vec<_>>();
        for count in 1..components.len() {
            directories.insert(components[..count].join("/"));
        }
    };
    for step in plan
        .steps
        .iter()
        .chain(std::iter::once(&plan.contract_step))
        .chain(plan.contract_cleanup_step.iter())
    {
        for transition in step
            .transitions
            .iter()
            .filter(|transition| transition.location == Location::Quarantine)
        {
            add_parents(&transition.path);
        }
        if step.kind == MutationKind::PruneEmptyDirectory {
            add_parents(&parked_directory(step));
        }
    }
    let mut directories = directories.into_iter().collect::<Vec<_>>();
    directories.sort_by(|left, right| {
        path_depth(left)
            .cmp(&path_depth(right))
            .then_with(|| left.as_bytes().cmp(right.as_bytes()))
    });
    directories
}

fn path_depth(path: &str) -> usize {
    path.bytes().filter(|byte| *byte == b'/').count()
}

enum HeldParent<'a> {
    Root(&'a Pinned),
    Child(Pinned),
}

impl std::ops::Deref for HeldParent<'_> {
    type Target = Pinned;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Root(root) => root,
            Self::Child(child) => child,
        }
    }
}

fn holder<'a>(
    base: &'a Pinned,
    relative: &str,
    create_parents: bool,
) -> Result<Option<(HeldParent<'a>, String)>, TransactionError> {
    let (parents, name) = vibe_safefs::split_relative(relative)
        .map_err(fs_error("splitting capability-relative path"))?;
    let mut parents = parents.into_iter();
    let Some(first) = parents.next() else {
        return Ok(Some((HeldParent::Root(base), name)));
    };
    let mut directory = match base.open_child_checked(&first) {
        Ok(Some(child)) => child,
        Ok(None) if create_parents => create_parent(base, &first, relative)?,
        Ok(None) => return Ok(None),
        Err(error) => {
            return Err(TransactionError::ThirdState(format!(
                "parent `{first}` for `{relative}` is unsafe: {error:#}"
            )));
        }
    };
    for component in parents {
        match directory.open_child_checked(&component) {
            Ok(Some(child)) => directory = child,
            Ok(None) if create_parents => {
                directory = create_parent(&directory, &component, relative)?;
            }
            Ok(None) => return Ok(None),
            Err(error) => {
                return Err(TransactionError::ThirdState(format!(
                    "parent `{component}` for `{relative}` is unsafe: {error:#}"
                )));
            }
        }
    }
    Ok(Some((HeldParent::Child(directory), name)))
}

fn create_parent(
    parent: &Pinned,
    component: &str,
    relative: &str,
) -> Result<Pinned, TransactionError> {
    match parent.create_child_exclusive(component) {
        Ok(child) => {
            require_namespace_checkpoint(
                parent.sync_directory(),
                &format!("parent created for `{relative}`"),
            )?;
            Ok(child)
        }
        Err(vibe_safefs::ExclusiveChildError::NotCreated(error)) => {
            Err(TransactionError::ThirdState(format!(
                "parent `{component}` for `{relative}` was raced: {error:#}"
            )))
        }
        Err(vibe_safefs::ExclusiveChildError::CreatedNotReopened { path, source }) => {
            Err(TransactionError::ThirdState(format!(
                "created parent `{}` could not be reopened: {source:#}",
                path.display()
            )))
        }
    }
}

fn require_absent(root: &Pinned, relative: &str) -> Result<(), TransactionError> {
    let Some((parent, name)) = holder(root, relative, false)? else {
        return Ok(());
    };
    match parent.inspect_child_state(&name) {
        Ok(None) => Ok(()),
        Ok(Some(_)) => Err(TransactionError::ThirdState(format!(
            "destination `{relative}` is occupied"
        ))),
        Err(error) => Err(TransactionError::ThirdState(format!(
            "destination `{relative}` cannot be inspected: {error:#}"
        ))),
    }
}

fn create_directory_exact(
    project: &SafefsProject,
    root: &Pinned,
    relative: &str,
    mode: Option<u32>,
) -> Result<(), TransactionError> {
    let state = PathState::EmptyDirectory { mode };
    if state_matches(project, root, relative, &state)? {
        return Ok(());
    }
    let (parent, name) = holder(root, relative, true)?
        .ok_or_else(|| TransactionError::Filesystem(format!("parent of `{relative}` is absent")))?;
    if parent
        .inspect_child_state(&name)
        .map_err(fs_error("inspecting directory destination"))?
        .is_some()
    {
        return Err(TransactionError::ThirdState(format!(
            "directory destination `{relative}` is occupied"
        )));
    }
    let (child, durability) = parent
        .create_child_exclusive_journaled(&name)
        .map_err(|error| {
            TransactionError::ThirdState(format!(
                "exclusive directory creation for `{relative}` failed: {error}"
            ))
        })?;
    require_namespace_checkpoint(durability, &format!("parent of `{relative}`"))?;
    if child
        .unix_mode()
        .map_err(fs_error("reading created directory mode"))?
        != mode
    {
        return Err(TransactionError::ThirdState(format!(
            "created directory `{relative}` has an unexpected mode"
        )));
    }
    Ok(())
}

fn remove_empty_directory_exact(
    project: &SafefsProject,
    root: &Pinned,
    relative: &str,
    mode: Option<u32>,
) -> Result<(), TransactionError> {
    if !state_matches(project, root, relative, &PathState::EmptyDirectory { mode })? {
        return Err(TransactionError::ThirdState(format!(
            "directory `{relative}` is not the sealed empty state"
        )));
    }
    let (parent, name) = holder(root, relative, false)?.ok_or_else(|| {
        TransactionError::ThirdState(format!("directory `{relative}` disappeared"))
    })?;
    let actual = parent
        .inspect_child_state(&name)
        .map_err(fs_error("sealing empty directory for removal"))?
        .ok_or_else(|| {
            TransactionError::ThirdState(format!("directory `{relative}` disappeared"))
        })?;
    let durability = parent
        .remove_child_expected(&name, &actual)
        .map_err(map_cleanup)?;
    require_namespace_checkpoint(durability, &format!("parent of `{relative}`"))
}

fn remove_file_exact(
    project: &SafefsProject,
    root: &Pinned,
    relative: &str,
    expected: &FileState,
) -> Result<(), TransactionError> {
    if !state_matches(project, root, relative, &PathState::File(expected.clone()))? {
        return Err(TransactionError::ThirdState(format!(
            "file `{relative}` is not its sealed state"
        )));
    }
    let (parent, name) = holder(root, relative, false)?
        .ok_or_else(|| TransactionError::ThirdState(format!("file `{relative}` disappeared")))?;
    let actual = parent
        .inspect_child_state(&name)
        .map_err(fs_error("sealing file for removal"))?
        .ok_or_else(|| TransactionError::ThirdState(format!("file `{relative}` disappeared")))?;
    let durability = parent
        .remove_child_expected(&name, &actual)
        .map_err(map_cleanup)?;
    require_namespace_checkpoint(durability, &format!("parent of `{relative}`"))
}

#[allow(clippy::too_many_arguments)]
fn rename_exact(
    project: &SafefsProject,
    source_root: &Pinned,
    source_path: &str,
    destination_root: &Pinned,
    destination_path: &str,
    expected: &PathState,
    create_destination_parents: bool,
) -> Result<(), TransactionError> {
    if !state_matches(project, source_root, source_path, expected)? {
        return Err(TransactionError::ThirdState(format!(
            "rename source `{source_path}` is not its sealed state"
        )));
    }
    let (source_parent, source_name) = holder(source_root, source_path, false)?
        .ok_or_else(|| TransactionError::ThirdState(format!("`{source_path}` disappeared")))?;
    let (destination_parent, destination_name) = holder(
        destination_root,
        destination_path,
        create_destination_parents,
    )?
    .ok_or_else(|| {
        TransactionError::ThirdState(format!("parent of `{destination_path}` is absent"))
    })?;
    if destination_parent
        .inspect_child_state(&destination_name)
        .map_err(fs_error("inspecting rename destination"))?
        .is_some()
    {
        return Err(TransactionError::ThirdState(format!(
            "rename destination `{destination_path}` is occupied"
        )));
    }
    let source_state = source_parent
        .inspect_child_state(&source_name)
        .map_err(fs_error("sealing rename source"))?
        .ok_or_else(|| TransactionError::ThirdState(format!("`{source_path}` disappeared")))?;
    let durability = source_parent
        .rename_child_noreplace_to_durable(
            &destination_parent,
            &source_name,
            &destination_name,
            &source_state,
        )
        .map_err(map_rename)?;
    require_namespace_checkpoint(
        durability,
        &format!("journaled rename `{source_path}` -> `{destination_path}`"),
    )
}

fn read_sealed_file(
    project: &SafefsProject,
    relative: &str,
    expected: &FileState,
) -> Result<Vec<u8>, TransactionError> {
    let root = project
        .root_dir()
        .map_err(fs_error("pinning source project"))?;
    read_sealed_file_at(project, &root, relative, expected)
}

fn read_sealed_file_at(
    project: &SafefsProject,
    root: &Pinned,
    relative: &str,
    expected: &FileState,
) -> Result<Vec<u8>, TransactionError> {
    let expected_state = PathState::File(expected.clone());
    if !state_matches(project, root, relative, &expected_state)? {
        return Err(TransactionError::ThirdState(format!(
            "source file `{relative}` differs from its sealed before state"
        )));
    }
    let bytes = project
        .read_file_in(root, relative)
        .map_err(fs_error("reading sealed source file"))?
        .ok_or_else(|| TransactionError::ThirdState(format!("`{relative}` disappeared")))?;
    if digest_bytes(&bytes) != expected.sha256 || bytes.len() as u64 != expected.bytes {
        return Err(TransactionError::ThirdState(format!(
            "source file `{relative}` changed while it was copied"
        )));
    }
    if !state_matches(project, root, relative, &expected_state)? {
        return Err(TransactionError::ThirdState(format!(
            "source file `{relative}` changed during its copy"
        )));
    }
    Ok(bytes)
}

fn state_matches(
    project: &SafefsProject,
    root: &Pinned,
    relative: &str,
    expected: &PathState,
) -> Result<bool, TransactionError> {
    let Some((parent, name)) = holder(root, relative, false)? else {
        return Ok(*expected == PathState::Absent);
    };
    let actual = parent.inspect_child_state(&name).map_err(|error| {
        TransactionError::ThirdState(format!(
            "observing `{relative}` no-follow failed: {error:#}"
        ))
    })?;
    match expected {
        PathState::Absent => Ok(actual.is_none()),
        PathState::File(file) => Ok(actual.is_some_and(|state| {
            state.kind == vibe_safefs::EntryStateKind::File
                && state.sha256.as_deref().map(model_digest).as_ref() == Some(&file.sha256)
                && state.bytes == Some(file.bytes)
                && state.unix_mode == file.mode
        })),
        PathState::EmptyDirectory { mode } => {
            let Some(state) = actual else {
                return Ok(false);
            };
            if state.kind != vibe_safefs::EntryStateKind::Directory || state.unix_mode != *mode {
                return Ok(false);
            }
            let child = parent
                .open_child(&name)
                .map_err(fs_error("opening expected empty directory"))?;
            Ok(project
                .child_names(&child)
                .map_err(fs_error("enumerating expected empty directory"))?
                .is_empty())
        }
        PathState::Tree(tree) => {
            let Some(state) = actual else {
                return Ok(false);
            };
            if state.kind != vibe_safefs::EntryStateKind::Directory
                || state.unix_mode != tree.root_mode
            {
                return Ok(false);
            }
            let child = parent
                .open_child(&name)
                .map_err(fs_error("opening expected subtree"))?;
            let mut descendants = Vec::new();
            collect_subtree(project, &child, "", &mut descendants)?;
            descendants.sort_by(|left, right| {
                left.relative_path
                    .as_bytes()
                    .cmp(right.relative_path.as_bytes())
            });
            Ok(descendants == tree.descendants)
        }
    }
}

fn collect_subtree(
    project: &SafefsProject,
    directory: &Pinned,
    prefix: &str,
    answer: &mut Vec<super::SubtreeEntry>,
) -> Result<(), TransactionError> {
    let mut names = project
        .child_names(directory)
        .map_err(fs_error("enumerating sealed subtree"))?;
    names.sort_by(|left, right| left.as_bytes().cmp(right.as_bytes()));
    for name in names {
        let state = directory
            .inspect_child_state(&name)
            .map_err(fs_error("inspecting sealed subtree entry"))?
            .ok_or_else(|| {
                TransactionError::ThirdState("subtree entry vanished during observation".to_owned())
            })?;
        let path = if prefix.is_empty() {
            name.clone()
        } else {
            format!("{prefix}/{name}")
        };
        answer.push(super::SubtreeEntry {
            relative_path: path.clone(),
            kind: match state.kind {
                vibe_safefs::EntryStateKind::File => TreeEntryKind::File,
                vibe_safefs::EntryStateKind::Directory => TreeEntryKind::Directory,
            },
            sha256: state.sha256.as_deref().map(model_digest),
            bytes: state.bytes,
            mode: state.unix_mode,
        });
        if state.kind == vibe_safefs::EntryStateKind::Directory {
            let child = directory
                .open_child(&name)
                .map_err(fs_error("opening sealed subtree directory"))?;
            collect_subtree(project, &child, &path, answer)?;
        }
    }
    Ok(())
}

fn model_tree_at(project: &SafefsProject, root: &Pinned) -> Result<TreeManifest, TransactionError> {
    let mut observed = Vec::new();
    collect_subtree(project, root, "", &mut observed)?;
    Ok(transaction_manifest(
        observed
            .into_iter()
            .map(|entry| TreeEntry {
                path: entry.relative_path,
                kind: entry.kind,
                sha256: entry.sha256,
                bytes: entry.bytes,
                mode: entry.mode,
            })
            .collect(),
    ))
}

#[cfg(all(test, windows))]
mod tests {
    use std::fs;

    use super::*;
    use crate::transaction::{ContractCommit, PathTransition};

    fn file(bytes: &[u8]) -> FileState {
        FileState {
            sha256: digest_bytes(bytes),
            bytes: bytes.len() as u64,
            mode: None,
        }
    }

    fn entry(path: &str, bytes: &[u8]) -> TreeEntry {
        TreeEntry {
            path: path.to_owned(),
            kind: TreeEntryKind::File,
            sha256: Some(digest_bytes(bytes)),
            bytes: Some(bytes.len() as u64),
            mode: None,
        }
    }

    fn cleanup_journal(
        execution: PreparedMode,
        name: &str,
        owner: &str,
        seal: OwnedTreeSeal,
    ) -> Journal {
        Journal {
            schema: 1,
            revision: 0,
            project_key: super::super::ProjectKey("project".to_owned()),
            transaction_id: super::super::TransactionId("TXN001".to_owned()),
            mode: match &execution {
                PreparedMode::Export(_) => super::super::TransactionMode::Export,
                PreparedMode::InPlace(_) => super::super::TransactionMode::InPlace,
            },
            plan_id: Digest(format!("sha256:{}", "0".repeat(64))),
            canonical_plan: Vec::new(),
            verification_workspace: None,
            project_display_root: "test".to_owned(),
            execution,
            state: super::super::TransactionState::RollingBack,
            snapshots: Vec::new(),
            snapshots_persisted: 0,
            snapshot_active: None,
            candidate_name: name
                .starts_with(".vibe-scrape-candidate-")
                .then(|| name.to_owned()),
            quarantine_name: name
                .starts_with(".vibe-scrape-quarantine-")
                .then(|| name.to_owned()),
            owned_tree_token: Some(owner.to_owned()),
            owned_tree_seal: Some(seal),
            cleanup_wal: None,
            completed_steps: 0,
            active_step: None,
            mutation_progress: Vec::new(),
            actual_mutations: Vec::new(),
            settlement_intent: None,
            delivered_tree: None,
            verification: Vec::new(),
            events: Vec::new(),
            report: None,
        }
    }

    fn cleanup_tree(
        adapter: &mut SafefsTransactionFilesystem,
        execution: PreparedMode,
        name: &str,
        owner: &str,
    ) {
        let seal = adapter.owned_tree_seal(name, owner).unwrap();
        let journal = cleanup_journal(execution, name, owner, seal.clone());
        let mut completed = Vec::new();
        loop {
            let OwnedTreeCleanupPreparation::Intent(intent) = adapter
                .prepare_owned_tree_cleanup(&journal, name, owner, &seal, &completed)
                .unwrap()
            else {
                break;
            };
            let completion = adapter
                .execute_owned_tree_cleanup(&journal, name, owner, &seal, &completed, &intent)
                .unwrap();
            completed.push(completion.progress_key);
        }
    }

    fn export_fixture() -> (
        tempfile::TempDir,
        PathBuf,
        ExportPlan,
        SafefsTransactionFilesystem,
    ) {
        let scope = tempfile::tempdir().unwrap();
        let source = scope.path().join("source");
        fs::create_dir(&source).unwrap();
        fs::write(source.join("kept.txt"), b"kept").unwrap();
        let output = scope.path().join("scraped");
        let source_project = SafefsProject::open(&source).unwrap();
        let output_parent = SafefsProject::open(scope.path()).unwrap();
        let output_slot = SafefsProject::pin_absent_path(&output).unwrap();
        let final_manifest = transaction_manifest(vec![entry("kept.txt", b"kept")]);
        let plan = ExportPlan {
            output_identity: output_slot.identity_token(),
            output_parent_identity: output_parent.identity_token().unwrap(),
            output_display_path: output.display().to_string(),
            output_name: "scraped".to_owned(),
            before_same_display_path: false,
            after_same_display_path: false,
            entries: vec![ExportEntry {
                target_path: "kept.txt".to_owned(),
                kind: TreeEntryKind::File,
                mode: None,
                payload: Some(ExportPayload::Source {
                    source_path: "kept.txt".to_owned(),
                    before: file(b"kept"),
                }),
            }],
            source_tree: transaction_manifest(vec![entry("kept.txt", b"kept")]),
            final_manifest,
        };
        let adapter =
            SafefsTransactionFilesystem::open(&source, &source_project.identity_token().unwrap())
                .unwrap();
        (scope, output, plan, adapter)
    }

    #[test]
    fn export_publish_and_exact_rollback() {
        let (_scope, output, plan, mut adapter) = export_fixture();
        let candidate = ".vibe-scrape-candidate-TXN001";
        let owner = "owner-export";
        assert_eq!(
            adapter
                .create_export_candidate(&plan, candidate, owner)
                .unwrap(),
            ExclusiveTreeCreation::Owned
        );
        adapter
            .apply_export_entry(&plan, candidate, owner, &plan.entries[0], None)
            .unwrap();
        assert_eq!(
            adapter
                .observe_export_tree(&plan, ExportTreeSlot::Candidate, candidate, owner)
                .unwrap(),
            OwnedTreeObservation::Exact(plan.final_manifest.clone())
        );
        adapter
            .publish_export_noreplace(&plan, candidate, owner)
            .unwrap();
        assert_eq!(fs::read(output.join("kept.txt")).unwrap(), b"kept");
        assert_eq!(
            adapter
                .observe_export_tree(&plan, ExportTreeSlot::Output, candidate, owner)
                .unwrap(),
            OwnedTreeObservation::Exact(plan.final_manifest.clone())
        );
        adapter.unpublish_export(&plan, candidate, owner).unwrap();
        cleanup_tree(
            &mut adapter,
            PreparedMode::Export(Box::new(plan.clone())),
            candidate,
            owner,
        );
        assert!(!output.exists());
        assert!(!output.parent().unwrap().join(candidate).exists());
    }

    #[test]
    fn restart_never_adopts_a_created_root_before_its_first_identity_seal() {
        let (scope, _output, plan, mut adapter) = export_fixture();
        let candidate = ".vibe-scrape-candidate-TXN004";
        let owner = "owner-unsealed";
        assert_eq!(
            adapter
                .create_export_candidate(&plan, candidate, owner)
                .unwrap(),
            ExclusiveTreeCreation::Owned
        );
        let seal = adapter.owned_tree_seal(candidate, owner).unwrap();
        let mut journal =
            cleanup_journal(PreparedMode::Export(Box::new(plan)), candidate, owner, seal);
        journal.state = super::super::TransactionState::Prepared;
        journal.owned_tree_seal = None;
        drop(adapter);

        let source = scope.path().join("source");
        let project = SafefsProject::open(&source).unwrap();
        let mut restarted =
            SafefsTransactionFilesystem::open(&source, &project.identity_token().unwrap()).unwrap();
        assert!(matches!(
            restarted.rebind_from_journal(&journal),
            Err(TransactionError::ThirdState(message))
                if message.contains("automatic adoption is forbidden")
        ));
        assert!(scope.path().join(candidate).is_dir());
    }

    #[test]
    fn restart_rebinds_a_manifest_reduced_by_durable_cleanup_progress() {
        let (scope, _output, plan, mut adapter) = export_fixture();
        let candidate = ".vibe-scrape-candidate-TXN005";
        let owner = "owner-cleanup-restart";
        adapter
            .create_export_candidate(&plan, candidate, owner)
            .unwrap();
        adapter
            .apply_export_entry(&plan, candidate, owner, &plan.entries[0], None)
            .unwrap();
        let seal = adapter.owned_tree_seal(candidate, owner).unwrap();
        let mut journal = cleanup_journal(
            PreparedMode::Export(Box::new(plan.clone())),
            candidate,
            owner,
            seal.clone(),
        );
        journal.cleanup_wal = Some(super::super::OwnedTreeCleanupWal {
            name: candidate.to_owned(),
            directory_identity: seal.directory_identity.clone(),
            manifest_digest: seal.manifest_digest.clone(),
            completed: Vec::new(),
            active: None,
        });
        let OwnedTreeCleanupPreparation::Intent(first) = adapter
            .prepare_owned_tree_cleanup(&journal, candidate, owner, &seal, &[])
            .unwrap()
        else {
            panic!("non-empty candidate must have a cleanup entry")
        };
        journal.cleanup_wal.as_mut().unwrap().active = Some(first.clone());
        let completion = adapter
            .execute_owned_tree_cleanup(&journal, candidate, owner, &seal, &[], &first)
            .unwrap();
        let wal = journal.cleanup_wal.as_mut().unwrap();
        wal.completed.push(completion.progress_key);
        wal.active = None;
        drop(adapter);

        let source = scope.path().join("source");
        let project = SafefsProject::open(&source).unwrap();
        let mut restarted =
            SafefsTransactionFilesystem::open(&source, &project.identity_token().unwrap()).unwrap();
        restarted.rebind_from_journal(&journal).unwrap();
        let mut completed = journal.cleanup_wal.as_ref().unwrap().completed.clone();
        loop {
            let OwnedTreeCleanupPreparation::Intent(intent) = restarted
                .prepare_owned_tree_cleanup(&journal, candidate, owner, &seal, &completed)
                .unwrap()
            else {
                break;
            };
            let completion = restarted
                .execute_owned_tree_cleanup(&journal, candidate, owner, &seal, &completed, &intent)
                .unwrap();
            completed.push(completion.progress_key);
        }
        assert!(!scope.path().join(candidate).exists());
    }

    #[test]
    fn export_output_race_preserves_occupant_and_cleans_candidate() {
        let (_scope, output, plan, mut adapter) = export_fixture();
        let candidate = ".vibe-scrape-candidate-TXN002";
        let owner = "owner-race";
        assert_eq!(
            adapter
                .create_export_candidate(&plan, candidate, owner)
                .unwrap(),
            ExclusiveTreeCreation::Owned
        );
        adapter
            .apply_export_entry(&plan, candidate, owner, &plan.entries[0], None)
            .unwrap();
        fs::create_dir(&output).unwrap();
        fs::write(output.join("foreign.txt"), b"foreign").unwrap();
        assert!(matches!(
            adapter.publish_export_noreplace(&plan, candidate, owner),
            Err(TransactionError::OutputRace(_))
        ));
        cleanup_tree(
            &mut adapter,
            PreparedMode::Export(Box::new(plan.clone())),
            candidate,
            owner,
        );
        assert_eq!(fs::read(output.join("foreign.txt")).unwrap(), b"foreign");
    }

    #[test]
    fn export_file_stage_rebinds_before_and_after_publication() {
        for staged_before_publish in [true, false] {
            let (scope, _output, plan, mut adapter) = export_fixture();
            let candidate = if staged_before_publish {
                ".vibe-scrape-candidate-TXN008"
            } else {
                ".vibe-scrape-candidate-TXN009"
            };
            let owner = "owner-export-stage";
            adapter
                .create_export_candidate(&plan, candidate, owner)
                .unwrap();
            let before_seal = adapter.owned_tree_seal(candidate, owner).unwrap();
            let stage = transaction_stage_name(
                owner,
                &format!("export:{}", plan.entries[0].target_path),
                &plan.entries[0].target_path,
            );
            if staged_before_publish {
                fs::write(scope.path().join(candidate).join(&stage), b"kept").unwrap();
            } else {
                adapter
                    .apply_export_entry(&plan, candidate, owner, &plan.entries[0], None)
                    .unwrap();
            }
            let mut journal = cleanup_journal(
                PreparedMode::Export(Box::new(plan.clone())),
                candidate,
                owner,
                before_seal,
            );
            journal.state = super::super::TransactionState::Prepared;
            journal.active_step = Some(0);
            journal.mutation_progress = vec![super::super::MutationProgress {
                id: "export/entry/0/kept.txt".into(),
                kind: super::super::PlannedMutationKind::ExportEntry,
                status: super::super::MutationStatus::ApplyIntent,
            }];
            drop(adapter);
            let source = scope.path().join("source");
            let project = SafefsProject::open(&source).unwrap();
            let mut restarted =
                SafefsTransactionFilesystem::open(&source, &project.identity_token().unwrap())
                    .unwrap();
            restarted.rebind_from_journal(&journal).unwrap();
            if staged_before_publish {
                assert_eq!(
                    restarted
                        .observe_export_tree(&plan, ExportTreeSlot::Candidate, candidate, owner,)
                        .unwrap(),
                    OwnedTreeObservation::Exact(transaction_manifest(Vec::new()))
                );
                restarted
                    .apply_export_entry(&plan, candidate, owner, &plan.entries[0], None)
                    .unwrap();
            }
            assert_eq!(
                fs::read(scope.path().join(candidate).join("kept.txt")).unwrap(),
                b"kept"
            );
            assert!(!scope.path().join(candidate).join(stage).exists());
        }
    }

    fn transition(
        location: Location,
        path: &str,
        before: PathState,
        after: PathState,
    ) -> PathTransition {
        PathTransition {
            location,
            path: path.to_owned(),
            before,
            after,
        }
    }

    #[test]
    fn in_place_quarantine_contract_last_and_restore() {
        let scope = tempfile::tempdir().unwrap();
        let project_path = scope.path().join("project");
        fs::create_dir(&project_path).unwrap();
        fs::write(project_path.join("junk.txt"), b"junk").unwrap();
        fs::write(project_path.join("contract.toml"), b"contract").unwrap();
        let project = SafefsProject::open(&project_path).unwrap();
        let parent = SafefsProject::open(scope.path()).unwrap();
        let junk = file(b"junk");
        let contract = file(b"contract");
        let junk_step = MutationStep {
            id: "remove-junk".to_owned(),
            pair_id: None,
            kind: MutationKind::QuarantineFile,
            transitions: vec![
                transition(
                    Location::Project,
                    "junk.txt",
                    PathState::File(junk.clone()),
                    PathState::Absent,
                ),
                transition(
                    Location::Quarantine,
                    "payload/junk.txt",
                    PathState::Absent,
                    PathState::File(junk),
                ),
            ],
        };
        let contract_step = MutationStep {
            id: "contract-delete-last".to_owned(),
            pair_id: None,
            kind: MutationKind::ContractDeleteLast,
            transitions: vec![
                transition(
                    Location::Project,
                    "contract.toml",
                    PathState::File(contract.clone()),
                    PathState::Absent,
                ),
                transition(
                    Location::Quarantine,
                    "payload/contract.toml",
                    PathState::Absent,
                    PathState::File(contract),
                ),
            ],
        };
        let before = transaction_manifest(vec![
            entry("contract.toml", b"contract"),
            entry("junk.txt", b"junk"),
        ]);
        let plan = InPlacePlan {
            quarantine_parent_identity: parent.identity_token().unwrap(),
            before_same_display_path: false,
            after_same_display_path: false,
            steps: vec![junk_step.clone()],
            contract: ContractCommit::DeleteLast {
                path: "contract.toml".to_owned(),
                empty_ancestors: Vec::new(),
            },
            contract_step: contract_step.clone(),
            contract_cleanup_step: None,
            before_tree: before.clone(),
            pre_contract_tree: transaction_manifest(vec![entry("contract.toml", b"contract")]),
            post_contract_tree: transaction_manifest(Vec::new()),
            after_tree: transaction_manifest(Vec::new()),
        };
        let mut adapter =
            SafefsTransactionFilesystem::open(&project_path, &project.identity_token().unwrap())
                .unwrap();
        let quarantine = ".vibe-scrape-quarantine-TXN003";
        let owner = "owner-quarantine";
        assert_eq!(
            adapter.create_quarantine(&plan, quarantine, owner).unwrap(),
            ExclusiveTreeCreation::Owned
        );
        adapter
            .apply_step(&plan, quarantine, owner, &junk_step, None)
            .unwrap();
        assert!(project_path.join("contract.toml").exists());
        adapter
            .apply_step(&plan, quarantine, owner, &contract_step, None)
            .unwrap();
        assert!(!project_path.join("contract.toml").exists());
        adapter
            .rollback_step(&plan, quarantine, owner, &contract_step)
            .unwrap();
        adapter
            .rollback_step(&plan, quarantine, owner, &junk_step)
            .unwrap();
        assert_eq!(fs::read(project_path.join("junk.txt")).unwrap(), b"junk");
        assert_eq!(
            fs::read(project_path.join("contract.toml")).unwrap(),
            b"contract"
        );
        cleanup_tree(
            &mut adapter,
            PreparedMode::InPlace(Box::new(plan.clone())),
            quarantine,
            owner,
        );
        assert!(!scope.path().join(quarantine).exists());
    }

    #[test]
    fn nested_contract_ancestors_restore_before_reverse_rename_after_restart() {
        let scope = tempfile::tempdir().unwrap();
        let project_path = scope.path().join("project");
        fs::create_dir_all(project_path.join("vibevm/scrape")).unwrap();
        fs::write(
            project_path.join("vibevm/scrape/contract.toml"),
            b"contract",
        )
        .unwrap();
        let project = SafefsProject::open(&project_path).unwrap();
        let parent = SafefsProject::open(scope.path()).unwrap();
        let contract = file(b"contract");
        let contract_step = MutationStep {
            id: "contract-delete-last".to_owned(),
            pair_id: None,
            kind: MutationKind::ContractDeleteLast,
            transitions: vec![
                transition(
                    Location::Project,
                    "vibevm/scrape/contract.toml",
                    PathState::File(contract.clone()),
                    PathState::Absent,
                ),
                transition(
                    Location::Quarantine,
                    "payload/vibevm/scrape/contract.toml",
                    PathState::Absent,
                    PathState::File(contract),
                ),
            ],
        };
        let ancestor_tree = PathState::Tree(super::super::SubtreeState {
            digest: digest_bytes(b"contract-ancestor-tree"),
            root_mode: None,
            descendants: vec![super::super::SubtreeEntry {
                relative_path: "scrape".into(),
                kind: TreeEntryKind::Directory,
                sha256: None,
                bytes: None,
                mode: None,
            }],
        });
        let cleanup_step = MutationStep {
            id: "contract-ancestor-tree-park".into(),
            pair_id: None,
            kind: MutationKind::ContractAncestorTreePark,
            transitions: vec![
                transition(
                    Location::Project,
                    "vibevm",
                    ancestor_tree.clone(),
                    PathState::Absent,
                ),
                transition(
                    Location::Quarantine,
                    "directories/contract-ancestors",
                    PathState::Absent,
                    ancestor_tree,
                ),
            ],
        };
        let before = transaction_manifest(vec![
            TreeEntry {
                path: "vibevm".into(),
                kind: TreeEntryKind::Directory,
                sha256: None,
                bytes: None,
                mode: None,
            },
            TreeEntry {
                path: "vibevm/scrape".into(),
                kind: TreeEntryKind::Directory,
                sha256: None,
                bytes: None,
                mode: None,
            },
            entry("vibevm/scrape/contract.toml", b"contract"),
        ]);
        let plan = InPlacePlan {
            quarantine_parent_identity: parent.identity_token().unwrap(),
            before_same_display_path: false,
            after_same_display_path: false,
            steps: Vec::new(),
            contract: ContractCommit::DeleteLast {
                path: "vibevm/scrape/contract.toml".into(),
                empty_ancestors: vec!["vibevm/scrape".into(), "vibevm".into()],
            },
            contract_step: contract_step.clone(),
            contract_cleanup_step: Some(cleanup_step.clone()),
            before_tree: before.clone(),
            pre_contract_tree: before,
            post_contract_tree: transaction_manifest(vec![
                TreeEntry {
                    path: "vibevm".into(),
                    kind: TreeEntryKind::Directory,
                    sha256: None,
                    bytes: None,
                    mode: None,
                },
                TreeEntry {
                    path: "vibevm/scrape".into(),
                    kind: TreeEntryKind::Directory,
                    sha256: None,
                    bytes: None,
                    mode: None,
                },
            ]),
            after_tree: transaction_manifest(Vec::new()),
        };
        let quarantine = ".vibe-scrape-quarantine-TXN006";
        let owner = "owner-nested-contract";
        let mut adapter =
            SafefsTransactionFilesystem::open(&project_path, &project.identity_token().unwrap())
                .unwrap();
        adapter.create_quarantine(&plan, quarantine, owner).unwrap();
        adapter
            .apply_step(&plan, quarantine, owner, &contract_step, None)
            .unwrap();
        assert!(project_path.join("vibevm/scrape").is_dir());
        let contract_seal = adapter.owned_tree_seal(quarantine, owner).unwrap();
        let contract_journal = cleanup_journal(
            PreparedMode::InPlace(Box::new(plan.clone())),
            quarantine,
            owner,
            contract_seal,
        );
        drop(adapter);
        let mut adapter =
            SafefsTransactionFilesystem::open(&project_path, &project.identity_token().unwrap())
                .unwrap();
        adapter.rebind_from_journal(&contract_journal).unwrap();
        adapter
            .apply_step(&plan, quarantine, owner, &cleanup_step, None)
            .unwrap();
        assert!(!project_path.join("vibevm").exists());
        let seal = adapter.owned_tree_seal(quarantine, owner).unwrap();
        let journal = cleanup_journal(
            PreparedMode::InPlace(Box::new(plan.clone())),
            quarantine,
            owner,
            seal,
        );
        drop(adapter);

        let mut restarted =
            SafefsTransactionFilesystem::open(&project_path, &project.identity_token().unwrap())
                .unwrap();
        restarted.rebind_from_journal(&journal).unwrap();
        restarted
            .rollback_step(&plan, quarantine, owner, &cleanup_step)
            .unwrap();
        restarted
            .rollback_step(&plan, quarantine, owner, &contract_step)
            .unwrap();
        assert_eq!(
            fs::read(project_path.join("vibevm/scrape/contract.toml")).unwrap(),
            b"contract"
        );
        assert!(project_path.join("vibevm/scrape").is_dir());
    }

    #[test]
    fn precreated_quarantine_topology_rebinds_capture_removal_and_contract_intents() {
        let scope = tempfile::tempdir().unwrap();
        let project_path = scope.path().join("project");
        fs::create_dir_all(project_path.join("src")).unwrap();
        fs::create_dir_all(project_path.join("vibevm/scrape")).unwrap();
        fs::write(project_path.join("src/lib.rs"), b"source").unwrap();
        fs::write(project_path.join("vibevm/data"), b"data").unwrap();
        fs::write(
            project_path.join("vibevm/scrape/contract.toml"),
            b"contract",
        )
        .unwrap();
        let project = SafefsProject::open(&project_path).unwrap();
        let project_token = project.identity_token().unwrap();
        let parent = SafefsProject::open(scope.path()).unwrap();
        let source = file(b"source");
        let data = file(b"data");
        let contract = file(b"contract");
        let capture = MutationStep {
            id: "capture-source".into(),
            pair_id: Some("rewrite-source".into()),
            kind: MutationKind::CaptureBeforeImage,
            transitions: vec![
                transition(
                    Location::Project,
                    "src/lib.rs",
                    PathState::File(source.clone()),
                    PathState::File(source.clone()),
                ),
                transition(
                    Location::Quarantine,
                    "before/src/lib.rs",
                    PathState::Absent,
                    PathState::File(source),
                ),
            ],
        };
        let removal = MutationStep {
            id: "remove-data".into(),
            pair_id: None,
            kind: MutationKind::QuarantineFile,
            transitions: vec![
                transition(
                    Location::Project,
                    "vibevm/data",
                    PathState::File(data.clone()),
                    PathState::Absent,
                ),
                transition(
                    Location::Quarantine,
                    "payload/vibevm/data",
                    PathState::Absent,
                    PathState::File(data),
                ),
            ],
        };
        let contract_step = MutationStep {
            id: "contract-last".into(),
            pair_id: None,
            kind: MutationKind::ContractDeleteLast,
            transitions: vec![
                transition(
                    Location::Project,
                    "vibevm/scrape/contract.toml",
                    PathState::File(contract.clone()),
                    PathState::Absent,
                ),
                transition(
                    Location::Quarantine,
                    "payload/vibevm/scrape/contract.toml",
                    PathState::Absent,
                    PathState::File(contract),
                ),
                transition(
                    Location::Project,
                    "vibevm/scrape",
                    PathState::EmptyDirectory { mode: None },
                    PathState::Absent,
                ),
                transition(
                    Location::Project,
                    "vibevm",
                    PathState::EmptyDirectory { mode: None },
                    PathState::Absent,
                ),
            ],
        };
        let pre_contract = transaction_manifest(vec![
            TreeEntry {
                path: "src".into(),
                kind: TreeEntryKind::Directory,
                sha256: None,
                bytes: None,
                mode: None,
            },
            entry("src/lib.rs", b"source"),
            TreeEntry {
                path: "vibevm".into(),
                kind: TreeEntryKind::Directory,
                sha256: None,
                bytes: None,
                mode: None,
            },
            TreeEntry {
                path: "vibevm/scrape".into(),
                kind: TreeEntryKind::Directory,
                sha256: None,
                bytes: None,
                mode: None,
            },
            entry("vibevm/scrape/contract.toml", b"contract"),
        ]);
        let plan = InPlacePlan {
            quarantine_parent_identity: parent.identity_token().unwrap(),
            before_same_display_path: false,
            after_same_display_path: false,
            steps: vec![capture.clone(), removal.clone()],
            contract: ContractCommit::DeleteLast {
                path: "vibevm/scrape/contract.toml".into(),
                empty_ancestors: vec!["vibevm/scrape".into(), "vibevm".into()],
            },
            contract_step: contract_step.clone(),
            contract_cleanup_step: None,
            before_tree: model_tree_at(&project, &project.root_dir().unwrap()).unwrap(),
            pre_contract_tree: pre_contract,
            post_contract_tree: transaction_manifest(vec![
                TreeEntry {
                    path: "src".into(),
                    kind: TreeEntryKind::Directory,
                    sha256: None,
                    bytes: None,
                    mode: None,
                },
                entry("src/lib.rs", b"source"),
            ]),
            after_tree: transaction_manifest(vec![
                TreeEntry {
                    path: "src".into(),
                    kind: TreeEntryKind::Directory,
                    sha256: None,
                    bytes: None,
                    mode: None,
                },
                entry("src/lib.rs", b"source"),
            ]),
        };
        let quarantine = ".vibe-scrape-quarantine-TXN007";
        let owner = "owner-topology-restart";
        let mut adapter = SafefsTransactionFilesystem::open(&project_path, &project_token).unwrap();
        adapter.create_quarantine(&plan, quarantine, owner).unwrap();

        for (index, step) in [capture, removal, contract_step].iter().enumerate() {
            let before_seal = adapter.owned_tree_seal(quarantine, owner).unwrap();
            adapter
                .apply_step(&plan, quarantine, owner, step, None)
                .unwrap();
            let mut journal = cleanup_journal(
                PreparedMode::InPlace(Box::new(plan.clone())),
                quarantine,
                owner,
                before_seal,
            );
            journal.state = super::super::TransactionState::Mutating;
            journal.completed_steps = index;
            journal.active_step = Some(index);
            journal.mutation_progress = vec![super::super::MutationProgress {
                id: step.id.clone(),
                kind: super::super::PlannedMutationKind::InPlace(step.kind),
                status: super::super::MutationStatus::ApplyIntent,
            }];
            drop(adapter);
            adapter = SafefsTransactionFilesystem::open(&project_path, &project_token).unwrap();
            adapter.rebind_from_journal(&journal).unwrap();
        }
        assert!(!project_path.join("vibevm").exists());
        assert_eq!(
            fs::read(project_path.join("src/lib.rs")).unwrap(),
            b"source"
        );
    }

    #[test]
    fn capture_and_rewrite_stages_rebind_before_and_after_publication() {
        for kind in [
            MutationKind::CaptureBeforeImage,
            MutationKind::AtomicRewrite,
        ] {
            for staged_before_publish in [true, false] {
                let scope = tempfile::tempdir().unwrap();
                let project_path = scope.path().join("project");
                fs::create_dir_all(project_path.join("src")).unwrap();
                fs::write(project_path.join("src/lib.rs"), b"old").unwrap();
                let project = SafefsProject::open(&project_path).unwrap();
                let project_token = project.identity_token().unwrap();
                let parent = SafefsProject::open(scope.path()).unwrap();
                let old = file(b"old");
                let new = file(b"new");
                let capture = MutationStep {
                    id: "capture-rewrite".into(),
                    pair_id: Some("rewrite".into()),
                    kind: MutationKind::CaptureBeforeImage,
                    transitions: vec![
                        transition(
                            Location::Project,
                            "src/lib.rs",
                            PathState::File(old.clone()),
                            PathState::File(old.clone()),
                        ),
                        transition(
                            Location::Quarantine,
                            "before/src/lib.rs",
                            PathState::Absent,
                            PathState::File(old.clone()),
                        ),
                    ],
                };
                let rewrite = MutationStep {
                    id: "rewrite-source".into(),
                    pair_id: Some("rewrite".into()),
                    kind: MutationKind::AtomicRewrite,
                    transitions: vec![transition(
                        Location::Project,
                        "src/lib.rs",
                        PathState::File(old),
                        PathState::File(new),
                    )],
                };
                let tree = model_tree_at(&project, &project.root_dir().unwrap()).unwrap();
                let plan = InPlacePlan {
                    quarantine_parent_identity: parent.identity_token().unwrap(),
                    before_same_display_path: false,
                    after_same_display_path: false,
                    steps: vec![capture.clone(), rewrite.clone()],
                    contract: ContractCommit::ExternalPreserve,
                    contract_step: MutationStep {
                        id: "external-contract".into(),
                        pair_id: None,
                        kind: MutationKind::ContractExternalPreserve,
                        transitions: Vec::new(),
                    },
                    contract_cleanup_step: None,
                    before_tree: tree.clone(),
                    pre_contract_tree: tree.clone(),
                    post_contract_tree: tree.clone(),
                    after_tree: tree,
                };
                let quarantine = if kind == MutationKind::CaptureBeforeImage {
                    ".vibe-scrape-quarantine-TXN010"
                } else {
                    ".vibe-scrape-quarantine-TXN011"
                };
                let owner = if staged_before_publish {
                    "owner-stage-before"
                } else {
                    "owner-stage-after"
                };
                let mut adapter =
                    SafefsTransactionFilesystem::open(&project_path, &project_token).unwrap();
                adapter.create_quarantine(&plan, quarantine, owner).unwrap();
                if kind == MutationKind::AtomicRewrite {
                    adapter
                        .apply_step(&plan, quarantine, owner, &capture, None)
                        .unwrap();
                }
                let before_seal = adapter.owned_tree_seal(quarantine, owner).unwrap();
                let (step, index, target, bytes) = if kind == MutationKind::CaptureBeforeImage {
                    (&capture, 0, "before/src/lib.rs", b"old".as_slice())
                } else {
                    (&rewrite, 1, "src/lib.rs", b"new".as_slice())
                };
                let stage = transaction_stage_name(owner, &format!("apply:{}", step.id), target);
                let stage_path = if kind == MutationKind::CaptureBeforeImage {
                    scope
                        .path()
                        .join(quarantine)
                        .join("before/src")
                        .join(&stage)
                } else {
                    project_path.join("src").join(&stage)
                };
                if staged_before_publish {
                    fs::write(&stage_path, bytes).unwrap();
                } else {
                    adapter
                        .apply_step(
                            &plan,
                            quarantine,
                            owner,
                            step,
                            (kind == MutationKind::AtomicRewrite).then_some(bytes),
                        )
                        .unwrap();
                }
                let mut journal = cleanup_journal(
                    PreparedMode::InPlace(Box::new(plan.clone())),
                    quarantine,
                    owner,
                    before_seal,
                );
                journal.state = super::super::TransactionState::Mutating;
                journal.completed_steps = index;
                journal.active_step = Some(index);
                journal.mutation_progress = vec![super::super::MutationProgress {
                    id: step.id.clone(),
                    kind: super::super::PlannedMutationKind::InPlace(step.kind),
                    status: super::super::MutationStatus::ApplyIntent,
                }];
                drop(adapter);
                let mut restarted =
                    SafefsTransactionFilesystem::open(&project_path, &project_token).unwrap();
                restarted.rebind_from_journal(&journal).unwrap();
                if staged_before_publish {
                    if kind == MutationKind::AtomicRewrite {
                        restarted
                            .cleanup_unpublished_step_stage(&plan, quarantine, owner, step)
                            .unwrap();
                    } else {
                        restarted
                            .apply_step(&plan, quarantine, owner, step, None)
                            .unwrap();
                    }
                }
                assert!(!stage_path.exists());
                if kind == MutationKind::CaptureBeforeImage {
                    assert_eq!(
                        fs::read(scope.path().join(quarantine).join("before/src/lib.rs")).unwrap(),
                        b"old"
                    );
                } else if staged_before_publish {
                    assert_eq!(fs::read(project_path.join("src/lib.rs")).unwrap(), b"old");
                } else {
                    assert_eq!(
                        restarted
                            .observe_step(&plan, quarantine, owner, &rewrite)
                            .unwrap(),
                        SealedObservation::After
                    );
                    restarted
                        .rollback_step(&plan, quarantine, owner, &rewrite)
                        .unwrap();
                    assert_eq!(fs::read(project_path.join("src/lib.rs")).unwrap(), b"old");
                }
            }
        }
    }
}
