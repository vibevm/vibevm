//! End-to-end scan + reindex flow. Builds a fake org-dir of git
//! repositories programmatically, runs `vibe-index init` + `reindex
//! --from-clones`, then asserts the on-disk index records every
//! version, with the right metadata.

use std::path::Path;
use std::process::Command;

use assert_cmd::Command as AssertCommand;

fn cmd() -> AssertCommand {
    AssertCommand::cargo_bin("vibe-index").expect("vibe-index binary built")
}

fn git_available() -> bool {
    Command::new("git").arg("--version").output().is_ok()
}

fn manifest_for(name: &str, kind: &str, version: &str, license: Option<&str>) -> String {
    let mut s = format!(
        "[package]\nname = \"{name}\"\nkind = \"{kind}\"\nversion = \"{version}\"\n"
    );
    if let Some(l) = license {
        s.push_str(&format!("license = \"{l}\"\n"));
    }
    s
}

fn init_repo(repo: &Path) {
    fs_must_create(repo);
    git(repo, &["init", "--quiet", "-b", "main"]);
    git(repo, &["config", "user.email", "test@test.invalid"]);
    git(repo, &["config", "user.name", "Test"]);
}

fn commit_and_tag(repo: &Path, manifest: &str, tag: &str) {
    std::fs::write(repo.join("vibe-package.toml"), manifest).unwrap();
    std::fs::write(repo.join("README.md"), format!("# {tag}\n")).unwrap();
    git(repo, &["add", "."]);
    git(repo, &["commit", "--quiet", "-m", tag]);
    git(repo, &["tag", tag]);
}

fn git(repo: &Path, args: &[&str]) {
    let status = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .status()
        .expect("git invokable");
    assert!(
        status.success(),
        "git {:?} failed in {}",
        args,
        repo.display()
    );
}

fn fs_must_create(p: &Path) {
    std::fs::create_dir_all(p).expect("can create dir");
}

#[test]
fn reindex_from_clones_walks_three_packages() {
    if !git_available() {
        return;
    }
    let work = tempfile::tempdir().unwrap();
    let org = work.path().join("vibespecs-org");
    fs_must_create(&org);

    let wal = org.join("flow-wal");
    init_repo(&wal);
    commit_and_tag(
        &wal,
        &manifest_for("wal", "flow", "0.1.0", Some("EULA")),
        "v0.1.0",
    );

    let commits = org.join("flow-atomic-commits");
    init_repo(&commits);
    commit_and_tag(
        &commits,
        &manifest_for("atomic-commits", "flow", "0.1.0", Some("EULA")),
        "v0.1.0",
    );

    let rust = org.join("stack-rust");
    init_repo(&rust);
    commit_and_tag(
        &rust,
        &manifest_for("rust", "stack", "0.1.0", Some("EULA")),
        "v0.1.0",
    );
    commit_and_tag(
        &rust,
        &manifest_for("rust", "stack", "0.2.0", Some("EULA")),
        "v0.2.0",
    );

    let stranger = org.join("not-a-vibevm-package");
    std::fs::create_dir_all(&stranger).unwrap();
    std::fs::write(stranger.join("README.md"), "hello\n").unwrap();

    let data = work.path().join("index-data");
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
    assert_eq!(summary["package_count"], 3);
    assert_eq!(summary["version_count"], 4);
    let by_kind = summary["by_kind"].as_array().unwrap();
    let flow_count = by_kind
        .iter()
        .find(|kc| kc["kind"] == "flow")
        .unwrap()["count"]
        .as_u64()
        .unwrap();
    assert_eq!(flow_count, 2);
    let stack_count = by_kind
        .iter()
        .find(|kc| kc["kind"] == "stack")
        .unwrap()["count"]
        .as_u64()
        .unwrap();
    assert_eq!(stack_count, 1);
    let skipped = summary["skipped"].as_array().unwrap();
    assert!(
        skipped.iter().any(|s| s["repo"] == "not-a-vibevm-package"),
        "expected `not-a-vibevm-package` to be skipped, got {skipped:?}"
    );

    let primary = std::fs::read_to_string(data.join("primary.jsonl")).unwrap();
    assert_eq!(
        primary.lines().count(),
        4,
        "primary.jsonl content was: {primary}"
    );

    assert!(data.join("by-name/flow/wal.json").exists());
    assert!(data.join("by-name/flow/atomic-commits.json").exists());
    assert!(data.join("by-name/stack/rust.json").exists());

    let rust_json = std::fs::read_to_string(data.join("by-name/stack/rust.json")).unwrap();
    let rust: serde_json::Value = serde_json::from_str(&rust_json).unwrap();
    assert_eq!(rust["latest_stable"], "0.2.0");
    assert_eq!(rust["versions"].as_array().unwrap().len(), 2);

    cmd()
        .args(["verify", data.to_str().unwrap()])
        .assert()
        .success();
}

#[test]
fn reindex_skips_non_v_semver_tags() {
    if !git_available() {
        return;
    }
    let work = tempfile::tempdir().unwrap();
    let org = work.path().join("org");
    fs_must_create(&org);

    let repo = org.join("flow-wal");
    init_repo(&repo);
    commit_and_tag(
        &repo,
        &manifest_for("wal", "flow", "0.1.0", None),
        "v0.1.0",
    );
    git(&repo, &["tag", "release-candidate"]);

    let data = work.path().join("index-data");
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
    assert_eq!(summary["version_count"], 1);
    let skipped = summary["skipped"].as_array().unwrap();
    assert!(
        skipped
            .iter()
            .any(|s| s["tag"] == "release-candidate"
                && s["reason"]
                    .as_str()
                    .unwrap()
                    .contains("not a `v<semver>`")),
        "expected `release-candidate` skip note, got {skipped:?}"
    );
}

#[test]
fn reindex_text_output_lists_skipped_entries() {
    if !git_available() {
        return;
    }
    let work = tempfile::tempdir().unwrap();
    let org = work.path().join("org");
    std::fs::create_dir_all(&org).unwrap();
    let stranger = org.join("not-a-package");
    std::fs::create_dir_all(&stranger).unwrap();

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
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    assert!(stdout.contains("packages  : 0"));
    assert!(stdout.contains("not-a-package"));
}

#[test]
fn reindex_preserves_registry_metadata_from_init() {
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
            "vibespecs-gitverse",
            "--registry-url",
            "https://gitverse.ru/vibespecs",
            "--naming",
            "name",
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

    let repomd: serde_json::Value =
        serde_json::from_slice(&std::fs::read(data.join("repomd.json")).unwrap()).unwrap();
    assert_eq!(repomd["registry"], "vibespecs-gitverse");
    assert_eq!(repomd["naming"], "name");
}
