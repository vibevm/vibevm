use super::*;
use crate::commands::vvm::model::{InstallRecord, Kind, Origin, Profile};
use specmark::verifies;

fn rec(kind: Kind, id: &str, instance: u64) -> InstallRecord {
    InstallRecord {
        kind,
        id: id.into(),
        instance,
        commit: "c".into(),
        toolchain: "t".into(),
        profile: Profile::Debug,
        installed_at: "now".into(),
        origin: Origin::Managed,
        source_path: None,
        payload_sha256: None,
        distribution_manifest_sha256: None,
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-019#layout", r = 2)]
fn modern_bundle_paths_nest_under_kind_id_instance_and_legacy_vibe_still_resolves() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("opt");
    let store = VersionStore::new(&root);
    let id = VersionId::new(Kind::Tag, "1.2.3");
    let expect = root
        .join("vibevm")
        .join("versions")
        .join("tag")
        .join("1.2.3")
        .join("4");
    assert_eq!(store.instance_dir(&id, 4), expect);
    assert_eq!(
        store.binary_path(&id, 4),
        expect.join("bin").join(BINARY_NAME)
    );
    assert_eq!(
        store.index_binary_path(&id, 4),
        expect.join("bin").join(INDEX_BINARY_NAME)
    );
    assert_eq!(store.instance_source_dir(&id, 4), expect.join("source"));
    fs::create_dir_all(&expect).unwrap();
    fs::write(expect.join(BINARY_NAME), b"legacy").unwrap();
    assert_eq!(store.binary_path(&id, 4), expect.join(BINARY_NAME));
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-019#layout", r = 2)]
fn alloc_instance_is_monotonic_from_one() {
    let tmp = tempfile::tempdir().unwrap();
    let store = VersionStore::new(tmp.path());
    assert_eq!(store.alloc_instance().unwrap(), 1);
    assert_eq!(store.alloc_instance().unwrap(), 2);
    assert_eq!(store.alloc_instance().unwrap(), 3);
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-019#activation", r = 2)]
fn active_follows_the_current_pointer() {
    let tmp = tempfile::tempdir().unwrap();
    let store = VersionStore::new(tmp.path());
    let id = VersionId::new(Kind::Branch, "main");
    store.record_install(rec(Kind::Branch, "main", 1)).unwrap();
    let inst = store.instance_dir(&id, 1);
    fs::create_dir_all(&inst).unwrap();
    assert!(store.active().unwrap().is_none());
    store.write_current(&inst).unwrap();
    let active = store.active().unwrap().unwrap();
    assert_eq!(active.version_id(), id);
    assert_eq!(active.instance, 1);
}

#[test]
fn switching_tracks_an_exact_rollback_instance() {
    let tmp = tempfile::tempdir().unwrap();
    let store = VersionStore::new(tmp.path());
    let id = VersionId::new(Kind::Tag, "1.0.0");
    for instance in [1, 2] {
        store
            .record_install(rec(Kind::Tag, "1.0.0", instance))
            .unwrap();
        fs::create_dir_all(store.instance_dir(&id, instance)).unwrap();
    }
    let first = store.instance_dir(&id, 1);
    let second = store.instance_dir(&id, 2);
    store.write_current(&first).unwrap();
    assert!(store.previous().unwrap().is_none());
    store.write_current(&second).unwrap();
    assert_eq!(store.active().unwrap().unwrap().instance, 2);
    assert_eq!(store.previous().unwrap().unwrap().instance, 1);
    store.write_current(&first).unwrap();
    assert_eq!(store.active().unwrap().unwrap().instance, 1);
    assert_eq!(store.previous().unwrap().unwrap().instance, 2);
}

#[test]
fn an_existing_terminal_instance_number_is_immutable() {
    let tmp = tempfile::tempdir().unwrap();
    let store = VersionStore::new(tmp.path());
    let original = rec(Kind::Branch, "main", 7);
    store.record_install(original.clone()).unwrap();
    store.record_install(original).unwrap();
    let mut changed = rec(Kind::Branch, "main", 7);
    changed.commit = "different".into();
    assert!(
        store
            .record_install(changed)
            .unwrap_err()
            .to_string()
            .contains("branch:main#7")
    );
    assert_eq!(store.alloc_instance().unwrap(), 8);
}

#[test]
fn pending_activation_journal_converges_after_restart() {
    let tmp = tempfile::tempdir().unwrap();
    let store = VersionStore::new(tmp.path());
    let id = VersionId::new(Kind::Tag, "1.0.0");
    let current = store.instance_dir(&id, 2);
    let previous = store.instance_dir(&id, 1);
    store.record_install(rec(Kind::Tag, "1.0.0", 1)).unwrap();
    store.record_install(rec(Kind::Tag, "1.0.0", 2)).unwrap();
    fs::create_dir_all(&current).unwrap();
    fs::create_dir_all(&previous).unwrap();
    fs::create_dir_all(store.data_dir()).unwrap();
    fs::write(
        store.activation_journal_path(),
        toml::to_string(&ActivationJournal {
            current: Some(current.display().to_string()),
            previous: Some(previous.display().to_string()),
        })
        .unwrap(),
    )
    .unwrap();
    assert_eq!(store.read_current().unwrap().unwrap(), current);
    assert_eq!(store.read_previous().unwrap().unwrap(), previous);
    assert!(
        store.activation_journal_path().exists(),
        "readers never mutate recovery state"
    );
    crate::commands::vvm::install::InstallLock::acquire(&store).unwrap();
    assert!(!store.activation_journal_path().exists());
}

#[test]
fn arbitrary_pointer_and_journal_paths_are_rejected_before_replay() {
    let temp = tempfile::tempdir().unwrap();
    let store = VersionStore::new(temp.path().join("opt"));
    let id = VersionId::new(Kind::Tag, "1.0.0");
    let valid = store.instance_dir(&id, 1);
    store.record_install(rec(Kind::Tag, "1.0.0", 1)).unwrap();
    fs::create_dir_all(&valid).unwrap();
    store.write_current(&valid).unwrap();

    let outside = temp.path().join("outside");
    fs::create_dir_all(&outside).unwrap();
    assert!(store.write_current(&outside).is_err());
    let before = fs::read_to_string(store.current_path()).unwrap();
    fs::write(
        store.activation_journal_path(),
        toml::to_string(&ActivationJournal {
            current: Some(valid.display().to_string()),
            previous: Some(outside.display().to_string()),
        })
        .unwrap(),
    )
    .unwrap();
    assert!(store.read_current().is_err());
    assert!(crate::commands::vvm::install::InstallLock::acquire(&store).is_err());
    assert_eq!(fs::read_to_string(store.current_path()).unwrap(), before);
    assert!(store.activation_journal_path().exists());
}

#[test]
fn stale_exact_pointer_remains_identifiable_for_doctor_and_forced_repair() {
    let temp = tempfile::tempdir().unwrap();
    let store = VersionStore::new(temp.path());
    let id = VersionId::new(Kind::Tag, "1.0.0");
    store.record_install(rec(Kind::Tag, "1.0.0", 1)).unwrap();
    let missing = store.instance_dir(&id, 1);
    store.write_current(&missing).unwrap();
    assert!(!missing.exists());
    assert_eq!(store.active().unwrap().unwrap().instance, 1);
}

#[test]
fn poisoned_previous_destination_cannot_partially_switch_current() {
    let temp = tempfile::tempdir().unwrap();
    let store = VersionStore::new(temp.path());
    let id = VersionId::new(Kind::Tag, "1.0.0");
    for instance in [1, 2] {
        store
            .record_install(rec(Kind::Tag, "1.0.0", instance))
            .unwrap();
        fs::create_dir_all(store.instance_dir(&id, instance)).unwrap();
    }
    let first = store.instance_dir(&id, 1);
    let second = store.instance_dir(&id, 2);
    store.write_current(&first).unwrap();
    let before = fs::read_to_string(store.current_path()).unwrap();
    fs::create_dir(store.previous_path()).unwrap();
    let error = store
        .write_activation(Some(&second), Some(&first))
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("destination leaf is not a regular file"),
        "{error}"
    );
    assert_eq!(fs::read_to_string(store.current_path()).unwrap(), before);
    assert!(!store.activation_journal_path().exists());
}

#[test]
fn relative_pointer_is_never_resolved_against_process_cwd() {
    let temp = tempfile::tempdir().unwrap();
    let store = VersionStore::new(temp.path());
    let id = VersionId::new(Kind::Tag, "1.0.0");
    store.record_install(rec(Kind::Tag, "1.0.0", 1)).unwrap();
    fs::create_dir_all(store.instance_dir(&id, 1)).unwrap();
    fs::write(store.current_path(), "vibevm/versions/tag/1.0.0/1\n").unwrap();
    let error = store.read_current().unwrap_err().to_string();
    assert!(error.contains("absolute path"), "{error}");
}

#[test]
fn corrupt_state_cannot_smuggle_a_path_traversal_identity() {
    let tmp = tempfile::tempdir().unwrap();
    let store = VersionStore::new(tmp.path());
    fs::create_dir_all(store.data_dir()).unwrap();
    fs::write(
        store.state_path(),
        r#"next_instance = 2
[[install]]
kind = "branch"
id = "../../escape"
instance = 1
commit = "c"
toolchain = "t"
profile = "debug"
installed_at = "now"
origin = "managed"
"#,
    )
    .unwrap();
    assert!(
        store
            .load_state()
            .unwrap_err()
            .to_string()
            .contains("unsafe version id")
    );
    assert!(!tmp.path().join("escape").exists());
}

#[test]
fn mutation_guard_rejects_parent_components_before_normalization() {
    let temp = tempfile::tempdir().unwrap();
    let store = VersionStore::new(temp.path().join("opt"));
    let target = store.data_dir().join("versions").join("..").join("outside");
    let error = store.guard_mutation_path(&target).unwrap_err().to_string();
    assert!(
        error.contains("non-normal relative path component"),
        "{error}"
    );
}

#[test]
fn atomic_state_writer_never_deletes_an_unowned_colliding_temp() {
    let temp = tempfile::tempdir().unwrap();
    let destination = temp.path().join("state.toml");
    let collision = temp.path().join("preexisting.tmp");
    fs::write(&collision, b"owner bytes").unwrap();
    assert!(super::guard::atomic_replace_at(&destination, &collision, b"new").is_err());
    assert_eq!(fs::read(collision).unwrap(), b"owner bytes");
    assert!(!destination.exists());
}

#[cfg(unix)]
#[test]
fn mutation_guard_rejects_existing_symlink_ancestor_outside_store() {
    use std::os::unix::fs::symlink;

    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("opt");
    let outside = temp.path().join("outside");
    fs::create_dir_all(root.join("vibevm/versions")).unwrap();
    fs::create_dir_all(&outside).unwrap();
    symlink(&outside, root.join("vibevm/versions/tag")).unwrap();
    let store = VersionStore::new(&root);
    let target = root.join("vibevm/versions/tag/main/1");

    let error = store.guard_mutation_path(&target).unwrap_err().to_string();
    assert!(error.contains("symlink or reparse point"), "{error}");
    assert!(fs::read_dir(&outside).unwrap().next().is_none());
}

#[cfg(windows)]
#[test]
fn mutation_guard_rejects_windows_junction_ancestor() {
    use std::os::windows::process::CommandExt;

    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("opt");
    let outside = temp.path().join("outside");
    let link = root.join("vibevm").join("versions").join("tag");
    fs::create_dir_all(link.parent().unwrap()).unwrap();
    fs::create_dir_all(&outside).unwrap();
    let command = format!("mklink /J \"{}\" \"{}\"", link.display(), outside.display());
    let status = std::process::Command::new("cmd")
        .args(["/d", "/c"])
        .raw_arg(command)
        .stdout(std::process::Stdio::null())
        .status()
        .unwrap();
    assert!(status.success(), "creating test junction failed");
    let store = VersionStore::new(&root);
    let error = store
        .guard_mutation_path(&link.join("main/1"))
        .unwrap_err()
        .to_string();
    assert!(error.contains("symlink or reparse point"), "{error}");
    assert!(fs::read_dir(&outside).unwrap().next().is_none());
}
