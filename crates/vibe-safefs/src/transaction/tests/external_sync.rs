//! Retained image confirmation reaches actual sync and never rewrites bytes.
use super::external_read::snapshot;
use super::*;

struct Hook;
impl Drop for Hook {
    fn drop(&mut self) {
        ExternalStore::arm_before_retained_file_sync(None);
    }
}
fn arm(callback: impl FnMut() -> bool + 'static) -> Hook {
    ExternalStore::arm_before_retained_file_sync(Some(Box::new(callback)));
    Hook
}
fn fixture() -> anyhow::Result<(tempfile::TempDir, ExternalStore, crate::StableFileSnapshot)> {
    let scope = tempfile::tempdir()?;
    let store = ExternalStore::open_or_create(&scope.path().join("store"))?;
    store.write_durable("images/before.bin", b"approved")?;
    let expected = store
        .read_stable_bounded("images/before.bin", 8)?
        .ok_or_else(|| std::io::Error::other("fixture image missing"))?;
    Ok((scope, store, expected))
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-056#EXTERNAL-RETAINED-SYNC")]
fn retained_image_requires_real_sync_and_preserves_snapshot() {
    let (scope, store, expected) = fixture().unwrap();
    let before = snapshot(scope.path()).unwrap();
    let calls = std::rc::Rc::new(std::cell::Cell::new(0));
    let observed = calls.clone();
    let hook = arm(move || {
        observed.set(observed.get() + 1);
        true
    });
    let error = store
        .sync_stable_file_bounded("images/before.bin", 8, &expected)
        .unwrap_err();
    assert!(error.to_string().contains("refused before file sync"));
    assert_eq!(calls.get(), 1);
    assert_eq!(snapshot(scope.path()).unwrap(), before);
    assert_eq!(
        store
            .read_stable_bounded("images/before.bin", 8)
            .unwrap()
            .unwrap(),
        expected
    );
    drop(hook);
    let observed = calls.clone();
    let _hook = arm(move || {
        observed.set(observed.get() + 1);
        false
    });
    let actual = store
        .sync_stable_file_bounded("images/before.bin", 8, &expected)
        .unwrap();
    assert_eq!(calls.get(), 2);
    let project = Project::open(store.path()).unwrap();
    assert_eq!(
        actual,
        project.dir(&["images"], false).unwrap().sync_directory()
    );
    assert_eq!(snapshot(scope.path()).unwrap(), before);
    assert_eq!(
        store
            .read_stable_bounded("images/before.bin", 8)
            .unwrap()
            .unwrap(),
        expected
    );
}

#[test]
fn stale_identity_bytes_mode_and_bounds_refuse_before_sync() {
    let (scope, store, expected) = fixture().unwrap();
    store.write_durable("other.bin", b"approved").unwrap();
    let other = store.read_stable_bounded("other.bin", 8).unwrap().unwrap();
    let called = std::rc::Rc::new(std::cell::Cell::new(0));
    let count = called.clone();
    let _hook = arm(move || {
        count.set(count.get() + 1);
        false
    });
    let before = snapshot(scope.path()).unwrap();
    assert!(
        store
            .sync_stable_file_bounded("images/before.bin", 8, &other)
            .is_err()
    );
    assert!(
        store
            .sync_stable_file_bounded("images/before.bin", 7, &expected)
            .is_err()
    );
    assert!(
        store
            .sync_stable_file_bounded("images/before.bin", usize::MAX, &expected)
            .is_err()
    );
    assert!(
        store
            .sync_stable_file_bounded("missing/before.bin", 8, &expected)
            .is_err()
    );
    assert!(
        store
            .sync_stable_file_bounded("../before.bin", 8, &expected)
            .is_err()
    );
    assert_eq!(called.get(), 0);
    assert_eq!(snapshot(scope.path()).unwrap(), before);
    fs::write(store.path().join("images/before.bin"), b"modified").unwrap();
    let before = snapshot(scope.path()).unwrap();
    assert!(
        store
            .sync_stable_file_bounded("images/before.bin", 8, &expected)
            .is_err()
    );
    assert_eq!(called.get(), 0);
    assert_eq!(snapshot(scope.path()).unwrap(), before);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::write(store.path().join("images/before.bin"), b"approved").unwrap();
        fs::set_permissions(
            store.path().join("images/before.bin"),
            fs::Permissions::from_mode(expected.unix_mode.unwrap() ^ 0o100),
        )
        .unwrap();
        assert!(
            store
                .sync_stable_file_bounded("images/before.bin", 8, &expected)
                .is_err()
        );
        assert_eq!(called.get(), 0);
    }
}

#[test]
fn mutation_image_does_not_admit_hardlinks() {
    let (scope, store, expected) = fixture().unwrap();
    let image = store.path().join("images/before.bin");
    fs::hard_link(&image, scope.path().join("alias")).unwrap();
    let before = snapshot(scope.path()).unwrap();
    assert!(
        store
            .sync_stable_file_bounded("images/before.bin", 8, &expected)
            .is_err()
    );
    assert_eq!(snapshot(scope.path()).unwrap(), before);
}

#[test]
#[cfg(unix)]
fn retained_image_links_and_linked_parents_refuse_without_sync() {
    let (scope, store, expected) = fixture().unwrap();
    let images = store.path().join("images");
    let image = images.join("before.bin");
    let original = images.join("original.bin");
    fs::rename(&image, &original).unwrap();
    std::os::unix::fs::symlink(&original, &image).unwrap();
    let calls = std::rc::Rc::new(std::cell::Cell::new(0));
    let count = calls.clone();
    let _hook = arm(move || {
        count.set(count.get() + 1);
        false
    });
    let before = snapshot(scope.path()).unwrap();
    assert!(
        store
            .sync_stable_file_bounded("images/before.bin", 8, &expected)
            .is_err()
    );
    assert_eq!(snapshot(scope.path()).unwrap(), before);
    fs::remove_file(&image).unwrap();
    fs::rename(&original, &image).unwrap();
    std::os::unix::fs::symlink(&images, store.path().join("alias")).unwrap();
    let before = snapshot(scope.path()).unwrap();
    assert!(
        store
            .sync_stable_file_bounded("alias/before.bin", 8, &expected)
            .is_err()
    );
    assert_eq!(calls.get(), 0);
    assert_eq!(snapshot(scope.path()).unwrap(), before);
}

#[test]
#[cfg(unix)]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-056#NONBLOCKING-REGULAR-ADMISSION"
)]
fn owned_snapshot_and_retained_sync_refuse_fifo_without_a_writer() {
    use std::os::unix::fs::OpenOptionsExt as _;
    let mut observations = Vec::new();
    for retained_sync in [false, true] {
        let (_scope, store, expected) = fixture().unwrap();
        let image = store.path().join("images/before.bin");
        let original = store.path().join("images/original.bin");
        fs::rename(&image, &original).unwrap();
        assert!(
            std::process::Command::new("mkfifo")
                .arg(&image)
                .status()
                .unwrap()
                .success()
        );
        let root = store.path().to_path_buf();
        let (entered_tx, entered_rx) = std::sync::mpsc::channel();
        let (done_tx, done_rx) = std::sync::mpsc::channel();
        let join = std::thread::spawn(move || {
            let project = Project::open(&root).unwrap();
            entered_tx.send(()).unwrap();
            let refused = if retained_sync {
                store
                    .sync_stable_file_bounded("images/before.bin", 8, &expected)
                    .is_err()
            } else {
                project
                    .read_file_snapshot_bounded("images/before.bin", 8)
                    .is_err()
            };
            done_tx.send(refused).unwrap();
        });
        entered_rx.recv().unwrap();
        let settled = done_rx.recv_timeout(std::time::Duration::from_secs(2));
        let without_writer = settled.is_ok();
        // The RED path must not strand a blocked reader: a nonblocking RDWR
        // handle supplies the peer and remains held until the worker joins.
        let unblock = if without_writer {
            None
        } else {
            Some(
                std::fs::OpenOptions::new()
                    .read(true)
                    .write(true)
                    .custom_flags(libc::O_NONBLOCK)
                    .open(&image)
                    .unwrap(),
            )
        };
        let refused = settled.unwrap_or_else(|_| done_rx.recv().unwrap());
        join.join().unwrap();
        drop(unblock);
        assert!(refused);
        assert_eq!(fs::read(&original).unwrap(), b"approved");
        use std::os::unix::fs::FileTypeExt as _;
        assert!(fs::symlink_metadata(&image).unwrap().file_type().is_fifo());
        observations.push((retained_sync, without_writer));
    }
    println!("FIFO refusal without peer (retained_sync, completed): {observations:?}");
    assert!(observations.iter().all(|(_, completed)| *completed));
}

#[test]
#[cfg(unix)]
fn replacement_or_content_change_across_sync_refuses_without_rewriting() {
    for replace in [false, true] {
        let (scope, store, expected) = fixture().unwrap();
        let path = store.path().join("images/before.bin");
        let moved = store.path().join("images/displaced.bin");
        let hook_path = path.clone();
        let _hook = arm(move || {
            if replace {
                fs::rename(&hook_path, &moved).unwrap();
            }
            fs::write(&hook_path, if replace { b"approved" } else { b"modified" }).unwrap();
            false
        });
        assert!(
            store
                .sync_stable_file_bounded("images/before.bin", 8, &expected)
                .is_err()
        );
        assert_eq!(
            fs::read(&path).unwrap(),
            if replace { b"approved" } else { b"modified" }
        );
        assert_eq!(fs::read_dir(scope.path()).unwrap().count(), 1);
    }
}
