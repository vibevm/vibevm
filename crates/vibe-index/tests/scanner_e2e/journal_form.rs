//! The journal-form half of the scanner's end-to-end tests — what
//! `reindex` asserts once the catalog is a projection rather than a
//! thing the driver reads back (Ф3.2c3).
//!
//! Out of line for the 600-line file budget, by the crate's own idiom:
//! the parent declares it with `#[cfg(test)] #[path = …] mod`, so the
//! module-tree position is unchanged and `use super::*` still reaches
//! the fixtures — one `git_available` / `cmd` / `init_repo` /
//! `commit_and_tag` / `manifest_for` set, not a second copy.

use super::*;

/// Successor of `reindex_preserves_foreign_schema_version_of_read_catalog`
/// (F2-2's reindex half), retired with the Ф3.2c3 journal form: a
/// reindex no longer READS the catalog, so there is no read version to
/// preserve — the projection births the catalog from the journal and
/// stamps this build's own constant. Not a loss of protection: the
/// forged future-version catalog below is exactly the input a
/// journal-form reindex must ignore, and the guard against a
/// future-version world moved a floor up, into the journal's epoch and
/// each record's `must_understand` (PROP-044 §4.5). What this successor
/// pins is the new rule: whatever foreign version lies on disk, the
/// rewritten manifest carries THIS build's constant.
#[test]
fn reindex_rewrites_catalog_at_this_builds_schema_version() {
    use vibe_index::index::repomd;
    use vibe_index::types::Repomd;

    if !git_available() {
        return;
    }
    let work = tempfile::tempdir().unwrap();
    let org = work.path().join("org");
    std::fs::create_dir_all(&org).unwrap();
    let data = work.path().join("data");

    cmd()
        .args([
            "init",
            data.to_str().unwrap(),
            "--registry",
            "vibespecs",
            "--registry-url",
            "https://example.invalid/vibespecs",
        ])
        .assert()
        .success();

    // Pass the catalog off as a FUTURE writer's product: bump the
    // manifest's schema_version above ours, touch nothing else.
    let foreign = Repomd::SCHEMA_VERSION + 1;
    let manifest_path = data.join("repomd.json");
    let mut manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&manifest_path).unwrap()).unwrap();
    manifest["schema_version"] = serde_json::json!(foreign);
    std::fs::write(&manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();

    cmd()
        .args([
            "reindex",
            data.to_str().unwrap(),
            "--from-clones",
            org.to_str().unwrap(),
            "--full",
        ])
        .assert()
        .success();

    let rewritten = repomd::read(&data).unwrap();
    assert_eq!(
        rewritten.schema_version,
        Repomd::SCHEMA_VERSION,
        "a journal-form reindex never read the catalog, so the rewritten manifest carries this build's constant — the future-version guard lives in the journal epoch / must_understand, not in a read-back"
    );
}

/// Ф3.2c3 — `--full` differs from `--incremental` by exactly one event:
/// the `EntrySetReplaced` watershed. It must be the run's FIRST fact,
/// so the publications of the same run stand after it — a watershed
/// appended after the entries it clears would describe a catalog those
/// entries never populated.
#[test]
fn full_reindex_journals_the_watershed_before_its_publications() {
    use vibe_index::journal::{Event, default_dir, replay};

    if !git_available() {
        return;
    }
    let work = tempfile::tempdir().unwrap();
    let org = work.path().join("org");
    fs_must_create(&org);
    let wal = org.join("org.vibevm.wal");
    init_repo(&wal);
    commit_and_tag(&wal, &manifest_for("wal", "flow", "0.1.0", None), "v0.1.0");

    let data = work.path().join("data");
    cmd()
        .args([
            "init",
            data.to_str().unwrap(),
            "--registry",
            "vibespecs",
            "--registry-url",
            "https://example.invalid/vibespecs",
        ])
        .assert()
        .success();
    cmd()
        .args([
            "reindex",
            data.to_str().unwrap(),
            "--from-clones",
            org.to_str().unwrap(),
            "--full",
        ])
        .assert()
        .success();

    let records = replay(&default_dir(&data)).unwrap();
    let watershed = records
        .iter()
        .position(|r| matches!(r.event, Event::EntrySetReplaced { .. }))
        .expect("a --full run must journal an EntrySetReplaced");
    match &records[watershed].event {
        Event::EntrySetReplaced { source } => assert_eq!(
            source, "clones",
            "the watershed names the scan source the Plan carried"
        ),
        other => panic!("expected Event::EntrySetReplaced, got {other:?}"),
    }
    let publications: Vec<usize> = records
        .iter()
        .enumerate()
        .filter(|(_, r)| matches!(r.event, Event::Published { .. }))
        .map(|(i, _)| i)
        .collect();
    assert_eq!(
        publications,
        vec![watershed + 1],
        "the run's publications must stand AFTER the watershed: {records:?}"
    );
}

/// Ф3.2c3 — `--incremental` never writes the watershed: the scan's
/// result is a DIFF against the journal, not the whole entry set, so
/// an `EntrySetReplaced` here would clear entries the same run has no
/// replacement for.
#[test]
fn incremental_reindex_journals_no_watershed() {
    use vibe_index::journal::{Event, default_dir, replay};

    if !git_available() {
        return;
    }
    let work = tempfile::tempdir().unwrap();
    let org = work.path().join("org");
    fs_must_create(&org);
    let wal = org.join("org.vibevm.wal");
    init_repo(&wal);
    commit_and_tag(&wal, &manifest_for("wal", "flow", "0.1.0", None), "v0.1.0");

    let data = work.path().join("data");
    cmd()
        .args([
            "init",
            data.to_str().unwrap(),
            "--registry",
            "vibespecs",
            "--registry-url",
            "https://example.invalid/vibespecs",
        ])
        .assert()
        .success();
    cmd()
        .args([
            "reindex",
            data.to_str().unwrap(),
            "--from-clones",
            org.to_str().unwrap(),
            "--full",
        ])
        .assert()
        .success();

    // The org moves, so the incremental run has something to publish —
    // the claim is not "incremental appends nothing" but "it appends
    // no watershed".
    commit_and_tag(&wal, &manifest_for("wal", "flow", "0.2.0", None), "v0.2.0");
    let before = replay(&default_dir(&data)).unwrap();
    cmd()
        .args([
            "reindex",
            data.to_str().unwrap(),
            "--from-clones",
            org.to_str().unwrap(),
            "--incremental",
        ])
        .assert()
        .success();
    let after = replay(&default_dir(&data)).unwrap();

    let watersheds = |records: &[vibe_index::journal::JournalRecord]| {
        records
            .iter()
            .filter(|r| matches!(r.event, Event::EntrySetReplaced { .. }))
            .count()
    };
    assert_eq!(
        watersheds(&before),
        watersheds(&after),
        "an incremental run must not append EntrySetReplaced"
    );
    for record in &after[before.len()..] {
        assert!(
            matches!(record.event, Event::Published { .. }),
            "an incremental run appends only Published facts, got {record:?}"
        );
    }
}

/// Ф3.2c3 — the watershed is a real reset, not a label: a package that
/// stood in the catalog and vanished from the scan leaves the catalog
/// after a `--full` run. This is the pre-journal observable behaviour
/// (a full rebuild started from an empty index) and the journal form
/// must keep it — via the fold clearing `by_pkgref`, not via a fresh
/// in-memory Index.
#[test]
fn full_reindex_watershed_drops_entries_vanished_from_the_scan() {
    if !git_available() {
        return;
    }
    let work = tempfile::tempdir().unwrap();
    let org = work.path().join("org");
    fs_must_create(&org);

    let wal = org.join("org.vibevm.wal");
    init_repo(&wal);
    commit_and_tag(&wal, &manifest_for("wal", "flow", "0.1.0", None), "v0.1.0");

    let gone = org.join("org.vibevm.gone");
    init_repo(&gone);
    commit_and_tag(
        &gone,
        &manifest_for("gone", "flow", "0.1.0", None),
        "v0.1.0",
    );

    let data = work.path().join("data");
    cmd()
        .args([
            "init",
            data.to_str().unwrap(),
            "--registry",
            "vibespecs",
            "--registry-url",
            "https://example.invalid/vibespecs",
        ])
        .assert()
        .success();
    let out = cmd()
        .args([
            "reindex",
            data.to_str().unwrap(),
            "--from-clones",
            org.to_str().unwrap(),
            "--full",
            "--json",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    let summary: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(summary["package_count"], 2);

    // The repo leaves the org entirely — the next full scan cannot
    // see it, and the watershed must clear what it used to publish.
    std::fs::remove_dir_all(&gone).unwrap();

    let out = cmd()
        .args([
            "reindex",
            data.to_str().unwrap(),
            "--from-clones",
            org.to_str().unwrap(),
            "--full",
            "--json",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    let summary: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(summary["package_count"], 1);
    assert_eq!(summary["version_count"], 1);
    assert!(
        !data.join("by-name/gone.json").exists(),
        "a vanished package's by-name file must not survive --full"
    );
    let primary = std::fs::read_to_string(data.join("primary.jsonl")).unwrap();
    assert_eq!(
        primary.lines().count(),
        1,
        "primary.jsonl content was: {primary}"
    );
}

/// Ф3.2c3 — the retention the `kept_unchanged` block used to provide
/// now comes from the journal itself: a package whose repo the
/// incremental scan skipped as unchanged stays in the catalog because
/// its `Published` record still stands in the journal, with no merge
/// step in the driver. The red proof for this test is the watershed:
/// make `--incremental` append `EntrySetReplaced` too and the fold
/// clears the entry with nothing to replace it.
#[test]
fn incremental_keeps_entries_the_scanner_did_not_rewalk() {
    if !git_available() {
        return;
    }
    let work = tempfile::tempdir().unwrap();
    let org = work.path().join("org");
    fs_must_create(&org);

    let wal = org.join("org.vibevm.wal");
    init_repo(&wal);
    commit_and_tag(&wal, &manifest_for("wal", "flow", "0.1.0", None), "v0.1.0");

    let held = org.join("org.vibevm.held");
    init_repo(&held);
    commit_and_tag(
        &held,
        &manifest_for("held", "flow", "0.1.0", None),
        "v0.1.0",
    );

    let data = work.path().join("data");
    cmd()
        .args([
            "init",
            data.to_str().unwrap(),
            "--registry",
            "vibespecs",
            "--registry-url",
            "https://example.invalid/vibespecs",
        ])
        .assert()
        .success();
    cmd()
        .args([
            "reindex",
            data.to_str().unwrap(),
            "--from-clones",
            org.to_str().unwrap(),
            "--full",
        ])
        .assert()
        .success();
    assert!(data.join("state/checkpoint.json").exists());

    // Only `wal` moves; `held`'s snapshot matches the checkpoint, so
    // the incremental scan does not re-walk it — the skip note says
    // so, and the entry must survive regardless.
    commit_and_tag(&wal, &manifest_for("wal", "flow", "0.2.0", None), "v0.2.0");
    let out = cmd()
        .args([
            "reindex",
            data.to_str().unwrap(),
            "--from-clones",
            org.to_str().unwrap(),
            "--incremental",
            "--json",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    let summary: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(summary["package_count"], 2);
    assert_eq!(summary["version_count"], 3);
    let held_json: serde_json::Value =
        serde_json::from_slice(&std::fs::read(data.join("by-name/held.json")).unwrap()).unwrap();
    assert_eq!(
        held_json["packages"][0]["versions"]
            .as_array()
            .unwrap()
            .len(),
        1,
        "the unscanned package's entry must survive --incremental via its journal record"
    );
}
