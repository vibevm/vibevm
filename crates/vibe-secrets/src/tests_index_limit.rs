use super::*;
#[cfg(unix)]
#[test]
fn oversized_index_refuses_before_value_and_preserves_previous_selection() {
    use std::os::unix::fs::PermissionsExt;
    let root = tempfile::Builder::new()
        .permissions(std::fs::Permissions::from_mode(0o700))
        .tempdir()
        .unwrap();
    let store = Store::at_settings(root.path()).unwrap();
    let id = store
        .create(
            "api-key",
            "Synthetic",
            ProtectedValue::new(b"SYNTHETIC_OLD".to_vec()).unwrap(),
        )
        .unwrap();
    let metadata = store.details(&id).unwrap();
    let index_path = root.path().join("secrets/index.toml");
    let old = std::fs::read(&index_path).unwrap();
    let revisions = root.path().join("secrets/items").join(id.as_str());
    let before = std::fs::read_dir(&revisions).unwrap().count();
    let foreign = root.path().join("secrets/foreign-note");
    std::fs::write(&foreign, b"FOREIGN").unwrap();
    INDEX_WRITE_LIMIT.with(|limit| limit.set(old.len() - 1));
    let refused = store.rotate(
        &id,
        &metadata.current_revision,
        ProtectedValue::new(b"SYNTHETIC_NEW".to_vec()).unwrap(),
    );
    INDEX_WRITE_LIMIT.with(|limit| limit.set(INDEX_LIMIT));
    assert_eq!(refused, Err(Error::Invalid));
    assert_eq!(std::fs::read(&index_path).unwrap(), old);
    assert_eq!(
        std::fs::read_dir(&revisions).unwrap().count(),
        before,
        "no value revision published before bounded index preflight"
    );
    assert_eq!(store.details(&id).unwrap(), metadata);
    assert_eq!(std::fs::read(&foreign).unwrap(), b"FOREIGN");
    // UUID spellings and one-digit revision have fixed length: exact boundary succeeds.
    INDEX_WRITE_LIMIT.with(|limit| limit.set(old.len()));
    let accepted = store.rotate(
        &id,
        &metadata.current_revision,
        ProtectedValue::new(b"SYNTHETIC_NEW".to_vec()).unwrap(),
    );
    INDEX_WRITE_LIMIT.with(|limit| limit.set(INDEX_LIMIT));
    assert!(accepted.is_ok());
    assert_eq!(std::fs::read(&index_path).unwrap().len(), old.len());
}
