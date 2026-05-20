//! End-to-end coverage of the write-side subcommands landed in
//! slice 6: add / remove.

use std::path::Path;

use assert_cmd::Command;

fn cmd() -> Command {
    Command::cargo_bin("vibe-index").expect("vibe-index binary built")
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

fn write_pkg(dir: &Path, name: &str, kind: &str, version: &str, license: &str) -> std::path::PathBuf {
    let body = format!(
        r#"[package]
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

    let by_name = data.join("by-name/flow/wal.json");
    assert!(by_name.exists());
    let parsed: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&by_name).unwrap()).unwrap();
    assert_eq!(parsed["versions"].as_array().unwrap().len(), 1);
    assert_eq!(parsed["versions"][0]["name"], "wal");
    assert_eq!(parsed["versions"][0]["license"], "EULA");
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
        .args(["add", data.to_str().unwrap(), "--manifest", manifest.to_str().unwrap()])
        .assert()
        .success();
    cmd()
        .args(["add", data.to_str().unwrap(), "--manifest", manifest.to_str().unwrap()])
        .assert()
        .success();

    let by_name = data.join("by-name/flow/wal.json");
    let parsed: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&by_name).unwrap()).unwrap();
    assert_eq!(parsed["versions"].as_array().unwrap().len(), 1);
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

    let by_name = data.join("by-name/flow/wal.json");
    let parsed: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&by_name).unwrap()).unwrap();
    let entry = &parsed["versions"][0];
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
        .args(["add", data.to_str().unwrap(), "--manifest", manifest.to_str().unwrap()])
        .assert()
        .success();
    write_pkg(&pkg_dir, "wal", "flow", "0.2.0", "EULA");
    cmd()
        .args(["add", data.to_str().unwrap(), "--manifest", manifest.to_str().unwrap()])
        .assert()
        .success();

    cmd()
        .args([
            "remove",
            data.to_str().unwrap(),
            "flow",
            "wal",
            "--version",
            "0.1.0",
        ])
        .assert()
        .success();

    let by_name = data.join("by-name/flow/wal.json");
    let parsed: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&by_name).unwrap()).unwrap();
    assert_eq!(parsed["versions"].as_array().unwrap().len(), 1);
    assert_eq!(parsed["versions"][0]["version"], "0.2.0");
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
        .args(["add", data.to_str().unwrap(), "--manifest", manifest.to_str().unwrap()])
        .assert()
        .success();

    cmd()
        .args(["remove", data.to_str().unwrap(), "flow", "wal"])
        .assert()
        .success();

    assert!(!data.join("by-name/flow/wal.json").exists());
}

#[test]
fn remove_unknown_errors() {
    let work = tempfile::tempdir().unwrap();
    let data = work.path().join("data");
    init_at(&data);
    cmd()
        .args(["remove", data.to_str().unwrap(), "flow", "ghost"])
        .assert()
        .failure();
}
