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
