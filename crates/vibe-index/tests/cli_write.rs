//! End-to-end coverage of the write-side subcommands landed in
//! slice 6: add / remove.

use std::path::Path;

use assert_cmd::Command;
use specmark::verifies;
use vibe_index::index::Index;
use vibe_index::journal::{Event, JournalRecord, default_dir, project, replay};

fn cmd() -> Command {
    vibe_test_support::cargo_bin("vibe-index")
}

fn init_at(dir: &Path) {
    cmd()
        .args([
            "init",
            dir.to_str().unwrap(),
            "--registry",
            "vibespecs",
            "--registry-url",
            "https://example.invalid/vibespecs",
        ])
        .assert()
        .success();
}

fn write_pkg(
    dir: &Path,
    name: &str,
    kind: &str,
    version: &str,
    license: &str,
) -> std::path::PathBuf {
    let body = format!(
        r#"[package]
group = "org.vibevm"
name = "{name}"
kind = "{kind}"
version = "{version}"
license = "{license}"
description = "test {kind}:{name}@{version}"
"#
    );
    let path = dir.join("vibe.toml");
    std::fs::write(&path, body).unwrap();
    std::fs::write(dir.join("README.md"), format!("# {name}@{version}\n")).unwrap();
    path
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#cli", r = 1)]
fn add_inserts_entry_from_manifest() {
    let work = tempfile::tempdir().unwrap();
    let data = work.path().join("data");
    init_at(&data);

    let pkg_dir = work.path().join("pkg");
    std::fs::create_dir_all(&pkg_dir).unwrap();
    let manifest = write_pkg(&pkg_dir, "wal", "flow", "0.1.0", "EULA");

    cmd()
        .args([
            "add",
            data.to_str().unwrap(),
            "--manifest",
            manifest.to_str().unwrap(),
        ])
        .assert()
        .success();

    let by_name = data.join("by-name/wal.json");
    assert!(by_name.exists());
    let parsed: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&by_name).unwrap()).unwrap();
    let versions = &parsed["packages"][0]["versions"];
    assert_eq!(versions.as_array().unwrap().len(), 1);
    assert_eq!(versions[0]["name"], "wal");
    assert_eq!(versions[0]["license"], "EULA");
}

#[test]
fn add_upserts_when_version_already_present() {
    let work = tempfile::tempdir().unwrap();
    let data = work.path().join("data");
    init_at(&data);

    let pkg_dir = work.path().join("pkg");
    std::fs::create_dir_all(&pkg_dir).unwrap();
    let manifest = write_pkg(&pkg_dir, "wal", "flow", "0.1.0", "EULA");

    cmd()
        .args([
            "add",
            data.to_str().unwrap(),
            "--manifest",
            manifest.to_str().unwrap(),
        ])
        .assert()
        .success();
    cmd()
        .args([
            "add",
            data.to_str().unwrap(),
            "--manifest",
            manifest.to_str().unwrap(),
        ])
        .assert()
        .success();

    let by_name = data.join("by-name/wal.json");
    let parsed: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&by_name).unwrap()).unwrap();
    assert_eq!(
        parsed["packages"][0]["versions"].as_array().unwrap().len(),
        1
    );
}

#[test]
fn add_with_repo_url_overrides_default() {
    let work = tempfile::tempdir().unwrap();
    let data = work.path().join("data");
    init_at(&data);

    let pkg_dir = work.path().join("pkg");
    std::fs::create_dir_all(&pkg_dir).unwrap();
    let manifest = write_pkg(&pkg_dir, "wal", "flow", "0.1.0", "EULA");

    cmd()
        .args([
            "add",
            data.to_str().unwrap(),
            "--manifest",
            manifest.to_str().unwrap(),
            "--repo-url",
            "git@example.invalid:custom/path.git",
            "--ref",
            "release-0.1",
            "--commit",
            "deadbeefdeadbeef",
        ])
        .assert()
        .success();

    let by_name = data.join("by-name/wal.json");
    let parsed: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&by_name).unwrap()).unwrap();
    let entry = &parsed["packages"][0]["versions"][0];
    assert_eq!(entry["source_url"], "git@example.invalid:custom/path.git");
    assert_eq!(entry["source_ref"], "release-0.1");
    assert_eq!(entry["resolved_commit"], "deadbeefdeadbeef");
}

#[test]
fn remove_deletes_specific_version() {
    let work = tempfile::tempdir().unwrap();
    let data = work.path().join("data");
    init_at(&data);

    let pkg_dir = work.path().join("pkg");
    std::fs::create_dir_all(&pkg_dir).unwrap();
    let manifest = write_pkg(&pkg_dir, "wal", "flow", "0.1.0", "EULA");
    cmd()
        .args([
            "add",
            data.to_str().unwrap(),
            "--manifest",
            manifest.to_str().unwrap(),
        ])
        .assert()
        .success();
    write_pkg(&pkg_dir, "wal", "flow", "0.2.0", "EULA");
    cmd()
        .args([
            "add",
            data.to_str().unwrap(),
            "--manifest",
            manifest.to_str().unwrap(),
        ])
        .assert()
        .success();

    cmd()
        .args([
            "remove",
            data.to_str().unwrap(),
            "org.vibevm",
            "wal",
            "--version",
            "0.1.0",
        ])
        .assert()
        .success();

    let by_name = data.join("by-name/wal.json");
    let parsed: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&by_name).unwrap()).unwrap();
    let versions = &parsed["packages"][0]["versions"];
    assert_eq!(versions.as_array().unwrap().len(), 1);
    assert_eq!(versions[0]["version"], "0.2.0");
}

#[test]
fn remove_drops_entire_package_without_version_flag() {
    let work = tempfile::tempdir().unwrap();
    let data = work.path().join("data");
    init_at(&data);

    let pkg_dir = work.path().join("pkg");
    std::fs::create_dir_all(&pkg_dir).unwrap();
    let manifest = write_pkg(&pkg_dir, "wal", "flow", "0.1.0", "EULA");
    cmd()
        .args([
            "add",
            data.to_str().unwrap(),
            "--manifest",
            manifest.to_str().unwrap(),
        ])
        .assert()
        .success();

    cmd()
        .args(["remove", data.to_str().unwrap(), "org.vibevm", "wal"])
        .assert()
        .success();

    assert!(!data.join("by-name/wal.json").exists());
}

#[test]
fn remove_unknown_errors() {
    let work = tempfile::tempdir().unwrap();
    let data = work.path().join("data");
    init_at(&data);
    cmd()
        .args(["remove", data.to_str().unwrap(), "org.vibevm", "ghost"])
        .assert()
        .failure();
}

/// Ф3.2 — the journal is the truth: the records it holds, in order.
fn journal_records(data: &Path) -> Vec<JournalRecord> {
    replay(&default_dir(data)).unwrap()
}

/// The raw journal bytes — shard files concatenated in journal order.
/// Comparing before/after is the strongest form of "the journal did
/// not grow": no appended line, no rewritten line.
fn raw_journal(data: &Path) -> String {
    let dir = default_dir(data);
    let mut shards: Vec<std::path::PathBuf> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "ndjson"))
        .collect();
    shards.sort();
    let mut out = String::new();
    for shard in shards {
        out.push_str(&std::fs::read_to_string(&shard).unwrap());
    }
    out
}

fn add_pkg(work: &tempfile::TempDir, data: &Path, name: &str, version: &str) {
    let pkg_dir = work.path().join("pkg");
    std::fs::create_dir_all(&pkg_dir).unwrap();
    let manifest = write_pkg(&pkg_dir, name, "flow", version, "EULA");
    cmd()
        .args([
            "add",
            data.to_str().unwrap(),
            "--manifest",
            manifest.to_str().unwrap(),
        ])
        .assert()
        .success();
}

#[test]
fn add_appends_published_event_to_journal() {
    let work = tempfile::tempdir().unwrap();
    let data = work.path().join("data");
    init_at(&data);
    add_pkg(&work, &data, "wal", "0.1.0");

    let records = journal_records(&data);
    // The journal opens with the identity `init` wrote.
    assert!(
        matches!(records[0].event, Event::Initialised { .. }),
        "journal must open with the `Initialised` record from init, got {:?}",
        records[0].event
    );
    // ...and the mutation itself is a fact with the full coordinate.
    match &records[records.len() - 1].event {
        Event::Published { entry } => {
            assert_eq!(entry.group.to_string(), "org.vibevm");
            assert_eq!(entry.name, "wal");
            assert_eq!(entry.version.to_string(), "0.1.0");
        }
        other => panic!("expected `Published`, got {other:?}"),
    }
}

#[test]
fn remove_appends_removed_event_to_journal() {
    let work = tempfile::tempdir().unwrap();
    let data = work.path().join("data");
    init_at(&data);
    add_pkg(&work, &data, "wal", "0.1.0");

    cmd()
        .args([
            "remove",
            data.to_str().unwrap(),
            "org.vibevm",
            "wal",
            "--version",
            "0.1.0",
        ])
        .assert()
        .success();

    let records = journal_records(&data);
    match &records[records.len() - 1].event {
        Event::Removed {
            group,
            name,
            version,
        } => {
            assert_eq!(group.to_string(), "org.vibevm");
            assert_eq!(name, "wal");
            assert_eq!(
                version.as_ref().map(|v| v.to_string()),
                Some("0.1.0".to_string())
            );
        }
        other => panic!("expected `Removed`, got {other:?}"),
    }
}

#[test]
fn catalog_matches_the_journal_projection() {
    let work = tempfile::tempdir().unwrap();
    let data = work.path().join("data");
    init_at(&data);
    add_pkg(&work, &data, "wal", "0.1.0");
    add_pkg(&work, &data, "wal", "0.2.0");

    // The published catalog (read back the way any consumer reads it)
    // against a fold of the journal alone: the two must agree.
    let from_catalog = Index::load_from(&data).unwrap();
    let from_journal = project(journal_records(&data)).unwrap();

    assert_eq!(from_catalog.package_count(), from_journal.package_count());
    assert_eq!(from_catalog.version_count(), from_journal.version_count());
    let coordinates = |i: &Index| -> Vec<String> {
        i.iter_versions()
            .map(|e| format!("{}:{}/{}@{}", e.kind, e.group, e.name, e.version))
            .collect()
    };
    assert_eq!(
        coordinates(&from_catalog),
        coordinates(&from_journal),
        "by-name catalog and journal projection disagree"
    );
}

#[test]
fn remove_of_missing_target_appends_no_event() {
    let work = tempfile::tempdir().unwrap();
    let data = work.path().join("data");
    init_at(&data);
    add_pkg(&work, &data, "wal", "0.1.0");

    let before = raw_journal(&data);
    cmd()
        .args([
            "remove",
            data.to_str().unwrap(),
            "org.vibevm",
            "wal",
            "--version",
            "9.9.9",
        ])
        .assert()
        .failure();
    let after = raw_journal(&data);
    assert_eq!(
        before, after,
        "a refused removal must not grow the journal — a `Removed` record \
         for what never stood in the projection would be a fact that never \
         held"
    );
}
