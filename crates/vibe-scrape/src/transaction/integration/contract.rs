use super::tree::*;
use super::*;

pub(super) fn snapshots(
    prepared: &PreparedScrape,
) -> Result<Vec<tx::Snapshot>, tx::TransactionError> {
    let contract_mode = prepared
        .inventory
        .entries
        .iter()
        .find(|entry| entry.path == prepared.contract.display_path)
        .and_then(|entry| entry.unix_mode);
    let canonical_contract = serde_json::to_vec(&prepared.contract.value)
        .map_err(|error| tx::TransactionError::InvalidPrepared(error.to_string()))?;
    let plan = serde_json::to_vec(
        &prepared
            .plan
            .to_wire()
            .map_err(|error| tx::TransactionError::InvalidPrepared(error.to_string()))?,
    )
    .map_err(|error| tx::TransactionError::InvalidPrepared(error.to_string()))?;
    let health = serde_json::to_vec(&prepared.health)
        .map_err(|error| tx::TransactionError::InvalidPrepared(error.to_string()))?;
    let mut answer = vec![
        tx::Snapshot {
            kind: tx::SnapshotKind::Contract,
            name: "contract".to_owned(),
            bytes: prepared.contract.bytes.clone(),
            mode: contract_mode,
        },
        tx::Snapshot {
            kind: tx::SnapshotKind::CanonicalContract,
            name: "canonical-contract".to_owned(),
            bytes: canonical_contract,
            mode: None,
        },
        tx::Snapshot {
            kind: tx::SnapshotKind::CanonicalPlan,
            name: "plan".to_owned(),
            bytes: plan,
            mode: None,
        },
        tx::Snapshot {
            kind: tx::SnapshotKind::Verifier,
            name: "health-plan".to_owned(),
            bytes: health,
            mode: None,
        },
    ];
    for check in &prepared.health.checks {
        if let Some(bundle) = &check.custom_bundle {
            for entry in &bundle.entries {
                if let Some(bytes) = &entry.content {
                    answer.push(tx::Snapshot {
                        kind: tx::SnapshotKind::Verifier,
                        name: format!("verifier/{}/{}", check.id, entry.path),
                        bytes: bytes.clone(),
                        mode: entry.mode,
                    });
                }
            }
        }
    }
    Ok(answer)
}

pub(super) fn export_plan(
    prepared: &PreparedScrape,
    source_tree: tx::TreeManifest,
    final_manifest: tx::TreeManifest,
) -> Result<tx::ExportPlan, tx::TransactionError> {
    let ScrapeMode::Export { output } = &prepared.mode else {
        return Err(tx::TransactionError::InvalidPrepared(
            "export plan lost requested output".to_owned(),
        ));
    };
    let output_name = output
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| tx::TransactionError::InvalidPrepared("output has no UTF-8 name".into()))?
        .to_owned();
    let parent = output
        .parent()
        .ok_or_else(|| tx::TransactionError::InvalidPrepared("output has no parent".to_owned()))?;
    let output_pin = vibe_safefs::Project::pin_absent_path(output)
        .map_err(|error| tx::TransactionError::Filesystem(format!("pinning output: {error:#}")))?;
    let parent_project = vibe_safefs::Project::open(parent).map_err(|error| {
        tx::TransactionError::Filesystem(format!("opening output parent: {error:#}"))
    })?;
    let rewrites = final_rewrites(prepared);
    let entries = final_manifest
        .entries
        .iter()
        .map(|entry| {
            let payload = if entry.kind == tx::TreeEntryKind::Directory {
                None
            } else if let Some(rewrite) = rewrites.get(entry.path.as_str()) {
                let transaction_id = rewrite_transaction_id(rewrite);
                Some(tx::ExportPayload::PreparedAfter {
                    snapshot_name: format!("after/{transaction_id}"),
                })
            } else {
                let source_path = source_for_final_path(prepared, &entry.path);
                let source = prepared
                    .inventory
                    .entries
                    .iter()
                    .find(|candidate| candidate.path == source_path)
                    .ok_or_else(|| {
                        tx::TransactionError::InvalidPrepared(format!(
                            "final file `{}` has no sealed source",
                            entry.path
                        ))
                    })?;
                Some(tx::ExportPayload::Source {
                    source_path,
                    before: file_state(source)?,
                })
            };
            Ok(tx::ExportEntry {
                target_path: entry.path.clone(),
                kind: entry.kind,
                mode: entry.mode,
                payload,
            })
        })
        .collect::<Result<Vec<_>, tx::TransactionError>>()?;
    Ok(tx::ExportPlan {
        output_identity: output_pin.identity_token(),
        output_parent_identity: parent_project.identity_token().map_err(|error| {
            tx::TransactionError::Filesystem(format!("sealing output parent: {error:#}"))
        })?,
        output_display_path: output.display().to_string(),
        output_name,
        before_same_display_path: false,
        after_same_display_path: false,
        entries,
        source_tree,
        final_manifest,
    })
}

pub(super) fn in_place_plan(
    prepared: &PreparedScrape,
    before_tree: tx::TreeManifest,
    after_tree: tx::TreeManifest,
    final_entries: &BTreeMap<String, tx::TreeEntry>,
) -> Result<tx::InPlacePlan, tx::TransactionError> {
    let rewrites = final_rewrites(prepared);
    let mut steps = Vec::new();
    for (path, rewrite) in &rewrites {
        let transaction_id = rewrite_transaction_id(rewrite);
        let source = prepared
            .inventory
            .entries
            .iter()
            .find(|entry| &entry.path == path)
            .ok_or_else(|| tx::TransactionError::InvalidPrepared("rewrite source absent".into()))?;
        let before = tx::PathState::File(file_state(source)?);
        steps.push(tx::MutationStep {
            id: format!("capture-{transaction_id}"),
            pair_id: Some(transaction_id.clone()),
            kind: tx::MutationKind::CaptureBeforeImage,
            transitions: vec![
                transition(tx::Location::Project, path, before.clone(), before.clone()),
                transition(
                    tx::Location::Quarantine,
                    &format!("before/{path}"),
                    tx::PathState::Absent,
                    before.clone(),
                ),
            ],
        });
        steps.push(tx::MutationStep {
            id: transaction_id.clone(),
            pair_id: Some(transaction_id),
            kind: tx::MutationKind::AtomicRewrite,
            transitions: vec![transition(
                tx::Location::Project,
                path,
                before,
                tx::PathState::File(tx::FileState {
                    sha256: digest_text(&rewrite.after_sha256)?,
                    bytes: rewrite.after_bytes.len() as u64,
                    mode: source.unix_mode,
                }),
            )],
        });
    }
    // Create only destination ancestors absent from the inventoried source,
    // shallowest-first. A directory relocation creates its own destination
    // name atomically, so that target itself is never pre-created.
    let source_paths = prepared
        .inventory
        .entries
        .iter()
        .map(|entry| entry.path.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    let mut relocation_parents = std::collections::BTreeSet::new();
    for relocation in &prepared.plan.relocations {
        let mut current = relocation.to.rsplit_once('/').map(|(parent, _)| parent);
        while let Some(parent) = current {
            if !source_paths.contains(parent) {
                relocation_parents.insert(parent.to_owned());
            }
            current = parent.rsplit_once('/').map(|(ancestor, _)| ancestor);
        }
    }
    let mut relocation_parents = relocation_parents.into_iter().collect::<Vec<_>>();
    relocation_parents.sort_by(|left, right| {
        left.matches('/')
            .count()
            .cmp(&right.matches('/').count())
            .then(left.as_bytes().cmp(right.as_bytes()))
    });
    for path in relocation_parents {
        let mode = final_entries.get(&path).and_then(|entry| entry.mode);
        steps.push(tx::MutationStep {
            id: format!("relocation-parent-{}", stable_id(&path)),
            pair_id: None,
            kind: tx::MutationKind::CreateRelocationParent,
            transitions: vec![transition(
                tx::Location::Project,
                &path,
                tx::PathState::Absent,
                tx::PathState::EmptyDirectory { mode },
            )],
        });
    }

    // Relocation rows are exact descendant maps; physical moves are canonical
    // by source path after every required parent exists.
    let mut relocations = prepared.plan.relocations.iter().collect::<Vec<_>>();
    relocations.sort_by(|left, right| left.from.as_bytes().cmp(right.from.as_bytes()));
    for relocation in relocations {
        let state = relocation_path_state(final_entries, relocation)?;
        steps.push(tx::MutationStep {
            id: relocation.id.clone(),
            pair_id: None,
            kind: tx::MutationKind::Relocate,
            transitions: vec![
                transition(
                    tx::Location::Project,
                    &relocation.from,
                    state.clone(),
                    tx::PathState::Absent,
                ),
                transition(
                    tx::Location::Project,
                    &relocation.to,
                    tx::PathState::Absent,
                    state,
                ),
            ],
        });
    }
    let contract_path = match &prepared.plan.contract_boundary {
        crate::model::ContractBoundary::DeleteLast { path, .. } => Some(path.as_str()),
        crate::model::ContractBoundary::Preserve => None,
    };
    let mut removals = prepared
        .plan
        .items
        .iter()
        .filter(|item| {
            item.disposition == Disposition::Delete && item.entry_kind == EntryKind::File
        })
        .collect::<Vec<_>>();
    removals.sort_by(|a, b| a.path.as_bytes().cmp(b.path.as_bytes()));
    for item in removals {
        if Some(item.path.as_str()) == contract_path {
            continue;
        }
        let state = tx::PathState::File(file_state_from_item(item)?);
        steps.push(tx::MutationStep {
            id: format!("remove-{}", stable_id(&item.path)),
            pair_id: None,
            kind: tx::MutationKind::QuarantineFile,
            transitions: vec![
                transition(
                    tx::Location::Project,
                    &item.path,
                    state.clone(),
                    tx::PathState::Absent,
                ),
                transition(
                    tx::Location::Quarantine,
                    &format!("payload/{}", item.path),
                    tx::PathState::Absent,
                    state,
                ),
            ],
        });
    }
    let mut directories = prepared
        .plan
        .items
        .iter()
        .filter(|item| {
            item.disposition == Disposition::Delete && item.entry_kind == EntryKind::Directory
        })
        .map(|item| item.path.clone())
        .filter(|path| {
            !matches!(
                &prepared.plan.contract_boundary,
                crate::model::ContractBoundary::DeleteLast { empty_ancestors, .. }
                    if empty_ancestors.contains(path)
            )
        })
        .collect::<Vec<_>>();
    directories.sort_by(|a, b| {
        b.matches('/')
            .count()
            .cmp(&a.matches('/').count())
            .then(a.cmp(b))
    });
    for path in directories {
        steps.push(tx::MutationStep {
            id: format!("prune-{}", stable_id(&path)),
            pair_id: None,
            kind: tx::MutationKind::PruneEmptyDirectory,
            transitions: vec![transition(
                tx::Location::Project,
                &path,
                tx::PathState::EmptyDirectory {
                    mode: prepared
                        .inventory
                        .entries
                        .iter()
                        .find(|e| e.path == path)
                        .and_then(|e| e.unix_mode),
                },
                tx::PathState::Absent,
            )],
        });
    }
    let (contract, contract_step, contract_cleanup_step, pre_contract_tree, post_contract_tree) =
        contract_step(prepared, final_entries)?;
    let project =
        vibe_safefs::Project::open(std::path::Path::new(&prepared.plan.project_display_root))
            .map_err(|error| tx::TransactionError::Filesystem(error.to_string()))?;
    let parent = project
        .root_path()
        .parent()
        .ok_or_else(|| tx::TransactionError::InvalidPrepared("project has no parent".into()))?;
    let parent = vibe_safefs::Project::open(parent)
        .map_err(|error| tx::TransactionError::Filesystem(error.to_string()))?;
    Ok(tx::InPlacePlan {
        quarantine_parent_identity: parent
            .identity_token()
            .map_err(|error| tx::TransactionError::Filesystem(error.to_string()))?,
        before_same_display_path: false,
        after_same_display_path: false,
        steps,
        contract,
        contract_step,
        contract_cleanup_step,
        before_tree,
        pre_contract_tree,
        post_contract_tree,
        after_tree,
    })
}

fn contract_step(
    prepared: &PreparedScrape,
    final_entries: &BTreeMap<String, tx::TreeEntry>,
) -> Result<
    (
        tx::ContractCommit,
        tx::MutationStep,
        Option<tx::MutationStep>,
        tx::TreeManifest,
        tx::TreeManifest,
    ),
    tx::TransactionError,
> {
    match &prepared.plan.contract_boundary {
        crate::model::ContractBoundary::Preserve => Ok((
            tx::ContractCommit::ExternalPreserve,
            tx::MutationStep {
                id: "external-contract-preserve".into(),
                pair_id: None,
                kind: tx::MutationKind::ContractExternalPreserve,
                transitions: Vec::new(),
            },
            None,
            manifest(final_entries.values().cloned().collect()),
            manifest(final_entries.values().cloned().collect()),
        )),
        crate::model::ContractBoundary::DeleteLast {
            path,
            empty_ancestors,
        } => {
            let entry = prepared
                .inventory
                .entries
                .iter()
                .find(|entry| &entry.path == path)
                .ok_or_else(|| {
                    tx::TransactionError::InvalidPrepared("contract absent from inventory".into())
                })?;
            let state = tx::PathState::File(file_state(entry)?);
            let mut pre = final_entries.clone();
            pre.insert(path.clone(), tree_entry(entry)?);
            for ancestor in empty_ancestors {
                pre.entry(ancestor.clone()).or_insert(tx::TreeEntry {
                    path: ancestor.clone(),
                    kind: tx::TreeEntryKind::Directory,
                    sha256: None,
                    bytes: None,
                    mode: prepared
                        .inventory
                        .entries
                        .iter()
                        .find(|e| &e.path == ancestor)
                        .and_then(|e| e.unix_mode),
                });
            }
            let transitions = vec![
                transition(
                    tx::Location::Project,
                    path,
                    state.clone(),
                    tx::PathState::Absent,
                ),
                transition(
                    tx::Location::Quarantine,
                    &format!("payload/{path}"),
                    tx::PathState::Absent,
                    state,
                ),
            ];
            let mut post = final_entries.clone();
            for ancestor in empty_ancestors {
                post.entry(ancestor.clone()).or_insert(tx::TreeEntry {
                    path: ancestor.clone(),
                    kind: tx::TreeEntryKind::Directory,
                    sha256: None,
                    bytes: None,
                    mode: prepared
                        .inventory
                        .entries
                        .iter()
                        .find(|e| &e.path == ancestor)
                        .and_then(|e| e.unix_mode),
                });
            }
            let cleanup = empty_ancestors.last().map(|topmost| {
                let root_mode = prepared
                    .inventory
                    .entries
                    .iter()
                    .find(|entry| &entry.path == topmost)
                    .and_then(|entry| entry.unix_mode);
                let prefix = format!("{topmost}/");
                let mut descendants = empty_ancestors
                    .iter()
                    .filter(|ancestor| *ancestor != topmost)
                    .map(|ancestor| tx::SubtreeEntry {
                        relative_path: ancestor
                            .strip_prefix(&prefix)
                            .unwrap_or(ancestor)
                            .to_owned(),
                        kind: tx::TreeEntryKind::Directory,
                        sha256: None,
                        bytes: None,
                        mode: prepared
                            .inventory
                            .entries
                            .iter()
                            .find(|entry| &entry.path == ancestor)
                            .and_then(|entry| entry.unix_mode),
                    })
                    .collect::<Vec<_>>();
                descendants.sort_by(|left, right| {
                    left.relative_path
                        .as_bytes()
                        .cmp(right.relative_path.as_bytes())
                });
                let tree = tx::PathState::Tree(tx::SubtreeState {
                    digest: subtree_digest(root_mode, &descendants),
                    root_mode,
                    descendants,
                });
                tx::MutationStep {
                    id: "contract-ancestor-tree-park".into(),
                    pair_id: None,
                    kind: tx::MutationKind::ContractAncestorTreePark,
                    transitions: vec![
                        transition(
                            tx::Location::Project,
                            topmost,
                            tree.clone(),
                            tx::PathState::Absent,
                        ),
                        transition(
                            tx::Location::Quarantine,
                            "directories/contract-ancestors",
                            tx::PathState::Absent,
                            tree,
                        ),
                    ],
                }
            });
            Ok((
                tx::ContractCommit::DeleteLast {
                    path: path.clone(),
                    empty_ancestors: empty_ancestors.clone(),
                },
                tx::MutationStep {
                    id: "contract-delete-last".into(),
                    pair_id: None,
                    kind: tx::MutationKind::ContractDeleteLast,
                    transitions,
                },
                cleanup,
                manifest(pre.into_values().collect()),
                manifest(post.into_values().collect()),
            ))
        }
    }
}
