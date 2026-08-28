//! One bounded verify law for both safefs publications: the replacing writer
//! and the create-new writer prove the visible bytes are the candidate under
//! the candidate's own length cap, never an unbounded read of whatever the
//! name holds when the read runs.
//!
//! The publication lands, and the very next step re-opens the destination to
//! prove the visible bytes are the candidate. That re-open is a read of a
//! name any racer can replace in the window between the two steps — so its
//! allocation must be bounded by what the caller wrote, not by what the
//! replacement reports. The armed hook fires in exactly that window (after
//! the rename for the replacing writer, after the link and stage collection
//! for the create-new writer): what it leaves at the name is what
//! verification must judge, on the candidate's own byte budget.
//!
//! For the replacing writer these tests pin all three arms of the verify
//! through the bounded read — exact success (including the zero-length
//! candidate, which is the zero cap), same-size and shorter mismatches,
//! absence — and the hostile case the ceiling exists for. The create-new
//! writer then proves the same law through its own call site: a hostile
//! larger replacement refusing on the candidate budget (with the create-new
//! residue fact — no surviving stage), and a mismatch that length alone
//! cannot acquit. Exact create-new success is already pinned in
//! `create_new_tests.rs`. The hostile payload in every RED **starts with**
//! the candidate and runs on, so a prefix-presenting reader would call it
//! success — the one outcome these tests forbid.

use std::fs;
use std::path::Path;

use crate::{Project, PublishStage};

fn project() -> (tempfile::TempDir, Project) {
    let dir = tempfile::tempdir().unwrap();
    let project = Project::open(dir.path()).unwrap();
    (dir, project)
}

/// The candidate every test here publishes: ordinary prose-shaped bytes, long
/// enough that truncation and case games are distinguishable.
const CANDIDATE: &[u8] = b"the candidate bytes";

/// No `.vibe-stage-*` file survives any outcome. The prefix is the crate's
/// own, so a leftover is always ours and always a bug.
fn stages_in(path: &Path) -> Vec<String> {
    fs::read_dir(path)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| name.starts_with(crate::STAGE_PREFIX))
        .collect()
}

/// The ordinary success, pinned: exact candidate bytes verify, land whole,
/// and collect the stage. The zero-length candidate is the zero cap, and an
/// empty publication is the whole answer under it — not a zero-cap refusal.
#[test]
fn the_replacing_writer_verifies_exact_candidate_bytes() {
    let (dir, project) = project();
    let holder = project.dir(&["holder"], true).unwrap();

    project
        .write_atomic_in(&holder, "index.json", CANDIDATE)
        .expect("exact candidate bytes verify");
    assert_eq!(
        fs::read(dir.path().join("holder/index.json")).unwrap(),
        CANDIDATE
    );

    project
        .write_atomic_in(&holder, "empty.json", b"")
        .expect("an empty candidate verifies as the zero cap");
    assert_eq!(fs::read(dir.path().join("holder/empty.json")).unwrap(), b"");
    assert!(stages_in(&dir.path().join("holder")).is_empty());
}

/// The hostile case the ceiling exists for. In the verify window the
/// destination is replaced by a payload that starts with the exact candidate
/// bytes and runs on for a megabyte — so a verifier that read only a
/// cap-sized prefix and compared would answer **success**, which is the one
/// outcome this test forbids. The bounded reader refuses on the metadata
/// before allocating or reading anything past the candidate budget, the
/// publication reports `PossiblyPublished`, and the diagnostic carries the
/// three numbers a caller needs: the destination, the real length, and the
/// cap the candidate itself set.
#[test]
fn a_larger_replacement_in_the_verify_window_refuses_on_the_candidate_budget() {
    let (dir, project) = project();
    let holder = project.dir(&["holder"], true).unwrap();
    let root = dir.path().to_path_buf();

    let mut hostile = CANDIDATE.to_vec();
    hostile.extend(std::iter::repeat_n(b'X', 1 << 20));
    let hostile_len = hostile.len();
    crate::arm_before_publish_verify(Some(Box::new(move |_, name| {
        std::fs::write(root.join("holder").join(name), &hostile).unwrap();
    })));
    let outcome = project.write_atomic_in(&holder, "index.json", CANDIDATE);
    crate::arm_before_publish_verify(None);

    let error = outcome.expect_err("a prefix of the replacement is not the candidate");
    assert_eq!(
        error.stage,
        PublishStage::PossiblyPublished,
        "the rename crossed the irreversible line; only verification refused"
    );
    let rendered = format!("{error:#}");
    assert!(
        rendered.contains("index.json"),
        "the destination: {rendered}"
    );
    assert!(
        rendered.contains(&format!("is {hostile_len} bytes")),
        "the replacement's real length: {rendered}"
    );
    assert!(
        rendered.contains(&format!("over the {}-byte cap", CANDIDATE.len())),
        "the cap the candidate itself set: {rendered}"
    );
    // The refusal is the over-cap verdict, not a mismatch that read the
    // foreign payload first — and not a success that compared a prefix.
    assert!(
        !rendered.contains("do not match the staged bytes"),
        "the unbounded-read verdict would mean the foreign payload was read: {rendered}"
    );
    assert_eq!(
        fs::metadata(dir.path().join("holder/index.json"))
            .unwrap()
            .len() as usize,
        hostile_len,
        "the refusal remediated nothing — the replacement is exactly what was left",
    );
    assert!(
        stages_in(&dir.path().join("holder")).is_empty(),
        "the rename consumed the stage before the window opened"
    );
}

/// Length alone cannot acquit a replacement: same-size different bytes and a
/// shorter payload both mismatch through the same branch, because the cap is
/// an allocation ceiling, not a content test.
#[test]
fn same_size_and_shorter_replacements_in_the_verify_window_mismatch() {
    let (dir, project) = project();
    let holder = project.dir(&["holder"], true).unwrap();

    let same_size = dir.path().to_path_buf();
    crate::arm_before_publish_verify(Some(Box::new(move |_, name| {
        std::fs::write(same_size.join("holder").join(name), b"THE CANDIDATE BYTES").unwrap();
    })));
    let outcome = project.write_atomic_in(&holder, "index.json", CANDIDATE);
    crate::arm_before_publish_verify(None);

    let error = outcome.expect_err("same length, different bytes is a mismatch");
    assert_eq!(error.stage, PublishStage::PossiblyPublished);
    assert!(
        format!("{error:#}").contains("do not match the staged bytes"),
        "the mismatch branch, reached through the bounded read"
    );

    let shorter = dir.path().to_path_buf();
    crate::arm_before_publish_verify(Some(Box::new(move |_, name| {
        std::fs::write(shorter.join("holder").join(name), b"fewer bytes").unwrap();
    })));
    let outcome = project.write_atomic_in(&holder, "index.json", CANDIDATE);
    crate::arm_before_publish_verify(None);

    let error = outcome.expect_err("fewer bytes than the candidate is a mismatch");
    assert_eq!(error.stage, PublishStage::PossiblyPublished);
    assert!(
        format!("{error:#}").contains("do not match the staged bytes"),
        "a shorter file reads whole under the cap and still mismatches"
    );
}

/// The third arm of the verify: a destination that vanished inside the window
/// is reported absent, not as success and not as a read error.
#[test]
fn a_destination_removed_in_the_verify_window_reports_absence() {
    let (dir, project) = project();
    let holder = project.dir(&["holder"], true).unwrap();
    let root = dir.path().to_path_buf();

    crate::arm_before_publish_verify(Some(Box::new(move |_, name| {
        std::fs::remove_file(root.join("holder").join(name)).unwrap();
    })));
    let outcome = project.write_atomic_in(&holder, "index.json", CANDIDATE);
    crate::arm_before_publish_verify(None);

    let error = outcome.expect_err("a vanished destination cannot verify");
    assert_eq!(error.stage, PublishStage::PossiblyPublished);
    assert!(
        format!("{error:#}").contains("absent immediately after publication"),
        "absence stays absence under a cap"
    );
}

/// The create-new writer under the same hostile case, through its own call
/// site: the hook fires after the link landed and the stage was collected, so
/// what it replaces is the freshly claimed archive entry. The refusal keeps
/// the create-new residue fact — no staging name survives beside the entry —
/// and everything else matches the replacing writer's verdict: the stage is
/// `PossiblyPublished` and the diagnostic carries the destination, the real
/// length and the candidate-set cap.
#[test]
fn a_larger_replacement_after_a_create_new_publication_refuses_on_the_candidate_budget() {
    let (dir, project) = project();
    let archive = project.dir(&["archive"], true).unwrap();
    let root = dir.path().to_path_buf();

    let mut hostile = CANDIDATE.to_vec();
    hostile.extend(std::iter::repeat_n(b'X', 1 << 20));
    let hostile_len = hostile.len();
    crate::arm_before_publish_verify(Some(Box::new(move |_, name| {
        std::fs::write(root.join("archive").join(name), &hostile).unwrap();
    })));
    let outcome = project.publish_new_in(&archive, "0000.json", CANDIDATE);
    crate::arm_before_publish_verify(None);

    let error = outcome.expect_err("a prefix of the replacement is not the candidate");
    assert_eq!(
        error.stage,
        PublishStage::PossiblyPublished,
        "the link crossed the irreversible line; only verification refused"
    );
    let rendered = format!("{error:#}");
    assert!(
        rendered.contains("0000.json"),
        "the destination: {rendered}"
    );
    assert!(
        rendered.contains(&format!("is {hostile_len} bytes")),
        "the replacement's real length: {rendered}"
    );
    assert!(
        rendered.contains(&format!("over the {}-byte cap", CANDIDATE.len())),
        "the cap the candidate itself set: {rendered}"
    );
    assert!(
        !rendered.contains("do not match the staged bytes"),
        "the unbounded-read verdict would mean the foreign payload was read: {rendered}"
    );
    assert_eq!(
        fs::metadata(dir.path().join("archive/0000.json"))
            .unwrap()
            .len() as usize,
        hostile_len,
        "the refusal remediated nothing — the replacement is exactly what was left",
    );
    assert!(
        stages_in(&dir.path().join("archive")).is_empty(),
        "the stage was collected before the window; the verify refusal adds no residue"
    );
}

/// And the create-new mismatch: length alone cannot acquit a replacement,
/// here through the archive path's own verify. Same-size different bytes and a
/// shorter payload both mismatch through the same branch the replacing writer
/// uses.
#[test]
fn same_size_and_shorter_replacements_after_a_create_new_publication_mismatch() {
    let (dir, project) = project();
    let archive = project.dir(&["archive"], true).unwrap();

    let same_size = dir.path().to_path_buf();
    crate::arm_before_publish_verify(Some(Box::new(move |_, name| {
        std::fs::write(same_size.join("archive").join(name), b"THE CANDIDATE BYTES").unwrap();
    })));
    let outcome = project.publish_new_in(&archive, "0000.json", CANDIDATE);
    crate::arm_before_publish_verify(None);

    let error = outcome.expect_err("same length, different bytes is a mismatch");
    assert_eq!(error.stage, PublishStage::PossiblyPublished);
    assert!(
        format!("{error:#}").contains("do not match the staged bytes"),
        "the mismatch branch, reached through the bounded read"
    );

    let shorter = dir.path().to_path_buf();
    crate::arm_before_publish_verify(Some(Box::new(move |_, name| {
        std::fs::write(shorter.join("archive").join(name), b"fewer bytes").unwrap();
    })));
    let outcome = project.publish_new_in(&archive, "0001.json", CANDIDATE);
    crate::arm_before_publish_verify(None);

    let error = outcome.expect_err("fewer bytes than the candidate is a mismatch");
    assert_eq!(error.stage, PublishStage::PossiblyPublished);
    assert!(
        format!("{error:#}").contains("do not match the staged bytes"),
        "a shorter file reads whole under the cap and still mismatches"
    );
}
