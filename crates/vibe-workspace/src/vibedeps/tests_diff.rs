//! Incremental slot-materialisation oracles (PROP-054 §9.3).

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, UNIX_EPOCH};

use tempfile::TempDir;
use vibe_core::manifest::SpecFormat;
use vibe_core::{ContentHash, Group};

use super::*;

#[cfg(unix)]
use std::os::unix::fs::symlink as symlink_dir;
#[cfg(windows)]
use std::os::windows::fs::symlink_dir;

fn group() -> Group {
    Group::parse("org.example").unwrap()
}

fn version() -> semver::Version {
    semver::Version::parse("1.0.0").unwrap()
}

fn source_hash(digit: char) -> ContentHash {
    ContentHash::parse(&format!("sha256:{}", digit.to_string().repeat(64))).unwrap()
}

fn write(root: &Path, rel: impl AsRef<Path>, body: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, body).unwrap();
}

fn slot(workspace: &Path, name: &str) -> PathBuf {
    slot_abs_path(workspace, &group(), name, &version())
}

#[cfg(windows)]
#[allow(clippy::permissions_set_readonly_false)]
fn make_fixture_writable(path: &Path) {
    // Windows deletion refuses a read-only file; this fixture deliberately
    // set the bit and must undo it before TempDir cleanup.
    let mut permissions = fs::metadata(path).unwrap().permissions();
    permissions.set_readonly(false);
    fs::set_permissions(path, permissions).unwrap();
}

#[test]
fn rematerialisation_is_a_diff_and_preserves_unrecorded_build_output() {
    let workspace = TempDir::new().unwrap();
    let source = TempDir::new().unwrap();
    write(source.path(), "stable.txt", "stable\n");
    write(source.path(), "changed.txt", "before\n");
    write(source.path(), "stale/deep/old.txt", "remove me\n");

    materialise(
        workspace.path(),
        &group(),
        "incremental",
        &version(),
        source.path(),
        &source_hash('1'),
    )
    .unwrap();
    let slot = slot(workspace.path(), "incremental");
    write(&slot, "target/probe.txt", "build output\n");
    fs::OpenOptions::new()
        .write(true)
        .open(slot.join("stable.txt"))
        .unwrap()
        .set_times(fs::FileTimes::new().set_modified(UNIX_EPOCH + Duration::from_secs(1_000_000)))
        .unwrap();
    let stable_before = fs::metadata(slot.join("stable.txt"))
        .unwrap()
        .modified()
        .unwrap();

    write(source.path(), "changed.txt", "after\n");
    fs::remove_file(source.path().join("stale/deep/old.txt")).unwrap();
    write(source.path(), "new.txt", "new\n");

    let footprint = materialise(
        workspace.path(),
        &group(),
        "incremental",
        &version(),
        source.path(),
        &source_hash('2'),
    )
    .unwrap();

    assert_eq!(
        fs::read_to_string(slot.join("target/probe.txt")).unwrap(),
        "build output\n",
        "paths outside the old record are never materialiser-owned"
    );
    assert_eq!(
        fs::metadata(slot.join("stable.txt"))
            .unwrap()
            .modified()
            .unwrap(),
        stable_before,
        "an unchanged recorded file must not be rewritten"
    );
    assert_eq!(
        fs::read_to_string(slot.join("changed.txt")).unwrap(),
        "after\n"
    );
    assert_eq!(fs::read_to_string(slot.join("new.txt")).unwrap(), "new\n");
    assert!(!slot.join("stale/deep/old.txt").exists());
    assert!(
        !slot.join("stale").exists(),
        "empty parents of removed recorded files are pruned best-effort"
    );
    assert_eq!(
        footprint,
        vec![
            PathBuf::from("changed.txt"),
            PathBuf::from("new.txt"),
            PathBuf::from("stable.txt")
        ]
    );
}

#[test]
fn malformed_record_refuses_before_touching_the_slot() {
    let workspace = TempDir::new().unwrap();
    let source = TempDir::new().unwrap();
    write(source.path(), "owned.txt", "before\n");
    materialise(
        workspace.path(),
        &group(),
        "malformed",
        &version(),
        source.path(),
        &source_hash('3'),
    )
    .unwrap();
    let slot = slot(workspace.path(), "malformed");
    write(&slot, "target/probe.txt", "keep me\n");
    fs::write(slot.join(SLOT_RECORD_FILENAME), "schema = 999\n").unwrap();
    write(source.path(), "owned.txt", "after\n");

    let error = materialise(
        workspace.path(),
        &group(),
        "malformed",
        &version(),
        source.path(),
        &source_hash('4'),
    )
    .unwrap_err();

    assert!(error.to_string().contains("slot record"), "{error}");
    assert_eq!(
        fs::read_to_string(slot.join("owned.txt")).unwrap(),
        "before\n"
    );
    assert_eq!(
        fs::read_to_string(slot.join("target/probe.txt")).unwrap(),
        "keep me\n"
    );
}

#[test]
fn replacing_a_recorded_hardlink_cannot_mutate_the_old_cache_inode() {
    let workspace = TempDir::new().unwrap();
    let source = TempDir::new().unwrap();
    let cache = TempDir::new().unwrap();
    write(source.path(), "payload.txt", "cached before\n");
    fs::hard_link(
        source.path().join("payload.txt"),
        cache.path().join("cache-copy.txt"),
    )
    .unwrap();
    materialise_with(
        workspace.path(),
        &group(),
        "hardlink",
        &version(),
        source.path(),
        CopyMode::Hardlink,
        &source_hash('5'),
    )
    .unwrap();

    // Replace the source path with a new inode; the cache keeps the old inode.
    fs::remove_file(source.path().join("payload.txt")).unwrap();
    write(source.path(), "payload.txt", "source after\n");
    let mut permissions = fs::metadata(source.path().join("payload.txt"))
        .unwrap()
        .permissions();
    permissions.set_readonly(true);
    fs::set_permissions(source.path().join("payload.txt"), permissions).unwrap();
    materialise_with(
        workspace.path(),
        &group(),
        "hardlink",
        &version(),
        source.path(),
        CopyMode::Hardlink,
        &source_hash('6'),
    )
    .unwrap();

    assert_eq!(
        fs::read_to_string(cache.path().join("cache-copy.txt")).unwrap(),
        "cached before\n",
        "copying over the old destination would truncate a cache hardlink"
    );
    assert_eq!(
        fs::read_to_string(slot(workspace.path(), "hardlink").join("payload.txt")).unwrap(),
        "source after\n"
    );
    assert!(
        fs::metadata(source.path().join("payload.txt"))
            .unwrap()
            .permissions()
            .readonly(),
        "placing a temporary hardlink must not normalise source/cache attributes"
    );
    #[cfg(windows)]
    make_fixture_writable(&source.path().join("payload.txt"));
}

#[test]
fn transformed_diff_removes_renamed_outputs_without_wiping_build_output() {
    let workspace = TempDir::new().unwrap();
    let source = TempDir::new().unwrap();
    write(
        source.path(),
        crate::layout_paths::specs_path("old.md"),
        "# Old\n\nBefore.\n",
    );
    materialise_with_spec_format(
        workspace.path(),
        &group(),
        "transformed-diff",
        &version(),
        source.path(),
        CopyMode::Copy,
        SpecFormat::Xml,
        &source_hash('7'),
    )
    .unwrap();
    let slot = slot(workspace.path(), "transformed-diff");
    write(&slot, "target/probe.txt", "build output\n");
    fs::remove_file(
        source
            .path()
            .join(crate::layout_paths::specs_path("old.md")),
    )
    .unwrap();
    write(
        source.path(),
        crate::layout_paths::specs_path("new.md"),
        "# New\n\nAfter.\n",
    );

    materialise_with_spec_format(
        workspace.path(),
        &group(),
        "transformed-diff",
        &version(),
        source.path(),
        CopyMode::Copy,
        SpecFormat::Xml,
        &source_hash('8'),
    )
    .unwrap();

    assert!(
        !slot
            .join(crate::layout_paths::specs_path("old.xml"))
            .exists()
    );
    assert!(
        slot.join(crate::layout_paths::specs_path("new.xml"))
            .is_file()
    );
    assert_eq!(
        fs::read_to_string(slot.join("target/probe.txt")).unwrap(),
        "build output\n"
    );
}

#[test]
fn different_unrecorded_collision_refuses_before_touching_recorded_files() {
    let workspace = TempDir::new().unwrap();
    let source = TempDir::new().unwrap();
    write(source.path(), "owned.txt", "before\n");
    materialise(
        workspace.path(),
        &group(),
        "collision",
        &version(),
        source.path(),
        &source_hash('9'),
    )
    .unwrap();
    let slot = slot(workspace.path(), "collision");
    write(&slot, "new.txt", "unrecorded\n");
    write(source.path(), "owned.txt", "after\n");
    write(source.path(), "new.txt", "incoming\n");

    let error = materialise(
        workspace.path(),
        &group(),
        "collision",
        &version(),
        source.path(),
        &source_hash('a'),
    )
    .unwrap_err();

    assert!(error.to_string().contains("unrecorded"), "{error}");
    assert_eq!(
        fs::read_to_string(slot.join("owned.txt")).unwrap(),
        "before\n"
    );
    assert_eq!(
        fs::read_to_string(slot.join("new.txt")).unwrap(),
        "unrecorded\n"
    );
}

#[test]
fn matching_unrecorded_file_is_adopted_after_an_interrupted_record_flip() {
    let workspace = TempDir::new().unwrap();
    let source = TempDir::new().unwrap();
    write(source.path(), "owned.txt", "owned\n");
    materialise(
        workspace.path(),
        &group(),
        "crash-heal",
        &version(),
        source.path(),
        &source_hash('b'),
    )
    .unwrap();
    let slot = slot(workspace.path(), "crash-heal");
    write(source.path(), "new.txt", "already placed\n");
    write(&slot, "new.txt", "already placed\n");
    fs::OpenOptions::new()
        .write(true)
        .open(slot.join("new.txt"))
        .unwrap()
        .set_times(fs::FileTimes::new().set_modified(UNIX_EPOCH + Duration::from_secs(2_000_000)))
        .unwrap();
    let before = fs::metadata(slot.join("new.txt"))
        .unwrap()
        .modified()
        .unwrap();

    materialise(
        workspace.path(),
        &group(),
        "crash-heal",
        &version(),
        source.path(),
        &source_hash('c'),
    )
    .unwrap();

    assert_eq!(
        fs::metadata(slot.join("new.txt"))
            .unwrap()
            .modified()
            .unwrap(),
        before
    );
    assert!(
        read_slot_record(&slot)
            .unwrap()
            .files
            .iter()
            .any(|file| file.path == "new.txt")
    );
}

#[test]
fn recorded_file_that_became_a_directory_is_a_hard_error() {
    let workspace = TempDir::new().unwrap();
    let source = TempDir::new().unwrap();
    write(source.path(), "owned.txt", "before\n");
    materialise(
        workspace.path(),
        &group(),
        "directory-collision",
        &version(),
        source.path(),
        &source_hash('d'),
    )
    .unwrap();
    let slot = slot(workspace.path(), "directory-collision");
    fs::remove_file(slot.join("owned.txt")).unwrap();
    write(&slot, "owned.txt/unrecorded.txt", "preserve\n");
    write(source.path(), "owned.txt", "after\n");

    let error = materialise(
        workspace.path(),
        &group(),
        "directory-collision",
        &version(),
        source.path(),
        &source_hash('e'),
    )
    .unwrap_err();

    assert!(error.to_string().contains("became a directory"), "{error}");
    assert_eq!(
        fs::read_to_string(slot.join("owned.txt/unrecorded.txt")).unwrap(),
        "preserve\n"
    );
}

#[test]
fn recorded_stale_subtree_can_be_replaced_by_an_incoming_file() {
    let workspace = TempDir::new().unwrap();
    let source = TempDir::new().unwrap();
    write(source.path(), "shape/leaf.txt", "old shape\n");
    materialise(
        workspace.path(),
        &group(),
        "shape-change",
        &version(),
        source.path(),
        &source_hash('f'),
    )
    .unwrap();
    let slot = slot(workspace.path(), "shape-change");
    write(&slot, "target/probe.txt", "preserve\n");
    fs::remove_dir_all(source.path().join("shape")).unwrap();
    write(source.path(), "shape", "new shape\n");

    materialise(
        workspace.path(),
        &group(),
        "shape-change",
        &version(),
        source.path(),
        &source_hash('0'),
    )
    .unwrap();

    assert_eq!(
        fs::read_to_string(slot.join("shape")).unwrap(),
        "new shape\n"
    );
    assert_eq!(
        fs::read_to_string(slot.join("target/probe.txt")).unwrap(),
        "preserve\n"
    );
}

#[test]
fn stale_recorded_path_cannot_escape_through_a_symlinked_parent() {
    let workspace = TempDir::new().unwrap();
    let source = TempDir::new().unwrap();
    let outside = TempDir::new().unwrap();
    write(source.path(), "owned/victim.txt", "recorded\n");
    write(outside.path(), "victim.txt", "outside\n");
    materialise(
        workspace.path(),
        &group(),
        "symlink-parent",
        &version(),
        source.path(),
        &source_hash('1'),
    )
    .unwrap();
    let slot = slot(workspace.path(), "symlink-parent");
    fs::remove_dir_all(slot.join("owned")).unwrap();
    if let Err(error) = symlink_dir(outside.path(), slot.join("owned")) {
        if error.kind() == std::io::ErrorKind::PermissionDenied
            || error.raw_os_error() == Some(1314)
        {
            return;
        }
        panic!("cannot create directory symlink fixture: {error}");
    }
    fs::remove_file(source.path().join("owned/victim.txt")).unwrap();

    let error = materialise(
        workspace.path(),
        &group(),
        "symlink-parent",
        &version(),
        source.path(),
        &source_hash('2'),
    )
    .unwrap_err();

    assert!(error.to_string().contains("symbolic link"), "{error}");
    assert_eq!(
        fs::read_to_string(outside.path().join("victim.txt")).unwrap(),
        "outside\n",
        "a recorded lexical path must never delete through a symlinked parent"
    );
}
