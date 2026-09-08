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
