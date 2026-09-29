specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-APPLY");
macro_rules! external_sync_tests {
    () => {
#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-APPLY",
    r = 1
)]
fn retained_image_sync_confirms_data_without_rewrite_and_refuses_injected_failure() {
    let (scope, project, store_path) = existing_external_fixture();
    let store = ExternalStore::open_or_create_disjoint(&store_path, &project).unwrap();
    store.write_durable("before.bin", b"approved").unwrap();
    let expected = store
        .read_stable_bounded("before.bin", 32)
        .unwrap()
        .unwrap();
    let before = external_namespace_snapshot(scope.path());
    let calls = std::rc::Rc::new(std::cell::Cell::new(0));
    let observed = calls.clone();
    crate::arm_before_retained_file_sync(Some(Box::new(move || {
        observed.set(observed.get() + 1);
        false
    })));
    let durability = store
        .sync_stable_file_bounded("before.bin", 32, &expected)
        .unwrap();
    crate::arm_before_retained_file_sync(None);
    assert_eq!(calls.get(), 1);
    #[cfg(unix)]
    assert_eq!(durability, DirectoryDurability::Synced);
    println!("confirmed retained file sync; parent tier: {durability:?}");
    assert_eq!(external_namespace_snapshot(scope.path()), before);
    crate::arm_before_retained_file_sync(Some(Box::new(|| true)));
    assert!(
        store
            .sync_stable_file_bounded("before.bin", 32, &expected)
            .is_err()
    );
    crate::arm_before_retained_file_sync(None);
    assert_eq!(external_namespace_snapshot(scope.path()), before);
    assert!(
        store
            .sync_stable_file_bounded("before.bin", 1, &expected)
            .is_err()
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-056#SEC-NO-FOLLOW")]
fn retained_image_sync_refuses_replacement_changed_bytes_missing_and_links() {
    let (scope, project, store_path) = existing_external_fixture();
    let store = ExternalStore::open_or_create_disjoint(&store_path, &project).unwrap();
    store.write_durable("before.bin", b"approved").unwrap();
    let expected = store
        .read_stable_bounded("before.bin", 32)
        .unwrap()
        .unwrap();
    let path = store_path.join("before.bin");
    let saved = store_path.join("saved.bin");
    fs::rename(&path, &saved).unwrap();
    fs::write(&path, b"approved").unwrap();
    let before = external_namespace_snapshot(scope.path());
    assert!(
        store
            .sync_stable_file_bounded("before.bin", 32, &expected)
            .is_err()
    );
    assert_eq!(external_namespace_snapshot(scope.path()), before);
    let replacement = store
        .read_stable_bounded("before.bin", 32)
        .unwrap()
        .unwrap();
    fs::write(&path, b"changed").unwrap();
    let before = external_namespace_snapshot(scope.path());
    assert!(
        store
            .sync_stable_file_bounded("before.bin", 32, &replacement)
            .is_err()
    );
    assert_eq!(external_namespace_snapshot(scope.path()), before);
    fs::remove_file(&path).unwrap();
    let before = external_namespace_snapshot(scope.path());
    assert!(
        store
            .sync_stable_file_bounded("before.bin", 32, &expected)
            .is_err()
    );
    assert_eq!(external_namespace_snapshot(scope.path()), before);
    fs::hard_link(&saved, &path).unwrap();
    let before = external_namespace_snapshot(scope.path());
    assert!(
        store
            .sync_stable_file_bounded("before.bin", 32, &expected)
            .is_err()
    );
    assert_eq!(external_namespace_snapshot(scope.path()), before);
    fs::remove_file(&path).unwrap();
    if link_file(&saved, &path) {
        let before = external_namespace_snapshot(scope.path());
        assert!(
            store
                .sync_stable_file_bounded("before.bin", 32, &expected)
                .is_err()
        );
        assert_eq!(external_namespace_snapshot(scope.path()), before);
    }
}
    };
}
