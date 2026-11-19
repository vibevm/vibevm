use super::*;
#[cfg(unix)]
use std::os::unix::fs::{DirBuilderExt, PermissionsExt};

fn private_temp() -> tempfile::TempDir {
    tempfile::Builder::new()
        .permissions({
            #[cfg(unix)]
            {
                std::fs::Permissions::from_mode(0o700)
            }
            #[cfg(not(unix))]
            {
                std::fs::metadata(std::env::temp_dir())
                    .unwrap()
                    .permissions()
            }
        })
        .tempdir()
        .unwrap()
}

fn value() -> ProtectedValue {
    ProtectedValue::new(b"SYNTHETIC_KEYRING_REDACTION_SENTINEL".to_vec()).unwrap()
}

#[test]
fn ids_are_canonical_random_v4_not_value_derivatives() {
    let first = SecretId::generate().unwrap();
    let second = SecretId::generate().unwrap();
    assert_ne!(first, second);
    assert_eq!(
        SecretId::try_from(first.as_str().to_owned()).unwrap(),
        first
    );
    assert!(SecretId::try_from("../foreign".to_owned()).is_err());
    assert!(SecretId::try_from(first.as_str().to_uppercase()).is_err());
    assert!(!format!("{:?}", value()).contains("SYNTHETIC_KEYRING_REDACTION_SENTINEL"));
}

#[cfg(unix)]
#[test]
fn private_revisions_rotate_without_changing_logical_identity_or_read_effects() {
    let root = private_temp();
    let store = Store::at_settings(root.path()).unwrap();
    let capability = Project::open(root.path()).unwrap().root_dir().unwrap();
    capability.verify_user_owned_parent().unwrap();
    assert!(store.list().unwrap().is_empty());
    assert!(!root.path().join("secrets").exists());
    let id = store
        .create("api-key", "Synthetic account", value())
        .unwrap();
    let before = store.details(&id).unwrap();
    let selected = root
        .path()
        .join("secrets/items")
        .join(id.as_str())
        .join(before.current_revision.as_str())
        .join("value");
    assert_eq!(
        std::fs::metadata(&selected).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert_eq!(
        std::fs::metadata(root.path().join("secrets"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o700
    );
    let snapshot = std::fs::read(root.path().join("secrets/index.toml")).unwrap();
    let modified = std::fs::metadata(root.path().join("secrets/index.toml"))
        .unwrap()
        .modified()
        .unwrap();
    let metadata = store.list().unwrap();
    assert!(!format!("{metadata:?}").contains("SYNTHETIC_KEYRING_REDACTION_SENTINEL"));
    assert_eq!(
        std::fs::read(root.path().join("secrets/index.toml")).unwrap(),
        snapshot
    );
    assert_eq!(
        std::fs::metadata(root.path().join("secrets/index.toml"))
            .unwrap()
            .modified()
            .unwrap(),
        modified
    );
    let revision = store
        .rotate(
            &id,
            &before.current_revision,
            ProtectedValue::new(b"SYNTHETIC_ROTATED".to_vec()).unwrap(),
        )
        .unwrap();
    assert_ne!(revision, before.current_revision);
    assert_eq!(store.details(&id).unwrap().current_revision, revision);
    assert!(
        selected.exists(),
        "old immutable revision retained, not eagerly deleted"
    );
    assert_eq!(
        store.rotate(&id, &before.current_revision, value()),
        Err(Error::Conflict)
    );
    assert_eq!(store.list().unwrap().len(), 1);
}

#[cfg(unix)]
#[test]
fn interrupted_rotation_keeps_previous_index_and_foreign_neighbor() {
    let root = private_temp();
    let store = Store::at_settings(root.path()).unwrap();
    let id = store.create("api-key", "Synthetic", value()).unwrap();
    let prior = store.details(&id).unwrap();
    let neighbor = root.path().join("secrets/foreign-note");
    std::fs::write(&neighbor, b"FOREIGN").unwrap();
    let old = std::fs::read(root.path().join("secrets/index.toml")).unwrap();
    FAIL_BEFORE_INDEX.with(|flag| flag.set(true));
    assert_eq!(
        store.rotate(&id, &prior.current_revision, value()),
        Err(Error::Publication)
    );
    assert_eq!(
        std::fs::read(root.path().join("secrets/index.toml")).unwrap(),
        old
    );
    assert_eq!(store.details(&id).unwrap(), prior);
    assert_eq!(std::fs::read(neighbor).unwrap(), b"FOREIGN");
}

#[cfg(unix)]
#[test]
fn two_rotations_of_same_expected_revision_have_one_winner() {
    let root = private_temp();
    let store = Store::at_settings(root.path()).unwrap();
    let id = store.create("api-key", "Synthetic", value()).unwrap();
    let rev = store.details(&id).unwrap().current_revision;
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(3));
    let mut threads = vec![];
    for _ in 0..2 {
        let barrier = barrier.clone();
        let path = root.path().to_owned();
        let id = id.clone();
        let rev = rev.clone();
        threads.push(std::thread::spawn(move || {
            barrier.wait();
            Store::at_settings(&path)
                .unwrap()
                .rotate(&id, &rev, value())
        }));
    }
    barrier.wait();
    let results: Vec<_> = threads.into_iter().map(|t| t.join().unwrap()).collect();
    assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter(|r| matches!(r, Err(Error::Conflict)))
            .count(),
        1
    );
}

#[cfg(unix)]
#[test]
fn symlinks_unsafe_permissions_and_hardlinked_values_refuse() {
    let root = private_temp();
    let outside = private_temp();
    std::os::unix::fs::symlink(outside.path(), root.path().join("secrets")).unwrap();
    let store = Store::at_settings(root.path()).unwrap();
    assert_eq!(
        store.create("api-key", "Synthetic", value()),
        Err(Error::Unavailable)
    );
    assert!(std::fs::read_dir(outside.path()).unwrap().next().is_none());
    std::fs::remove_file(root.path().join("secrets")).unwrap();
    std::fs::create_dir(root.path().join("secrets")).unwrap();
    std::fs::set_permissions(
        root.path().join("secrets"),
        std::fs::Permissions::from_mode(0o755),
    )
    .unwrap();
    assert_eq!(store.list(), Err(Error::Unavailable));
    std::fs::set_permissions(
        root.path().join("secrets"),
        std::fs::Permissions::from_mode(0o700),
    )
    .unwrap();
    let id = store.create("api-key", "Synthetic", value()).unwrap();
    let rev = store.details(&id).unwrap().current_revision;
    let file = root
        .path()
        .join("secrets/items")
        .join(id.as_str())
        .join(rev.as_str())
        .join("value");
    std::fs::hard_link(&file, root.path().join("alias")).unwrap();
    assert_eq!(store.list(), Err(Error::Unavailable));
}

#[cfg(not(unix))]
#[test]
fn unsupported_acl_refuses_without_creating_store() {
    let root = private_temp();
    let store = Store::at_settings(root.path()).unwrap();
    assert_eq!(
        store.create("api-key", "Synthetic", value()),
        Err(Error::Unavailable)
    );
    assert!(!root.path().join("secrets").exists());
}

#[cfg(target_os = "linux")]
#[test]
fn fifo_index_refuses_without_waiting_for_a_writer() {
    let root = private_temp();
    let secrets = root.path().join("secrets");
    std::fs::DirBuilder::new()
        .mode(0o700)
        .create(&secrets)
        .unwrap();
    let fifo = secrets.join("index.toml");
    assert!(
        std::process::Command::new("mkfifo")
            .arg(&fifo)
            .status()
            .unwrap()
            .success()
    );
    let start = std::time::Instant::now();
    assert_eq!(
        Store::at_settings(root.path()).unwrap().list(),
        Err(Error::Unavailable)
    );
    assert!(start.elapsed() < std::time::Duration::from_secs(1));
}
