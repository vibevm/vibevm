//! End-to-end coverage for the read-side subcommands landed in
//! slice 4: get / list / search / capabilities / purls / outdated.

use std::path::Path;
use std::process::Command;

use assert_cmd::Command as AssertCommand;

fn cmd() -> AssertCommand {
    AssertCommand::cargo_bin("vibe-index").expect("vibe-index binary built")
}

fn git_available() -> bool {
    Command::new("git").arg("--version").output().is_ok()
}

fn manifest(name: &str, kind: &str, version: &str) -> String {
    format!(
        r#"[package]
group = "org.vibevm"
name = "{name}"
kind = "{kind}"
version = "{version}"
license = "EULA"
description = "test package {kind}:{name} version {version}"
keywords = ["{name}", "{kind}", "test"]

[provides]
capabilities = ["interface:{name}"]
"#
    )
}

fn manifest_with_describes(name: &str, kind: &str, version: &str, purl: &str) -> String {
    format!(
        r#"[package]
group = "org.vibevm"
name = "{name}"
kind = "{kind}"
version = "{version}"
license = "EULA"
description = "binds to {purl}"
describes = "{purl}"
"#
    )
}

fn make_repo(parent: &Path, dir_name: &str, manifests: &[(&str, &str)]) -> std::path::PathBuf {
    let repo = parent.join(dir_name);
    std::fs::create_dir_all(&repo).unwrap();
    git(&repo, &["init", "--quiet", "-b", "main"]);
    git(&repo, &["config", "user.email", "test@test.invalid"]);
    git(&repo, &["config", "user.name", "Test"]);
    for (tag, body) in manifests {
        std::fs::write(repo.join("vibe.toml"), body).unwrap();
        std::fs::write(repo.join("README.md"), format!("# {tag}\n")).unwrap();
        git(&repo, &["add", "."]);
        git(&repo, &["commit", "--quiet", "-m", tag]);
        git(&repo, &["tag", tag]);
    }
    repo
}

fn git(repo: &Path, args: &[&str]) {
    let status = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .status()
        .expect("git invokable");
    assert!(status.success(), "git {args:?} failed");
}

fn populated_index() -> Option<(tempfile::TempDir, std::path::PathBuf)> {
    if !git_available() {
        return None;
    }
    let work = tempfile::tempdir().unwrap();
    let org = work.path().join("org");
    std::fs::create_dir_all(&org).unwrap();

    make_repo(
        &org,
        "flow-wal",
        &[("v0.1.0", &manifest("wal", "flow", "0.1.0"))],
    );
    make_repo(
        &org,
        "stack-rust",
        &[
            ("v0.1.0", &manifest("rust", "stack", "0.1.0")),
            ("v0.2.0", &manifest("rust", "stack", "0.2.0")),
        ],
    );
    make_repo(
        &org,
        "flow-sqlx-skin",
        &[(
            "v0.1.0",
            &manifest_with_describes("sqlx-skin", "flow", "0.1.0", "pkg:cargo/sqlx@0.8.0"),
        )],
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
    Some((work, data))
}

#[test]
fn get_returns_versions_for_known_package() {
    let Some((_work, data)) = populated_index() else {
        return;
    };
    let out = cmd()
        .args([
            "get",
            data.to_str().unwrap(),
            "org.vibevm",
            "rust",
            "--json",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    let env: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(env["found"], true);
    assert_eq!(env["versions"].as_array().unwrap().len(), 2);
}

#[test]
fn get_specific_version_filters_correctly() {
    let Some((_work, data)) = populated_index() else {
        return;
    };
    let out = cmd()
        .args([
            "get",
            data.to_str().unwrap(),
            "org.vibevm",
            "rust",
            "--version",
            "0.2.0",
            "--json",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    let env: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(env["versions"].as_array().unwrap().len(), 1);
    assert_eq!(env["versions"][0]["version"], "0.2.0");
}

#[test]
fn get_unknown_package_text_form_errors() {
    let Some((_work, data)) = populated_index() else {
        return;
    };
    cmd()
        .args([
            "get",
            data.to_str().unwrap(),
            "org.vibevm",
            "definitely-absent",
        ])
        .assert()
        .failure();
}

#[test]
fn get_unknown_json_form_returns_found_false() {
    let Some((_work, data)) = populated_index() else {
        return;
    };
    let out = cmd()
        .args([
            "get",
            data.to_str().unwrap(),
            "org.vibevm",
            "definitely-absent",
            "--json",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    let env: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(env["found"], false);
    assert_eq!(env["versions"].as_array().unwrap().len(), 0);
}

#[test]
fn list_paginates_with_offset_and_limit() {
    let Some((_work, data)) = populated_index() else {
        return;
    };
    let out = cmd()
        .args([
            "list",
            data.to_str().unwrap(),
            "--limit",
            "1",
            "--offset",
            "1",
            "--json",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    let env: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(env["package_count"], 3);
    assert_eq!(env["packages"].as_array().unwrap().len(), 1);
}

#[test]
fn list_filters_by_kind() {
    let Some((_work, data)) = populated_index() else {
        return;
    };
    let out = cmd()
        .args(["list", data.to_str().unwrap(), "--kind", "stack", "--json"])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    let env: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(env["packages"].as_array().unwrap().len(), 1);
    assert_eq!(env["packages"][0]["kind"], "stack");
    assert_eq!(env["packages"][0]["name"], "rust");
}

#[test]
fn search_finds_matching_packages() {
    let Some((_work, data)) = populated_index() else {
        return;
    };
    let out = cmd()
        .args(["search", data.to_str().unwrap(), "rust", "--json"])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    let env: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let hits = env["hits"].as_array().unwrap();
    assert!(!hits.is_empty());
    assert!(hits.iter().any(|h| h["name"] == "rust"));
}

#[test]
fn search_no_match_returns_zero_hits() {
    let Some((_work, data)) = populated_index() else {
        return;
    };
    let out = cmd()
        .args(["search", data.to_str().unwrap(), "totallyabsent", "--json"])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    let env: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(env["hit_count"], 0);
}

#[test]
fn capabilities_lookup_returns_advertisers() {
    let Some((_work, data)) = populated_index() else {
        return;
    };
    let out = cmd()
        .args([
            "capabilities",
            data.to_str().unwrap(),
            "interface:wal",
            "--json",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    let env: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(env["hit_count"].as_u64().unwrap() >= 1);
    assert!(
        env["hits"]
            .as_array()
            .unwrap()
            .iter()
            .any(|h| h["name"] == "wal" && h["kind"] == "flow")
    );
}

#[test]
fn purls_lookup_returns_describing_packages() {
    let Some((_work, data)) = populated_index() else {
        return;
    };
    let out = cmd()
        .args([
            "purls",
            data.to_str().unwrap(),
            "pkg:cargo/sqlx@0.8.0",
            "--json",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    let env: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(env["hit_count"], 1);
    assert_eq!(env["hits"][0]["name"], "sqlx-skin");
    assert_eq!(env["hits"][0]["binding_site"], "package");
}

#[test]
fn outdated_flags_upgrade_candidates() {
    let Some((work, data)) = populated_index() else {
        return;
    };
    // Lockfile pinning rust@0.1.0 (older) and wal@0.1.0 (latest).
    let lock = work.path().join("vibe.lock");
    std::fs::write(
        &lock,
        r#"[meta]
schema_version = 5

[[package]]
kind = "flow"
group = "org.vibevm"
name = "wal"
version = "0.1.0"

[[package]]
kind = "stack"
group = "org.vibevm"
name = "rust"
version = "0.1.0"
"#,
    )
    .unwrap();
    let out = cmd()
        .args([
            "outdated",
            data.to_str().unwrap(),
            "--lockfile",
            lock.to_str().unwrap(),
            "--json",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    let env: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(env["update_available"], 1);
    let rows = env["rows"].as_array().unwrap();
    let rust_row = rows.iter().find(|r| r["name"] == "rust").expect("rust row");
    assert_eq!(rust_row["status"], "update-available");
    assert_eq!(rust_row["latest"], "0.2.0");
    let wal_row = rows.iter().find(|r| r["name"] == "wal").expect("wal row");
    assert_eq!(wal_row["status"], "up-to-date");
}

#[test]
fn outdated_unknown_packages_marked_unknown() {
    let Some((work, data)) = populated_index() else {
        return;
    };
    let lock = work.path().join("vibe.lock");
    std::fs::write(
        &lock,
        r#"[[package]]
kind = "flow"
group = "org.vibevm"
name = "ghost-pkg"
version = "0.1.0"
"#,
    )
    .unwrap();
    let out = cmd()
        .args([
            "outdated",
            data.to_str().unwrap(),
            "--lockfile",
            lock.to_str().unwrap(),
            "--json",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    let env: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(env["rows"][0]["status"], "unknown");
    assert_eq!(env["update_available"], 0);
}
