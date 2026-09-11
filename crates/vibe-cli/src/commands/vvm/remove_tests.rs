use super::*;
use crate::cli::{ForcedKind, VvmGcArgs, VvmRemoveArgs};
use crate::commands::vvm::model::{InstallRecord, Kind, Origin, Profile};
use crate::commands::vvm::store::{BINARY_NAME, INDEX_BINARY_NAME};
use specmark::verifies;
use vibe_publish::release_manifest::DISTRIBUTION_SOURCE_ARCHIVE_FILENAME;

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

fn quiet() -> output::Context {
    output::Context::from_flags(true, false, None, true, crate::cli::AgentModeArg::Auto)
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-019#remove", r = 2)]
fn removal_scope_defaults_to_both() {
    assert_eq!(removal_scope(false, false), RemoveScope::Both);
    assert_eq!(removal_scope(true, false), RemoveScope::Bin);
    assert_eq!(removal_scope(false, true), RemoveScope::Src);
}

#[test]
fn running_inactive_instance_is_remove_protected_and_gc_retained() {
    let active = rec(Kind::Tag, "1.0.0", 1);
    let running = rec(Kind::Tag, "1.0.0", 2);
    let rollback = rec(Kind::Tag, "1.0.0", 3);
    let target = RemoveTarget::Instance(running.selector());
    let protected = protection_reason(&target, Some(&active), Some(&running)).unwrap();
    assert_eq!(protected.0, "running");
    assert!(protection_blocks("running", false));
    assert!(protection_blocks("running", true));
    assert!(protection_blocks("active", false));
    assert!(!protection_blocks("active", true));
    let same = protection_reason(&target, Some(&running), Some(&running)).unwrap();
    assert_eq!(same.0, "running", "running protection wins over active");
    assert!(gc_protected(
        &running,
        &active,
        Some(&rollback),
        Some(&running)
    ));
}

#[test]
fn failed_managed_instance_removal_retains_the_shared_mirror() {
    let retained = model::State {
        installs: vec![rec(Kind::Branch, "main", 1)],
        ..model::State::default()
    };
    assert!(!remove_mirror_after(true, RemoveScope::Both, &retained));
    assert!(remove_mirror_after(true, RemoveScope::Src, &retained));
    assert_eq!(
        scoped_removal_notes(
            RemoveScope::Src,
            RemovalEffects::default(),
            false,
            true,
            false,
            false,
        ),
        vec![
            "shared managed mirror retained because another installed managed record still uses it."
        ]
    );
    assert_eq!(
        scoped_removal_notes(
            RemoveScope::Src,
            RemovalEffects::default(),
            true,
            false,
            false,
            false,
        ),
        vec!["external checkout retained: VVM never deletes contributor-owned source trees."]
    );
    assert_eq!(
        scoped_removal_notes(
            RemoveScope::Src,
            RemovalEffects::default(),
            false,
            true,
            false,
            true,
        ),
        vec!["no managed source was present to remove."]
    );

    let mixed_absent_binary = scoped_removal_notes(
        RemoveScope::Src,
        RemovalEffects {
            any: true,
            managed_source: true,
            ..RemovalEffects::default()
        },
        false,
        true,
        true,
        true,
    );
    assert_eq!(
        mixed_absent_binary,
        vec![
            "no instance-owned binary source was present to remove.",
            "warning: shared managed source was removed; retained binaries no longer have managed source provenance.",
        ]
    );
    let mixed_present = scoped_removal_notes(
        RemoveScope::Src,
        RemovalEffects {
            any: true,
            binary_source: true,
            managed_source: true,
            ..RemovalEffects::default()
        },
        false,
        true,
        true,
        true,
    );
    assert_eq!(mixed_present.len(), 2);
    assert!(mixed_present[0].contains("partial binary bundle"));
    assert!(mixed_present[1].contains("shared managed source was removed"));
}

#[test]
fn managed_source_scope_removes_shared_mirror_only_when_all_managed_are_selected() {
    let temp = tempfile::tempdir().unwrap();
    let store = VersionStore::new(temp.path());
    for (id, instance) in [("main", 1), ("next", 2)] {
        store
            .record_install(rec(Kind::Branch, id, instance))
            .unwrap();
    }
    fs::create_dir_all(store.mirror_dir()).unwrap();
    fs::write(store.mirror_dir().join("sentinel"), b"mirror").unwrap();
    let env = VvmEnv {
        root: Some(temp.path().to_path_buf()),
        ..VvmEnv::default()
    };
    run_remove_cmd(
        &quiet(),
        &env,
        VvmRemoveArgs {
            selector: Some("branch:main".into()),
            kind: no_forced_kind(),
            all: false,
            bin: false,
            src: true,
            force: false,
            yes: true,
        },
    )
    .unwrap();
    assert!(store.mirror_dir().join("sentinel").is_file());

    run_remove_cmd(
        &quiet(),
        &env,
        VvmRemoveArgs {
            selector: None,
            kind: no_forced_kind(),
            all: true,
            bin: false,
            src: true,
            force: false,
            yes: true,
        },
    )
    .unwrap();
    assert!(!store.mirror_dir().exists());
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-019#remove", r = 2)]
fn remove_id_drops_all_instances_and_forgets() {
    let tmp = tempfile::tempdir().unwrap();
    let store = VersionStore::new(tmp.path());
    let id = VersionId::new(Kind::Tag, "1.0.0");
    for n in [1u64, 2] {
        store.record_install(rec(Kind::Tag, "1.0.0", n)).unwrap();
        fs::create_dir_all(store.instance_dir(&id, n)).unwrap();
    }
    let mut state = store.load_state().unwrap();
    remove_target(
        &quiet(),
        &store,
        &mut state,
        &RemoveTarget::Version(id.clone()),
        RemoveScope::Both,
    )
    .unwrap();
    store.save_state(&state).unwrap();
    assert!(!store.instance_dir(&id, 1).exists());
    assert!(!store.instance_dir(&id, 2).exists());
    assert!(store.instances_of(&id).unwrap().is_empty());
}

fn no_forced_kind() -> ForcedKind {
    ForcedKind {
        tag: false,
        branch: false,
        commit: false,
    }
}

fn installed_store(instances: &[u64]) -> (tempfile::TempDir, VersionStore, VersionId) {
    let tmp = tempfile::tempdir().unwrap();
    let store = VersionStore::new(tmp.path());
    let id = VersionId::new(Kind::Tag, "1.0.0");
    for &instance in instances {
        store
            .record_install(rec(Kind::Tag, "1.0.0", instance))
            .unwrap();
        let dir = store.instance_dir(&id, instance);
        let bin = dir.join("bin");
        fs::create_dir_all(&bin).unwrap();
        let vibe = bin.join(BINARY_NAME);
        let index = bin.join(INDEX_BINARY_NAME);
        fs::write(&vibe, format!("payload-{instance}")).unwrap();
        fs::write(&index, format!("index-{instance}")).unwrap();
        #[cfg(unix)]
        for path in [&vibe, &index] {
            use std::os::unix::fs::PermissionsExt;
            let mut permissions = fs::metadata(path).unwrap().permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(path, permissions).unwrap();
        }
        let manifest = crate::commands::vvm::placer::manifest_for(&[
            (vibe, format!("bin/{BINARY_NAME}")),
            (index, format!("bin/{INDEX_BINARY_NAME}")),
        ])
        .unwrap();
        fs::write(
            dir.join(".vvm-manifest.toml"),
            toml::to_string(&manifest).unwrap(),
        )
        .unwrap();
        store.write_current(&dir).unwrap();
    }
    (tmp, store, id)
}

#[test]
fn forced_active_exact_removal_repoints_current_to_rollback_first() {
    let (tmp, store, id) = installed_store(&[1, 2]);
    let env = VvmEnv {
        root: Some(tmp.path().to_path_buf()),
        ..VvmEnv::default()
    };
    run_remove_cmd(
        &quiet(),
        &env,
        VvmRemoveArgs {
            selector: Some("tag:1.0.0#2".into()),
            kind: no_forced_kind(),
            all: false,
            bin: false,
            src: false,
            force: true,
            yes: true,
        },
    )
    .unwrap();

    assert_eq!(store.active().unwrap().unwrap().instance, 1);
    assert!(!store.instance_dir(&id, 2).exists());
    assert!(store.instance_dir(&id, 1).exists());
    assert!(store.previous().unwrap().is_none());
}

#[test]
fn removing_saved_rollback_repoints_previous_without_moving_current() {
    let (tmp, store, id) = installed_store(&[1, 2, 3]);
    let env = VvmEnv {
        root: Some(tmp.path().to_path_buf()),
        ..VvmEnv::default()
    };
    run_remove_cmd(
        &quiet(),
        &env,
        VvmRemoveArgs {
            selector: Some("tag:1.0.0#2".into()),
            kind: no_forced_kind(),
            all: false,
            bin: false,
            src: false,
            force: false,
            yes: true,
        },
    )
    .unwrap();

    assert_eq!(store.active().unwrap().unwrap().instance, 3);
    assert_eq!(store.previous().unwrap().unwrap().instance, 1);
    assert!(!store.instance_dir(&id, 2).exists());
}

#[test]
fn gc_keeps_active_and_immediate_rollback_instances() {
    let (tmp, store, id) = installed_store(&[1, 2, 3]);
    let env = VvmEnv {
        root: Some(tmp.path().to_path_buf()),
        ..VvmEnv::default()
    };
    run_gc_cmd(
        &quiet(),
        &env,
        VvmGcArgs {
            build: false,
            prune_others: true,
            yes: true,
        },
    )
    .unwrap();

    assert!(!store.instance_dir(&id, 1).exists());
    assert!(store.instance_dir(&id, 2).exists());
    assert!(store.instance_dir(&id, 3).exists());
    assert_eq!(store.active().unwrap().unwrap().instance, 3);
    assert_eq!(store.previous().unwrap().unwrap().instance, 2);
}

#[test]
fn gc_preflights_linked_build_descendant_before_any_instance_or_pointer_change() {
    let (temp, store, id) = installed_store(&[1, 2, 3]);
    let before = fs::read_to_string(store.state_path()).unwrap();
    let outside = temp.path().join("outside");
    fs::create_dir_all(&outside).unwrap();
    fs::write(outside.join("sentinel"), b"keep").unwrap();
    fs::create_dir_all(store.build_dir().join("debug")).unwrap();
    make_dir_redirect(&outside, &store.build_dir().join("debug/deps"));
    let env = VvmEnv {
        root: Some(temp.path().to_path_buf()),
        ..VvmEnv::default()
    };
    assert!(
        run_gc_cmd(
            &quiet(),
            &env,
            VvmGcArgs {
                build: false,
                prune_others: true,
                yes: true,
            },
        )
        .is_err()
    );
    assert_eq!(fs::read_to_string(store.state_path()).unwrap(), before);
    assert!(store.instance_dir(&id, 1).exists());
    assert_eq!(store.active().unwrap().unwrap().instance, 3);
    assert_eq!(store.previous().unwrap().unwrap().instance, 2);
    assert_eq!(fs::read(outside.join("sentinel")).unwrap(), b"keep");
}

#[cfg(unix)]
fn make_dir_redirect(target: &std::path::Path, link: &std::path::Path) {
    std::os::unix::fs::symlink(target, link).unwrap();
}

#[cfg(windows)]
fn make_dir_redirect(target: &std::path::Path, link: &std::path::Path) {
    use std::os::windows::process::CommandExt;
    let command = format!("mklink /J \"{}\" \"{}\"", link.display(), target.display());
    let status = std::process::Command::new("cmd")
        .args(["/d", "/c"])
        .raw_arg(command)
        .stdout(std::process::Stdio::null())
        .status()
        .unwrap();
    assert!(status.success(), "creating test junction failed");
}

#[cfg(unix)]
#[test]
fn gc_preflights_linked_build_descendant_before_any_instance_or_state_change() {
    use std::os::unix::fs::symlink;

    let (temp, store, id) = installed_store(&[1, 2, 3]);
    let outside = temp.path().join("outside");
    fs::create_dir_all(&outside).unwrap();
    fs::write(outside.join("sentinel"), b"keep").unwrap();
    fs::create_dir_all(store.build_dir().join("debug")).unwrap();
    symlink(&outside, store.build_dir().join("debug/deps")).unwrap();
    let before = fs::read(store.state_path()).unwrap();
    let env = VvmEnv {
        root: Some(temp.path().to_path_buf()),
        ..VvmEnv::default()
    };
    assert!(
        run_gc_cmd(
            &quiet(),
            &env,
            VvmGcArgs {
                build: false,
                prune_others: true,
                yes: true,
            },
        )
        .is_err()
    );
    assert!(store.instance_dir(&id, 1).exists());
    assert_eq!(fs::read(store.state_path()).unwrap(), before);
    assert_eq!(store.active().unwrap().unwrap().instance, 3);
    assert_eq!(store.previous().unwrap().unwrap().instance, 2);
    assert_eq!(fs::read(outside.join("sentinel")).unwrap(), b"keep");
}

#[test]
fn binary_bin_and_source_scopes_preserve_the_other_half_and_record() {
    let tmp = tempfile::tempdir().unwrap();
    let store = VersionStore::new(tmp.path());
    let id = VersionId::new(Kind::Tag, "1.0.0");
    let mut record = rec(Kind::Tag, "1.0.0", 1);
    record.origin = Origin::Binary;
    store.record_install(record).unwrap();
    let home = store.instance_dir(&id, 1);
    fs::create_dir_all(home.join("bin")).unwrap();
    fs::create_dir_all(home.join("source")).unwrap();
    fs::write(home.join("bin").join(BINARY_NAME), b"vibe").unwrap();
    fs::write(home.join("bin").join(INDEX_BINARY_NAME), b"index").unwrap();
    fs::write(home.join("source/Cargo.toml"), b"source").unwrap();
    fs::write(
        home.join(DISTRIBUTION_SOURCE_ARCHIVE_FILENAME),
        b"source zip",
    )
    .unwrap();
    fs::write(home.join("DISTRIBUTION.json"), b"manifest").unwrap();

    let mut state = store.load_state().unwrap();
    remove_target(
        &quiet(),
        &store,
        &mut state,
        &RemoveTarget::Instance(InstanceId::new(id.clone(), 1)),
        RemoveScope::Bin,
    )
    .unwrap();
    assert!(!home.join("bin").exists());
    assert!(home.join("source/Cargo.toml").is_file());
    assert!(home.join("DISTRIBUTION.json").is_file());
    assert_eq!(store.instances_of(&id).unwrap().len(), 1);

    fs::create_dir_all(home.join("bin")).unwrap();
    fs::write(home.join("bin").join(BINARY_NAME), b"vibe").unwrap();
    remove_target(
        &quiet(),
        &store,
        &mut state,
        &RemoveTarget::Instance(InstanceId::new(id.clone(), 1)),
        RemoveScope::Src,
    )
    .unwrap();
    assert!(home.join("bin").is_dir());
    assert!(!home.join("source").exists());
    assert!(!home.join(DISTRIBUTION_SOURCE_ARCHIVE_FILENAME).exists());
    assert!(home.join("DISTRIBUTION.json").is_file());
    assert_eq!(store.instances_of(&id).unwrap().len(), 1);
}

#[cfg(unix)]
#[test]
fn linked_instance_and_bin_paths_never_delete_outside_store() {
    use std::os::unix::fs::symlink;

    let temp = tempfile::tempdir().unwrap();
    let store = VersionStore::new(temp.path().join("opt"));
    let id = VersionId::new(Kind::Tag, "1.0.0");
    let mut record = rec(Kind::Tag, "1.0.0", 1);
    record.origin = Origin::Binary;
    store.record_install(record).unwrap();
    let outside = temp.path().join("outside");
    fs::create_dir_all(&outside).unwrap();
    fs::write(outside.join("sentinel"), b"keep").unwrap();
    let home = store.instance_dir(&id, 1);
    fs::create_dir_all(&home).unwrap();
    symlink(&outside, home.join("bin")).unwrap();
    let mut state = store.load_state().unwrap();
    let error = remove_target(
        &quiet(),
        &store,
        &mut state,
        &RemoveTarget::Instance(InstanceId::new(id, 1)),
        RemoveScope::Bin,
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("symlink or reparse point"), "{error}");
    assert_eq!(fs::read(outside.join("sentinel")).unwrap(), b"keep");
}
