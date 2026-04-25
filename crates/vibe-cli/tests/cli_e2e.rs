//! End-to-end tests for the full M0 walk: init → install → list → uninstall.
//!
//! The registry used here is the hand-written `packages/` tree that ships in
//! the vibevm repo itself (the canonical `flow:wal` fixture per
//! `VIBEVM-SPEC.md` §13).

use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use predicates::prelude::*;

fn vibe() -> Command {
    Command::cargo_bin("vibe").expect("vibe binary built")
}

/// The `packages/` directory at the repo root is the fixture registry.
fn fixture_registry() -> PathBuf {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace = crate_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root");
    workspace.join("packages")
}

fn init_project(dir: &Path) {
    vibe()
        .arg("init")
        .arg("--path")
        .arg(dir)
        .assert()
        .success();
}

#[test]
fn full_install_cycle() {
    let project = tempfile::tempdir().unwrap();
    init_project(project.path());

    // Install flow:wal from the local fixture registry.
    vibe()
        .arg("install")
        .arg("flow:wal")
        .arg("--path")
        .arg(project.path())
        .arg("--registry")
        .arg(fixture_registry())
        .arg("--assume-yes")
        .assert()
        .success();

    // Expect all declared files to exist in the project.
    for rel in [
        "spec/flows/wal/WAL-PROTOCOL.md",
        "spec/flows/wal/session-end-hook.md",
        "spec/flows/wal/morning-routine.md",
        "spec/boot/10-flow-wal.md",
    ] {
        assert!(
            project.path().join(rel).is_file(),
            "expected {rel:?} to exist after install"
        );
    }

    // User-owned file survived untouched.
    let core_before = fs::read_to_string(project.path().join("spec/boot/00-core.md")).unwrap();

    // Lockfile must now carry the entry.
    let lock_text = fs::read_to_string(project.path().join("vibe.lock")).unwrap();
    let lock: vibe_core::manifest::Lockfile = toml::from_str(&lock_text).unwrap();
    assert_eq!(lock.packages.len(), 1);
    assert_eq!(lock.packages[0].name, "wal");
    assert_eq!(lock.packages[0].version.to_string(), "0.1.0");
    assert_eq!(lock.packages[0].boot_snippet.as_deref(), Some("10-flow-wal.md"));
    assert!(lock.packages[0].content_hash.starts_with("sha256:"));

    // Cache directory populated.
    assert!(project
        .path()
        .join(".vibe/cache/flow/wal/v0.1.0/vibe-package.toml")
        .is_file());

    // `vibe list` reflects the install.
    vibe()
        .arg("list")
        .arg("--path")
        .arg(project.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("flow"))
        .stdout(predicate::str::contains("wal"))
        .stdout(predicate::str::contains("0.1.0"));

    // `vibe uninstall` removes the declared files.
    vibe()
        .arg("uninstall")
        .arg("flow:wal")
        .arg("--path")
        .arg(project.path())
        .arg("--assume-yes")
        .assert()
        .success();

    for rel in [
        "spec/flows/wal/WAL-PROTOCOL.md",
        "spec/flows/wal/session-end-hook.md",
        "spec/flows/wal/morning-routine.md",
        "spec/boot/10-flow-wal.md",
    ] {
        assert!(
            !project.path().join(rel).exists(),
            "{rel:?} should be gone after uninstall"
        );
    }

    // User-owned file still intact.
    let core_after = fs::read_to_string(project.path().join("spec/boot/00-core.md")).unwrap();
    assert_eq!(core_before, core_after);

    // Lockfile entry removed.
    let lock_text = fs::read_to_string(project.path().join("vibe.lock")).unwrap();
    let lock: vibe_core::manifest::Lockfile = toml::from_str(&lock_text).unwrap();
    assert!(lock.packages.is_empty());

    // `list` after uninstall shows no packages.
    vibe()
        .arg("list")
        .arg("--path")
        .arg(project.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("no packages"));
}

#[test]
fn install_rejects_second_install_of_same_package() {
    let project = tempfile::tempdir().unwrap();
    init_project(project.path());

    vibe()
        .arg("install")
        .arg("flow:wal")
        .arg("--path")
        .arg(project.path())
        .arg("--registry")
        .arg(fixture_registry())
        .arg("--assume-yes")
        .assert()
        .success();

    // Second install should fail with a clear "already installed" error.
    vibe()
        .arg("install")
        .arg("flow:wal")
        .arg("--path")
        .arg(project.path())
        .arg("--registry")
        .arg(fixture_registry())
        .arg("--assume-yes")
        .assert()
        .failure()
        .stderr(predicate::str::contains("already installed"));
}

#[test]
fn install_reports_json() {
    let project = tempfile::tempdir().unwrap();
    init_project(project.path());

    let out = vibe()
        .arg("--json")
        .arg("install")
        .arg("flow:wal")
        .arg("--path")
        .arg(project.path())
        .arg("--registry")
        .arg(fixture_registry())
        .arg("--assume-yes")
        .output()
        .unwrap();
    assert!(out.status.success());

    // The plan and the report are both emitted as JSON documents,
    // concatenated on stdout. Use StreamDeserializer to read every document
    // in order and inspect the last one (the install report).
    let stdout = String::from_utf8(out.stdout).unwrap();
    let de = serde_json::Deserializer::from_str(&stdout);
    let docs: Vec<serde_json::Value> = de
        .into_iter::<serde_json::Value>()
        .collect::<Result<_, _>>()
        .expect("stdout is a stream of JSON documents");
    assert!(docs.len() >= 2, "expected at least a plan and a report");
    let last = docs.last().unwrap();
    assert_eq!(last["ok"], true);
    assert_eq!(last["command"], "install");
    assert_eq!(last["installed"].as_array().unwrap().len(), 1);
}

#[test]
fn uninstall_errors_when_package_not_installed() {
    let project = tempfile::tempdir().unwrap();
    init_project(project.path());

    vibe()
        .arg("uninstall")
        .arg("flow:wal")
        .arg("--path")
        .arg(project.path())
        .arg("--assume-yes")
        .assert()
        .failure()
        .stderr(predicate::str::contains("not installed"));
}

#[test]
fn install_boot_snippet_conflict_exits_with_code_three() {
    let project = tempfile::tempdir().unwrap();
    init_project(project.path());

    // Plant a conflicting boot snippet with the same NN-prefix as flow:wal's
    // `10-flow-wal.md`.
    fs::create_dir_all(project.path().join("spec/boot")).unwrap();
    fs::write(
        project.path().join("spec/boot/10-flow-squatter.md"),
        "squatter\n",
    )
    .unwrap();

    let assertion = vibe()
        .arg("install")
        .arg("flow:wal")
        .arg("--path")
        .arg(project.path())
        .arg("--registry")
        .arg(fixture_registry())
        .arg("--assume-yes")
        .assert()
        .failure();
    let output = assertion.get_output();
    assert_eq!(output.status.code(), Some(3));
}

// ---------------------------------------------------------------------------
// M1.1 — install from a git-backed registry
// ---------------------------------------------------------------------------

fn git_available() -> bool {
    std::process::Command::new("git")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn run_git(cwd: &Path, args: &[&str]) {
    let out = std::process::Command::new("git")
        .args(args)
        .current_dir(cwd)
        .env("LC_ALL", "C")
        .env("LANG", "C")
        .output()
        .expect("spawn git");
    assert!(
        out.status.success(),
        "git {} failed: {}",
        args.join(" "),
        String::from_utf8_lossy(&out.stderr)
    );
}

/// Build a per-package bare git registry under `root/`: one bare repo
/// per package, content at the repo root, tagged `v<semver>`.
///
/// For this test we seed exactly one package: `flow:wal@0.1.0` →
/// `<root>/flow-wal.git`. The "registry" is then `<root>` itself —
/// `MultiRegistryResolver` composes per-package URLs by appending
/// `<kind>-<name>.git` to the org URL.
///
/// Returns the org root path (not any single repo), since the install
/// flow points `[[registry]]` at the org URL.
fn make_per_package_registry(root: &Path) -> PathBuf {
    let src = root.join("src-flow-wal");
    fs::create_dir_all(&src).unwrap();
    run_git(&src, &["init", "--initial-branch=main"]);
    run_git(&src, &["config", "user.email", "t@example.com"]);
    run_git(&src, &["config", "user.name", "Test"]);

    // Per-package layout: package contents live AT THE ROOT of the repo,
    // not under `<kind>/<name>/v<ver>/`.
    copy_tree(&fixture_registry().join("flow/wal/v0.1.0"), &src);
    run_git(&src, &["add", "-A"]);
    run_git(&src, &["commit", "-m", "flow:wal@0.1.0"]);
    run_git(&src, &["tag", "v0.1.0"]);

    let bare = root.join("flow-wal.git");
    run_git(root, &[
        "clone", "--bare", src.to_str().unwrap(), bare.to_str().unwrap(),
    ]);
    run_git(&bare, &["symbolic-ref", "HEAD", "refs/heads/main"]);
    root.to_path_buf()
}

fn copy_tree(src: &Path, dst: &Path) {
    for entry in walkdir::WalkDir::new(src).into_iter().filter_map(|e| e.ok()) {
        let rel = entry.path().strip_prefix(src).unwrap();
        let target = dst.join(rel);
        if entry.file_type().is_dir() {
            fs::create_dir_all(&target).unwrap();
        } else if entry.file_type().is_file() {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::copy(entry.path(), target).unwrap();
        }
    }
}

fn write_project_with_per_package_registry(project_dir: &Path, registry_url: &str) {
    // [[registry]] in PROP-002 shape, pointing at the per-package org URL.
    // `naming = "kind-name"` is the default convention for vibespecs.
    let manifest = format!(
        r#"[project]
name = "demo"
version = "0.0.1"

[[registry]]
name = "default"
url = "{registry_url}"
"#
    );
    fs::write(project_dir.join("vibe.toml"), manifest).unwrap();
}

#[test]
fn install_from_git_registry() {
    if !git_available() {
        eprintln!("skipping install_from_git_registry: git not on PATH");
        return;
    }

    let outer = tempfile::tempdir().unwrap();
    let org_root = make_per_package_registry(outer.path());
    let cache = outer.path().join("cache");
    fs::create_dir_all(&cache).unwrap();

    let project = tempfile::tempdir().unwrap();
    init_project(project.path());

    // Org URL = parent of `flow-wal.git`. `git+file://` prefix is the
    // Cargo / pip convention recorded in lockfiles; the resolver strips
    // it before invoking `git`, so it works with both prefixed and bare
    // forms in `vibe.toml`.
    let url = format!("git+file://{}", org_root.to_string_lossy().replace('\\', "/"));
    write_project_with_per_package_registry(project.path(), &url);

    vibe()
        .env("VIBE_REGISTRY_CACHE", &cache)
        .arg("install")
        .arg("flow:wal")
        .arg("--path")
        .arg(project.path())
        .arg("--assume-yes")
        .assert()
        .success();

    // Lockfile reflects the per-package shape.
    let lock_text = fs::read_to_string(project.path().join("vibe.lock")).unwrap();
    let lock: vibe_core::manifest::Lockfile = toml::from_str(&lock_text).unwrap();
    assert_eq!(lock.packages.len(), 1);
    let entry = &lock.packages[0];

    // schema_version = 2 in the meta block.
    assert_eq!(lock.meta.schema_version, 2);

    // PROP-002 §2.7 provenance: registry name, full per-package source_url,
    // tag in source_ref. No more `#flow/wal/v0.1.0` fragment shape.
    assert_eq!(entry.registry.as_deref(), Some("default"));
    assert!(
        entry.source_url.starts_with("git+file://"),
        "expected git+file:// prefix, got: {}",
        entry.source_url
    );
    assert!(
        entry.source_url.ends_with("/flow-wal.git"),
        "expected per-package URL ending in /flow-wal.git, got: {}",
        entry.source_url
    );
    assert_eq!(entry.source_ref.as_deref(), Some("v0.1.0"));
    assert!(!entry.overridden);

    // Cache layout: one bucket dir under cache/, with packages/<kind>-<name>/
    // and a clone subdir. The registry-level meta.toml lands together with
    // freshness machinery in a follow-up — this commit only requires the
    // bucket directory itself plus the package clone.
    let clone_dirs: Vec<_> = fs::read_dir(&cache)
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert_eq!(clone_dirs.len(), 1, "expected one registry cache bucket");
    let bucket = clone_dirs[0].path();
    let pkg_clone = bucket.join("packages/flow-wal/clone");
    assert!(
        pkg_clone.join(".git").exists(),
        "per-package clone missing .git/: {}",
        pkg_clone.display()
    );
    assert!(
        pkg_clone.join("vibe-package.toml").exists(),
        "vibe-package.toml not in per-package clone: {}",
        pkg_clone.display()
    );

    // `vibe registry sync` is migrated separately — the legacy single-repo
    // sync path doesn't fit a per-package org URL. The follow-up commit
    // walks the lockfile to refresh per-package clones.
}
