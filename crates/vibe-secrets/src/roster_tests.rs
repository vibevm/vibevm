use super::*;
use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
fn root() -> tempfile::TempDir {
    tempfile::Builder::new()
        .permissions(std::fs::Permissions::from_mode(0o700))
        .tempdir()
        .unwrap()
}
fn value() -> ProtectedValue {
    ProtectedValue::new(b"SYNTHETIC_ROSTER_VALUE".to_vec()).unwrap()
}
fn expected(store: &Store, id: &SecretId) -> Expected {
    let s = store.snapshot().unwrap();
    Expected {
        store_revision: s.store_revision,
        current_revision: s.items[id].current_revision.clone(),
    }
}
#[test]
fn fresh_genesis_allows_logical_removal_and_reserves_bytes() {
    let dir = root();
    let store = Store::at_settings(dir.path()).unwrap();
    let id = store.create("api-key", "Synthetic", value()).unwrap();
    let exp = expected(&store, &id);
    let path = dir.path().join("secrets/index.toml");
    let index: Index = toml::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(index.schema, 2);
    assert_eq!(index.participants, Some(vec![]));
    store.remove_coordinated(&id, &exp).unwrap();
    assert_eq!(store.details(&id), Err(Error::NotFound));
    assert_eq!(store.remove_coordinated(&id, &exp), Err(Error::NotFound));
    assert!(
        dir.path()
            .join("secrets/items")
            .join(id.as_str())
            .join(exp.current_revision.as_str())
            .join("value")
            .exists()
    );
}
#[test]
fn incomplete_genesis_and_legacy_never_gain_empty_attestation() {
    for residue in [false, true] {
        let dir = root();
        let secrets = dir.path().join("secrets");
        std::fs::DirBuilder::new()
            .mode(0o700)
            .create(&secrets)
            .unwrap();
        if residue {
            std::fs::write(secrets.join("foreign"), b"KEEP").unwrap();
        }
        let store = Store::at_settings(dir.path()).unwrap();
        let id = store.create("api-key", "Legacy", value()).unwrap();
        let before = std::fs::read(secrets.join("index.toml")).unwrap();
        let index: Index = toml::from_str(std::str::from_utf8(&before).unwrap()).unwrap();
        assert_eq!(index.schema, 1);
        assert_eq!(
            store.remove_coordinated(&id, &expected(&store, &id)),
            Err(Error::RefusedUnknown)
        );
        assert_eq!(std::fs::read(secrets.join("index.toml")).unwrap(), before);
        if residue {
            assert_eq!(std::fs::read(secrets.join("foreign")).unwrap(), b"KEEP");
        }
    }
}
#[test]
fn unknown_participant_and_protocol_refuse_without_index_change() {
    let dir = root();
    let store = Store::at_settings(dir.path()).unwrap();
    let id = store.create("api-key", "Synthetic", value()).unwrap();
    let exp = expected(&store, &id);
    let (project, root) = store.open(false).unwrap().unwrap();
    let mut index = Store::load(&project, &root).unwrap();
    index.participants = Some(vec!["future-owner".into()]);
    let bytes = Store::prepare_index(&index).unwrap();
    project
        .write_atomic_in_with_mode(&root, "index.toml", bytes.as_bytes(), Some(0o600))
        .unwrap();
    assert_eq!(
        store.remove_coordinated(&id, &exp),
        Err(Error::RefusedUnknown)
    );
    assert_eq!(
        std::fs::read(root.join("index.toml")).unwrap(),
        bytes.as_bytes()
    );
    index.reference_protocol = Some(2);
    let bytes = toml::to_string(&index).unwrap();
    project
        .write_atomic_in_with_mode(&root, "index.toml", bytes.as_bytes(), Some(0o600))
        .unwrap();
    assert_eq!(
        store.remove_coordinated(&id, &exp),
        Err(Error::RefusedUnknown)
    );
    assert_eq!(
        std::fs::read(root.join("index.toml")).unwrap(),
        bytes.as_bytes()
    );
}

#[test]
fn process_roster_worker() {
    let Some(settings) = std::env::var_os("VIBE_TEST_ROSTER_ROOT") else {
        return;
    };
    let action = std::env::var("VIBE_TEST_ROSTER_ACTION").unwrap();
    let id = SecretId::try_from(std::env::var("VIBE_TEST_ROSTER_ID").unwrap()).unwrap();
    let store = Store::at_settings(std::path::Path::new(&settings)).unwrap();
    let path = std::path::Path::new(&settings);
    std::fs::write(path.join(format!("ready-{action}")), b"ready").unwrap();
    let start = std::time::Instant::now();
    while !path.join("go").exists() {
        assert!(start.elapsed() < std::time::Duration::from_secs(10));
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    let won = if action == "delete" {
        let expected = Expected {
            store_revision: 1,
            current_revision: SecretId::try_from(std::env::var("VIBE_TEST_ROSTER_REV").unwrap())
                .unwrap(),
        };
        store.remove_coordinated(&id, &expected).is_ok()
    } else {
        let (project, root) = store.open(false).unwrap().unwrap();
        let _inventory = root.private_lock("inventory.lock").unwrap();
        let _lock = root.private_lock("store.lock").unwrap();
        let mut index = Store::load(&project, &root).unwrap();
        if !index.items.contains_key(&id) {
            false
        } else {
            // Test-only consumer owns its own binding; participant admission is
            // durable before publication and protected through the entire write.
            index.participants = Some(vec!["test-consumer".into()]);
            index.store_revision += 1;
            let bytes = Store::prepare_index(&index).unwrap();
            project
                .write_atomic_in_with_mode(&root, "index.toml", bytes.as_bytes(), Some(0o600))
                .unwrap();
            project
                .write_atomic_in_with_mode(
                    &root,
                    "test-binding",
                    id.as_str().as_bytes(),
                    Some(0o600),
                )
                .unwrap();
            true
        }
    };
    std::fs::write(
        path.join(format!("result-{action}")),
        if won {
            b"won".as_slice()
        } else {
            b"refused".as_slice()
        },
    )
    .unwrap();
}

#[test]
fn independent_process_binding_and_delete_have_one_winner() {
    let dir = root();
    let store = Store::at_settings(dir.path()).unwrap();
    let id = store.create("api-key", "Synthetic", value()).unwrap();
    let exp = expected(&store, &id);
    let mut children = vec![];
    for action in ["delete", "bind"] {
        children.push(
            std::process::Command::new(std::env::current_exe().unwrap())
                .args(["--exact", "roster_tests::process_roster_worker", "--quiet"])
                .env("VIBE_TEST_ROSTER_ROOT", dir.path())
                .env("VIBE_TEST_ROSTER_ACTION", action)
                .env("VIBE_TEST_ROSTER_ID", id.as_str())
                .env("VIBE_TEST_ROSTER_REV", exp.current_revision.as_str())
                .stdout(std::process::Stdio::null())
                .spawn()
                .unwrap(),
        );
    }
    let start = std::time::Instant::now();
    while !["delete", "bind"]
        .iter()
        .all(|a| dir.path().join(format!("ready-{a}")).exists())
    {
        assert!(start.elapsed() < std::time::Duration::from_secs(10));
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    std::fs::write(dir.path().join("go"), b"go").unwrap();
    for mut child in children {
        assert!(child.wait().unwrap().success());
    }
    let deleted = std::fs::read(dir.path().join("result-delete")).unwrap() == b"won";
    let bound = std::fs::read(dir.path().join("result-bind")).unwrap() == b"won";
    assert_ne!(deleted, bound);
    assert!(!bound || store.details(&id).is_ok());
    assert_eq!(dir.path().join("secrets/test-binding").exists(), bound);
}
