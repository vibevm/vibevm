//! The width fence on capability enumeration.
//!
//! The fence exists so a walk over an unknown tree cannot be sized by what it
//! finds. That only holds if the refusal happens *inside* the loop: a listing
//! that collects everything and checks the length afterwards has already paid
//! for the directory it was trying not to pay for, and would pass a test that
//! only looks at the returned error.

use std::fs;

use crate::Project;

fn project_with(children: usize) -> (tempfile::TempDir, Project) {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join("pack")).unwrap();
    for index in 0..children {
        fs::write(dir.path().join("pack").join(format!("{index}.txt")), b"x").unwrap();
    }
    let project = Project::open(dir.path()).unwrap();
    (dir, project)
}

#[test]
fn exactly_max_children_is_an_answer_not_a_refusal() {
    let (_dir, project) = project_with(4);
    let pack = project.dir(&["pack"], false).unwrap();
    let mut names = project.child_names_bounded(&pack, 4).unwrap();
    names.sort();
    assert_eq!(names, ["0.txt", "1.txt", "2.txt", "3.txt"]);
}

#[test]
fn one_child_past_max_refuses_and_names_the_fence() {
    let (_dir, project) = project_with(5);
    let pack = project.dir(&["pack"], false).unwrap();
    let text = format!(
        "{:#}",
        project
            .child_names_bounded(&pack, 4)
            .expect_err("a directory over the fence has no bounded answer")
    );
    assert!(
        text.contains("more than 4 direct children"),
        "the refusal must name the fence it hit, got: {text}",
    );
    assert!(
        text.contains("truncated"),
        "and must say it refused rather than shortened the listing, got: {text}",
    );
}

/// Zero is a real fence, not a disabled one: an empty directory answers, and
/// any occupant at all refuses.
#[test]
fn a_zero_fence_admits_only_an_empty_directory() {
    let (dir, project) = project_with(0);
    let pack = project.dir(&["pack"], false).unwrap();
    assert!(project.child_names_bounded(&pack, 0).unwrap().is_empty());

    fs::write(dir.path().join("pack/one.txt"), b"x").unwrap();
    assert!(
        project.child_names_bounded(&pack, 0).is_err(),
        "one child is already past a fence of zero",
    );
}

/// `usize::MAX` is the unbounded case and must not be reached by computing
/// `max + 1` anywhere — the fence compares a retained count instead.
#[test]
fn an_unbounded_fence_matches_the_plain_wrapper() {
    let (_dir, project) = project_with(6);
    let pack = project.dir(&["pack"], false).unwrap();
    let mut bounded = project.child_names_bounded(&pack, usize::MAX).unwrap();
    let mut plain = project.child_names(&pack).unwrap();
    bounded.sort();
    plain.sort();
    assert_eq!(bounded, plain);
    assert_eq!(plain.len(), 6);
}

/// The wrapper is the bounded primitive with the fence lifted — same answer,
/// same refusals — so there is one enumeration to review, not two.
#[test]
fn the_plain_wrapper_still_lists_a_directory_it_can_reach() {
    let (_dir, project) = project_with(2);
    let pack = project.dir(&["pack"], false).unwrap();
    assert_eq!(project.child_names(&pack).unwrap().len(), 2);
}

/// Enumeration order belongs to the filesystem: the fence must not quietly
/// sort, because a caller whose canonical order is byte-wise over names has to
/// be the one that applies it.
#[test]
fn the_fence_returns_names_without_imposing_an_order() {
    let (dir, project) = project_with(0);
    for name in ["b.txt", "a.txt", "c.txt"] {
        fs::write(dir.path().join("pack").join(name), b"x").unwrap();
    }
    let pack = project.dir(&["pack"], false).unwrap();
    let names = project.child_names_bounded(&pack, 8).unwrap();
    let mut sorted = names.clone();
    sorted.sort();
    assert_eq!(sorted, ["a.txt", "b.txt", "c.txt"]);
    assert_eq!(names.len(), 3, "and every name survived the fence");
}

/// A non-UTF8 name is a refusal, not a skipped entry: a listing that silently
/// drops a name it cannot spell is a listing its caller cannot trust to be the
/// whole directory. Only reachable where the platform admits such a name.
#[cfg(unix)]
#[test]
fn a_non_utf8_name_refuses_rather_than_vanishing() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    let (dir, project) = project_with(0);
    let raw = OsStr::from_bytes(b"\xff\xfe.bin");
    if fs::write(dir.path().join("pack").join(raw), b"x").is_err() {
        return; // The volume refuses the name; nothing to prove here.
    }
    let pack = project.dir(&["pack"], false).unwrap();
    let text = format!(
        "{:#}",
        project
            .child_names_bounded(&pack, 8)
            .expect_err("an unspellable name is not a name to skip")
    );
    assert!(text.contains("non-UTF8"), "got: {text}");
}
