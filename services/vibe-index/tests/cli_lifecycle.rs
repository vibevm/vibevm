//! End-to-end smoke for slice 2 — init / dump / verify exercised
//! through the binary against a temp data directory.

use std::path::Path;

use assert_cmd::Command;
use predicates::prelude::*;

fn cmd() -> Command {
    Command::cargo_bin("vibe-index").expect("vibe-index binary built")
}

#[test]
fn init_creates_repomd_and_empty_primary() {
    let dir = tempfile::tempdir().unwrap();
    cmd()
        .args([
            "init",
            dir.path().to_str().unwrap(),
            "--registry",
            "vibespecs",
            "--registry-url",
            "https://github.com/vibespecs",
            "--naming",
            "kind-name",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Initialised empty index"));

    assert_disk_has_files(dir.path(), &["repomd.json", "primary.jsonl"]);

    let repomd = std::fs::read_to_string(dir.path().join("repomd.json")).unwrap();
    assert!(repomd.contains("\"registry\": \"vibespecs\""));
    assert!(repomd.contains("\"naming\": \"kind-name\""));
    assert!(repomd.contains("\"package_count\": 0"));
}

#[test]
fn init_refuses_existing_index_without_force() {
    let dir = tempfile::tempdir().unwrap();
    let init_args = [
        "init",
        dir.path().to_str().unwrap(),
        "--registry",
        "vibespecs",
        "--registry-url",
        "https://github.com/vibespecs",
    ];
    cmd().args(init_args).assert().success();
    cmd()
        .args(init_args)
        .assert()
        .failure()
        .stderr(predicate::str::contains("already carries an index"));
    cmd().args(init_args).args(["--force"]).assert().success();
}

#[test]
fn dump_jsonl_emits_no_lines_for_empty_index() {
    let dir = tempfile::tempdir().unwrap();
    init_at(dir.path());
    let out = cmd()
        .args(["dump", dir.path().to_str().unwrap()])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    assert!(
        stdout.trim().is_empty(),
        "expected empty dump, got: {stdout}"
    );
}

#[test]
fn dump_json_emits_envelope_for_empty_index() {
    let dir = tempfile::tempdir().unwrap();
    init_at(dir.path());
    let out = cmd()
        .args(["dump", dir.path().to_str().unwrap(), "--format", "json"])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(parsed["registry"], "vibespecs");
    assert_eq!(parsed["package_count"], 0);
    assert_eq!(parsed["entries"].as_array().unwrap().len(), 0);
}

#[test]
fn verify_passes_on_freshly_initialised_index() {
    let dir = tempfile::tempdir().unwrap();
    init_at(dir.path());
    let out = cmd()
        .args(["verify", dir.path().to_str().unwrap(), "--json"])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(parsed["ok"], true);
    assert_eq!(parsed["mismatches"].as_array().unwrap().len(), 0);
    assert_eq!(parsed["missing"].as_array().unwrap().len(), 0);
}

#[test]
fn verify_fails_when_primary_jsonl_is_tampered() {
    let dir = tempfile::tempdir().unwrap();
    init_at(dir.path());
    // Tamper the file out from under the manifest.
    let primary = dir.path().join("primary.jsonl");
    std::fs::write(&primary, b"{\"i\":\"am-not-a-real-entry\"}\n").unwrap();
    let out = cmd()
        .args(["verify", dir.path().to_str().unwrap(), "--json"])
        .assert()
        .failure();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(parsed["ok"], false);
    assert!(!parsed["mismatches"].as_array().unwrap().is_empty());
}

#[test]
fn verify_text_format_human_readable() {
    let dir = tempfile::tempdir().unwrap();
    init_at(dir.path());
    cmd()
        .args(["verify", dir.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("registry"))
        .stdout(predicate::str::contains("status    : OK"));
}

#[test]
fn init_writes_primary_jsonl_gz_alongside_plain() {
    let dir = tempfile::tempdir().unwrap();
    init_at(dir.path());
    assert!(dir.path().join("primary.jsonl").exists());
    assert!(dir.path().join("primary.jsonl.gz").exists());
}

#[test]
fn init_seeds_empty_repomd_with_inverted_dirs() {
    let dir = tempfile::tempdir().unwrap();
    init_at(dir.path());
    let repomd: serde_json::Value =
        serde_json::from_slice(&std::fs::read(dir.path().join("repomd.json")).unwrap()).unwrap();
    let files = repomd["files"].as_object().unwrap();
    assert!(files.contains_key("primary.jsonl"));
    assert!(files.contains_key("primary.jsonl.gz"));
    assert!(files.contains_key("by-name"));
    assert!(files.contains_key("by-cap"));
    assert!(files.contains_key("by-purl"));
}

#[test]
fn init_writes_gitignore_and_readme() {
    let dir = tempfile::tempdir().unwrap();
    init_at(dir.path());
    let gi = std::fs::read_to_string(dir.path().join(".gitignore")).unwrap();
    assert!(gi.contains("/state/"));
    let readme = std::fs::read_to_string(dir.path().join("README.md")).unwrap();
    assert!(readme.contains("vibespecs"));
    assert!(readme.contains("primary.jsonl"));
    assert!(readme.contains("by-cap"));
    assert!(readme.contains("PROP-005"));
}

#[test]
fn init_preserves_existing_readme_on_force() {
    let dir = tempfile::tempdir().unwrap();
    init_at(dir.path());
    let custom = "# Custom README\n\nThis was hand-written by the operator.\n";
    std::fs::write(dir.path().join("README.md"), custom).unwrap();
    cmd()
        .args([
            "init",
            dir.path().to_str().unwrap(),
            "--registry",
            "vibespecs",
            "--registry-url",
            "https://example.invalid/vibespecs",
            "--force",
        ])
        .assert()
        .success();
    let readme = std::fs::read_to_string(dir.path().join("README.md")).unwrap();
    assert_eq!(
        readme, custom,
        "operator-edited README must survive --force"
    );
}

fn init_at(dir: &Path) {
    cmd()
        .args([
            "init",
            dir.to_str().unwrap(),
            "--registry",
            "vibespecs",
            "--registry-url",
            "https://github.com/vibespecs",
        ])
        .assert()
        .success();
}

fn assert_disk_has_files(root: &Path, names: &[&str]) {
    for name in names {
        let p = root.join(name);
        assert!(p.exists(), "expected `{}` to exist", p.display());
    }
}
