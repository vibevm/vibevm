//! Existing-only recovery capabilities and actual retained-state synchronization.
use super::*;
use std::path::PathBuf;

pub(super) fn snapshot(root: &Path) -> std::io::Result<Vec<(PathBuf, &'static str, Vec<u8>)>> {
    fn walk(
        root: &Path,
        dir: &Path,
        out: &mut Vec<(PathBuf, &'static str, Vec<u8>)>,
    ) -> std::io::Result<()> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            let relative = path
                .strip_prefix(root)
                .map_err(std::io::Error::other)?
                .to_path_buf();
            let kind = entry.file_type()?;
            if kind.is_symlink() {
                out.push((
                    relative,
                    "link",
                    fs::read_link(path)?.to_string_lossy().as_bytes().to_vec(),
                ));
            } else if kind.is_dir() {
                out.push((relative, "directory", Vec::new()));
                walk(root, &path, out)?;
            } else {
                out.push((relative, "file", fs::read(path)?));
            }
        }
        Ok(())
    }
    let mut out = Vec::new();
    walk(root, root, &mut out)?;
    out.sort();
    Ok(out)
}

fn fixture() -> anyhow::Result<(tempfile::TempDir, Project, PathBuf)> {
    let scope = tempfile::tempdir()?;
    let path = scope.path().join("project");
    fs::create_dir(&path)?;
    fs::write(path.join("keep"), b"unchanged")?;
    let project = Project::open(&path)?;
    let store = scope.path().join("store");
    Ok((scope, project, store))
}

#[test]
#[cfg(unix)]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-056#READABLE-DIRECTORY-SYNC"
)]
fn directory_flush_reopens_same_readable_capability_without_creation() {
    let (scope, project) = project();
    fs::write(scope.path().join("sentinel"), b"unchanged").unwrap();
    let root = project.root_dir().unwrap();
    let held = root.dir.try_clone().unwrap().into_std_file();
    println!("held directory sync: {:?}", held.sync_all());
    let readable = root.dir.open(".").unwrap().into_std();
    assert!(readable.metadata().unwrap().is_dir());
    assert_eq!(
        crate::transaction::file_identity(&readable, root.path()).unwrap(),
        root.identity().unwrap()
    );
    readable.sync_all().unwrap();
    assert_eq!(root.sync_directory(), DirectoryDurability::Synced);
    assert_eq!(
        fs::read(scope.path().join("sentinel")).unwrap(),
        b"unchanged"
    );
    assert_eq!(fs::read_dir(scope.path()).unwrap().count(), 1);
}

#[test]
#[cfg(unix)]
fn directory_flush_refuses_wrong_identity() {
    let (scope, project, store_path) = fixture().unwrap();
    fs::create_dir(&store_path).unwrap();
    let other = Project::open(&store_path).unwrap().root_dir().unwrap();
    let held = project.root_dir().unwrap();
    let before = snapshot(scope.path()).unwrap();
    assert!(crate::transaction::sync_readable_directory(&held, other.identity().unwrap()).is_err());
    assert_eq!(snapshot(scope.path()).unwrap(), before);
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-056#EXTERNAL-EXISTING-OBSERVATION"
)]
fn existing_store_absence_overlap_and_error_never_create_state() {
    let (scope, project, store_path) = fixture().unwrap();
    let before = snapshot(scope.path()).unwrap();
    assert!(
        ExternalStore::open_existing_disjoint(&store_path.join("nested"), &project)
            .unwrap()
            .is_none()
    );
    for path in [
        project.root_path().to_path_buf(),
        project.root_path().join("missing/store"),
        scope.path().to_path_buf(),
    ] {
        assert!(
            ExternalStore::open_existing_disjoint(&path, &project).is_err(),
            "{}",
            path.display()
        );
    }
    assert_eq!(snapshot(scope.path()).unwrap(), before);
    fs::write(&store_path, b"not a directory").unwrap();
    let before = snapshot(scope.path()).unwrap();
    assert!(ExternalStore::open_existing_disjoint(&store_path, &project).is_err());
    assert_eq!(snapshot(scope.path()).unwrap(), before);
    fs::remove_file(&store_path).unwrap();
    fs::create_dir(&store_path).unwrap();
    let before = snapshot(scope.path()).unwrap();
    let store = ExternalStore::open_existing_disjoint(&store_path, &project)
        .unwrap()
        .unwrap();
    assert!(store.bootstrap_durability().is_empty());
    store.prove_disjoint_from(&project).unwrap();
    assert_eq!(snapshot(scope.path()).unwrap(), before);
    #[cfg(unix)]
    {
        let alias = scope.path().join("store-link");
        std::os::unix::fs::symlink(&store_path, &alias).unwrap();
        let before = snapshot(scope.path()).unwrap();
        assert!(ExternalStore::open_existing_disjoint(&alias, &project).is_err());
        assert_eq!(snapshot(scope.path()).unwrap(), before);
    }
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-056#EXTERNAL-EXISTING-OBSERVATION"
)]
fn existing_lock_is_absent_or_same_named_rechecked_barrier() {
    let (scope, project, store_path) = fixture().unwrap();
    let store = ExternalStore::open_or_create_disjoint(&store_path, &project).unwrap();
    let key = project.identity_token().unwrap();
    let before = snapshot(scope.path()).unwrap();
    assert!(store.lock_existing_project(&key).unwrap().is_none());
    assert!(store.lock_existing_project("").is_err());
    assert_eq!(snapshot(scope.path()).unwrap(), before);
    fs::create_dir(store_path.join("locks")).unwrap();
    let before = snapshot(scope.path()).unwrap();
    assert!(store.lock_existing_project(&key).unwrap().is_none());
    assert_eq!(snapshot(scope.path()).unwrap(), before);
    drop(store.open_and_lock_project(&key).unwrap());
    let before = snapshot(scope.path()).unwrap();
    let token = store.identity_token().unwrap();
    let reopened = ExternalStore::open_existing_disjoint(&store_path, &project)
        .unwrap()
        .unwrap();
    assert_eq!(reopened.identity_token().unwrap(), token);
    assert_ne!(
        token,
        Project::open(&store_path)
            .unwrap()
            .identity_token()
            .unwrap()
    );
    let attempts = std::rc::Rc::new(std::cell::Cell::new(0));
    let count = attempts.clone();
    crate::arm_lock_identity_check(Some(Box::new(move |actual| {
        assert!(actual);
        let previous = count.get();
        count.set(previous + 1);
        previous != 0
    })));
    let acquired = reopened.lock_existing_project(&key);
    crate::arm_lock_identity_check(None);
    let lock = acquired.unwrap().unwrap();
    assert_eq!(attempts.get(), 2);
    lock.require_still_named().unwrap();
    drop(lock);
    assert_eq!(snapshot(scope.path()).unwrap(), before);
    let path = fs::read_dir(store_path.join("locks"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let foreign = scope.path().join("foreign-lock");
    fs::write(&foreign, b"foreign").unwrap();
    fs::remove_file(&path).unwrap();
    fs::hard_link(&foreign, &path).unwrap();
    let before = snapshot(scope.path()).unwrap();
    assert!(store.lock_existing_project(&key).is_err());
    assert_eq!(snapshot(scope.path()).unwrap(), before);
    #[cfg(unix)]
    {
        fs::remove_file(&path).unwrap();
        std::os::unix::fs::symlink(&foreign, &path).unwrap();
        let before = snapshot(scope.path()).unwrap();
        assert!(store.lock_existing_project(&key).is_err());
        assert_eq!(snapshot(scope.path()).unwrap(), before);
    }
}

#[test]
fn existing_lock_contends_with_creating_lock_in_same_store() {
    let (scope, project, store_path) = fixture().unwrap();
    let store = ExternalStore::open_or_create_disjoint(&store_path, &project).unwrap();
    let key = project.identity_token().unwrap();
    let held = store.open_and_lock_project(&key).unwrap();
    let before = snapshot(scope.path()).unwrap();
    let (entered_tx, entered_rx) = std::sync::mpsc::channel();
    let (acquired_tx, acquired_rx) = std::sync::mpsc::channel();
    let project_path = project.root_path().to_path_buf();
    let join = std::thread::spawn(move || {
        let project = Project::open(&project_path).unwrap();
        let store = ExternalStore::open_existing_disjoint(&store_path, &project)
            .unwrap()
            .unwrap();
        crate::arm_before_lock(Some(Box::new(move |_, _| {
            entered_tx.send(()).unwrap();
        })));
        let lock = store.lock_existing_project(&key).unwrap().unwrap();
        crate::arm_before_lock(None);
        lock.require_still_named().unwrap();
        acquired_tx.send(()).unwrap();
    });
    entered_rx
        .recv_timeout(std::time::Duration::from_secs(5))
        .unwrap();
    assert!(matches!(
        acquired_rx.recv_timeout(std::time::Duration::from_millis(100)),
        Err(std::sync::mpsc::RecvTimeoutError::Timeout)
    ));
    drop(held);
    acquired_rx
        .recv_timeout(std::time::Duration::from_secs(5))
        .unwrap();
    join.join().unwrap();
    assert_eq!(snapshot(scope.path()).unwrap(), before);
}

#[test]
#[cfg(unix)]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-056#NONBLOCKING-REGULAR-ADMISSION"
)]
fn replaced_lock_name_fifo_refuses_without_a_peer() {
    use std::os::unix::fs::OpenOptionsExt as _;
    let (scope, project, store_path) = fixture().unwrap();
    let store = ExternalStore::open_or_create_disjoint(&store_path, &project).unwrap();
    let key = project.identity_token().unwrap();
    drop(store.open_and_lock_project(&key).unwrap());
    let path = fs::read_dir(store_path.join("locks"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let (replaced_tx, replaced_rx) = std::sync::mpsc::channel();
    let (done_tx, done_rx) = std::sync::mpsc::channel();
    let join = std::thread::spawn(move || {
        crate::arm_before_lock(Some(Box::new(move |directory, name| {
            let named = directory.path().join(name);
            fs::rename(&named, directory.path().join("displaced.lock")).unwrap();
            assert!(
                std::process::Command::new("mkfifo")
                    .arg(&named)
                    .status()
                    .unwrap()
                    .success()
            );
            replaced_tx.send(()).unwrap();
        })));
        let result = store.lock_existing_project(&key);
        crate::arm_before_lock(None);
        done_tx.send(result.is_err()).unwrap();
    });
    replaced_rx.recv().unwrap();
    let settled = done_rx.recv_timeout(std::time::Duration::from_secs(2));
    let without_peer = settled.is_ok();
    let unblock = if without_peer {
        None
    } else {
        Some(
            std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .custom_flags(libc::O_NONBLOCK)
                .open(&path)
                .unwrap(),
        )
    };
    let refused = settled.unwrap_or_else(|_| done_rx.recv().unwrap());
    join.join().unwrap();
    drop(unblock);
    assert!(refused);
    use std::os::unix::fs::FileTypeExt as _;
    assert!(fs::symlink_metadata(&path).unwrap().file_type().is_fifo());
    assert!(store_path.join("locks/displaced.lock").exists());
    assert_eq!(
        fs::read(project.root_path().join("keep")).unwrap(),
        b"unchanged"
    );
    assert!(scope.path().exists());
    assert!(
        without_peer,
        "lock final-name recheck waited for a FIFO peer"
    );
}
