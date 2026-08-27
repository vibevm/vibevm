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

use std::fs;
use std::io::Write;
use std::path::Path;

use crate::{Project, arm_before_bounded_read};

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
