macro_rules! external_tests {
    () => {
#[test]
fn project_identity_is_stable_and_never_exposes_raw_os_numbers() {
    let (_root, project) = project();
    let first = project.identity_token().unwrap();
    let second = project.identity_token().unwrap();
    assert_eq!(first, second);
    assert_eq!(first.len(), 71);
    assert!(first.starts_with("sha256:"));
    assert!(first[7..].bytes().all(|byte| byte.is_ascii_hexdigit()));
}

#[test]
fn an_explicit_external_store_is_nofollow_and_proven_disjoint() {
    let scope = tempfile::tempdir().unwrap();
    let project_path = scope.path().join("project");
    let store_path = scope.path().join("state/scrape");
    fs::create_dir(&project_path).unwrap();
    let project = Project::open(&project_path).unwrap();
    let store = ExternalStore::open_or_create_disjoint(&store_path, &project).unwrap();
    store.prove_disjoint_from(&project).unwrap();
    assert_eq!(store.path(), store_path);
    assert_eq!(store.bootstrap_durability().len(), 2);
    assert_eq!(
        store.require_durable_bootstrap().is_ok(),
        store.bootstrap_durability().iter().all(|sync| matches!(
            sync.durability,
            DirectoryDurability::Synced
        ) || (cfg!(windows)
            && sync.durability == DirectoryDurability::JournalRecoverable))
    );

    let inside = ExternalStore::open_or_create(&project_path.join("private-state")).unwrap();
    assert!(inside.prove_disjoint_from(&project).is_err());
}

#[test]
fn disjoint_store_preflight_leaves_zero_mutation_for_an_inside_or_linked_path() {
    let scope = tempfile::tempdir().unwrap();
    let project_path = scope.path().join("project");
    fs::create_dir(&project_path).unwrap();
    let project = Project::open(&project_path).unwrap();

    let inside = project_path.join("must-not-exist/store");
    assert!(ExternalStore::open_or_create_disjoint(&inside, &project).is_err());
    assert!(!project_path.join("must-not-exist").exists());

    let alias = scope.path().join("project-alias");
    if link_directory(&project_path, &alias) {
        let linked_store = alias.join("also-must-not-exist/store");
        assert!(ExternalStore::open_or_create_disjoint(&linked_store, &project).is_err());
        assert!(!project_path.join("also-must-not-exist").exists());
    }
}

#[test]
#[cfg(windows)]
fn windows_external_bootstrap_admits_only_the_liveness_safe_journal_recoverable_class() {
    let scope = tempfile::tempdir().unwrap();
    let project_path = scope.path().join("project");
    fs::create_dir(&project_path).unwrap();
    let project = Project::open(&project_path).unwrap();
    let store =
        ExternalStore::open_or_create_disjoint(&scope.path().join("external/state"), &project)
            .unwrap();

    assert!(!store.bootstrap_durability().is_empty());
    assert!(store.bootstrap_durability().iter().all(|sync| matches!(
        sync.durability,
        DirectoryDurability::Synced | DirectoryDurability::JournalRecoverable
    )));
    store.require_durable_bootstrap().unwrap();
}

#[test]
#[cfg(unix)]
fn disjoint_proof_uses_retained_ancestry_after_a_namespace_alias_swap() {
    let scope = tempfile::tempdir().unwrap();
    let project_path = scope.path().join("project");
    let moved_project = scope.path().join("project-moved");
    let store_path = scope.path().join("store");
    fs::create_dir(&project_path).unwrap();
    fs::create_dir(&store_path).unwrap();
    let project = Project::open(&project_path).unwrap();
    let store = ExternalStore::open_or_create_disjoint(&store_path, &project).unwrap();
    fs::rename(&project_path, &moved_project).unwrap();
    if link_directory(&store_path, &project_path) {
        store.prove_disjoint_from(&project).unwrap();
        remove_directory_link(&project_path);
    }
    drop(store);
    drop(project);
    fs::rename(&moved_project, &project_path).unwrap();
}

#[test]
#[cfg(windows)]
fn pinned_project_namespace_prevents_the_alias_swap_on_windows() {
    let scope = tempfile::tempdir().unwrap();
    let project_path = scope.path().join("project");
    let moved_project = scope.path().join("project-moved");
    let store_path = scope.path().join("store");
    fs::create_dir(&project_path).unwrap();
    fs::create_dir(&store_path).unwrap();
    let project = Project::open(&project_path).unwrap();
    let store = ExternalStore::open_or_create_disjoint(&store_path, &project).unwrap();
    assert!(fs::rename(&project_path, &moved_project).is_err());
    store.prove_disjoint_from(&project).unwrap();
    assert!(project_path.is_dir());
}

#[test]
fn external_lock_rechecks_identity_and_durable_writes_report_parent_support() {
    let scope = tempfile::tempdir().unwrap();
    let store = ExternalStore::open_or_create(&scope.path().join("state")).unwrap();
    let _lock = store.open_and_lock_project("sha256:project-key").unwrap();
    let write = store
        .write_durable("transactions/TX0001/journal", b"sealed")
        .unwrap();
    assert!(write.file_synced);
    assert!(matches!(
        write.parent,
        DirectoryDurability::Synced
            | DirectoryDurability::JournalRecoverable
            | DirectoryDurability::Unsupported(_)
            | DirectoryDurability::Failed(_)
    ));
    assert_eq!(
        fs::read(scope.path().join("state/transactions/TX0001/journal")).unwrap(),
        b"sealed"
    );
}

#[test]
#[cfg(windows)]
fn held_external_lock_denies_namespace_replacement_for_its_lifetime() {
    let scope = tempfile::tempdir().unwrap();
    let store = ExternalStore::open_or_create(&scope.path().join("state")).unwrap();
    let lock = store.open_and_lock_project("sha256:project-key").unwrap();
    let locks = scope.path().join("state/locks");
    let lock_path = fs::read_dir(&locks)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let moved = locks.join("replacement.lock");

    assert!(fs::rename(&lock_path, &moved).is_err());
    lock.require_still_named().unwrap();
    drop(lock);
    fs::rename(&lock_path, &moved).unwrap();
}

#[test]
fn external_capabilities_read_list_and_open_without_ambient_reopen() {
    let scope = tempfile::tempdir().unwrap();
    let project_path = scope.path().join("project");
    fs::create_dir(&project_path).unwrap();
    let project = Project::open(&project_path).unwrap();
    let store =
        ExternalStore::open_or_create_disjoint(&scope.path().join("store"), &project).unwrap();
    store
        .write_durable("pending/z/journal", b"z-journal")
        .unwrap();
    store
        .write_durable("pending/a/journal", b"a-journal")
        .unwrap();

    let pending = store.open_directory("pending").unwrap().unwrap();
    assert_eq!(pending.child_names_bounded(2).unwrap(), ["a", "z"]);
    assert!(pending.child_names_bounded(1).is_err());
    let a = pending.open_child("a").unwrap().unwrap();
    let journal = a.read_stable_bounded("journal", 64).unwrap().unwrap();
    assert_eq!(journal.bytes, b"a-journal");
    assert_eq!(
        store
            .read_stable_bounded("pending/a/journal", 64)
            .unwrap()
            .unwrap()
            .bytes,
        b"a-journal"
    );
    assert!(pending.open_child("missing").unwrap().is_none());
}

#[test]
#[cfg(windows)]
fn external_owned_transaction_directory_retires_one_durable_intent_at_a_time() {
    let scope = tempfile::tempdir().unwrap();
    let project_path = scope.path().join("project");
    fs::create_dir(&project_path).unwrap();
    let project = Project::open(&project_path).unwrap();
    let store =
        ExternalStore::open_or_create_disjoint(&scope.path().join("store"), &project).unwrap();
    let root = store.root_directory().unwrap();
    let (transactions, _, _) = root.ensure_child("transactions").unwrap();
    let (project_home, _, _) = transactions.ensure_child("project-a").unwrap();
    let owned = project_home
        .create_owned_child_exclusive("TX000001", "transaction-owner")
        .unwrap();
    store
        .write_durable("transactions/project-a/TX000001/journal", b"journal")
        .unwrap();
    store
        .write_durable(
            "transactions/project-a/TX000001/snapshots/contract",
            b"contract",
        )
        .unwrap();
    drop(owned);

    let tx = project_home.open_child("TX000001").unwrap().unwrap();
    let journal_state = tx.inspect_child_state("journal").unwrap().unwrap();
    assert_eq!(
        tx.read_stable_bounded("journal", 64)
            .unwrap()
            .unwrap()
            .bytes,
        b"journal"
    );
    assert!(matches!(
        tx.remove_file_expected("journal", &journal_state),
        Ok(DirectoryDurability::Synced)
            | Ok(DirectoryDurability::JournalRecoverable)
            | Ok(DirectoryDurability::Unsupported(_))
            | Ok(DirectoryDurability::Failed(_))
    ));

    // Recreate a fresh sealed transaction for the manifest-bound retirement
    // path; expected-state single-file removal above is intentionally separate.
    let owned = project_home
        .create_owned_child_exclusive("TX000002", "transaction-owner-2")
        .unwrap();
    store
        .write_durable("transactions/project-a/TX000002/journal", b"journal")
        .unwrap();
    let identity = owned.identity().clone();
    let manifest = owned.manifest().unwrap();
    drop(owned);
    let mut progress = OwnedTreeCleanupProgress::new();
    loop {
        match project_home
            .prepare_owned_child_retirement(
                "TX000002",
                "transaction-owner-2",
                &identity,
                &manifest,
                &progress,
            )
            .unwrap()
        {
            CleanupPreparation::Complete => break,
            CleanupPreparation::Intent(intent) => {
                let completion = project_home
                    .execute_owned_child_retirement(
                        "TX000002",
                        "transaction-owner-2",
                        &identity,
                        &manifest,
                        &progress,
                        &intent,
                    )
                    .unwrap();
                progress.record(&completion).unwrap();
            }
        }
    }
    assert!(project_home.open_child("TX000002").unwrap().is_none());
    assert!(!identity.as_str().is_empty());
}

#[test]
fn same_filesystem_comparison_has_a_deterministic_mismatch_gate() {
    let (_root, project) = project();
    let root = project.root_dir().unwrap();
    let child = root.ensure_child("candidate").unwrap();
    assert!(root.same_filesystem(&child).unwrap());
    arm_same_filesystem_check(Some(Box::new(|actual| {
        assert!(actual);
        false
    })));
    assert!(!root.same_filesystem(&child).unwrap());
    arm_same_filesystem_check(None);
}
    };
}
