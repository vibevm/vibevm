//! Losing the directory-creation race, deterministically.
//!
//! The interesting branch is the loser's: it must report that it did **not**
//! create the directory (so a caller never offers to remove one it does not
//! own) and it must still reopen no-follow (so losing does not become a way to
//! get a link followed). A real two-process race would exercise that branch by
//! luck; the armed hook fires in exactly the window the other creator would
//! land in, so the branch is reachable on demand.

use std::fs;

use crate::{ExclusiveChildError, Project, arm_after_create_dir, arm_before_create_dir};

fn project() -> (tempfile::TempDir, Project) {
    let dir = tempfile::tempdir().unwrap();
    let project = Project::open(dir.path()).unwrap();
    (dir, project)
}

/// The three outcomes of an exclusive creation, each proven separately —
/// because the two failures differ in what *this call* did, and a caller that
/// cannot tell them apart either walks away from a name its own `create_dir`
/// succeeded at, or reaches for one it never created.
#[test]
fn exclusive_creation_distinguishes_all_three_outcomes() {
    let (_dir, project) = project();
    let root = project.root_dir().unwrap();

    // 1. Created and reopened.
    let child = root
        .create_child_exclusive("first")
        .expect("a fresh name is created and reopened");
    assert!(child.path().is_dir());
    drop(child);

    // 2. Not created: nothing at that name came from this call.
    let error = root
        .create_child_exclusive("first")
        .expect_err("an existing entry refuses");
    assert!(
        matches!(error, ExclusiveChildError::NotCreated(_)) && !error.created(),
        "a refused creation must not claim it created anything: {error}",
    );
    assert!(
        error.to_string().contains("exclusively creating"),
        "{error}",
    );
    assert!(
        root.path().join("first").is_dir(),
        "and the entry it refused to reuse is untouched",
    );

    // 3. This call's create succeeded and the reopen then failed. The type has
    //    to carry that, or the entry becomes one nobody collects.
    arm_after_create_dir(Some(Box::new(|_parent, _name| {
        Some(std::io::Error::other("injected reopen failure"))
    })));
    let error = root.create_child_exclusive("second").expect_err("injected");
    arm_after_create_dir(None);

    assert!(
        error.created(),
        "a create that succeeded must be reported as such: {error}",
    );
    assert!(
        root.path().join("second").is_dir(),
        "the entry really is there — that is why the caller must be told",
    );
}

/// The diagnostic states **only what was proved**, and stays true in both
/// worlds the same failure spans.
///
/// The reopen is the step that would have verified the entry, and it is the
/// step that failed. So the message may say this call created something at that
/// name — that is proved by `create_dir` returning `Ok` — and it may say the
/// entry could not be verified and may have been replaced. It may **not** say
/// the entry is still the caller's: in the swap case below, the identical
/// failure is reported while the name holds a foreign junction.
#[test]
fn the_created_not_reopened_diagnostic_claims_only_what_it_proved() {
    let (dir, project) = project();
    let root = project.root_dir().unwrap();

    // (a) An ordinary reopen failure: the directory this call made is still
    //     there, untouched.
    arm_after_create_dir(Some(Box::new(|_parent, _name| {
        Some(std::io::Error::other("injected reopen failure"))
    })));
    let ordinary = root.create_child_exclusive("plain").expect_err("injected");
    arm_after_create_dir(None);

    // (b) The same failure with the entry swapped for something foreign.
    let outside = tempfile::tempdir().unwrap();
    let planted = dir.path().join("swapped");
    let target = outside.path().to_path_buf();
    arm_after_create_dir(Some(Box::new(move |_parent, _name| {
        fs::remove_dir(&planted).unwrap();
        plant_link(&planted, &target);
        None
    })));
    let swapped = root.create_child_exclusive("swapped").expect_err("swapped");
    arm_after_create_dir(None);

    for (label, error, cause) in [
        ("ordinary", &ordinary, "injected reopen failure"),
        ("swapped", &swapped, "reopening created"),
    ] {
        let rendered = error.to_string();
        assert!(rendered.contains(cause), "`{label}`: {rendered}");
        assert!(
            rendered.contains(&format!(
                "(this call created `{}`, but the entry now at that name could not be reopened \
                 no-follow and may have been replaced since)",
                root.path()
                    .join(label.replace("ordinary", "plain"))
                    .display()
            )),
            "`{label}` must render the exact proved-facts clause: {rendered}",
        );
        assert!(
            !rendered.contains("owned") && !rendered.contains("still"),
            "`{label}` must not claim continuing ownership: {rendered}",
        );
        assert!(error.created(), "`{label}` did create the entry");
    }

    // The claim the wording refuses to make is exactly the one that would be
    // false here: the swapped name is a link to somewhere else entirely.
    assert!(
        fs::symlink_metadata(dir.path().join("swapped"))
            .unwrap()
            .file_type()
            .is_symlink()
            || fs::read_dir(dir.path().join("swapped")).is_ok(),
        "the swapped entry is the planted one, not the directory this call made",
    );
}

#[cfg(windows)]
fn plant_link(planted: &std::path::Path, target: &std::path::Path) {
    // `cmd` reads a forward slash as the start of a switch.
    let planted = std::path::PathBuf::from(planted.to_string_lossy().replace('/', "\\"));
    let output = std::process::Command::new("cmd")
        .args(["/c", "mklink", "/J"])
        .arg(&planted)
        .arg(target)
        .output()
        .expect("mklink is available on Windows");
    assert!(
        output.status.success(),
        "planting the junction: {}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}

#[cfg(unix)]
fn plant_link(planted: &std::path::Path, target: &std::path::Path) {
    std::os::unix::fs::symlink(target, planted).unwrap();
}

/// The hook fires strictly between the create and the reopen: when it does not
/// force a failure, the real reopen runs and sees what the hook left.
#[test]
fn the_after_create_hook_fires_before_the_reopen() {
    let (_dir, project) = project();
    let root = project.root_dir().unwrap();

    arm_after_create_dir(Some(Box::new(|parent, name| {
        assert!(
            parent.path().join(name).is_dir(),
            "the directory exists by the time the hook runs",
        );
        // Remove it again: the reopen that follows must fail, not succeed on a
        // stale handle.
        fs::remove_dir(parent.path().join(name)).unwrap();
        None
    })));
    let error = root
        .create_child_exclusive("gone")
        .expect_err("the reopen fails");
    arm_after_create_dir(None);

    assert!(error.created(), "this call did create it: {error}");
    assert!(
        !root.path().join("gone").exists(),
        "and the hook's removal is what the reopen ran into",
    );
}

#[test]
fn exactly_one_of_two_creators_claims_ownership() {
    let (_dir, project) = project();
    let root = project.root_dir().unwrap();

    let (first, created_first) = root.ensure_child_recording("docs").unwrap();
    let (second, created_second) = root.ensure_child_recording("docs").unwrap();

    assert!(created_first, "the first caller created it");
    assert!(!created_second, "the second caller did not");
    assert!(first.path().is_dir() && second.path().is_dir());
}

/// The window the probe cannot see: absent when checked, present when created.
#[test]
fn a_creator_that_loses_the_race_reports_that_it_did_not_create() {
    let (_dir, project) = project();
    let root = project.root_dir().unwrap();

    // Fire in the gap: create the directory after the probe said absent.
    arm_before_create_dir(Some(Box::new(|parent, name| {
        let _ = parent.ensure_child(name);
    })));
    let outcome = root.ensure_child_recording("docs");
    arm_before_create_dir(None);

    let (pinned, created) = outcome.expect("losing the race is not a failure");
    assert!(
        !created,
        "a loser that reported `created` would offer to remove another owner's directory",
    );
    assert!(pinned.path().is_dir());
}

/// Losing the race must not become a way to get a link followed: the loser
/// reopens no-follow, so a planted link refuses instead of resolving.
#[test]
#[cfg(windows)]
fn a_loser_refuses_a_link_planted_by_the_winner() {
    let outside = tempfile::tempdir().unwrap();
    let (dir, project) = project();
    let root = project.root_dir().unwrap();
    let target = outside.path().to_path_buf();
    let planted = dir.path().join("docs");

    arm_before_create_dir(Some(Box::new(move |_parent, _name| {
        let status = std::process::Command::new("cmd")
            .args(["/c", "mklink", "/J"])
            .arg(&planted)
            .arg(&target)
            .output()
            .expect("mklink is available on Windows");
        assert!(status.status.success(), "planting the junction");
    })));
    let outcome = root.ensure_child_recording("docs");
    arm_before_create_dir(None);

    assert!(
        outcome.is_err(),
        "a junction planted by the winner must refuse, not resolve",
    );
    assert!(
        fs::read_dir(outside.path()).unwrap().next().is_none(),
        "and nothing outside the project was touched",
    );
}

#[test]
#[cfg(unix)]
fn a_loser_refuses_a_link_planted_by_the_winner() {
    let outside = tempfile::tempdir().unwrap();
    let (dir, project) = project();
    let root = project.root_dir().unwrap();
    let target = outside.path().to_path_buf();
    let planted = dir.path().join("docs");

    arm_before_create_dir(Some(Box::new(move |_parent, _name| {
        std::os::unix::fs::symlink(&target, &planted).unwrap();
    })));
    let outcome = root.ensure_child_recording("docs");
    arm_before_create_dir(None);

    assert!(outcome.is_err(), "a symlink planted by the winner refuses");
    assert!(fs::read_dir(outside.path()).unwrap().next().is_none());
}

/// An OS file lock is taken on an open file description, not on a name. If the
/// lock file is unlinked and recreated between the open and the lock, a naive
/// acquisition ends up holding a lock on an object the path no longer names —
/// and a second holder can lock the *new* one, leaving two owners of one
/// project.
///
/// The post-lock identity recheck refuses that: a stale lock is released and
/// the acquisition re-contends for whatever the name means now. The hook
/// performs the rebind without asserting that the host permits it — hosts
/// differ on unlink-while-open — and the test records which world it ran in,
/// then asserts the invariant that must hold in both: the guard covers the
/// file the path names NOW.
#[test]
fn a_lock_file_replaced_before_the_lock_is_not_accepted() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    let dir = tempfile::tempdir().unwrap();
    let project = Project::open(dir.path()).unwrap();
    drop(project.try_lock("swap.lock").unwrap().unwrap());
    let lock_path = dir.path().join(".vibe").join("swap.lock");

    let rebound = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&rebound);
    let planted = lock_path.clone();
    crate::arm_before_lock(Some(Box::new(move |_, _| {
        let done = std::fs::remove_file(&planted)
            .and_then(|()| std::fs::write(&planted, b"a different lock file"));
        flag.store(done.is_ok(), Ordering::SeqCst);
    })));
    let guard = project.try_lock("swap.lock");
    crate::arm_before_lock(None);

    let guard = guard
        .expect("the acquisition re-contends rather than failing")
        .expect("and it ends up holding the current object");

    if rebound.load(Ordering::SeqCst) {
        // Read through METADATA, not content: a held OS lock can refuse a
        // second handle's read of the locked range, and the length is enough
        // to tell the planted file (non-empty) from the original (empty).
        assert_eq!(
            std::fs::metadata(&lock_path).unwrap().len(),
            b"a different lock file".len() as u64,
            "the rebind really happened on this host, so the recheck is what saved it",
        );
    }
    // True either way, and the only thing that matters: nobody else can take
    // the lock the path currently names.
    assert!(
        project.try_lock("swap.lock").unwrap().is_none(),
        "the held guard covers the file the path currently names",
    );
    drop(guard);
    assert!(project.try_lock("swap.lock").unwrap().is_some());
}

/// The post-lock comparison itself is a mandatory gate even on a host that
/// refuses the real unlink-while-open race above. The first otherwise-true
/// comparison is forced false; acquisition must drop that handle, contend a
/// second time, and consult the gate again before returning a guard.
#[test]
fn a_post_lock_identity_mismatch_recontends_deterministically() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    let dir = tempfile::tempdir().unwrap();
    let project = Project::open(dir.path()).unwrap();
    let checks = Arc::new(AtomicUsize::new(0));
    let observed = Arc::clone(&checks);
    crate::arm_lock_identity_check(Some(Box::new(move |actual| {
        assert!(actual, "the real path and handle agree in this fixture");
        observed.fetch_add(1, Ordering::SeqCst) != 0
    })));
    let guard = project.try_lock("identity.lock");
    crate::arm_lock_identity_check(None);

    let guard = guard
        .expect("the injected mismatch is retried")
        .expect("the second, matching attempt owns the lock");
    assert_eq!(
        checks.load(Ordering::SeqCst),
        2,
        "one rejected comparison plus one successful recheck",
    );
    assert!(project.try_lock("identity.lock").unwrap().is_none());
    drop(guard);
}

/// The ordinary path is unchanged: one holder at a time, and the guard is
/// released by drop.
#[test]
fn an_exclusive_lock_admits_one_holder_at_a_time() {
    let dir = tempfile::tempdir().unwrap();
    let project = Project::open(dir.path()).unwrap();

    let held = project.try_lock("run.lock").unwrap().expect("first holder");
    assert!(
        project.try_lock("run.lock").unwrap().is_none(),
        "a second holder is refused while the first lives",
    );
    drop(held);
    assert!(
        project.try_lock("run.lock").unwrap().is_some(),
        "and admitted once it drops",
    );
}
