use super::*;

pub(super) fn build_projected_final(
    project: &vibe_safefs::Project,
    inventory: &Inventory,
    items: &[PlanItem],
    rewrites: &[PreparedRewrite],
    contract: &crate::contract::Contract,
) -> Result<Vec<crate::rewrite::ProjectedEntry>, ScrapeError> {
    let inventory_by_path = inventory
        .entries
        .iter()
        .map(|entry| (entry.path.as_str(), entry))
        .collect::<BTreeMap<_, _>>();
    let mut projected = BTreeMap::<String, crate::rewrite::ProjectedEntry>::new();
    for item in items {
        let path = match item.disposition {
            Disposition::Delete | Disposition::DeleteLast => continue,
            Disposition::Relocate => {
                let row = contract
                    .relocate
                    .iter()
                    .find(|row| at_or_below(&item.path, &row.from))
                    .ok_or_else(|| {
                        ScrapeError::inventory(format!(
                            "relocated plan item `{}` has no relocation row",
                            item.path
                        ))
                    })?;
                let suffix = item.path.strip_prefix(&row.from).ok_or_else(|| {
                    ScrapeError::inventory(format!(
                        "relocated plan item `{}` is outside `{}`",
                        item.path, row.from
                    ))
                })?;
                format!("{}{}", row.to, suffix)
            }
            Disposition::Keep | Disposition::Rewrite => item.path.clone(),
        };
        let source = inventory_by_path.get(item.path.as_str()).ok_or_else(|| {
            ScrapeError::inventory(format!(
                "plan item `{}` is absent from inventory",
                item.path
            ))
        })?;
        let bytes = if item.entry_kind == EntryKind::Directory {
            None
        } else if let Some(rewrite) = rewrites
            .iter()
            .rev()
            .find(|rewrite| rewrite.path == item.path)
        {
            Some(rewrite.after_bytes.clone())
        } else if item.disposition == Disposition::Rewrite {
            return Err(ScrapeError::rewrite(format!(
                "rewrite plan item `{}` has no prepared rewrite",
                item.path
            )));
        } else {
            Some(read_inventoried_bytes(project, source)?)
        };
        projected.insert(
            path.clone(),
            crate::rewrite::ProjectedEntry {
                path,
                kind: item.entry_kind,
                bytes,
                unix_mode: item.unix_mode,
            },
        );
    }

    let paths = projected.keys().cloned().collect::<Vec<_>>();
    for path in paths {
        let mut current = path.as_str();
        while let Some((parent, _)) = current.rsplit_once('/') {
            projected
                .entry(parent.to_owned())
                .or_insert_with(|| crate::rewrite::ProjectedEntry {
                    path: parent.to_owned(),
                    kind: EntryKind::Directory,
                    bytes: None,
                    unix_mode: inventory_by_path
                        .get(parent)
                        .and_then(|entry| entry.unix_mode),
                });
            current = parent;
        }
    }
    Ok(projected.into_values().collect())
}

fn read_inventoried_bytes(
    project: &vibe_safefs::Project,
    entry: &crate::model::InventoryEntry,
) -> Result<Vec<u8>, ScrapeError> {
    let expected_size = entry.bytes.ok_or_else(|| {
        ScrapeError::inventory(format!("file `{}` has no inventoried size", entry.path))
    })?;
    let cap = usize::try_from(expected_size).map_err(|_| {
        ScrapeError::inventory(format!("file `{}` is too large to project", entry.path))
    })?;
    let snapshot = project
        .read_file_snapshot_bounded(&entry.path, cap)
        .map_err(|error| {
            ScrapeError::inventory(format!(
                "re-reading `{}` for final projection: {error:#}",
                entry.path
            ))
        })?
        .ok_or_else(|| ScrapeError::inventory(format!("`{}` disappeared", entry.path)))?;
    if snapshot.size != expected_size
        || Some(snapshot.identity) != entry.identity
        || snapshot.unix_mode != entry.unix_mode
        || entry.sha256.as_deref() != Some(&format!("sha256:{}", snapshot.sha256))
    {
        return Err(ScrapeError::inventory(format!(
            "`{}` changed after inventory while projecting the final tree",
            entry.path
        )));
    }
    Ok(snapshot.bytes)
}

pub(super) fn validate_contract_last(
    snapshot: &ContractSnapshot,
    inventory: &Inventory,
    items: &mut [PlanItem],
    rewrites: &[PreparedRewrite],
    contract: &crate::contract::Contract,
    blockers: &mut Vec<Blocker>,
) -> ContractBoundary {
    let path = &snapshot.display_path;
    let entry = inventory.entries.iter().find(|entry| &entry.path == path);
    match entry {
        Some(entry) if entry.kind == EntryKind::File => {}
        Some(_) => blockers.push(
            Blocker::new(
                "contract-not-regular",
                "contained contract is not a regular file",
            )
            .at(path),
        ),
        None => blockers.push(
            Blocker::new(
                "contract-not-inventory",
                "contained contract is absent from inventory",
            )
            .at(path),
        ),
    }
    if entry.and_then(|entry| entry.sha256.as_deref()) != Some(snapshot.sha256.as_str()) {
        blockers.push(
            Blocker::new(
                "contract-snapshot-drift",
                "inventoried contract digest differs from the validated snapshot",
            )
            .at(path),
        );
    }
    if entry.and_then(|entry| entry.identity) != Some(snapshot.identity) {
        blockers.push(
            Blocker::new(
                "contract-snapshot-identity-drift",
                "inventoried contract identity differs from the validated snapshot",
            )
            .at(path),
        );
    }
    match items.iter_mut().find(|item| &item.path == path) {
        Some(item) if item.disposition == Disposition::Delete => {
            item.disposition = Disposition::DeleteLast
        }
        Some(_) => blockers.push(
            Blocker::new(
                "contract-not-effective-delete",
                "contained contract must resolve to exactly one effective-delete file",
            )
            .at(path),
        ),
        None => {}
    }
    if rewrites.iter().any(|row| &row.path == path) {
        blockers.push(
            Blocker::new(
                "contract-rewrite-conflict",
                "delete-last contract cannot be a rewrite target",
            )
            .at(path),
        );
    }
    if contract
        .relocate
        .iter()
        .any(|row| &row.from == path || &row.to == path)
    {
        blockers.push(
            Blocker::new(
                "contract-relocation-conflict",
                "delete-last contract cannot be relocation source/destination",
            )
            .at(path),
        );
    }
    if contract.baseline.iter().any(|row| &row.path == path) {
        blockers.push(
            Blocker::new(
                "contract-baseline-conflict",
                "delete-last contract cannot be a baseline target",
            )
            .at(path),
        );
    }
    let mut ancestors = Vec::new();
    let mut current = path.as_str();
    while let Some((parent, _)) = current.rsplit_once('/') {
        let surviving_descendant = items.iter().any(|item| {
            item.path != *path
                && item.path != parent
                && at_or_below(&item.path, parent)
                && matches!(item.disposition, Disposition::Keep | Disposition::Rewrite)
        }) || contract
            .relocate
            .iter()
            .any(|row| at_or_below(&row.to, parent));
        if surviving_descendant {
            break;
        }
        ancestors.push(parent.to_owned());
        current = parent;
    }
    ContractBoundary::DeleteLast {
        path: path.clone(),
        empty_ancestors: ancestors,
    }
}
