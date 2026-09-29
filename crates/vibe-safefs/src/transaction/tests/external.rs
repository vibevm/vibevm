macro_rules! external_tests {
    () => {
#[test]
#[cfg(unix)]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-056#SEC-NO-FOLLOW")]
fn directory_flush_reopens_readable_capability_without_namespace_creation() {
    let (scope, project, _store) = existing_external_fixture();
    let root = project.root_dir().unwrap();
    let before = external_namespace_snapshot(scope.path());
    let held = root.dir.try_clone().unwrap().into_std_file();
    #[cfg(target_os = "linux")]
    {
        use std::os::fd::AsRawFd;
        let flags = fs::read_to_string(format!("/proc/self/fdinfo/{}", held.as_raw_fd())).unwrap();
        println!("held directory fdinfo: {flags}");
    }
    println!("held directory sync result: {:?}", held.sync_all());
    let readable = root.dir.open(".").unwrap().into_std();
    assert!(readable.metadata().unwrap().is_dir());
    assert_eq!(
        crate::transaction::file_identity(&readable, root.path()).unwrap(),
        root.identity().unwrap()
    );
    println!("readable directory sync result: {:?}", readable.sync_all());
    readable.sync_all().unwrap();
    assert_eq!(root.sync_directory(), DirectoryDurability::Synced);
    assert_eq!(external_namespace_snapshot(scope.path()), before);
}

#[test]
#[cfg(unix)]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-056#SEC-NO-FOLLOW")]
fn directory_flush_refuses_a_foreign_expected_identity() {
    let (scope, project, store_path) = existing_external_fixture();
    fs::create_dir(&store_path).unwrap();
    let other = Project::open(&store_path).unwrap().root_dir().unwrap();
    let root = project.root_dir().unwrap();
    let before = external_namespace_snapshot(scope.path());
    assert!(crate::transaction::sync_readable_directory(&root, other.identity().unwrap()).is_err());
    assert_eq!(external_namespace_snapshot(scope.path()), before);
}

fn existing_external_fixture() -> (tempfile::TempDir, Project, std::path::PathBuf) {
    let scope = tempfile::tempdir().unwrap();
    let project_path = scope.path().join("project");
    fs::create_dir(&project_path).unwrap();
    fs::write(project_path.join("keep"), b"unchanged").unwrap();
    let project = Project::open(&project_path).unwrap();
    let store_path = scope.path().join("store");
    (scope, project, store_path)
}

fn external_namespace_snapshot(
    root: &Path,
) -> Vec<(std::path::PathBuf, &'static str, Vec<u8>)> {
    fn walk(
        root: &Path,
        current: &Path,
        rows: &mut Vec<(std::path::PathBuf, &'static str, Vec<u8>)>,
    ) {
        for entry in fs::read_dir(current).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            let relative = path.strip_prefix(root).unwrap().to_owned();
            let kind = entry.file_type().unwrap();
            if kind.is_symlink() {
                rows.push((
                    relative,
                    "link",
                    fs::read_link(&path)
                        .unwrap()
                        .to_string_lossy()
                        .as_bytes()
                        .to_vec(),
                ));
            } else if kind.is_dir() {
                rows.push((relative, "directory", Vec::new()));
                walk(root, &path, rows);
            } else {
                rows.push((relative, "file", fs::read(&path).unwrap()));
            }
        }
    }
    let mut rows = Vec::new();
    walk(root, root, &mut rows);
    rows.sort();
    rows
}

#[test]
fn existing_store_absence_has_no_creation() {
    let (scope, project, store_path) = existing_external_fixture();
    let before = external_namespace_snapshot(scope.path());
    assert!(
        ExternalStore::open_existing_disjoint(&store_path.join("nested"), &project)
            .unwrap()
            .is_none()
    );
    assert_eq!(external_namespace_snapshot(scope.path()), before);
    assert!(!store_path.exists());
}

#[test]
fn existing_disjoint_store_opens_and_invalid_scope_refuses() {
    let (scope, project, store_path) = existing_external_fixture();
    fs::create_dir(&store_path).unwrap();
    let before = external_namespace_snapshot(scope.path());
    let store = ExternalStore::open_existing_disjoint(&store_path, &project)
        .unwrap()
        .unwrap();
    store.prove_disjoint_from(&project).unwrap();
    assert!(store.bootstrap_durability().is_empty());
    assert!(ExternalStore::open_existing_disjoint(project.root_path(), &project).is_err());
    assert!(
        ExternalStore::open_existing_disjoint(
            &project.root_path().join("missing/suffix"),
            &project
        )
        .is_err()
    );
    assert!(ExternalStore::open_existing_disjoint(scope.path(), &project).is_err());
    assert_eq!(external_namespace_snapshot(scope.path()), before);

    let file = scope.path().join("file-store");
    fs::write(&file, b"not a directory").unwrap();
    let before = external_namespace_snapshot(scope.path());
    assert!(ExternalStore::open_existing_disjoint(&file, &project).is_err());
    assert_eq!(external_namespace_snapshot(scope.path()), before);
    let alias = scope.path().join("store-link");
    if link_directory(&store_path, &alias) {
        let before = external_namespace_snapshot(scope.path());
        assert!(ExternalStore::open_existing_disjoint(&alias, &project).is_err());
        assert_eq!(external_namespace_snapshot(scope.path()), before);
    }
}

#[test]
fn existing_project_lock_absence_has_no_creation() {
    let (scope, project, store_path) = existing_external_fixture();
    fs::create_dir(&store_path).unwrap();
    let store = ExternalStore::open_existing_disjoint(&store_path, &project)
        .unwrap()
        .unwrap();
    let key = project.identity_token().unwrap();
    let before = external_namespace_snapshot(scope.path());
    assert!(store.lock_existing_project(&key).unwrap().is_none());
    assert_eq!(external_namespace_snapshot(scope.path()), before);
    fs::create_dir(store_path.join("locks")).unwrap();
    let before = external_namespace_snapshot(scope.path());
    assert!(store.lock_existing_project(&key).unwrap().is_none());
    assert!(store.lock_existing_project("").is_err());
    assert_eq!(external_namespace_snapshot(scope.path()), before);
}

#[test]
fn existing_project_lock_reacquires_rechecks_and_store_token_is_stable() {
    let (scope, project, store_path) = existing_external_fixture();
    let store = ExternalStore::open_or_create_disjoint(&store_path, &project).unwrap();
    let key = project.identity_token().unwrap();
    drop(store.open_and_lock_project(&key).unwrap());
    let before = external_namespace_snapshot(scope.path());
    let identity = store.identity_token().unwrap();
    let reopened = ExternalStore::open_existing_disjoint(&store_path, &project)
        .unwrap()
        .unwrap();
    assert_eq!(reopened.identity_token().unwrap(), identity);
    assert_eq!(identity.len(), 71);
    assert!(identity.starts_with("sha256:"));
    assert!(identity[7..].bytes().all(|byte| byte.is_ascii_hexdigit()));
    let same_root_project = Project::open(&store_path).unwrap();
    assert_ne!(
        same_root_project.identity_token().unwrap(),
        identity,
        "store and project domains differ even on the same object"
    );
    let attempts = std::rc::Rc::new(std::cell::Cell::new(0));
    let observed = attempts.clone();
    crate::arm_lock_identity_check(Some(Box::new(move |actual| {
        assert!(actual);
        let count = observed.get();
        observed.set(count + 1);
        count > 0
    })));
    let lock = reopened.lock_existing_project(&key).unwrap().unwrap();
    crate::arm_lock_identity_check(None);
    assert_eq!(
        attempts.get(),
        2,
        "rejected first handle re-contends for the named barrier"
    );
    lock.require_still_named().unwrap();
    drop(lock);
    assert_eq!(external_namespace_snapshot(scope.path()), before);

    let lock_path = fs::read_dir(store_path.join("locks"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    fs::remove_file(&lock_path).unwrap();
    let target = scope.path().join("target");
    fs::write(&target, b"foreign lock payload").unwrap();
    fs::hard_link(&target, &lock_path).unwrap();
    let before = external_namespace_snapshot(scope.path());
    assert!(reopened.lock_existing_project(&key).is_err());
    assert_eq!(external_namespace_snapshot(scope.path()), before);
    fs::remove_file(&lock_path).unwrap();
    if link_file(&target, &lock_path) {
        let before = external_namespace_snapshot(scope.path());
        assert!(reopened.lock_existing_project(&key).is_err());
        assert_eq!(external_namespace_snapshot(scope.path()), before);
    }
}

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
