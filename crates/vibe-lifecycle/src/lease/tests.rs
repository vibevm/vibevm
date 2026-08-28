//! The lease's own REDs: one holder per workspace, release by drop and by
//! process death, Arc clones as proofs rather than reacquisitions, and the
//! forward lock order the lease exists to head.

use std::env;
use std::fs;
use std::process::Command;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use super::*;

const HELPER_ROOT_ENV: &str = "VIBEVM_A5_LEASE_HELPER_ROOT";

#[test]
fn a_second_same_process_acquisition_is_a_typed_busy() {
    let dir = tempfile::tempdir().unwrap();
    let lease = LifecycleLease::acquire(dir.path()).expect("the first holder owns it");
    match LifecycleLease::acquire(dir.path()) {
        Err(LifecycleLeaseError::Busy { root }) => {
            assert_eq!(root, dir.path(), "the refusal names the contended root");
        }
        other => panic!("a nested acquisition is a typed Busy, not {other:?}"),
    }
    let rendered = LifecycleLease::acquire(dir.path())
        .expect_err("still held")
        .to_string();
    assert!(rendered.contains("PROP-054"), "{rendered}");
    assert!(
        rendered.contains(LOCK_NAME),
        "the citation names the lock file: {rendered}"
    );
    drop(lease);
    drop(LifecycleLease::acquire(dir.path()).expect("release follows the last owner"));
}

#[test]
fn arc_clones_prove_the_one_acquisition_and_never_reacquire() {
    let dir = tempfile::tempdir().unwrap();
    let lease = Arc::new(LifecycleLease::acquire(dir.path()).unwrap());
    let shared = Arc::clone(&lease);
    assert_eq!(shared.root(), dir.path());
    let weak = Arc::downgrade(&shared);
    drop(lease);
    // The one remaining Arc keeps the OS acquisition alive: a fresh
    // acquisition is still refused, which is what makes "cloning the Arc is
    // sharing, not reacquiring" a measurement rather than a hope.
    assert!(matches!(
        LifecycleLease::acquire(dir.path()),
        Err(LifecycleLeaseError::Busy { .. })
    ));
    drop(shared);
    assert!(weak.upgrade().is_none());
    LifecycleLease::acquire(dir.path()).expect("the last drop released the lock");
}

/// The lease is OUTERMOST: holding it, the inner cooperative locks are
/// takeable in the allowed order. The reverse edge — acquiring this lease
/// while holding an inner one — has no production path (acquire is called
/// only at the five mutating command boundaries), and this file pins the
/// forward direction so a regression that inverted the order would deadlock
/// here rather than in an operator's terminal.
#[test]
fn the_lease_is_outermost_and_inner_locks_remain_takeable_under_it() {
    let dir = tempfile::tempdir().unwrap();
    let lease = LifecycleLease::acquire(dir.path()).unwrap();
    let project = lease.project();
    for inner in ["compile-trace.lock", "package-skills.lock"] {
        let guard = project
            .try_lock(inner)
            .expect("opening the inner lock")
            .unwrap_or_else(|| panic!("`{inner}` is takeable while the lease is held"));
        drop(guard);
    }
    drop(lease);
}

/// Process death releases the lease — the property that makes Busy a
/// refusal an operator can wait out rather than a wedged tree. A child
/// process (this same test binary, re-executed) acquires and parks; the
/// parent proves contention, kills it, and acquires.
#[test]
fn process_death_releases_the_lease() {
    let dir = tempfile::tempdir().unwrap();
    let ready = dir.path().join("LEASE-READY");
    let mut holder = Command::new(env::current_exe().unwrap())
        .arg("--exact")
        .arg("lease::tests::lease_process_helper")
        .arg("--nocapture")
        .env(HELPER_ROOT_ENV, dir.path())
        .spawn()
        .unwrap();
    let started = Instant::now();
    while fs::symlink_metadata(&ready).is_err() {
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "the holder process never signalled readiness"
        );
        thread::sleep(Duration::from_millis(20));
    }
    assert!(
        matches!(
            LifecycleLease::acquire(dir.path()),
            Err(LifecycleLeaseError::Busy { .. })
        ),
        "the child's acquisition is visible to this process",
    );
    holder.kill().expect("the holder is killable");
    holder.wait().unwrap();
    LifecycleLease::acquire(dir.path())
        .expect("the OS released the lock when the holder process died");
}

#[test]
fn lease_process_helper() {
    let Ok(root) = env::var(HELPER_ROOT_ENV) else {
        return;
    };
    let lease = LifecycleLease::acquire(Path::new(&root)).unwrap();
    fs::write(Path::new(&root).join("LEASE-READY"), b"ready").unwrap();
    let _still_held = lease;
    loop {
        thread::sleep(Duration::from_secs(1));
    }
}

/// A root that cannot be pinned is a ROOT problem: the refusal is the typed
/// `Directory`, names the offending root, offers the root-shaped remedy — and
/// never advises deleting the erasable lifecycle cache, because the cache is
/// not what refused. Relative and missing roots are the two shapes a
/// mis-resolved locator can produce.
#[test]
fn relative_and_missing_roots_refuse_as_directory_with_a_root_remedy() {
    let missing = tempfile::tempdir().unwrap();
    let gone = missing.path().join("gone");
    drop(missing);
    let gone = gone.as_path();
    for (label, root) in [
        ("relative", Path::new("definitely/not/absolute")),
        ("missing", gone),
    ] {
        match LifecycleLease::acquire(root) {
            Err(LifecycleLeaseError::Directory {
                root: named,
                reason,
            }) => {
                assert_eq!(named, root, "{label}: the refusal names the root");
                assert!(!reason.is_empty(), "{label}: the safefs chain is rendered");
            }
            other => panic!("{label} root must refuse as Directory, not {other:?}"),
        }
        let rendered = LifecycleLease::acquire(root)
            .expect_err("still refuses")
            .to_string();
        assert!(rendered.contains("PROP-054"), "{label}: {rendered}");
        assert!(
            rendered.contains("ensure the workspace root exists"),
            "{label}: the remedy is root-shaped: {rendered}"
        );
        assert!(
            !rendered.contains("erasable cache"),
            "{label}: a root problem never advises deleting state: {rendered}"
        );
        assert!(
            rendered.contains(&root.display().to_string()),
            "{label}: the offending root is named: {rendered}"
        );
    }
}

/// The one root-agreement gate: an agreeing root passes silently, a
/// disagreeing one is the TYPED mismatch naming both spellings and the
/// boundary — never a hand-rolled per-site refusal string.
#[test]
fn ensure_root_passes_agreement_and_types_the_mismatch() {
    let dir = tempfile::tempdir().unwrap();
    let lease = LifecycleLease::acquire(dir.path()).unwrap();
    lease
        .ensure_root(dir.path(), "run prelude")
        .expect("the same root agrees");
    let other = dir.path().join("elsewhere");
    match lease.ensure_root(&other, "phase dispatch") {
        Err(LifecycleLeaseError::RootMismatch {
            leased,
            observed,
            boundary,
        }) => {
            assert_eq!(leased, dir.path());
            assert_eq!(observed, other);
            assert_eq!(boundary, "phase dispatch");
        }
        other => panic!("a disagreement is the typed mismatch, not {other:?}"),
    }
    let rendered = lease
        .ensure_root(&other, "phase dispatch")
        .expect_err("still refuses")
        .to_string();
    assert!(rendered.contains("PROP-054"), "{rendered}");
    assert!(
        rendered.contains(&other.display().to_string())
            && rendered.contains(&dir.path().display().to_string()),
        "both spellings are named: {rendered}"
    );
    assert!(
        rendered.contains("inspect any earlier effects already reported"),
        "the late boundary refusal does not erase earlier effects: {rendered}"
    );
    drop(lease);
}
