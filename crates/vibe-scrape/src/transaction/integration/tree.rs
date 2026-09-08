use super::*;

pub(super) fn final_entries(
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
pub(super) fn source_for_final_path(prepared: &PreparedScrape, final_path: &str) -> String {
    prepared
        .plan
        .relocations
        .iter()
        .flat_map(|r| &r.mapped_descendants)
        .find(|m| m.to == final_path)
        .map(|m| m.from.clone())
        .unwrap_or_else(|| final_path.to_owned())
}
pub(super) fn final_rewrites(
    prepared: &PreparedScrape,
) -> BTreeMap<&str, &crate::model::PreparedRewrite> {
    let mut m = BTreeMap::new();
    for r in &prepared.rewrites {
        m.insert(r.path.as_str(), r);
    }
    m
}
pub(super) fn inventory_manifest(
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
pub(super) fn manifest(entries: Vec<tx::TreeEntry>) -> tx::TreeManifest {
    tx::logical_tree_manifest(entries)
}
pub(super) fn tree_entry(
    e: &crate::model::InventoryEntry,
) -> Result<tx::TreeEntry, tx::TransactionError> {
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
pub(super) fn file_state(
    e: &crate::model::InventoryEntry,
) -> Result<tx::FileState, tx::TransactionError> {
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
pub(super) fn file_state_from_item(
    e: &crate::model::PlanItem,
) -> Result<tx::FileState, tx::TransactionError> {
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
pub(super) fn relocation_path_state(
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

pub(super) fn subtree_digest(
    root_mode: Option<u32>,
    descendants: &[tx::SubtreeEntry],
) -> tx::Digest {
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
pub(super) fn transition(
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
pub(super) fn digest_text(value: &str) -> Result<tx::Digest, tx::TransactionError> {
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
pub(super) fn stable_id(path: &str) -> String {
    format!("{:x}", Sha256::digest(path.as_bytes()))
}

pub(super) fn rewrite_transaction_id(rewrite: &crate::model::PreparedRewrite) -> String {
    format!("rewrite-{}-{}", rewrite.id, stable_id(&rewrite.path))
}
