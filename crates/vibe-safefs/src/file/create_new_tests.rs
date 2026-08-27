//! Create-new publication: every way a destination can already be taken, and
//! the two injected faults on either side of the one irreversible step.

use std::fs;
use std::path::Path;

use crate::{Project, PublishStage};

fn project() -> (tempfile::TempDir, Project) {
    let dir = tempfile::tempdir().unwrap();
    let project = Project::open(dir.path()).unwrap();
    (dir, project)
}

/// No `.vibe-stage-*` file survives any outcome. The prefix is the crate's
/// own, so a leftover is always ours and always a bug.
fn stages_in(path: &Path) -> Vec<String> {
    fs::read_dir(path)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| name.starts_with(crate::STAGE_PREFIX))
        .collect()
}

#[test]
fn a_fresh_name_is_published_whole_and_leaves_no_stage() {
    let (dir, project) = project();
    let archive = project.dir(&["archive"], true).unwrap();
    project
        .publish_new_in(&archive, "0000.json", b"{}\n")
        .expect("a fresh name publishes");
    assert_eq!(
        fs::read_to_string(dir.path().join("archive/0000.json")).unwrap(),
        "{}\n"
    );
    assert!(stages_in(&dir.path().join("archive")).is_empty());
    let (_, len) = project
        .inspect_file_in(&archive, "0000.json")
        .unwrap()
        .expect("the published file has a proof and a length");
    assert_eq!(len, 3);
    assert!(
        project
            .inspect_file_in(&archive, "absent.json")
            .unwrap()
            .is_none()
    );
}

/// An existing regular file is the case a replacing publication would
/// destroy. It refuses, the bytes stay, and nothing is left behind.
#[test]
fn an_existing_file_is_never_replaced() {
    let (dir, project) = project();
    let archive = project.dir(&["archive"], true).unwrap();
    fs::write(dir.path().join("archive/0000.json"), "original").unwrap();

    let error = project
        .publish_new_in(&archive, "0000.json", b"replacement")
        .expect_err("an occupied name refuses");
    assert_eq!(error.stage, PublishStage::BeforePublication);
    assert_eq!(
        fs::read_to_string(dir.path().join("archive/0000.json")).unwrap(),
        "original",
        "the refusal wrote nothing",
    );
    assert!(stages_in(&dir.path().join("archive")).is_empty());
}

/// A directory at the destination is not a file this call may claim.
#[test]
fn a_directory_at_the_destination_refuses() {
    let (dir, project) = project();
    let archive = project.dir(&["archive"], true).unwrap();
    fs::create_dir(dir.path().join("archive/0000.json")).unwrap();

    let error = project
        .publish_new_in(&archive, "0000.json", b"payload")
        .expect_err("a directory occupies the name");
    assert_eq!(error.stage, PublishStage::BeforePublication);
    assert!(dir.path().join("archive/0000.json").is_dir());
    assert!(stages_in(&dir.path().join("archive")).is_empty());
}

/// Another name of the same inode. The destination itself does not exist as
/// far as this call is concerned — the collision is the *hard link* — so this
/// is the case that proves the link step, not the pre-check, is the authority.
#[test]
fn a_hard_linked_destination_refuses_and_keeps_both_names() {
    let (dir, project) = project();
    let archive = project.dir(&["archive"], true).unwrap();
    fs::write(dir.path().join("archive/original.json"), "shared").unwrap();
    if fs::hard_link(
        dir.path().join("archive/original.json"),
        dir.path().join("archive/0000.json"),
    )
    .is_err()
    {
        // Some filesystems refuse hard links; the collision is then
        // unreachable and the plain-file case above already covers it.
        return;
    }
    let error = project
        .publish_new_in(&archive, "0000.json", b"replacement")
        .expect_err("a second name of an existing file is still an occupant");
    assert_eq!(error.stage, PublishStage::BeforePublication);
    assert_eq!(
        fs::read_to_string(dir.path().join("archive/original.json")).unwrap(),
        "shared",
    );
    assert_eq!(
        fs::read_to_string(dir.path().join("archive/0000.json")).unwrap(),
        "shared",
    );
    assert!(stages_in(&dir.path().join("archive")).is_empty());
}

/// A symlink at the destination is never followed to write through it. Where
/// the host will not create one (unprivileged Windows) the case is simply
/// unreachable and the test says so instead of pretending to prove it.
#[test]
fn a_symlinked_destination_is_never_followed() {
    let (dir, project) = project();
    let archive = project.dir(&["archive"], true).unwrap();
    fs::write(dir.path().join("outside.json"), "outside").unwrap();
    if !link_file(
        &dir.path().join("outside.json"),
        &dir.path().join("archive/0000.json"),
    ) {
        return;
    }
    let error = project
        .publish_new_in(&archive, "0000.json", b"through the link")
        .expect_err("a link occupies the name");
    assert_eq!(error.stage, PublishStage::BeforePublication);
    assert_eq!(
        fs::read_to_string(dir.path().join("outside.json")).unwrap(),
        "outside",
        "nothing was written through the link",
    );
    assert!(stages_in(&dir.path().join("archive")).is_empty());
}

#[cfg(unix)]
fn link_file(target: &Path, link: &Path) -> bool {
    std::os::unix::fs::symlink(target, link).is_ok()
}

#[cfg(windows)]
fn link_file(target: &Path, link: &Path) -> bool {
    std::os::windows::fs::symlink_file(target, link).is_ok()
}

/// **Which step is the authority.** The preflight ran and said the name was
/// free; the stage is written and synced; and only *then* does somebody plant
/// an ordinary file at the final name. Nothing re-checks after that — so if
/// the publication succeeded here, the preflight would have been the
/// authority and `hard_link` merely decorative.
///
/// It refuses. The planted bytes survive exactly, our owned stage is
/// collected, and no payload was linked over or written through.
#[test]
fn an_occupant_planted_after_the_preflight_still_refuses_at_the_link() {
    let (dir, project) = project();
    let archive = project.dir(&["archive"], true).unwrap();
    let planted_root = dir.path().to_path_buf();

    crate::arm_before_link(Some(Box::new(move |_, name| {
        // The window between "the name was free" and "claim the name".
        std::fs::write(
            planted_root.join("archive").join(name),
            "SOMEBODY ELSE'S FILE",
        )
        .unwrap();
    })));
    let outcome = project.publish_new_in(&archive, "0000.json", b"our payload");
    crate::arm_before_link(None);

    let error = outcome.expect_err("the link is what refuses, and it did");
    assert_eq!(
        error.stage,
        PublishStage::BeforePublication,
        "a refused link leaves the destination untouched",
    );
    assert_eq!(
        fs::read_to_string(dir.path().join("archive/0000.json")).unwrap(),
        "SOMEBODY ELSE'S FILE",
        "the planted bytes are exactly as planted",
    );
    assert!(
        stages_in(&dir.path().join("archive")).is_empty(),
        "and our own stage was collected",
    );
    assert_eq!(
        fs::read_dir(dir.path().join("archive")).unwrap().count(),
        1,
        "nothing else was left beside it",
    );
}

/// The link hook is single-shot and always disarmed, so the very next
/// publication in the same directory succeeds — the refusal above is a real
/// discrimination, not a permanently broken path.
#[test]
fn the_link_hook_fires_once_and_leaves_nothing_armed() {
    let (dir, project) = project();
    let archive = project.dir(&["archive"], true).unwrap();
    let planted_root = dir.path().to_path_buf();

    crate::arm_before_link(Some(Box::new(move |_, name| {
        std::fs::write(planted_root.join("archive").join(name), "planted").unwrap();
    })));
    assert!(project.publish_new_in(&archive, "0000.json", b"x").is_err());
    project
        .publish_new_in(&archive, "0001.json", b"y")
        .expect("the hook fired once");
    crate::arm_before_link(None);
    assert_eq!(
        fs::read_to_string(dir.path().join("archive/0001.json")).unwrap(),
        "y"
    );
}

/// The provably-invisible branch: the stage is written and synced, then the
/// publication fails before the link. The destination never existed and the
/// stage is collected.
#[test]
fn an_injected_pre_publication_fault_leaves_nothing_at_all() {
    let (dir, project) = project();
    let archive = project.dir(&["archive"], true).unwrap();

    crate::fail_before_publish(Some("0000.json"));
    let outcome = project.publish_new_in(&archive, "0000.json", b"payload");
    crate::fail_before_publish(None);

    let error = outcome.expect_err("the injected fault refuses");
    assert_eq!(error.stage, PublishStage::BeforePublication);
    assert!(!dir.path().join("archive/0000.json").exists());
    assert!(stages_in(&dir.path().join("archive")).is_empty());

    // Disarmed, the very same call succeeds — so the fault is a real
    // discrimination and not a permanently broken path.
    project
        .publish_new_in(&archive, "0000.json", b"payload")
        .expect("the disarmed publication succeeds");
}

/// The other side of the one irreversible step: the file IS on disk and the
/// call still fails. The caller must be told it may be published, because it
/// is.
#[test]
fn an_injected_post_publication_fault_reports_the_visible_file() {
    let (dir, project) = project();
    let archive = project.dir(&["archive"], true).unwrap();

    crate::fail_after_publish(Some("0000.json"));
    let outcome = project.publish_new_in(&archive, "0000.json", b"payload");
    crate::fail_after_publish(None);

    let error = outcome.expect_err("the injected fault refuses");
    assert_eq!(error.stage, PublishStage::PossiblyPublished);
    assert_eq!(
        fs::read_to_string(dir.path().join("archive/0000.json")).unwrap(),
        "payload",
        "the file really is published, which is what the stage reports",
    );
    assert!(stages_in(&dir.path().join("archive")).is_empty());
}

/// The injections are keyed by name, so an unrelated publication in the same
/// directory is untouched by an armed one.
#[test]
fn the_injections_are_inert_for_other_names() {
    let (dir, project) = project();
    let archive = project.dir(&["archive"], true).unwrap();

    crate::fail_before_publish(Some("0000.json"));
    crate::fail_after_publish(Some("0000.json"));
    let outcome = project.publish_new_in(&archive, "0001.json", b"payload");
    crate::fail_before_publish(None);
    crate::fail_after_publish(None);

    outcome.expect("a different name is not the armed one");
    assert_eq!(
        fs::read_to_string(dir.path().join("archive/0001.json")).unwrap(),
        "payload"
    );
}

/// The name is one component, never a path: an archive entry that could name
/// a subdirectory — or an ancestor — would be an archive that escapes itself.
#[test]
fn only_a_single_safe_component_may_be_published() {
    let (dir, project) = project();
    let archive = project.dir(&["archive"], true).unwrap();
    for name in ["nested/0000.json", "..", ".", "", "a\\b", ".vibe-stage-1-0"] {
        let outcome = project.publish_new_in(&archive, name, b"payload");
        assert!(outcome.is_err(), "{name:?} must refuse");
        assert_eq!(
            outcome.unwrap_err().stage,
            PublishStage::BeforePublication,
            "{name:?} is refused before anything is staged",
        );
    }
    assert!(stages_in(&dir.path().join("archive")).is_empty());
    assert!(!dir.path().join("archive/nested").exists());
}

/// The crash-shaped window the publication cannot undo: the `hard_link`
/// landed and the owned stage was not collected. Both names exist, they are
/// one inode, and the caller is told `PossiblyPublished` — because the payload
/// really is published, just not exclusively owned.
///
/// This is the case a probe-based accounting misses: `inspect_file_in`
/// correctly refuses a two-named file, so a caller that asked "is a payload
/// there?" would be told "no" while a full payload sits on the disk.
#[test]
fn a_failed_stage_cleanup_leaves_two_names_of_one_payload() {
    let (dir, project) = project();
    let archive = project.dir(&["archive"], true).unwrap();

    crate::fail_before_stage_cleanup(Some("0000.json"));
    let outcome = project.publish_new_in(&archive, "0000.json", b"the payload");
    crate::fail_before_stage_cleanup(None);

    let error = outcome.expect_err("the injected cleanup failure refuses");
    assert_eq!(
        error.stage,
        PublishStage::PossiblyPublished,
        "the irreversible step was crossed",
    );

    let final_name = dir.path().join("archive/0000.json");
    assert_eq!(
        fs::read(&final_name).unwrap(),
        b"the payload",
        "the payload really is at the final name",
    );
    let stages = stages_in(&dir.path().join("archive"));
    assert_eq!(
        stages.len(),
        1,
        "and the stage survives beside it: {stages:?}"
    );
    assert_eq!(
        fs::read(dir.path().join("archive").join(&stages[0])).unwrap(),
        b"the payload",
        "both names are the same bytes",
    );

    // Exactly the probe that must NOT be the accounting gate: a two-named
    // file is not exclusively owned, so it reports no proof at all.
    assert!(
        project.inspect_file_in(&archive, "0000.json").is_err(),
        "a probe cannot see this payload, which is why it cannot be the gate",
    );

    // One inode, two names — where the host reports link counts at all.
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let a = fs::metadata(&final_name).unwrap();
        let b = fs::metadata(dir.path().join("archive").join(&stages[0])).unwrap();
        assert_eq!(a.ino(), b.ino(), "one inode");
        assert_eq!(a.nlink(), 2, "under two names");
    }
}

/// A genuine BEFORE-publication failure of the replacing writer, so a caller
/// that stages an index and cannot land it is told nothing was published.
#[test]
fn an_injected_pre_publication_fault_also_covers_the_replacing_writer() {
    let (dir, project) = project();
    let holder = project.dir(&["holder"], true).unwrap();
    fs::write(dir.path().join("holder/index.json"), "the old whole index").unwrap();

    crate::fail_before_publish(Some("index.json"));
    let outcome = project.write_atomic_in(&holder, "index.json", b"the new index");
    crate::fail_before_publish(None);

    let error = outcome.expect_err("the injected fault refuses");
    assert_eq!(error.stage, PublishStage::BeforePublication);
    assert_eq!(
        fs::read_to_string(dir.path().join("holder/index.json")).unwrap(),
        "the old whole index",
        "the previous whole file is still the only claim",
    );
    assert!(stages_in(&dir.path().join("holder")).is_empty());
}
