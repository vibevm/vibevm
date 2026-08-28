//! Bounded reads: the same walk and the same single-link law as the ordinary
//! read, plus an allocation ceiling that cannot be talked over.
//!
//! `read_file` trusts whatever length the filesystem reports, and a file that
//! reports — or grows to — gigabytes turns that trust into an unbounded
//! allocation. These tests pin the ceiling's edges — the exact fit, one byte
//! over, growth inside the single read epoch (both the whole-file and the
//! refusal side), the zero cap, absence, and the link refusals the ordinary
//! read already enforces — and the returned buffer's *capacity*, which is the
//! mechanical half of the promise: metadata-derived on the stable path,
//! exactly-reserved and never past `cap + 1` when growth stays inside the cap.
//! They also pin the final-name identity law: bytes count only while the
//! capability-relative name still denotes the object that supplied them.

use std::fs;
use std::io::Write;
use std::path::Path;

use crate::{Project, arm_before_bounded_read, arm_bounded_read_identity_check};

fn project() -> (tempfile::TempDir, Project) {
    let dir = tempfile::tempdir().unwrap();
    let project = Project::open(dir.path()).unwrap();
    (dir, project)
}

/// A file exactly at the cap is the whole answer, not a refusal: the ceiling
/// bounds allocation, it does not demand slack. Both entry points walk the
/// same pinned capability to get it.
#[test]
fn an_exact_cap_file_is_returned_whole() {
    let (dir, project) = project();
    fs::create_dir_all(dir.path().join("docs")).unwrap();
    fs::write(dir.path().join("docs/state.json"), b"12345").unwrap();

    let bytes = project
        .read_file_bounded("docs/state.json", 5)
        .unwrap()
        .expect("exactly `cap` bytes is inside the ceiling");
    assert_eq!(bytes, b"12345");

    let docs = project.dir(&["docs"], false).unwrap();
    assert_eq!(
        project
            .read_file_bounded_in(&docs, "state.json", 5)
            .unwrap(),
        Some(b"12345".to_vec()),
        "and the pinned-directory entry point agrees",
    );
}

/// On the metadata-stable path the returned buffer's *capacity* proves the
/// promise, not just its length: the reservation is metadata-derived, so a
/// small file under a huge cap never pays a cap-sized allocation, and the
/// capacity never doubles — `read_to_end` would grow a full metadata-sized
/// buffer geometrically to probe for EOF, which is exactly the overshoot the
/// exact-reservation loop exists to exclude. Only the allocator's own
/// rounding may lift the capacity above the metadata length.
#[test]
fn a_stable_read_reserves_metadata_bytes_not_cap_bytes() {
    let (dir, project) = project();
    fs::write(dir.path().join("small.json"), b"12345").unwrap();

    let cap = 1 << 20;
    let bytes = project
        .read_file_bounded("small.json", cap)
        .unwrap()
        .expect("a five-byte file is inside a one-megabyte cap");
    assert_eq!(bytes, b"12345");
    assert!(
        bytes.capacity() < bytes.len() * 2,
        "capacity {} is a doubling of the {} content bytes — geometric growth, \
         not exact reservation",
        bytes.capacity(),
        bytes.len(),
    );
    assert!(
        bytes.capacity() < cap,
        "a huge cap must not shape a small file's allocation",
    );
}

/// Growth that stays inside the cap is *not* a refusal: the whole file comes
/// back, and the buffer it lands in is still mechanically bounded — every
/// appended chunk was reserved exactly. The numbers are chosen so geometric
/// growth would overshoot: metadata reserves six bytes, the file grows to
/// eight under a cap of eight, and a doubling probe of the six-byte buffer
/// would carry the capacity to twelve, past `cap + 1` — the exact allocation
/// overshoot under a cap that the loop forbids.
#[test]
fn in_cap_growth_returns_the_whole_file_with_a_bounded_buffer() {
    let (dir, project) = project();
    let path = dir.path().join("state.json");
    fs::write(&path, b"123456").unwrap();
    // Opened before the read starts; `RefCell` because the hook seam is `Fn`
    // and it fires exactly once.
    let writer = std::cell::RefCell::new(fs::OpenOptions::new().write(true).open(&path).unwrap());

    arm_before_bounded_read(Some(Box::new(move |_, _| {
        // Metadata said six bytes; the file becomes eight, still under the
        // cap of eight.
        let mut writer = writer.borrow_mut();
        writer.write_all(b"12345678").unwrap();
        writer.flush().unwrap();
    })));
    let bytes = project
        .read_file_bounded("state.json", 8)
        .unwrap()
        .expect("growth that stays inside the cap is the whole file");
    arm_before_bounded_read(None);

    assert_eq!(bytes, b"12345678");
    assert!(
        bytes.capacity() <= 8 + 1,
        "capacity {} must stay bounded at cap + 1: every appended chunk was \
         reserved exactly",
        bytes.capacity(),
    );
    assert!(bytes.capacity() >= bytes.len());
}

/// One byte over the cap refuses and says what to do about it: the caller
/// sees the real length and the cap it offered, because "too big" without
/// numbers cannot be remediated. A truncated prefix must never come back as
/// `Ok` — five bytes of a six-byte file is not the file.
#[test]
fn one_byte_over_the_cap_refuses_with_remediation() {
    let (dir, project) = project();
    fs::write(dir.path().join("state.json"), b"123456").unwrap();

    let error = project
        .read_file_bounded("state.json", 5)
        .expect_err("six bytes under a five-byte cap is not a read");
    let rendered = format!("{error:#}");
    assert!(rendered.contains("state.json"), "{rendered}");
    assert!(rendered.contains('6'), "the real length: {rendered}");
    assert!(
        rendered.contains("5"),
        "the cap that was passed: {rendered}"
    );
    assert!(rendered.contains("cap"), "{rendered}");
}

/// The metadata length is a promise the file is free to break: it can grow
/// between the size check and the read, on this thread, inside the one read
/// epoch. The hook extends the file in exactly that window — no sleeps, no
/// second process — so only the `take(cap + 1)` fence stands between the
/// growth and a silent prefix.
#[test]
fn growth_inside_the_read_epoch_refuses_rather_than_returning_a_prefix() {
    let (dir, project) = project();
    let path = dir.path().join("state.json");
    fs::write(&path, b"12345").unwrap();
    // Opened before the read starts, so the only thing the hook does inside
    // the window is extend the file the bounded read already holds open.
    // `RefCell` because the hook seam is `Fn`, and it fires exactly once.
    let writer = std::cell::RefCell::new(fs::OpenOptions::new().write(true).open(&path).unwrap());

    arm_before_bounded_read(Some(Box::new(move |_, _| {
        // One byte past the cap, written from the start: the metadata the
        // read just checked said five bytes, and now there are six.
        let mut writer = writer.borrow_mut();
        writer.write_all(b"123456").unwrap();
        writer.flush().unwrap();
    })));
    let outcome = project.read_file_bounded("state.json", 5);
    arm_before_bounded_read(None);

    let error = outcome.expect_err("a file that grew past the cap mid-read is refused");
    let rendered = format!("{error:#}");
    assert!(rendered.contains("grew"), "{rendered}");
    assert!(rendered.contains("state.json"), "{rendered}");
    assert!(rendered.contains("5"), "the cap: {rendered}");
}

/// The zero cap is not special: it admits exactly the empty file and refuses
/// every byte. A caller asking "is this file empty" gets a real answer,
/// never a truncated `Ok`.
#[test]
fn a_zero_cap_admits_only_the_empty_file() {
    let (dir, project) = project();
    fs::write(dir.path().join("empty.json"), b"").unwrap();
    fs::write(dir.path().join("one.json"), b"x").unwrap();

    assert_eq!(
        project.read_file_bounded("empty.json", 0).unwrap(),
        Some(Vec::new()),
        "an empty file is inside even a zero cap",
    );
    let error = project
        .read_file_bounded("one.json", 0)
        .expect_err("one byte is over a zero cap");
    let rendered = format!("{error:#}");
    assert!(rendered.contains("0-byte cap"), "{rendered}");
    assert!(rendered.contains("one.json"), "{rendered}");
}

/// Absence stays `Ok(None)` — the one non-error outcome besides a read — for
/// a missing file and for a missing ancestor alike.
#[test]
fn an_absent_file_is_none_not_an_error() {
    let (_dir, project) = project();
    assert_eq!(project.read_file_bounded("missing.json", 16).unwrap(), None);
    assert_eq!(
        project
            .read_file_bounded("no/such/dir/missing.json", 16)
            .unwrap(),
        None,
        "a missing ancestor is the same absence",
    );
    let docs = project.dir(&["docs"], true).unwrap();
    assert_eq!(
        project
            .read_file_bounded_in(&docs, "missing.json", 16)
            .unwrap(),
        None,
    );
}

/// A symlink at the final name still refuses under a cap. Where the host will
/// not create one (unprivileged Windows) the case is unreachable and the test
/// says so instead of pretending to prove it — symlink creation is a host
/// privilege, the one skip this module allows.
#[test]
fn a_symlink_at_the_final_name_still_refuses_under_a_cap() {
    let (dir, project) = project();
    fs::write(dir.path().join("outside.json"), b"outside").unwrap();
    if !link_file(
        &dir.path().join("outside.json"),
        &dir.path().join("linked.json"),
    ) {
        return;
    }
    let error = project
        .read_file_bounded("linked.json", 64)
        .expect_err("a link at the final name refuses");
    let rendered = format!("{error:#}");
    assert!(rendered.contains("linked.json"), "{rendered}");
}

/// A second hard link to the file still refuses, and this proof may not skip:
/// both names live in the same temp fixture on the same filesystem, so a
/// `hard_link` failure is an environment worth failing on, not a case to
/// silently leave unproven.
#[test]
fn a_second_hard_link_still_refuses_under_a_cap() {
    let (dir, project) = project();
    fs::write(dir.path().join("original.json"), b"shared").unwrap();
    fs::hard_link(
        dir.path().join("original.json"),
        dir.path().join("second.json"),
    )
    .expect(
        "both names sit in one temp fixture on one filesystem; a hard link must be creatable \
         here or the single-link law has no proof on this host",
    );
    let error = project
        .read_file_bounded("second.json", 64)
        .expect_err("a second name of one file refuses");
    let rendered = format!("{error:#}");
    assert!(
        rendered.contains("names") || rendered.contains("hard link"),
        "{rendered}",
    );
    assert!(rendered.contains("second.json"), "{rendered}");
}

#[cfg(unix)]
fn link_file(target: &Path, link: &Path) -> bool {
    std::os::unix::fs::symlink(target, link).is_ok()
}

#[cfg(windows)]
fn link_file(target: &Path, link: &Path) -> bool {
    std::os::windows::fs::symlink_file(target, link).is_ok()
}

/// `usize::MAX` is not "no cap": the protocol leans on `cap + 1`, and letting
/// that wrap would fence the read at zero bytes instead of refusing. The
/// refusal is checked before anything is opened, so it holds whatever the
/// file's state is.
#[test]
fn a_max_cap_refuses_overflow_rather_than_wrapping() {
    let (dir, project) = project();
    fs::write(dir.path().join("state.json"), b"12345").unwrap();
    let error = project
        .read_file_bounded("state.json", usize::MAX)
        .expect_err("usize::MAX would wrap cap + 1");
    assert!(
        format!("{error:#}").contains("overflow"),
        "the refusal must name the overflow, not report a read",
    );
}

/// A stable name is the ordinary outcome, and the final-name gate really ran
/// to allow it: the comparator observed an actual `true` exactly once, and
/// the exact bytes come back whole. This is the anchor the override and
/// mutation proofs hang off — a gate that never fires discriminates nothing.
#[test]
fn a_stable_name_reads_whole_through_an_observed_true_identity() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    let (dir, project) = project();
    fs::write(dir.path().join("state.json"), b"12345").unwrap();

    let checks = Arc::new(AtomicUsize::new(0));
    let observed = Arc::clone(&checks);
    arm_bounded_read_identity_check(Some(Box::new(move |actual| {
        assert!(
            actual,
            "a stable fixture leaves the held and named identities equal"
        );
        observed.fetch_add(1, Ordering::SeqCst);
        actual
    })));
    let bytes = project
        .read_file_bounded("state.json", 16)
        .unwrap()
        .expect("a stable file reads whole");
    arm_bounded_read_identity_check(None);

    assert_eq!(bytes, b"12345");
    assert_eq!(
        checks.load(Ordering::SeqCst),
        1,
        "the final-name comparison ran exactly once and saw a real `true`",
    );
}

/// The final-name comparison is a mandatory gate even where the physical race
/// cannot be staged — Windows sharing rules may hold the name while the read
/// handle is open. Forcing the next otherwise-true comparison false must
/// refuse: this is the deterministic, all-host RED for the swap the Unix
/// tests below perform for real.
#[test]
fn a_forced_identity_mismatch_refuses_the_read_deterministically() {
    let (dir, project) = project();
    fs::write(dir.path().join("state.json"), b"12345").unwrap();

    arm_bounded_read_identity_check(Some(Box::new(|actual| {
        assert!(actual, "the real handle and name agree in this fixture");
        false
    })));
    let outcome = project.read_file_bounded("state.json", 16);
    arm_bounded_read_identity_check(None);

    let error = outcome.expect_err("an identity mismatch is a hard refusal");
    let rendered = format!("{error:#}");
    assert!(rendered.contains("state.json"), "{rendered}");
    assert!(rendered.contains("final-name race"), "{rendered}");
}

/// The race the gate exists for, staged physically: in the read window the
/// held inode is renamed to a sibling (still regular, still single-link) and
/// a different valid file takes the original name. A held handle's metadata
/// alone cannot see this — both objects are ordinary files with one link —
/// but the reopened name's identity can, and the read must refuse rather
/// than return either the old or the replacement bytes.
#[cfg(unix)]
#[test]
fn a_file_renamed_under_the_read_refuses_rather_than_returning_either_object() {
    let (dir, project) = project();
    let path = dir.path().join("state.json");
    let moved = dir.path().join("moved.json");
    fs::write(&path, b"original").unwrap();
    let (from, to) = (path.clone(), moved.clone());
    arm_before_bounded_read(Some(Box::new(move |_, _| {
        // The held inode keeps exactly one link under its new name; a fresh
        // regular single-link file occupies the original name.
        fs::rename(&from, &to).unwrap();
        fs::write(&from, b"replacement").unwrap();
    })));
    let outcome = project.read_file_bounded("state.json", 64);
    arm_before_bounded_read(None);

    let error = outcome.expect_err("the swap is refused, old and new bytes alike");
    let rendered = format!("{error:#}");
    assert!(rendered.contains("state.json"), "{rendered}");
    assert!(rendered.contains("final-name race"), "{rendered}");
    // Prove the race truly fired: the moved original and the replacement
    // both exist, so the refusal judged two distinct live objects.
    assert_eq!(
        fs::read(&moved).unwrap(),
        b"original",
        "the read object survives under its new name",
    );
    assert_eq!(
        fs::read(&path).unwrap(),
        b"replacement",
        "a different object now occupies the original name",
    );
}

/// The other half of the race: the name vanishes entirely (renamed away,
/// nothing takes its place). Absence at the INITIAL open stays `Ok(None)`;
/// absence after a handle was read is a hard refusal — never a quiet `None`
/// dressed as "not found", and never the stale bytes.
#[cfg(unix)]
#[test]
fn a_name_that_disappears_under_the_read_refuses() {
    let (dir, project) = project();
    let path = dir.path().join("state.json");
    let moved = dir.path().join("moved.json");
    fs::write(&path, b"original").unwrap();
    let (from, to) = (path.clone(), moved.clone());
    arm_before_bounded_read(Some(Box::new(move |_, _| {
        fs::rename(&from, &to).unwrap();
    })));
    let outcome = project.read_file_bounded("state.json", 64);
    arm_before_bounded_read(None);

    let error = outcome.expect_err("a vanished name is a hard refusal after a read began");
    let rendered = format!("{error:#}");
    assert!(rendered.contains("state.json"), "{rendered}");
    assert!(rendered.contains("final-name race"), "{rendered}");
    assert_eq!(fs::read(&moved).unwrap(), b"original");
    assert!(!path.exists(), "the original name is truly gone");
}

/// A second hard link planted under the read is a physical rebind the reopen
/// must refuse: the name still denotes the same object, but as one of TWO
/// names now — the single-link law bites exactly as it does at the initial
/// open. Staging it needs no rename-under-handle and no symlink privilege,
/// so it runs on every host whose filesystem offers hard links — the same
/// capability the existing initial-open test already relies on.
#[test]
fn a_hard_link_planted_under_the_read_refuses_at_the_reopen() {
    let (dir, project) = project();
    let path = dir.path().join("state.json");
    let second = dir.path().join("second.json");
    fs::write(&path, b"shared").unwrap();
    let (origin, alias) = (path.clone(), second.clone());
    arm_before_bounded_read(Some(Box::new(move |_, _| {
        fs::hard_link(&origin, &alias).expect(
            "both names sit in one temp fixture on one filesystem; a hard link must be creatable \
             here or the single-link law has no proof on this host",
        );
    })));
    let outcome = project.read_file_bounded("state.json", 64);
    arm_before_bounded_read(None);

    let error = outcome.expect_err("two names for the read object refuse at the reopen");
    let rendered = format!("{error:#}");
    assert!(
        rendered.contains("names") || rendered.contains("hard link"),
        "{rendered}",
    );
    assert!(rendered.contains("state.json"), "{rendered}");
    assert!(rendered.contains("final-name race"), "{rendered}");
}

/// Where the host permits the physical rebind (Unix renames the held inode
/// away freely), each hostile occupant of the final name — a symlink and a
/// directory — refuses at the reopen. Windows may make the rename-under-
/// handle unreachable through sharing rules and needs the symlink privilege
/// for the other arm; there the deterministic override above is the
/// non-skipping proof, so these physical cases are honestly cfg'd to Unix
/// rather than skipped mid-test.
#[cfg(unix)]
#[test]
fn a_rebound_final_name_refuses_symlink_and_directory_occupants() {
    let (dir, project) = project();

    let linked = dir.path().join("linked.json");
    let linked_moved = dir.path().join("linked-moved.json");
    fs::write(&linked, b"original").unwrap();
    let (from, to) = (linked.clone(), linked_moved.clone());
    arm_before_bounded_read(Some(Box::new(move |_, _| {
        fs::rename(&from, &to).unwrap();
        std::os::unix::fs::symlink(&to, &from).unwrap();
    })));
    let outcome = project.read_file_bounded("linked.json", 64);
    arm_before_bounded_read(None);
    let error = outcome.expect_err("a symlink at the final name refuses at the reopen");
    let rendered = format!("{error:#}");
    assert!(rendered.contains("linked.json"), "{rendered}");
    assert!(rendered.contains("final-name race"), "{rendered}");

    let occupied = dir.path().join("occupied.json");
    let occupied_moved = dir.path().join("occupied-moved.json");
    fs::write(&occupied, b"original").unwrap();
    let (from, to) = (occupied.clone(), occupied_moved.clone());
    arm_before_bounded_read(Some(Box::new(move |_, _| {
        fs::rename(&from, &to).unwrap();
        fs::create_dir(&from).unwrap();
    })));
    let outcome = project.read_file_bounded("occupied.json", 64);
    arm_before_bounded_read(None);
    let error = outcome.expect_err("a directory at the final name refuses at the reopen");
    let rendered = format!("{error:#}");
    assert!(rendered.contains("occupied.json"), "{rendered}");
    assert!(rendered.contains("final-name race"), "{rendered}");
}
