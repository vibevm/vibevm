use super::*;
use crate::model::InventoryEntry;

#[test]
fn kept_descendant_shields_delete_directory() {
    let mut items = vec![
        PlanItem {
            path: "vibevm".into(),
            entry_kind: EntryKind::Directory,
            disposition: Disposition::Delete,
            class: FileClass::AuthoredMetadata,
            proof: None,
            modification: ModificationState::NotApplicable,
            owner: crate::contract::Owner::Vibe,
            sha256: None,
            bytes: None,
            unix_mode: None,
            rule_ids: vec![],
        },
        PlanItem {
            path: "vibevm/keep".into(),
            entry_kind: EntryKind::File,
            disposition: Disposition::Keep,
            class: FileClass::AuthoredProduct,
            proof: None,
            modification: ModificationState::NotApplicable,
            owner: crate::contract::Owner::Project,
            sha256: None,
            bytes: Some(0),
            unix_mode: None,
            rule_ids: vec![],
        },
    ];
    retain_delete_ancestors(&mut items);
    assert_eq!(items[0].disposition, Disposition::Keep);
}

#[test]
fn inventory_type_is_canonical_input() {
    let inventory = Inventory {
        entries: vec![InventoryEntry {
            path: "a".into(),
            kind: EntryKind::File,
            sha256: Some(format!("sha256:{}", "0".repeat(64))),
            bytes: Some(0),
            unix_mode: None,
            identity: None,
        }],
        tree_digest: "sha256:x".into(),
    };
    assert_eq!(inventory.entries[0].path, "a");
}
