use super::*;
use std::os::unix::fs::PermissionsExt;
use std::sync::{Arc, Barrier, Mutex, MutexGuard};

fn setup() -> (tempfile::TempDir, Store, SecretId, Expected) {
    let dir = tempfile::Builder::new()
        .permissions(std::fs::Permissions::from_mode(0o700))
        .tempdir()
        .unwrap();
    let store = Store::at_settings(dir.path()).unwrap();
    let id = store
        .create(
            "api-key",
            "Before",
            ProtectedValue::new(b"SYNTHETIC_PRIVATE_VALUE".to_vec()).unwrap(),
        )
        .unwrap();
    let snapshot = store.snapshot().unwrap();
    let expected = Expected {
        store_revision: snapshot.store_revision,
        current_revision: snapshot.items[&id].current_revision.clone(),
    };
    (dir, store, id, expected)
}
#[derive(Default)]
struct Inventory(Mutex<Vec<String>>);
struct Guard<'a>(MutexGuard<'a, Vec<String>>);
impl ReferenceInventory for Inventory {
    fn acquire(&self) -> Result<Box<dyn ReferenceGuard + '_>, Error> {
        Ok(Box::new(Guard(
            self.0.lock().map_err(|_| Error::RefusedUnknown)?,
        )))
    }
}
impl ReferenceGuard for Guard<'_> {
    fn consumers(&self, _: &SecretId) -> Result<Vec<String>, Error> {
        Ok(self.0.clone())
    }
}

#[test]
fn edit_preserves_identity_noop_and_stale_tokens() {
    let (dir, store, id, expected) = setup();
    let path = dir.path().join("secrets/index.toml");
    let before = std::fs::read(&path).unwrap();
    assert_eq!(
        store
            .edit_label(&id, &expected, "Before")
            .unwrap()
            .store_revision,
        expected.store_revision
    );
    assert_eq!(std::fs::read(&path).unwrap(), before);
    let next = store.edit_label(&id, &expected, "After").unwrap();
    assert_eq!(next.store_revision, expected.store_revision + 1);
    assert_eq!(next.items[&id].current_revision, expected.current_revision);
    assert_eq!(next.items[&id].kind, "api-key");
    assert_eq!(
        store.edit_label(&id, &expected, "After"),
        Err(Error::Conflict)
    );
    assert!(!format!("{next:?}").contains("SYNTHETIC_PRIVATE_VALUE"));
}

#[test]
fn removal_requires_complete_inventory_and_retains_reserved_paths() {
    let (dir, store, id, expected) = setup();
    let index_path = dir.path().join("secrets/index.toml");
    let original = std::fs::read(&index_path).unwrap();
    assert_eq!(
        store.remove(&id, &expected, None),
        Err(Error::RefusedUnknown)
    );
    assert_eq!(std::fs::read(&index_path).unwrap(), original);
    let inventory = Inventory(Mutex::new(vec!["ai:primary".into()]));
    assert_eq!(
        store.remove(&id, &expected, Some(&inventory)),
        Err(Error::RefusedReferenced {
            consumers: vec!["ai:primary".into()]
        })
    );
    assert_eq!(std::fs::read(&index_path).unwrap(), original);
    *inventory.0.lock().unwrap() = vec!["unsafe/path".into()];
    assert_eq!(
        store.remove(&id, &expected, Some(&inventory)),
        Err(Error::RefusedUnknown)
    );
    assert_eq!(std::fs::read(&index_path).unwrap(), original);
    inventory.0.lock().unwrap().clear();
    FAIL_BEFORE_INDEX.with(|flag| flag.set(true));
    assert_eq!(
        store.remove(&id, &expected, Some(&inventory)),
        Err(Error::Publication)
    );
    assert_eq!(std::fs::read(&index_path).unwrap(), original);
    assert!(store.details(&id).is_ok());
    let removed = store.remove(&id, &expected, Some(&inventory)).unwrap();
    assert_eq!(removed.store_revision, expected.store_revision + 1);
    assert_eq!(store.details(&id), Err(Error::NotFound));
    assert_eq!(
        store.remove(&id, &expected, Some(&inventory)),
        Err(Error::NotFound)
    );
    assert!(store.list().unwrap().is_empty());
    let (project, root) = store.open(false).unwrap().unwrap();
    let items = root.private_child("items", false).unwrap().unwrap();
    assert!(items.create_private_child_exclusive(id.as_str()).is_err());
    drop(project);
    assert!(
        dir.path()
            .join("secrets/items")
            .join(id.as_str())
            .join(expected.current_revision.as_str())
            .join("value")
            .exists()
    );
}

#[test]
fn interrupted_edit_preserves_snapshot() {
    let (_dir, store, id, expected) = setup();
    let before = store.snapshot().unwrap();
    FAIL_BEFORE_INDEX.with(|flag| flag.set(true));
    assert_eq!(
        store.edit_label(&id, &expected, "After"),
        Err(Error::Publication)
    );
    assert_eq!(store.snapshot().unwrap(), before);
}

#[test]
fn concurrent_rename_and_rotation_preserve_both_changes_or_conflict() {
    let (dir, _store, id, expected) = setup();
    let barrier = Arc::new(Barrier::new(3));
    let path = dir.path().to_owned();
    let (i, e, b) = (id.clone(), expected.clone(), barrier.clone());
    let edit = std::thread::spawn(move || {
        b.wait();
        Store::at_settings(&path)
            .unwrap()
            .edit_label(&i, &e, "After")
    });
    let path = dir.path().to_owned();
    let (i, e, b) = (id.clone(), expected.clone(), barrier.clone());
    let rotate = std::thread::spawn(move || {
        b.wait();
        Store::at_settings(&path).unwrap().rotate(
            &i,
            &e.current_revision,
            ProtectedValue::new(b"SYNTHETIC_NEW".to_vec()).unwrap(),
        )
    });
    barrier.wait();
    let edit = edit.join().unwrap();
    assert!(rotate.join().unwrap().is_ok());
    let final_item = Store::at_settings(dir.path())
        .unwrap()
        .details(&id)
        .unwrap();
    assert_ne!(final_item.current_revision, expected.current_revision);
    match edit {
        Ok(_) => assert_eq!(final_item.label, "After"),
        Err(Error::Conflict) => assert_eq!(final_item.label, "Before"),
        other => panic!("unexpected {other:?}"),
    }
}

#[test]
fn delete_and_new_reference_never_leave_dangling_binding() {
    let (dir, _store, id, expected) = setup();
    let inventory = Arc::new(Inventory::default());
    let barrier = Arc::new(Barrier::new(3));
    let (path, i, e, inv, b) = (
        dir.path().to_owned(),
        id.clone(),
        expected,
        inventory.clone(),
        barrier.clone(),
    );
    let delete = std::thread::spawn(move || {
        b.wait();
        Store::at_settings(&path)
            .unwrap()
            .remove(&i, &e, Some(inv.as_ref()))
    });
    let (path, i, inv, b) = (
        dir.path().to_owned(),
        id.clone(),
        inventory.clone(),
        barrier.clone(),
    );
    let bind = std::thread::spawn(move || {
        b.wait();
        let mut references = inv.0.lock().unwrap();
        if Store::at_settings(&path).unwrap().details(&i).is_ok() {
            references.push("ai:primary".into());
            true
        } else {
            false
        }
    });
    barrier.wait();
    let deleted = delete.join().unwrap();
    let bound = bind.join().unwrap();
    assert_eq!(usize::from(deleted.is_ok()) + usize::from(bound), 1);
    let exists = Store::at_settings(dir.path()).unwrap().details(&id).is_ok();
    assert!(!bound || exists);
    assert_eq!(!inventory.0.lock().unwrap().is_empty(), bound);
}
