//! End-to-end scan + reindex flow. Builds a fake org-dir of git
//! repositories programmatically, runs `vibe-index init` + `reindex
//! --from-clones`, then asserts the on-disk index records every
//! version, with the right metadata.

use std::path::Path;
use std::process::Command;

use assert_cmd::Command as AssertCommand;
use specmark::verifies;

fn cmd() -> AssertCommand {
    AssertCommand::cargo_bin("vibe-index").expect("vibe-index binary built")
}

fn git_available() -> bool {
    Command::new("git").arg("--version").output().is_ok()
}

fn manifest_for(name: &str, kind: &str, version: &str, license: Option<&str>) -> String {
    let mut s = format!(
        "[package]\ngroup = \"org.vibevm\"\nname = \"{name}\"\nkind = \"{kind}\"\nversion = \"{version}\"\n"
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
    std::fs::write(repo.join("vibe.toml"), manifest).unwrap();
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

/// Drives the `from-clones` cell directly through the `PackageScanner`
/// seam — the characterization oracle a cell replacement diffs against
/// (the binary-level reindex tests below exercise the same walk
/// end-to-end through the composition root).
#[test]
#[verifies("spec://vibevm/modules/vibe-index/PROP-005#reindex", r = 1)]
fn from_clones_cell_scans_through_the_seam() {
    use vibe_index::scanner::{FromClonesOptions, FromClonesScanner, PackageScanner};
    use vibe_index::types::NamingConvention;

    if !git_available() {
        return;
    }
    let work = tempfile::tempdir().unwrap();
    let org = work.path().join("org");
    fs_must_create(&org);
    let wal = org.join("org.vibevm.wal");
    init_repo(&wal);
    commit_and_tag(
        &wal,
        &manifest_for("wal", "flow", "0.1.0", Some("EULA")),
        "v0.1.0",
    );

    let scanner = FromClonesScanner {
        org_dir: org.clone(),
    };
    let opts = FromClonesOptions {
        registry: "vibespecs".into(),
        registry_url: "https://github.com/vibespecs".into(),
        naming: NamingConvention::Fqdn,
        generator: "oracle".into(),
        indexed_at: chrono::Utc::now(),
    };
    let seam: &dyn PackageScanner = &scanner;
    let report = seam.scan(&opts, None).unwrap();

    assert_eq!(report.entries.len(), 1, "{:?}", report.skipped);
    let entry = &report.entries[0];
    assert_eq!(entry.name, "wal");
    assert_eq!(entry.group.to_string(), "org.vibevm");
    assert_eq!(entry.version.to_string(), "0.1.0");
    assert!(
        report.snapshots.contains_key("org.vibevm.wal"),
        "walked repos record a checkpoint snapshot: {:?}",
        report.snapshots.keys().collect::<Vec<_>>()
    );
}

#[test]
#[verifies("spec://vibevm/modules/vibe-index/PROP-005#reindex", r = 1)]
fn reindex_from_clones_walks_three_packages() {
    if !git_available() {
        return;
    }
    let work = tempfile::tempdir().unwrap();
    let org = work.path().join("vibespecs-org");
    fs_must_create(&org);

    let wal = org.join("org.vibevm.wal");
    init_repo(&wal);
    commit_and_tag(
        &wal,
        &manifest_for("wal", "flow", "0.1.0", Some("EULA")),
        "v0.1.0",
    );

    let commits = org.join("org.vibevm.atomic-commits");
    init_repo(&commits);
    commit_and_tag(
        &commits,
        &manifest_for("atomic-commits", "flow", "0.1.0", Some("EULA")),
        "v0.1.0",
    );

    let rust = org.join("org.vibevm.rust");
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
    let flow_count = by_kind.iter().find(|kc| kc["kind"] == "flow").unwrap()["count"]
        .as_u64()
        .unwrap();
    assert_eq!(flow_count, 2);
    let stack_count = by_kind.iter().find(|kc| kc["kind"] == "stack").unwrap()["count"]
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

    assert!(data.join("by-name/wal.json").exists());
    assert!(data.join("by-name/atomic-commits.json").exists());
    assert!(data.join("by-name/rust.json").exists());

    let rust_json = std::fs::read_to_string(data.join("by-name/rust.json")).unwrap();
    let rust: serde_json::Value = serde_json::from_str(&rust_json).unwrap();
    let rust_pkg = &rust["packages"][0];
    assert_eq!(rust_pkg["latest_stable"], "0.2.0");
    assert_eq!(rust_pkg["versions"].as_array().unwrap().len(), 2);

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

    let repo = org.join("org.vibevm.wal");
    init_repo(&repo);
    commit_and_tag(&repo, &manifest_for("wal", "flow", "0.1.0", None), "v0.1.0");
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
        skipped.iter().any(|s| s["tag"] == "release-candidate"
            && s["reason"].as_str().unwrap().contains("not a `v<semver>`")),
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
#[verifies("spec://vibevm/modules/vibe-index/PROP-005#reindex", r = 1)]
fn incremental_skips_unchanged_repos_and_picks_up_new_tags() {
    if !git_available() {
        return;
    }
    let work = tempfile::tempdir().unwrap();
    let org = work.path().join("org");
    fs_must_create(&org);

    let wal = org.join("org.vibevm.wal");
    init_repo(&wal);
    commit_and_tag(
        &wal,
        &manifest_for("wal", "flow", "0.1.0", Some("EULA")),
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

    // First full run.
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

    // Incremental run with no changes — should skip the repo as
    // unchanged but keep its entry.
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
    assert_eq!(summary["mode"], "incremental");
    assert_eq!(summary["package_count"], 1);
    assert_eq!(summary["version_count"], 1);
    assert!(summary["skipped"].as_array().unwrap().iter().any(|s| {
        s["reason"]
            .as_str()
            .unwrap()
            .contains("unchanged since last checkpoint")
    }));

    // Add a new tag and reindex incrementally — the new version
    // should land while existing ones are preserved via checkpoint.
    commit_and_tag(
        &wal,
        &manifest_for("wal", "flow", "0.2.0", Some("EULA")),
        "v0.2.0",
    );
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
    assert_eq!(summary["package_count"], 1);
    assert_eq!(summary["version_count"], 2);

    let by_name: serde_json::Value =
        serde_json::from_slice(&std::fs::read(data.join("by-name/wal.json")).unwrap()).unwrap();
    assert_eq!(by_name["packages"][0]["latest_stable"], "0.2.0");
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

#[test]
fn reindex_captures_current_schema_manifest() {
    // Regression gate against manifest-schema rot. A package whose
    // `vibe.toml` carries the M1.17 unified-manifest + M1.18 loading-model
    // shape — a `[requires.packages]` table and a `[boot_snippet]` with
    // `source` / `category` — must scan into a complete index entry. Before
    // the 2026-05-22 de-rot the scanner only parsed the pre-M1.17 shape and
    // silently rotted; this test fails loudly if that ever recurs.
    if !git_available() {
        return;
    }
    let work = tempfile::tempdir().unwrap();
    let org = work.path().join("org");
    fs_must_create(&org);

    let feat = org.join("org.vibevm.welcome");
    init_repo(&feat);
    let modern = r#"[package]
group = "org.vibevm"
name = "welcome"
kind = "feat"
version = "0.3.0"
license = "EULA"
description = "landing page feat"
describes = "pkg:cargo/welcome@0.3.0"

[provides]
capabilities = ["ui:landing-page@0.3.0"]

[requires]
capabilities = ["db:any@>=1.0"]

[requires.packages]
"org.vibevm/wal" = "^0.1"

[boot_snippet]
source = "boot/10-feat-welcome.md"
category = "flow"
link = "static"
"#;
    std::fs::create_dir_all(feat.join("boot")).unwrap();
    std::fs::write(feat.join("boot/10-feat-welcome.md"), "# welcome boot\n").unwrap();
    commit_and_tag(&feat, modern, "v0.3.0");

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

    let feat_json: serde_json::Value =
        serde_json::from_slice(&std::fs::read(data.join("by-name/welcome.json")).unwrap()).unwrap();
    let entry = &feat_json["packages"][0]["versions"][0];
    assert_eq!(entry["describes"], "pkg:cargo/welcome@0.3.0");
    // The scanner parses each capability through `vibe-core`'s
    // `CapabilityRef` and records its canonical form: a bare version
    // (`@0.3.0`) canonicalises to the caret constraint, the same
    // Cargo-style normalisation `PackageRef` applies.
    assert_eq!(
        entry["provides"]["capabilities"][0],
        "ui:landing-page@^0.3.0"
    );
    // The modern `[requires.packages]` table flattens to a pkgref string.
    assert_eq!(entry["requires"]["packages"][0], "org.vibevm/wal@^0.1");
    assert_eq!(entry["requires"]["capabilities"][0], "db:any@>=1.0");
    // `[boot_snippet]` is recorded with `source` + `category` — the M1.18
    // loading model retired the pre-de-rot `filename`.
    assert_eq!(entry["boot_snippet"]["source"], "boot/10-feat-welcome.md");
    assert_eq!(entry["boot_snippet"]["category"], "flow");
}
