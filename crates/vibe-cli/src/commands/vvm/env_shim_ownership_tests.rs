//! Who owns a file at a shim's name — the two recognition tiers, the
//! refusal, and the case-insensitive collision Windows creates (asserted by
//! code, because the suite runs on Linux).

use super::*;

/// A POSIX shim exactly as `write_shims` generates it, marker and all.
const GENERATED_POSIX_SHIM: &str = "#!/bin/sh\n\
     # vibevm (VVM) shim — execs the active instance from ../vibevm/current.\n\
     exec \"$home/bin/vibe\" \"$@\"\n";

/// The `.cmd` shim's distinguishing line: it resolves vibe's own pointer.
const GENERATED_CMD_SHIM: &str = "@echo off\r\nset \"VVM_CUR=%~dp0..\\vibevm\\current\"\r\n";

const FOREIGN: &str = "#!/bin/sh\n# the operator's own wrapper\nexec /opt/other/vibe \"$@\"\n";

fn store_in(root: &Path) -> VersionStore {
    VersionStore::new(root.join("opt"))
}

fn write(path: &Path, text: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("creating the shim directory");
    }
    fs::write(path, text).expect("writing the fixture");
}

#[test]
fn a_name_nothing_occupies_is_absent() {
    let dir = tempfile::tempdir().expect("temp dir");
    let inventory = ShimInventory::empty();
    assert_eq!(
        inventory.classify("vibe", &dir.path().join("vibe")),
        ShimOwnership::Absent
    );
}

#[test]
fn the_exact_bytes_vibe_recorded_are_proven_its_own() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("vibe");
    write(&path, GENERATED_POSIX_SHIM);
    let mut inventory = ShimInventory::empty();
    inventory.record("vibe", GENERATED_POSIX_SHIM.as_bytes());
    assert_eq!(inventory.classify("vibe", &path), ShimOwnership::Recorded);

    // Edited since: no longer proven, but still recognisable by content.
    write(&path, &format!("{GENERATED_POSIX_SHIM}# mine\n"));
    assert_eq!(inventory.classify("vibe", &path), ShimOwnership::Recognised);
}

#[test]
fn a_shim_from_a_vibe_that_kept_no_inventory_is_recognised_by_content() {
    let dir = tempfile::tempdir().expect("temp dir");
    let posix = dir.path().join("vibe");
    let cmd = dir.path().join("vibe.cmd");
    write(&posix, GENERATED_POSIX_SHIM);
    write(&cmd, GENERATED_CMD_SHIM);
    let inventory = ShimInventory::empty();
    assert_eq!(
        inventory.classify("vibe", &posix),
        ShimOwnership::Recognised
    );
    assert_eq!(
        inventory.classify("vibe.cmd", &cmd),
        ShimOwnership::Recognised
    );
}

#[test]
fn a_foreign_file_at_a_shim_name_is_foreign_and_refused_naming_path_spec_and_fix() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("vibe");
    write(&path, FOREIGN);
    let inventory = ShimInventory::empty();
    assert_eq!(inventory.classify("vibe", &path), ShimOwnership::Foreign);

    let error = inventory
        .refuse_foreign(dir.path(), "vibe")
        .expect_err("a foreign file must refuse the write");
    let message = error.to_string();
    assert!(
        message.contains(&path.display().to_string()),
        "names the file: {message}"
    );
    assert!(
        message.contains("is not a file vibe wrote"),
        "says whose it is not: {message}"
    );
    assert!(
        message.contains("spec://org.vibevm.core/vibevm/common/PROP-019#layout"),
        "cites the requirement: {message}"
    );
    assert!(message.contains("fix: "), "offers a fix: {message}");

    // The refusal did not touch it.
    assert_eq!(fs::read_to_string(&path).expect("still there"), FOREIGN);
}

#[test]
fn a_recorded_name_whose_file_was_replaced_wholesale_is_foreign() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("vibe");
    let mut inventory = ShimInventory::empty();
    inventory.record("vibe", GENERATED_POSIX_SHIM.as_bytes());
    write(&path, FOREIGN);
    assert_eq!(inventory.classify("vibe", &path), ShimOwnership::Foreign);
}

#[test]
fn a_directory_or_an_oversized_file_at_a_shim_name_is_foreign() {
    let dir = tempfile::tempdir().expect("temp dir");
    let as_dir = dir.path().join("vibe");
    fs::create_dir(&as_dir).expect("directory at a shim name");
    let inventory = ShimInventory::empty();
    assert_eq!(inventory.classify("vibe", &as_dir), ShimOwnership::Foreign);

    let big = dir.path().join("vibe-index");
    let padding = "x".repeat(SHIM_MAX_BYTES as usize + 1);
    write(&big, &format!("{GENERATED_POSIX_SHIM}{padding}"));
    assert_eq!(
        inventory.classify("vibe-index", &big),
        ShimOwnership::Foreign
    );
}

#[cfg(unix)]
#[test]
fn a_symlink_at_a_shim_name_is_foreign() {
    let dir = tempfile::tempdir().expect("temp dir");
    let target = dir.path().join("elsewhere");
    write(&target, GENERATED_POSIX_SHIM);
    let link = dir.path().join("vibe");
    std::os::unix::fs::symlink(&target, &link).expect("symlink at a shim name");
    let inventory = ShimInventory::empty();
    assert_eq!(inventory.classify("vibe", &link), ShimOwnership::Foreign);
}

#[test]
fn a_case_variant_spelling_only_collides_where_names_are_case_insensitive() {
    let dir = tempfile::tempdir().expect("temp dir");
    let variant = dir.path().join("VIBE.CMD");
    write(&variant, FOREIGN);

    // Windows: the filesystem resolves `vibe.cmd` AS that file, so it is
    // the one classified before any write.
    let insensitive =
        resolving_entries(dir.path(), "vibe.cmd", true).expect("reading the shim directory");
    assert_eq!(insensitive, vec![variant.clone()]);

    // POSIX: `vibe.cmd` is a different name and the variant is untouched.
    let sensitive =
        resolving_entries(dir.path(), "vibe.cmd", false).expect("reading the shim directory");
    assert_eq!(sensitive, vec![dir.path().join("vibe.cmd")]);
}

#[test]
fn an_absent_name_still_resolves_to_itself() {
    let dir = tempfile::tempdir().expect("temp dir");
    for case_insensitive in [false, true] {
        let entries = resolving_entries(dir.path(), "vibe", case_insensitive)
            .expect("reading the shim directory");
        assert_eq!(entries, vec![dir.path().join("vibe")]);
    }
}

#[test]
fn the_inventory_round_trips_through_the_data_directory() {
    let root = tempfile::tempdir().expect("temp dir");
    let store = store_in(root.path());
    let mut inventory = ShimInventory::empty();
    inventory.record("vibe", GENERATED_POSIX_SHIM.as_bytes());
    inventory.record("vibe-index", GENERATED_POSIX_SHIM.as_bytes());
    save(&store, &inventory).expect("saving the inventory");

    let stored = fs::read_to_string(inventory_path(&store)).expect("reading the inventory");
    assert!(stored.contains("schema = 1"), "{stored}");
    assert!(stored.contains("name = \"vibe\""), "{stored}");

    let loaded = load(&store).expect("loading the inventory");
    assert_eq!(
        loaded.recorded_digest("vibe"),
        Some(digest_of(GENERATED_POSIX_SHIM.as_bytes()).as_str())
    );
    assert_eq!(loaded.recorded_digest("vibe-tree"), None);
}

#[test]
fn recording_a_name_twice_keeps_one_row_with_the_latest_digest() {
    let mut inventory = ShimInventory::empty();
    inventory.record("vibe", b"first");
    inventory.record("vibe", GENERATED_POSIX_SHIM.as_bytes());
    assert_eq!(inventory.shims.len(), 1);
    assert_eq!(
        inventory.recorded_digest("vibe"),
        Some(digest_of(GENERATED_POSIX_SHIM.as_bytes()).as_str())
    );
}

#[test]
fn a_damaged_or_missing_inventory_claims_nothing() {
    let root = tempfile::tempdir().expect("temp dir");
    let store = store_in(root.path());
    assert_eq!(load(&store).expect("no inventory yet").shims.len(), 0);

    let path = inventory_path(&store);
    write(&path, "this is not toml {{{\n");
    let loaded = load(&store).expect("a damaged inventory is not fatal");
    assert_eq!(loaded.recorded_digest("vibe"), None);

    // Content recognition still allows a rewrite, so an unreadable
    // inventory never blocks an upgrade.
    let shim_dir = root.path().join("opt").join("bin");
    write(&shim_dir.join("vibe"), GENERATED_POSIX_SHIM);
    assert!(loaded.refuse_foreign(&shim_dir, "vibe").is_ok());
}
