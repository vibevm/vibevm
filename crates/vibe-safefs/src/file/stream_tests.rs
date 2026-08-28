//! What a streamed content digest may and may not answer.
//!
//! The exactness half is checked against an *independently* recomputed digest
//! — a second hasher fed the same bytes by ordinary means — because a test that
//! calls the primitive twice and compares would pass for any self-consistent
//! wrong answer. The refusal half is checked at the one window a single pass
//! cannot see: between the two passes, where a same-length rewrite lands.

use std::fs;
use std::io::Write;
use std::path::Path;

use sha2::{Digest, Sha256};

use crate::{Project, arm_between_stream_passes, arm_bounded_read_identity_check};

fn project() -> (tempfile::TempDir, Project) {
    let dir = tempfile::tempdir().unwrap();
    let project = Project::open(dir.path()).unwrap();
    (dir, project)
}

/// The answer any other implementation would have to reproduce: a bare
/// SHA-256 over the bytes, with no framing of ours mixed in.
fn expected(bytes: &[u8]) -> [u8; 32] {
    let mut hash = Sha256::new();
    hash.update(bytes);
    hash.finalize().into()
}

fn write_and_digest(name: &str, bytes: &[u8]) -> crate::ContentDigest {
    let (dir, project) = project();
    fs::write(dir.path().join(name), bytes).unwrap();
    let root = project.root_dir().unwrap();
    project
        .digest_file_in(&root, name)
        .unwrap()
        .expect("the file is there")
}

/// Empty is a content identity like any other, not a missing one: it has a
/// length of zero and the well-known digest of no bytes, and confusing it with
/// absence would make an emptied artifact indistinguishable from a deleted one.
#[test]
fn an_empty_file_has_the_empty_digest_and_zero_length() {
    let digest = write_and_digest("empty.bin", b"");
    assert_eq!(digest.len, 0);
    assert_eq!(digest.sha256, expected(b""));
    assert_eq!(
        hex(&digest.sha256),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
        "and it is the published SHA-256 of the empty input, not a local convention",
    );
}

#[test]
fn an_ordinary_file_matches_an_independent_digest() {
    let bytes = b"the quick brown fox\n";
    let digest = write_and_digest("ordinary.txt", bytes);
    assert_eq!(digest.len, bytes.len() as u64);
    assert_eq!(digest.sha256, expected(bytes));
}

/// The window is 16 KiB, so a file of three whole windows plus one byte is the
/// case where the loop's last iteration is a partial read followed by EOF —
/// the arithmetic most likely to drop or double-count a tail.
#[test]
fn a_partial_final_chunk_is_counted_exactly_once() {
    let bytes: Vec<u8> = (0..(3 * 16 * 1024 + 1))
        .map(|index| (index % 251) as u8)
        .collect();
    let digest = write_and_digest("partial.bin", &bytes);
    assert_eq!(digest.len, 3 * 16 * 1024 + 1);
    assert_eq!(digest.sha256, expected(&bytes));
}

/// Multi-megabyte zeros: many windows, a length past any plausible buffer, and
/// content whose digest a mistake in the loop would silently preserve — a
/// dropped or repeated window of zeros still hashes to something, so this only
/// works against an independent digest.
#[test]
fn a_multi_megabyte_run_streams_without_retaining_it() {
    let bytes = vec![0_u8; 5 * 1024 * 1024 + 7];
    let digest = write_and_digest("zeros.bin", &bytes);
    assert_eq!(digest.len, 5 * 1024 * 1024 + 7);
    assert_eq!(digest.sha256, expected(&bytes));
}

/// The mutation that motivates the second pass: same length, so metadata is
/// unchanged; same object, so identity is unchanged; different bytes, so the
/// one-pass digest would have been a torn read blessed as content.
#[test]
fn a_same_length_rewrite_between_passes_refuses() {
    let (dir, project) = project();
    let path = dir.path().join("binary.bin");
    fs::write(&path, vec![b'a'; 64 * 1024]).unwrap();
    let root = project.root_dir().unwrap();

    let target = path.clone();
    arm_between_stream_passes(Some(Box::new(move |_directory, _name| {
        // Exactly the same number of bytes, written over the same inode.
        fs::write(&target, vec![b'b'; 64 * 1024]).unwrap();
    })));
    let refusal = project
        .digest_file_in(&root, "binary.bin")
        .expect_err("two passes disagreeing on content is not an answer");
    let text = format!("{refusal:#}");
    assert!(
        text.contains("changed content while it was measured"),
        "the refusal must name the disagreement, got: {text}",
    );

    assert!(
        project
            .digest_file_in(&root, "binary.bin")
            .unwrap()
            .is_some(),
        "and the single-shot hook disarmed itself, so a quiet file measures cleanly",
    );
}

#[test]
fn append_growth_between_passes_refuses() {
    let (dir, project) = project();
    let path = dir.path().join("growing.log");
    fs::write(&path, b"start").unwrap();
    let root = project.root_dir().unwrap();

    let target = path.clone();
    arm_between_stream_passes(Some(Box::new(move |_directory, _name| {
        let mut handle = fs::OpenOptions::new().append(true).open(&target).unwrap();
        handle.write_all(b"-more").unwrap();
    })));
    let text = format!(
        "{:#}",
        project
            .digest_file_in(&root, "growing.log")
            .expect_err("a file that grew mid-measurement has no one length")
    );
    assert!(
        text.contains("did not hold still"),
        "the refusal must name the length disagreement, got: {text}",
    );
}

#[test]
fn truncating_shrink_between_passes_refuses() {
    let (dir, project) = project();
    let path = dir.path().join("shrinking.log");
    fs::write(&path, b"a long enough line to lose some of").unwrap();
    let root = project.root_dir().unwrap();

    let target = path.clone();
    arm_between_stream_passes(Some(Box::new(move |_directory, _name| {
        fs::OpenOptions::new()
            .write(true)
            .open(&target)
            .unwrap()
            .set_len(4)
            .unwrap();
    })));
    let text = format!(
        "{:#}",
        project
            .digest_file_in(&root, "shrinking.log")
            .expect_err("a file that shrank mid-measurement has no one length")
    );
    assert!(
        text.contains("did not hold still"),
        "the refusal must name the length disagreement, got: {text}",
    );
}

/// The final-name law is the bounded read's, reused rather than reimplemented,
/// so the same deterministic seam proves it here: bytes count only while the
/// name still denotes the object that supplied them.
#[test]
fn a_final_name_rebind_after_the_second_pass_refuses() {
    let (dir, project) = project();
    fs::write(dir.path().join("witnessed.bin"), b"payload").unwrap();
    let root = project.root_dir().unwrap();

    arm_bounded_read_identity_check(Some(Box::new(|_actual| false)));
    let text = format!(
        "{:#}",
        project
            .digest_file_in(&root, "witnessed.bin")
            .expect_err("a rebound final name refuses, it does not answer with stale bytes")
    );
    arm_bounded_read_identity_check(None);
    assert!(
        text.contains("was replaced while being read"),
        "the refusal must name the final-name race, got: {text}",
    );

    assert!(
        project
            .digest_file_in(&root, "witnessed.bin")
            .unwrap()
            .is_some(),
        "and disarming the seam restores the ordinary answer for the next test",
    );
}

#[test]
fn a_missing_name_is_absence_not_an_error() {
    let (_dir, project) = project();
    let root = project.root_dir().unwrap();
    assert_eq!(project.digest_file_in(&root, "nowhere.bin").unwrap(), None);
}

/// A second name for the same bytes means this crate is not the object's only
/// owner, which is the same refusal every other read here makes.
#[test]
fn a_hard_linked_file_refuses() {
    let (dir, project) = project();
    fs::write(dir.path().join("origin.bin"), b"shared").unwrap();
    if fs::hard_link(dir.path().join("origin.bin"), dir.path().join("alias.bin")).is_err() {
        return; // No hard links on this volume; the law is proved elsewhere.
    }
    let root = project.root_dir().unwrap();
    let text = format!(
        "{:#}",
        project
            .digest_file_in(&root, "origin.bin")
            .expect_err("a hard-linked name is not exclusively owned")
    );
    assert!(text.contains("hard link"), "got: {text}");
}

#[test]
fn a_directory_at_the_name_refuses() {
    let (dir, project) = project();
    fs::create_dir(dir.path().join("subtree")).unwrap();
    let root = project.root_dir().unwrap();
    assert!(
        project.digest_file_in(&root, "subtree").is_err(),
        "a directory has no file content identity",
    );
}

#[test]
fn a_symlink_or_junction_at_the_name_refuses() {
    let (dir, project) = project();
    fs::write(dir.path().join("real.bin"), b"real").unwrap();
    if !link_to(&dir.path().join("real.bin"), &dir.path().join("link.bin")) {
        return; // Unprivileged Windows cannot plant one; covered where it can.
    }
    let root = project.root_dir().unwrap();
    assert!(
        project.digest_file_in(&root, "link.bin").is_err(),
        "a link is followed by nobody here, so it is refused rather than resolved",
    );
}

/// A traversal spelling never reaches the open: the component law refuses it
/// before a capability is asked anything.
#[test]
fn an_unsafe_component_refuses_before_opening() {
    let (_dir, project) = project();
    let root = project.root_dir().unwrap();
    for name in ["..", "a/b", ""] {
        assert!(
            project.digest_file_in(&root, name).is_err(),
            "`{name}` is not a direct safe component",
        );
    }
}

#[cfg(windows)]
fn link_to(target: &Path, link: &Path) -> bool {
    std::os::windows::fs::symlink_file(target, link).is_ok()
}

#[cfg(not(windows))]
fn link_to(target: &Path, link: &Path) -> bool {
    std::os::unix::fs::symlink(target, link).is_ok()
}

fn hex(bytes: &[u8; 32]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
