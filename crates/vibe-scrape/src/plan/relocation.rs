use super::*;

pub(super) fn validate_relocations(
    contract: &crate::contract::Contract,
    inventory: &Inventory,
    items: &[PlanItem],
    blockers: &mut Vec<Blocker>,
) {
    for row in &contract.relocate {
        let found = inventory
            .entries
            .iter()
            .any(|entry| at_or_below(&entry.path, &row.from));
        if row.required && !found {
            blockers.push(
                Blocker::new(
                    "required-relocation-absent",
                    "required relocation source is absent",
                )
                .at(&row.from)
                .rule(&row.id),
            );
        }
        if inventory
            .entries
            .iter()
            .any(|entry| at_or_below(&entry.path, &row.to))
        {
            blockers.push(
                Blocker::new(
                    "relocation-destination-exists",
                    "relocation destination already exists",
                )
                .at(&row.to)
                .rule(&row.id),
            );
        }
        if inventory.entries.iter().any(|entry| {
            entry.kind == EntryKind::File
                && entry.path != row.to
                && at_or_below(&row.to, &entry.path)
        }) {
            blockers.push(
                Blocker::new(
                    "relocation-destination-ancestor-file",
                    "a relocation destination ancestor is an existing file",
                )
                .at(&row.to)
                .rule(&row.id),
            );
        }
        let under_delete = items.iter().any(|item| {
            item.disposition == Disposition::Delete && at_or_below(&row.to, &item.path)
        });
        if under_delete {
            blockers.push(
                Blocker::new(
                    "relocation-destination-deleted",
                    "relocation destination lies below a deletion root",
                )
                .at(&row.to)
                .rule(&row.id),
            );
        }
    }
}

pub(super) fn retain_delete_ancestors(items: &mut [PlanItem]) {
    let retained = items
        .iter()
        .filter(|item| {
            matches!(
                item.disposition,
                Disposition::Keep | Disposition::Rewrite | Disposition::Relocate
            )
        })
        .map(|item| item.path.clone())
        .collect::<Vec<_>>();
    for item in items {
        if item.entry_kind == EntryKind::Directory
            && item.disposition == Disposition::Delete
            && retained
                .iter()
                .any(|path| path.starts_with(&(item.path.clone() + "/")))
        {
            item.disposition = Disposition::Keep;
        }
    }
}
