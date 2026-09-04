//! One-way adapter from the already prepared product model into the durable
//! transaction model. No contract read, inventory walk, rewrite, or health
//! preparation occurs here.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-056#IMPL-E");

use std::collections::BTreeMap;

use sha2::{Digest as _, Sha256};

use super::model as tx;
use super::traits::PreparedScrapeSource;
use crate::model::{Disposition, EntryKind, PreparedScrape, ScrapeMode};

impl PreparedScrapeSource for PreparedScrape {
    fn into_transaction(self) -> Result<tx::PreparedTransaction, tx::TransactionError> {
        prepared_transaction(self)
    }
}

pub fn prepared_transaction(
    prepared: PreparedScrape,
) -> Result<tx::PreparedTransaction, tx::TransactionError> {
    if !prepared.plan.blockers.is_empty() || !prepared.health.blockers.is_empty() {
        return Err(tx::TransactionError::InvalidPrepared(
            "a blocker-filled scrape plan cannot enter a transaction".to_owned(),
        ));
    }
    let project =
        vibe_safefs::Project::open(std::path::Path::new(&prepared.plan.project_display_root))
            .map_err(|error| {
                tx::TransactionError::Filesystem(format!("opening project: {error:#}"))
            })?;
    let project_identity_token = project.identity_token().map_err(|error| {
        tx::TransactionError::Filesystem(format!("sealing project identity: {error:#}"))
    })?;
    let source_tree = inventory_manifest(&prepared.inventory)?;
    let final_entries = final_entries(&prepared)?;
    let final_tree = manifest(final_entries.values().cloned().collect());
    let mut snapshots = snapshots(&prepared)?;
    let canonical_plan = snapshots
        .iter()
        .find(|snapshot| snapshot.kind == tx::SnapshotKind::CanonicalPlan)
        .map(|snapshot| snapshot.bytes.clone())
        .ok_or_else(|| {
            tx::TransactionError::InvalidPrepared(
                "prepared scrape has no canonical plan snapshot".to_owned(),
            )
        })?;
    for rewrite in &prepared.rewrites {
        let transaction_id = rewrite_transaction_id(rewrite);
        snapshots.push(tx::Snapshot {
            kind: tx::SnapshotKind::PreparedAfter,
            name: format!("after/{transaction_id}"),
            bytes: rewrite.after_bytes.clone(),
            mode: prepared
                .inventory
                .entries
                .iter()
                .find(|entry| entry.path == rewrite.path)
                .and_then(|entry| entry.unix_mode),
        });
    }
    let mode = match &prepared.plan.mode[..] {
        "export" => {
            tx::PreparedMode::Export(Box::new(export_plan(&prepared, source_tree, final_tree)?))
        }
        "in-place" => tx::PreparedMode::InPlace(Box::new(in_place_plan(
            &prepared,
            source_tree,
            final_tree,
            &final_entries,
        )?)),
        other => {
            return Err(tx::TransactionError::InvalidPrepared(format!(
                "unknown prepared mode `{other}`"
            )));
        }
    };
    Ok(tx::PreparedTransaction {
        project_identity_token,
        project_display_root: prepared.plan.project_display_root.clone(),
        plan_id: digest_text(&prepared.plan.plan_id)?,
        canonical_plan,
        snapshots,
        mode,
    })
}

pub fn project_identity_token(root: &std::path::Path) -> Result<String, tx::TransactionError> {
    let project = vibe_safefs::Project::open(root)
        .map_err(|error| tx::TransactionError::Filesystem(format!("opening project: {error:#}")))?;
    project.identity_token().map_err(|error| {
        tx::TransactionError::Filesystem(format!("sealing project identity: {error:#}"))
    })
}

fn snapshots(prepared: &PreparedScrape) -> Result<Vec<tx::Snapshot>, tx::TransactionError> {
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

fn export_plan(
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

fn in_place_plan(
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

fn final_entries(
    prepared: &PreparedScrape,
) -> Result<BTreeMap<String, tx::TreeEntry>, tx::TransactionError> {
    let rewrites = final_rewrites(prepared);
    let mut answer = BTreeMap::new();
    for item in &prepared.plan.items {
        if matches!(
            item.disposition,
            Disposition::Delete | Disposition::DeleteLast
        ) {
            continue;
        }
        let path = if item.disposition == Disposition::Relocate {
            relocation_target(prepared, &item.path)
        } else {
            item.path.clone()
        };
        let mut entry = tx::TreeEntry {
            path: path.clone(),
            kind: kind(item.entry_kind),
            sha256: item.sha256.as_deref().map(digest_text).transpose()?,
            bytes: item.bytes,
            mode: item.unix_mode,
        };
        if let Some(rewrite) = rewrites.get(item.path.as_str()) {
            entry.sha256 = Some(digest_text(&rewrite.after_sha256)?);
            entry.bytes = Some(rewrite.after_bytes.len() as u64);
        }
        answer.insert(path, entry);
    }
    let projected_paths = answer.keys().cloned().collect::<Vec<_>>();
    for path in projected_paths {
        let mut current = path.as_str();
        while let Some((parent, _)) = current.rsplit_once('/') {
            answer
                .entry(parent.to_owned())
                .or_insert_with(|| tx::TreeEntry {
                    path: parent.to_owned(),
                    kind: tx::TreeEntryKind::Directory,
                    sha256: None,
                    bytes: None,
                    mode: prepared
                        .inventory
                        .entries
                        .iter()
                        .find(|entry| entry.path == parent)
                        .and_then(|entry| entry.unix_mode),
                });
            current = parent;
        }
    }
    Ok(answer)
}

fn relocation_target(prepared: &PreparedScrape, source: &str) -> String {
    prepared
        .plan
        .relocations
        .iter()
        .flat_map(|r| &r.mapped_descendants)
        .find(|m| m.from == source)
        .map(|m| m.to.clone())
        .unwrap_or_else(|| source.to_owned())
}
fn source_for_final_path(prepared: &PreparedScrape, final_path: &str) -> String {
    prepared
        .plan
        .relocations
        .iter()
        .flat_map(|r| &r.mapped_descendants)
        .find(|m| m.to == final_path)
        .map(|m| m.from.clone())
        .unwrap_or_else(|| final_path.to_owned())
}
fn final_rewrites(prepared: &PreparedScrape) -> BTreeMap<&str, &crate::model::PreparedRewrite> {
    let mut m = BTreeMap::new();
    for r in &prepared.rewrites {
        m.insert(r.path.as_str(), r);
    }
    m
}
fn inventory_manifest(
    inventory: &crate::model::Inventory,
) -> Result<tx::TreeManifest, tx::TransactionError> {
    Ok(tx::TreeManifest {
        digest: digest_text(&inventory.tree_digest)?,
        entries: inventory
            .entries
            .iter()
            .map(tree_entry)
            .collect::<Result<_, _>>()?,
    })
}
fn manifest(entries: Vec<tx::TreeEntry>) -> tx::TreeManifest {
    tx::logical_tree_manifest(entries)
}
fn tree_entry(e: &crate::model::InventoryEntry) -> Result<tx::TreeEntry, tx::TransactionError> {
    Ok(tx::TreeEntry {
        path: e.path.clone(),
        kind: kind(e.kind),
        sha256: e.sha256.as_deref().map(digest_text).transpose()?,
        bytes: e.bytes,
        mode: e.unix_mode,
    })
}
fn kind(k: EntryKind) -> tx::TreeEntryKind {
    match k {
        EntryKind::File => tx::TreeEntryKind::File,
        EntryKind::Directory => tx::TreeEntryKind::Directory,
    }
}
fn file_state(e: &crate::model::InventoryEntry) -> Result<tx::FileState, tx::TransactionError> {
    Ok(tx::FileState {
        sha256: digest_text(
            e.sha256.as_deref().ok_or_else(|| {
                tx::TransactionError::InvalidPrepared("file digest absent".into())
            })?,
        )?,
        bytes: e
            .bytes
            .ok_or_else(|| tx::TransactionError::InvalidPrepared("file size absent".into()))?,
        mode: e.unix_mode,
    })
}
fn file_state_from_item(e: &crate::model::PlanItem) -> Result<tx::FileState, tx::TransactionError> {
    Ok(tx::FileState {
        sha256: digest_text(
            e.sha256.as_deref().ok_or_else(|| {
                tx::TransactionError::InvalidPrepared("file digest absent".into())
            })?,
        )?,
        bytes: e
            .bytes
            .ok_or_else(|| tx::TransactionError::InvalidPrepared("file size absent".into()))?,
        mode: e.unix_mode,
    })
}
fn relocation_path_state(
    final_entries: &BTreeMap<String, tx::TreeEntry>,
    r: &crate::model::PlannedRelocation,
) -> Result<tx::PathState, tx::TransactionError> {
    let root = final_entries.get(&r.to).ok_or_else(|| {
        tx::TransactionError::InvalidPrepared(format!(
            "relocation `{}` has no projected target root `{}`",
            r.id, r.to
        ))
    })?;
    if root.kind == tx::TreeEntryKind::File {
        return Ok(tx::PathState::File(tx::FileState {
            sha256: root.sha256.clone().ok_or_else(|| {
                tx::TransactionError::InvalidPrepared("relocation target file has no digest".into())
            })?,
            bytes: root.bytes.ok_or_else(|| {
                tx::TransactionError::InvalidPrepared("relocation target file has no size".into())
            })?,
            mode: root.mode,
        }));
    }
    let prefix = format!("{}/", r.to);
    let descendants = final_entries
        .values()
        .filter(|entry| entry.path.starts_with(&prefix))
        .map(|entry| tx::SubtreeEntry {
            relative_path: entry
                .path
                .strip_prefix(&prefix)
                .unwrap_or(&entry.path)
                .to_owned(),
            kind: entry.kind,
            sha256: entry.sha256.clone(),
            bytes: entry.bytes,
            mode: entry.mode,
        })
        .collect::<Vec<_>>();
    Ok(tx::PathState::Tree(tx::SubtreeState {
        digest: subtree_digest(root.mode, &descendants),
        root_mode: root.mode,
        descendants,
    }))
}

fn subtree_digest(root_mode: Option<u32>, descendants: &[tx::SubtreeEntry]) -> tx::Digest {
    let mut hash = Sha256::new();
    hash.update(b"vibe-scrape-subtree-e1\0");
    if let Some(mode) = root_mode {
        hash.update(mode.to_be_bytes());
    }
    for entry in descendants {
        hash.update(match entry.kind {
            tx::TreeEntryKind::File => b"f\0".as_slice(),
            tx::TreeEntryKind::Directory => b"d\0".as_slice(),
        });
        hash.update(entry.relative_path.as_bytes());
        hash.update(b"\0");
        if let Some(digest) = &entry.sha256 {
            hash.update(digest.0.as_bytes());
        }
        hash.update(b"\0");
        if let Some(bytes) = entry.bytes {
            hash.update(bytes.to_be_bytes());
        }
        hash.update(b"\0");
        if let Some(mode) = entry.mode {
            hash.update(mode.to_be_bytes());
        }
        hash.update(b"\n");
    }
    tx::Digest(format!("sha256:{:x}", hash.finalize()))
}
fn transition(
    location: tx::Location,
    path: &str,
    before: tx::PathState,
    after: tx::PathState,
) -> tx::PathTransition {
    tx::PathTransition {
        location,
        path: path.to_owned(),
        before,
        after,
    }
}
fn digest_text(value: &str) -> Result<tx::Digest, tx::TransactionError> {
    let d = tx::Digest(value.to_owned());
    if value.strip_prefix("sha256:").is_some_and(|h| {
        h.len() == 64
            && h.bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    }) {
        Ok(d)
    } else {
        Err(tx::TransactionError::InvalidPrepared(format!(
            "invalid digest `{value}`"
        )))
    }
}
fn stable_id(path: &str) -> String {
    format!("{:x}", Sha256::digest(path.as_bytes()))
}

fn rewrite_transaction_id(rewrite: &crate::model::PreparedRewrite) -> String {
    format!("rewrite-{}-{}", rewrite.id, stable_id(&rewrite.path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prepared_export_converts_without_a_second_plan_or_inventory() {
        let source = tempfile::tempdir().unwrap();
        crate::init_contract(source.path()).unwrap();
        std::fs::write(
            source.path().join("Cargo.toml"),
            "[package]\nname='sample'\nversion='0.1.0'\nedition='2024'\n",
        )
        .unwrap();
        std::fs::create_dir(source.path().join("src")).unwrap();
        std::fs::write(source.path().join("src/lib.rs"), "pub fn value() {}\n").unwrap();
        let contract_path = source.path().join("vibevm/scrape/contract.toml");
        let contract = std::fs::read_to_string(&contract_path)
            .unwrap()
            .replace("modified = \"refuse\"", "modified = \"delete\"")
            .replace("tests = \"required\"", "tests = \"skip\"");
        std::fs::write(contract_path, contract).unwrap();
        let output_parent = tempfile::tempdir().unwrap();
        let prepared = crate::prepare(crate::model::ScrapeRequest {
            root: source.path().to_path_buf(),
            contract: None,
            mode: ScrapeMode::Export {
                output: output_parent.path().join("release"),
            },
        })
        .unwrap();
        assert!(
            prepared.plan.blockers.is_empty(),
            "{:?}",
            prepared.plan.blockers
        );
        assert!(
            prepared.health.blockers.is_empty(),
            "{:?}",
            prepared.health.blockers
        );
        let wire_plan = prepared.plan.to_wire().unwrap();
        let refused_report = tx::TransactionReport {
            project_key: tx::ProjectKey(format!("sha256:{}", "1".repeat(64))),
            transaction_id: tx::TransactionId("TX000001".into()),
            plan_id: tx::Digest(prepared.plan.plan_id.clone()),
            mode: tx::TransactionMode::Export,
            outcome: tx::Outcome::Refused,
            assurance: tx::Assurance::Full,
            cleanup: tx::Cleanup::Complete,
            before_tree: Some(tx::Digest(prepared.inventory.tree_digest.clone())),
            after_tree: None,
            snapshots: Vec::new(),
            verification: Vec::new(),
            planned_mutations: Vec::new(),
            actual_mutations: Vec::new(),
            events: Vec::new(),
        };
        let refused_wire =
            super::super::report::report_to_wire_plan(&refused_report, &wire_plan).unwrap();
        assert!(refused_wire.deleted_artifacts.is_empty());
        assert!(refused_wire.rewrites.is_empty());
        assert!(refused_wire.relocations.is_empty());
        assert!(refused_wire.residuals.is_empty());
        let transaction = prepared_transaction(prepared).unwrap();
        super::super::validate::prepared(&transaction).unwrap();
        let tx::PreparedMode::Export(plan) = transaction.mode else {
            panic!("export adapter changed mode")
        };
        assert!(
            plan.entries
                .iter()
                .any(|entry| entry.target_path == "src/lib.rs")
        );
        assert!(
            !plan
                .entries
                .iter()
                .any(|entry| entry.target_path.starts_with("vibevm"))
        );

        let prepared = crate::prepare(crate::model::ScrapeRequest {
            root: source.path().to_path_buf(),
            contract: None,
            mode: ScrapeMode::InPlace,
        })
        .unwrap();
        assert!(
            prepared.plan.blockers.is_empty(),
            "{:?}",
            prepared.plan.blockers
        );
        let transaction = prepared_transaction(prepared).unwrap();
        super::super::validate::prepared(&transaction).unwrap();
    }
}
