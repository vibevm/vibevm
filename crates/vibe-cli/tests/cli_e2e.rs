//! End-to-end tests for the full M0 walk: init → install → list → uninstall.
//!
//! The registry used here is the hand-written `fixtures/registry/` tree that ships in
//! the vibevm repo itself (the canonical `flow:wal` fixture per
//! `VIBEVM-SPEC.md` §13).

use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use predicates::prelude::*;

fn vibe() -> Command {
    Command::cargo_bin("vibe").expect("vibe binary built")
}

/// The `fixtures/registry/` directory at the repo root holds the
/// hermetic fixture registry the e2e tests run against. Layout is the
/// M0/M1.1 monorepo shape (`<kind>/<name>/v<ver>/…`); the directory
/// is intentionally separate from the future `packages/` tree (where
/// vibevm dogfoods its own packages — keeps test fixtures and live
/// artefacts visually distinct).
fn fixture_registry() -> PathBuf {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace = crate_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root");
    workspace.join("fixtures").join("registry")
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

    // schema_version matches CURRENT (3 after PROP-003 r2).
    assert_eq!(
        lock.meta.schema_version,
        vibe_core::manifest::CURRENT_SCHEMA_VERSION,
    );

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

/// Build a per-package git registry where `flow-wal` carries TWO
/// tagged versions: `v0.1.0` (from the in-tree fixture, content
/// rewritten so the project file the test asserts on lives at a
/// stable known path) and `v0.2.0` (one file modified, one new file
/// added, one file removed, boot snippet unchanged). Used by the
/// `vibe update` end-to-end tests.
fn make_two_version_registry(root: &Path) -> PathBuf {
    let src = root.join("src-flow-wal");
    fs::create_dir_all(&src).unwrap();
    run_git(&src, &["init", "--initial-branch=main"]);
    run_git(&src, &["config", "user.email", "t@example.com"]);
    run_git(&src, &["config", "user.name", "Test"]);

    // v0.1.0 layout: A.md + B.md + boot snippet, capabilities empty.
    let manifest_v1 = r#"[package]
name = "wal"
kind = "flow"
version = "0.1.0"

[writes]
files = [
    "spec/flows/wal/A.md",
    "spec/flows/wal/B.md",
]

[boot_snippet]
filename = "10-flow-wal.md"
source = "boot/10-flow-wal.md"
"#;
    // Pin LF endings in the fixture repo — otherwise on Windows
    // git's `core.autocrlf` will rewrite text on checkout and the
    // bytes the test asserts on (`"v1 A\n"`) won't match what
    // ends up in the cache (`"v1 A\r\n"`).
    fs::write(src.join(".gitattributes"), "* text=auto eol=lf\n").unwrap();
    fs::write(src.join("vibe-package.toml"), manifest_v1).unwrap();
    fs::create_dir_all(src.join("spec/flows/wal")).unwrap();
    fs::create_dir_all(src.join("boot")).unwrap();
    fs::write(src.join("spec/flows/wal/A.md"), "v1 A\n").unwrap();
    fs::write(src.join("spec/flows/wal/B.md"), "v1 B\n").unwrap();
    fs::write(src.join("boot/10-flow-wal.md"), "v1 boot\n").unwrap();
    run_git(&src, &["add", "-A"]);
    run_git(&src, &["commit", "-m", "flow:wal@0.1.0"]);
    run_git(&src, &["tag", "v0.1.0"]);

    // v0.2.0: A modified, B removed, C added, boot unchanged.
    let manifest_v2 = r#"[package]
name = "wal"
kind = "flow"
version = "0.2.0"

[writes]
files = [
    "spec/flows/wal/A.md",
    "spec/flows/wal/C.md",
]

[boot_snippet]
filename = "10-flow-wal.md"
source = "boot/10-flow-wal.md"
"#;
    fs::write(src.join("vibe-package.toml"), manifest_v2).unwrap();
    fs::write(src.join("spec/flows/wal/A.md"), "v2 A — changed!\n").unwrap();
    fs::write(src.join("spec/flows/wal/C.md"), "v2 C\n").unwrap();
    fs::remove_file(src.join("spec/flows/wal/B.md")).unwrap();
    run_git(&src, &["add", "-A"]);
    run_git(&src, &["commit", "-m", "flow:wal@0.2.0"]);
    run_git(&src, &["tag", "v0.2.0"]);

    let bare = root.join("flow-wal.git");
    run_git(
        root,
        &[
            "clone",
            "--bare",
            src.to_str().unwrap(),
            bare.to_str().unwrap(),
        ],
    );
    run_git(&bare, &["symbolic-ref", "HEAD", "refs/heads/main"]);
    root.to_path_buf()
}

#[test]
fn update_bumps_to_new_version_and_diffs_files() {
    if !git_available() {
        eprintln!("skipping update_bumps_to_new_version_and_diffs_files: git not on PATH");
        return;
    }

    let outer = tempfile::tempdir().unwrap();
    let org_root = make_two_version_registry(outer.path());
    let cache = outer.path().join("cache");
    fs::create_dir_all(&cache).unwrap();

    let project = tempfile::tempdir().unwrap();
    init_project(project.path());
    let url = format!(
        "git+file://{}",
        org_root.to_string_lossy().replace('\\', "/")
    );
    write_project_with_per_package_registry(project.path(), &url);

    // Install v0.1.0 — pin to ^0.1 so update would otherwise pick v0.2.0.
    vibe()
        .env("VIBE_REGISTRY_CACHE", &cache)
        .arg("install")
        .arg("flow:wal@^0.1")
        .arg("--path")
        .arg(project.path())
        .arg("--assume-yes")
        .assert()
        .success();

    let lock_text = fs::read_to_string(project.path().join("vibe.lock")).unwrap();
    let lock: vibe_core::manifest::Lockfile = toml::from_str(&lock_text).unwrap();
    assert_eq!(lock.packages[0].version.to_string(), "0.1.0");

    // Verify v0.1.0 content is on disk.
    assert_eq!(
        fs::read_to_string(project.path().join("spec/flows/wal/A.md")).unwrap(),
        "v1 A\n"
    );
    assert!(project.path().join("spec/flows/wal/B.md").exists());
    assert!(!project.path().join("spec/flows/wal/C.md").exists());

    // The original constraint was `^0.1`, which excludes v0.2.0 — `vibe
    // update` must respect the user's pinned constraint and report
    // up-to-date. Verify the constraint surfaces in the lockfile root.
    assert_eq!(lock.meta.root_dependencies.len(), 1);

    // Manually rewrite the root constraint to `*` so the update would
    // pick v0.2.0. Easier than re-installing; matches the case where
    // the user originally typed `flow:wal` (Latest) and now wants the
    // bump.
    let mut lock_owned = lock;
    lock_owned.meta.root_dependencies[0] = vibe_core::PackageRef::parse("flow:wal").unwrap();
    fs::write(
        project.path().join("vibe.lock"),
        toml::to_string(&lock_owned).unwrap(),
    )
    .unwrap();

    // Now run `vibe update flow:wal` — should bump to v0.2.0, apply diff.
    vibe()
        .env("VIBE_REGISTRY_CACHE", &cache)
        .arg("update")
        .arg("flow:wal")
        .arg("--path")
        .arg(project.path())
        .arg("--assume-yes")
        .assert()
        .success();

    // Lockfile entry now records v0.2.0 + new content_hash.
    let lock_text = fs::read_to_string(project.path().join("vibe.lock")).unwrap();
    let lock: vibe_core::manifest::Lockfile = toml::from_str(&lock_text).unwrap();
    assert_eq!(lock.packages.len(), 1);
    let entry = &lock.packages[0];
    assert_eq!(entry.version.to_string(), "0.2.0");
    assert_eq!(entry.source_ref.as_deref(), Some("v0.2.0"));
    assert!(entry.content_hash.starts_with("sha256:"));

    // On-disk: A overwritten, B removed, C added, boot snippet unchanged.
    assert_eq!(
        fs::read_to_string(project.path().join("spec/flows/wal/A.md")).unwrap(),
        "v2 A — changed!\n"
    );
    assert!(!project.path().join("spec/flows/wal/B.md").exists());
    assert_eq!(
        fs::read_to_string(project.path().join("spec/flows/wal/C.md")).unwrap(),
        "v2 C\n"
    );
    assert_eq!(
        fs::read_to_string(project.path().join("spec/boot/10-flow-wal.md")).unwrap(),
        "v1 boot\n"
    );

    // files_written reflects the new shape.
    let written: Vec<String> = entry
        .files_written
        .iter()
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .collect();
    assert!(written.contains(&"spec/flows/wal/A.md".to_string()));
    assert!(written.contains(&"spec/flows/wal/C.md".to_string()));
    assert!(written.contains(&"spec/boot/10-flow-wal.md".to_string()));
    assert!(!written.contains(&"spec/flows/wal/B.md".to_string()));
}

#[test]
fn update_refuses_when_user_edited_file() {
    if !git_available() {
        eprintln!("skipping update_refuses_when_user_edited_file: git not on PATH");
        return;
    }

    let outer = tempfile::tempdir().unwrap();
    let org_root = make_two_version_registry(outer.path());
    let cache = outer.path().join("cache");
    fs::create_dir_all(&cache).unwrap();

    let project = tempfile::tempdir().unwrap();
    init_project(project.path());
    let url = format!(
        "git+file://{}",
        org_root.to_string_lossy().replace('\\', "/")
    );
    write_project_with_per_package_registry(project.path(), &url);

    vibe()
        .env("VIBE_REGISTRY_CACHE", &cache)
        .arg("install")
        .arg("flow:wal@^0.1")
        .arg("--path")
        .arg(project.path())
        .arg("--assume-yes")
        .assert()
        .success();

    // User edits A.md after install.
    fs::write(
        project.path().join("spec/flows/wal/A.md"),
        "user-edited bytes\n",
    )
    .unwrap();

    // Rewrite root constraint to allow the bump.
    let lock_text = fs::read_to_string(project.path().join("vibe.lock")).unwrap();
    let mut lock: vibe_core::manifest::Lockfile = toml::from_str(&lock_text).unwrap();
    lock.meta.root_dependencies[0] = vibe_core::PackageRef::parse("flow:wal").unwrap();
    fs::write(
        project.path().join("vibe.lock"),
        toml::to_string(&lock).unwrap(),
    )
    .unwrap();

    let assertion = vibe()
        .env("VIBE_REGISTRY_CACHE", &cache)
        .arg("update")
        .arg("flow:wal")
        .arg("--path")
        .arg(project.path())
        .arg("--assume-yes")
        .assert()
        .failure();
    let stderr = String::from_utf8_lossy(&assertion.get_output().stderr).to_string();
    assert!(
        stderr.contains("user-edited") || stderr.contains("UserEditedFile") || stderr.contains("user edited"),
        "expected user-edit refusal in stderr; got:\n{stderr}"
    );

    // User's edit survives.
    assert_eq!(
        fs::read_to_string(project.path().join("spec/flows/wal/A.md")).unwrap(),
        "user-edited bytes\n"
    );
}

#[test]
fn show_effective_emits_boot_files_and_wal_with_provenance() {
    let project = tempfile::tempdir().unwrap();
    init_project(project.path());

    let assertion = vibe()
        .arg("show")
        .arg("effective")
        .arg("--path")
        .arg(project.path())
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assertion.get_output().stdout);
    // 00-core and 90-user boot files, plus the WAL.
    assert!(
        stdout.contains("spec://project/boot/00-core.md"),
        "expected 00-core spec URI; got:\n{stdout}"
    );
    assert!(
        stdout.contains("spec://project/boot/90-user.md"),
        "expected 90-user spec URI; got:\n{stdout}"
    );
    assert!(
        stdout.contains("spec://project/WAL"),
        "expected WAL spec URI; got:\n{stdout}"
    );
    // Provenance for foundation files.
    assert!(
        stdout.contains("(user)") || stdout.contains("(wal)"),
        "expected user / wal provenance markers; got:\n{stdout}"
    );
}

#[test]
fn show_effective_attributes_installed_package_files() {
    let project = tempfile::tempdir().unwrap();
    init_project(project.path());

    // Install flow:wal from the local fixture so the lockfile carries
    // a real entry — show effective should attribute its files to
    // the package.
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

    let out = vibe()
        .arg("--json")
        .arg("show")
        .arg("effective")
        .arg("--path")
        .arg(project.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    let payload: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("valid JSON");
    assert_eq!(payload["ok"], true);
    assert_eq!(payload["command"], "show:effective");
    let sections = payload["sections"].as_array().unwrap();
    // Boot snippet `10-flow-wal.md` should be attributed to the package.
    let boot_section = sections
        .iter()
        .find(|s| s["path"] == "spec/boot/10-flow-wal.md")
        .expect("expected 10-flow-wal.md section");
    assert!(
        boot_section["origin"]
            .as_str()
            .unwrap()
            .starts_with("package:flow:wal"),
        "got origin: {}",
        boot_section["origin"]
    );
    // The package's spec/flows/wal/* files should each appear.
    assert!(
        sections
            .iter()
            .any(|s| s["path"]
                .as_str()
                .unwrap()
                .starts_with("spec/flows/wal/")
                && s["origin"]
                    .as_str()
                    .unwrap()
                    .starts_with("package:flow:wal"))
    );
}

#[test]
fn user_config_promotes_vibe_registry_cache_into_runtime() {
    // Smoke: a user-config file that defaults VIBE_REGISTRY_CACHE
    // must actually take effect at install time — not just surface
    // in `vibe show config`. Without the live env override, the
    // install's per-package clone must land in the user-config-
    // pointed cache.
    if !git_available() {
        eprintln!("skipping user_config_promotes_vibe_registry_cache_into_runtime: git not on PATH");
        return;
    }

    let outer = tempfile::tempdir().unwrap();
    let org_root = make_per_package_registry(outer.path());
    let user_cfg_dir = tempfile::tempdir().unwrap();
    let user_cfg_path = user_cfg_dir.path().join("config.toml");
    let cache = outer.path().join("user-config-cache");
    fs::create_dir_all(&cache).unwrap();
    fs::write(
        &user_cfg_path,
        format!(
            "[env]\nVIBE_REGISTRY_CACHE = \"{}\"\n",
            cache.to_string_lossy().replace('\\', "/")
        ),
    )
    .unwrap();

    let project = tempfile::tempdir().unwrap();
    init_project(project.path());
    let url = format!(
        "git+file://{}",
        org_root.to_string_lossy().replace('\\', "/")
    );
    write_project_with_per_package_registry(project.path(), &url);

    vibe()
        .env("VIBEVM_USER_CONFIG", &user_cfg_path)
        .env_remove("VIBE_REGISTRY_CACHE")
        .arg("install")
        .arg("flow:wal")
        .arg("--path")
        .arg(project.path())
        .arg("--assume-yes")
        .assert()
        .success();

    // The user-config-pointed cache must be populated with the
    // per-package clone — proves the promotion ran end-to-end.
    let bucket_count = fs::read_dir(&cache).unwrap().count();
    assert!(
        bucket_count >= 1,
        "expected user-config-pointed cache to have at least one bucket, got {bucket_count}"
    );
    let bucket = fs::read_dir(&cache)
        .unwrap()
        .filter_map(|e| e.ok())
        .next()
        .expect("bucket")
        .path();
    let pkg_clone = bucket.join("packages/flow-wal/clone");
    assert!(
        pkg_clone.join(".git").exists(),
        "per-package clone must land in user-config cache: {}",
        pkg_clone.display()
    );
}

#[test]
fn show_config_user_layer_provides_default_for_unset_env() {
    // User-level config defaults VIBE_REGISTRY_CACHE; live env is
    // unset. Provenance must surface as `user-config`, value is the
    // user-config string.
    let project = tempfile::tempdir().unwrap();
    init_project(project.path());

    let user_cfg_dir = tempfile::tempdir().unwrap();
    let user_cfg_path = user_cfg_dir.path().join("config.toml");
    fs::write(
        &user_cfg_path,
        r#"[env]
VIBE_REGISTRY_CACHE = "/from-user-config"
VIBE_LOG = "vibe_registry=info"
"#,
    )
    .unwrap();

    let out = vibe()
        .env("VIBEVM_USER_CONFIG", &user_cfg_path)
        .env_remove("VIBE_REGISTRY_CACHE")
        .env_remove("VIBE_LOG")
        .arg("--json")
        .arg("show")
        .arg("config")
        .arg("--path")
        .arg(project.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let payload: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("valid JSON");
    let env_arr = payload["env"].as_array().unwrap();
    let cache_entry = env_arr
        .iter()
        .find(|e| e["name"] == "VIBE_REGISTRY_CACHE")
        .unwrap();
    assert_eq!(cache_entry["provenance"], "user-config");
    assert_eq!(cache_entry["value"], "/from-user-config");
    let log_entry = env_arr
        .iter()
        .find(|e| e["name"] == "VIBE_LOG")
        .unwrap();
    assert_eq!(log_entry["provenance"], "user-config");
    assert_eq!(log_entry["value"], "vibe_registry=info");
    // The user_config block reports loaded = true and the resolved path.
    assert_eq!(payload["user_config"]["loaded"], true);
    assert!(payload["user_config"]["path"].is_string());
}

#[test]
fn show_config_live_env_overrides_user_config() {
    // Both layers set VIBE_REGISTRY_CACHE; the live env wins.
    let project = tempfile::tempdir().unwrap();
    init_project(project.path());

    let user_cfg_dir = tempfile::tempdir().unwrap();
    let user_cfg_path = user_cfg_dir.path().join("config.toml");
    fs::write(
        &user_cfg_path,
        "[env]\nVIBE_REGISTRY_CACHE = \"/from-user-config\"\n",
    )
    .unwrap();

    let out = vibe()
        .env("VIBEVM_USER_CONFIG", &user_cfg_path)
        .env("VIBE_REGISTRY_CACHE", "/from-live-env")
        .arg("--json")
        .arg("show")
        .arg("config")
        .arg("--path")
        .arg(project.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    let payload: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("valid JSON");
    let env_arr = payload["env"].as_array().unwrap();
    let cache_entry = env_arr
        .iter()
        .find(|e| e["name"] == "VIBE_REGISTRY_CACHE")
        .unwrap();
    assert_eq!(cache_entry["provenance"], "env");
    assert_eq!(cache_entry["value"], "/from-live-env");
}

#[test]
fn show_config_user_token_default_redacts_value() {
    // VIBEVM_PUBLISH_TOKEN is sensitive: even if defaulted via
    // user-config (which would be a poor operator choice — token
    // belongs in `~/.vibevm/<host>.publish.token` per PROP-000 §20
    // — but we can't refuse to load) the value must NEVER appear in
    // output.
    let project = tempfile::tempdir().unwrap();
    init_project(project.path());

    let user_cfg_dir = tempfile::tempdir().unwrap();
    let user_cfg_path = user_cfg_dir.path().join("config.toml");
    fs::write(
        &user_cfg_path,
        "[env]\nVIBEVM_PUBLISH_TOKEN = \"secret-do-not-leak\"\n",
    )
    .unwrap();

    let out = vibe()
        .env("VIBEVM_USER_CONFIG", &user_cfg_path)
        .env_remove("VIBEVM_PUBLISH_TOKEN")
        .arg("--json")
        .arg("show")
        .arg("config")
        .arg("--path")
        .arg(project.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        !stdout.contains("secret-do-not-leak"),
        "token bytes leaked into stdout:\n{stdout}"
    );
    let payload: serde_json::Value =
        serde_json::from_str(&stdout).expect("valid JSON");
    let env_arr = payload["env"].as_array().unwrap();
    let token = env_arr
        .iter()
        .find(|e| e["name"] == "VIBEVM_PUBLISH_TOKEN")
        .unwrap();
    assert_eq!(token["provenance"], "redacted");
    assert!(token["value"]
        .as_str()
        .unwrap()
        .contains("redacted"));
}

#[test]
fn show_config_emits_registry_block_with_provenance() {
    let project = tempfile::tempdir().unwrap();
    init_project(project.path());

    let out = vibe()
        .arg("--json")
        .arg("show")
        .arg("config")
        .arg("--path")
        .arg(project.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    let payload: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("valid JSON");
    assert_eq!(payload["ok"], true);
    assert_eq!(payload["command"], "show:config");
    let registries = payload["registries"].as_array().unwrap();
    assert!(!registries.is_empty(), "default `vibe init` configures a registry");
    assert_eq!(registries[0]["provenance"], "vibe.toml");
    let env = payload["env"].as_array().unwrap();
    assert!(
        env.iter()
            .any(|e| e["name"] == "VIBEVM_PUBLISH_TOKEN"),
        "VIBEVM_PUBLISH_TOKEN should appear in env block"
    );
    // Token entry must surface as either `default` (unset) or
    // `redacted` (set in env). Never the raw value.
    let token_entry = env
        .iter()
        .find(|e| e["name"] == "VIBEVM_PUBLISH_TOKEN")
        .unwrap();
    let prov = token_entry["provenance"].as_str().unwrap();
    assert!(
        prov == "default" || prov == "redacted",
        "VIBEVM_PUBLISH_TOKEN provenance must be default/redacted; got `{prov}`"
    );
}

#[test]
fn check_clean_project_exits_zero_with_no_findings() {
    let project = tempfile::tempdir().unwrap();
    init_project(project.path());
    let assertion = vibe()
        .arg("check")
        .arg("--path")
        .arg(project.path())
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assertion.get_output().stdout);
    assert!(
        stdout.contains("clean"),
        "expected clean summary; got:\n{stdout}"
    );
}

#[test]
fn check_boot_prefix_collision_exits_nonzero() {
    let project = tempfile::tempdir().unwrap();
    init_project(project.path());
    // Plant two `10-` boot snippets — collision.
    fs::write(project.path().join("spec/boot/10-flow-wal.md"), "x").unwrap();
    fs::write(project.path().join("spec/boot/10-flow-other.md"), "y").unwrap();
    let assertion = vibe()
        .arg("check")
        .arg("--path")
        .arg(project.path())
        .assert()
        .failure();
    let stdout = String::from_utf8_lossy(&assertion.get_output().stdout);
    assert!(
        stdout.contains("[E]"),
        "expected error sigil in stdout; got:\n{stdout}"
    );
    assert!(
        stdout.contains("boot prefix"),
        "expected boot prefix collision message; got:\n{stdout}"
    );
}

#[test]
fn check_emits_json_envelope() {
    let project = tempfile::tempdir().unwrap();
    init_project(project.path());
    let out = vibe()
        .arg("--json")
        .arg("check")
        .arg("--path")
        .arg(project.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    let payload: serde_json::Value = serde_json::from_str(stdout.trim()).expect("valid JSON");
    assert_eq!(payload["ok"], true);
    assert_eq!(payload["command"], "check");
    let summary = &payload["summary"];
    assert_eq!(summary["error"], 0);
    assert!(payload["findings"].is_array());
}

#[test]
fn update_when_constraint_pins_old_version_reports_up_to_date() {
    if !git_available() {
        eprintln!("skipping update_when_constraint_pins_old_version_reports_up_to_date: git not on PATH");
        return;
    }

    let outer = tempfile::tempdir().unwrap();
    let org_root = make_two_version_registry(outer.path());
    let cache = outer.path().join("cache");
    fs::create_dir_all(&cache).unwrap();

    let project = tempfile::tempdir().unwrap();
    init_project(project.path());
    let url = format!(
        "git+file://{}",
        org_root.to_string_lossy().replace('\\', "/")
    );
    write_project_with_per_package_registry(project.path(), &url);

    // Install v0.1.0 with constraint `^0.1`. v0.2.0 exists upstream
    // but is excluded by the constraint — `vibe update` must respect
    // the original pin and report up-to-date instead of jumping to
    // v0.2.0.
    vibe()
        .env("VIBE_REGISTRY_CACHE", &cache)
        .arg("install")
        .arg("flow:wal@^0.1")
        .arg("--path")
        .arg(project.path())
        .arg("--assume-yes")
        .assert()
        .success();

    let assertion = vibe()
        .env("VIBE_REGISTRY_CACHE", &cache)
        .arg("update")
        .arg("flow:wal")
        .arg("--path")
        .arg(project.path())
        .arg("--assume-yes")
        .assert()
        .success();
    let out = assertion.get_output();
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    assert!(
        stdout.contains("up-to-date"),
        "expected `up-to-date` summary; got:\n{stdout}"
    );

    // Lockfile entry stays at v0.1.0.
    let lock_text = fs::read_to_string(project.path().join("vibe.lock")).unwrap();
    let lock: vibe_core::manifest::Lockfile = toml::from_str(&lock_text).unwrap();
    assert_eq!(lock.packages[0].version.to_string(), "0.1.0");
}

#[test]
fn vendor_produces_bare_repo_per_lockfile_entry() {
    // End-to-end: install from a per-package git registry, then run
    // `vibe registry vendor`. The vendor dir should contain a bare
    // git repo per lockfile entry, ready for use as `[[mirror]] url
    // = "file:///<abs>"`. Verifies the repo is consumable by checking
    // that `git clone` succeeds against it and that the v0.1.0 tag
    // is preserved.
    if !git_available() {
        eprintln!("skipping vendor_produces_bare_repo_per_lockfile_entry: git not on PATH");
        return;
    }

    let outer = tempfile::tempdir().unwrap();
    let org_root = make_per_package_registry(outer.path());
    let cache = outer.path().join("cache");
    fs::create_dir_all(&cache).unwrap();

    let project = tempfile::tempdir().unwrap();
    init_project(project.path());
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

    // Run `vibe registry vendor` against the freshly installed project.
    let vendor_dir = outer.path().join("vendor-output");
    vibe()
        .env("VIBE_REGISTRY_CACHE", &cache)
        .arg("registry")
        .arg("vendor")
        .arg("--out")
        .arg(&vendor_dir)
        .arg("--path")
        .arg(project.path())
        .assert()
        .success();

    // The vendor dir contains a bare repo for the package and a
    // README.md explaining how to wire it.
    let bare_repo = vendor_dir.join("flow-wal.git");
    assert!(
        bare_repo.is_dir(),
        "expected vendor bare repo at {}",
        bare_repo.display()
    );
    assert!(
        bare_repo.join("HEAD").is_file(),
        "expected HEAD in vendor bare repo"
    );
    // Either loose ref or packed-refs is acceptable depending on git
    // version — tag presence is verified via `git ls-remote` below.
    assert!(
        vendor_dir.join("README.md").is_file(),
        "expected vendor/README.md"
    );

    // Verify the bare repo is a usable git source. `git ls-remote
    // <vendor>/flow-wal.git` must list the v0.1.0 tag the install
    // pulled in.
    let ls_out = std::process::Command::new("git")
        .args(["ls-remote", "--tags", bare_repo.to_str().unwrap()])
        .env("LC_ALL", "C")
        .env("LANG", "C")
        .output()
        .expect("spawn git ls-remote");
    assert!(
        ls_out.status.success(),
        "git ls-remote against vendored repo failed: {}",
        String::from_utf8_lossy(&ls_out.stderr)
    );
    let tags = String::from_utf8_lossy(&ls_out.stdout);
    assert!(
        tags.contains("refs/tags/v0.1.0"),
        "vendored repo missing v0.1.0 tag — got:\n{tags}"
    );

    // Cloning from the vendored repo into a fresh worktree must
    // produce the original package content. This is the
    // operationally-relevant promise of `vibe registry vendor`:
    // the dir is a true git source.
    let worktree = outer.path().join("clone-from-vendor");
    let clone_out = std::process::Command::new("git")
        .args([
            "clone",
            "--branch",
            "v0.1.0",
            bare_repo.to_str().unwrap(),
            worktree.to_str().unwrap(),
        ])
        .env("LC_ALL", "C")
        .env("LANG", "C")
        .output()
        .expect("spawn git clone");
    assert!(
        clone_out.status.success(),
        "git clone of vendored repo failed: {}",
        String::from_utf8_lossy(&clone_out.stderr)
    );
    assert!(
        worktree.join("vibe-package.toml").is_file(),
        "vendored repo's v0.1.0 tag did not produce expected payload"
    );
}

#[test]
fn vendor_refuses_non_empty_out_dir_without_force() {
    if !git_available() {
        eprintln!("skipping vendor_refuses_non_empty_out_dir_without_force: git not on PATH");
        return;
    }

    let outer = tempfile::tempdir().unwrap();
    let org_root = make_per_package_registry(outer.path());
    let cache = outer.path().join("cache");
    fs::create_dir_all(&cache).unwrap();

    let project = tempfile::tempdir().unwrap();
    init_project(project.path());
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

    // Plant operator content in the would-be vendor dir.
    let vendor_dir = outer.path().join("user-content");
    fs::create_dir_all(&vendor_dir).unwrap();
    fs::write(vendor_dir.join("important.txt"), "do not delete\n").unwrap();

    // Without --force, vendor must refuse. Operator content survives.
    vibe()
        .env("VIBE_REGISTRY_CACHE", &cache)
        .arg("registry")
        .arg("vendor")
        .arg("--out")
        .arg(&vendor_dir)
        .arg("--path")
        .arg(project.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("not empty"))
        .stderr(predicate::str::contains("--force"));
    assert!(
        vendor_dir.join("important.txt").exists(),
        "user content was wiped despite --force not being passed"
    );

    // With --force, vendor proceeds and the user content is gone.
    vibe()
        .env("VIBE_REGISTRY_CACHE", &cache)
        .arg("registry")
        .arg("vendor")
        .arg("--out")
        .arg(&vendor_dir)
        .arg("--force")
        .arg("--path")
        .arg(project.path())
        .assert()
        .success();
    assert!(
        !vendor_dir.join("important.txt").exists(),
        "important.txt should be gone after --force vendor"
    );
    assert!(
        vendor_dir.join("flow-wal.git").is_dir(),
        "vendored bare repo missing after --force"
    );
}

/// Build a self-contained local fixture registry that ships a flow
/// package with `[features]` + subskills + describes for feature-aware
/// install testing. Layout:
///
/// ```text
/// registry/flow/feat-pkg/v0.1.0/
/// ├── vibe-package.toml      [features].default = ["base"]
/// │                          [features].base = []
/// │                          [features].with-rust = ["subskill:stack/rust"]
/// │                          [package].describes = "pkg:cargo/sqlx@0.8.0"
/// ├── boot/10-feat-pkg.md
/// ├── spec/feats/feat-pkg/CORE.md
/// └── subskills/
///     ├── stack/rust/
///     │   ├── vibe-subskill.toml   activation.if_present = ["stack:rust"]
///     │   │                        delivery = "eager"
///     │   │                        content.files_written = ["spec/feats/feat-pkg/RUST.md"]
///     │   └── spec/feats/feat-pkg/RUST.md
///     └── doc/extra/
///         ├── vibe-subskill.toml   activation.if_files = ["**/Cargo.toml"]
///         │                        delivery = "eager"
///         │                        content.files_written = ["spec/feats/feat-pkg/EXTRA.md"]
///         └── spec/feats/feat-pkg/EXTRA.md
/// ```
fn make_features_fixture_registry(root: &Path) -> PathBuf {
    let registry = root.join("registry");
    let pkg = registry.join("flow").join("feat-pkg").join("v0.1.0");
    fs::create_dir_all(pkg.join("spec/feats/feat-pkg")).unwrap();
    fs::create_dir_all(pkg.join("boot")).unwrap();
    fs::create_dir_all(pkg.join("subskills/stack/rust/spec/feats/feat-pkg"))
        .unwrap();
    fs::create_dir_all(pkg.join("subskills/doc/extra/spec/feats/feat-pkg"))
        .unwrap();

    fs::write(
        pkg.join("vibe-package.toml"),
        r#"[package]
name = "feat-pkg"
kind = "flow"
version = "0.1.0"
describes = "pkg:cargo/sqlx@0.8.0"

[writes]
files = ["spec/feats/feat-pkg/CORE.md"]

[boot_snippet]
filename = "10-feat-pkg.md"
source = "boot/10-feat-pkg.md"

[features]
default = ["base"]
base = []
with-rust = ["subskill:stack/rust"]
"#,
    )
    .unwrap();
    fs::write(
        pkg.join("spec/feats/feat-pkg/CORE.md"),
        "# CORE protocol",
    )
    .unwrap();
    fs::write(
        pkg.join("boot/10-feat-pkg.md"),
        "# boot snippet",
    )
    .unwrap();

    fs::write(
        pkg.join("subskills/stack/rust/vibe-subskill.toml"),
        r#"[subskill]
path = "stack/rust"
delivery = "eager"

[activation]
if_present = ["stack:rust"]

[content]
files_written = ["spec/feats/feat-pkg/RUST.md"]
"#,
    )
    .unwrap();
    fs::write(
        pkg.join("subskills/stack/rust/spec/feats/feat-pkg/RUST.md"),
        "# Rust-specific guidance",
    )
    .unwrap();

    fs::write(
        pkg.join("subskills/doc/extra/vibe-subskill.toml"),
        r#"[subskill]
path = "doc/extra"
delivery = "eager"

[activation]
if_files = ["**/Cargo.toml"]

[content]
files_written = ["spec/feats/feat-pkg/EXTRA.md"]
"#,
    )
    .unwrap();
    fs::write(
        pkg.join("subskills/doc/extra/spec/feats/feat-pkg/EXTRA.md"),
        "# extra context",
    )
    .unwrap();

    registry
}

#[test]
fn install_with_features_activates_subskill_and_writes_lockfile_metadata() {
    let outer = tempfile::tempdir().unwrap();
    let registry = make_features_fixture_registry(outer.path());

    let project = tempfile::tempdir().unwrap();
    init_project(project.path());

    vibe()
        .arg("install")
        .arg("flow:feat-pkg")
        .arg("--registry")
        .arg(&registry)
        .arg("--path")
        .arg(project.path())
        .arg("--features")
        .arg("with-rust")
        .arg("--assume-yes")
        .assert()
        .success();

    // The base content always materialises.
    assert!(
        project
            .path()
            .join("spec/feats/feat-pkg/CORE.md")
            .is_file(),
        "expected CORE.md present"
    );
    // The subskill `stack/rust` materialises because the feature
    // `with-rust` was activated and its activation list pulled
    // `subskill:stack/rust` (manual channel).
    assert!(
        project
            .path()
            .join("spec/feats/feat-pkg/RUST.md")
            .is_file(),
        "expected RUST.md present from active stack/rust subskill"
    );

    // Lockfile records the active features + subskill + describes.
    let lock_text =
        fs::read_to_string(project.path().join("vibe.lock")).unwrap();
    let lock: vibe_core::manifest::Lockfile = toml::from_str(&lock_text).unwrap();
    let entry = &lock.packages[0];
    assert!(
        entry.features.contains(&"with-rust".to_string()),
        "expected with-rust in features; got {:?}",
        entry.features
    );
    assert!(
        entry.features.contains(&"default".to_string())
            || entry.features.contains(&"base".to_string()),
        "expected default/base activation; got {:?}",
        entry.features
    );
    assert_eq!(entry.subskills_active.len(), 1);
    assert_eq!(entry.subskills_active[0].path, "stack/rust");
    assert_eq!(entry.subskills_active[0].delivery, "eager");
    assert_eq!(
        entry.describes.as_deref(),
        Some("pkg:cargo/sqlx@0.8.0")
    );
}

#[test]
fn install_no_default_features_skips_default_subskills() {
    let outer = tempfile::tempdir().unwrap();
    let registry = make_features_fixture_registry(outer.path());
    let project = tempfile::tempdir().unwrap();
    init_project(project.path());

    vibe()
        .arg("install")
        .arg("flow:feat-pkg")
        .arg("--registry")
        .arg(&registry)
        .arg("--path")
        .arg(project.path())
        .arg("--no-default-features")
        .arg("--assume-yes")
        .assert()
        .success();

    // Base files still ship — they're outside `[features]`. CORE.md is
    // in the package's main `[writes]`, not behind a feature.
    assert!(project.path().join("spec/feats/feat-pkg/CORE.md").is_file());
    // No subskills activated.
    assert!(!project.path().join("spec/feats/feat-pkg/RUST.md").exists());
    let lock: vibe_core::manifest::Lockfile = toml::from_str(
        &fs::read_to_string(project.path().join("vibe.lock")).unwrap(),
    )
    .unwrap();
    assert_eq!(lock.packages[0].subskills_active.len(), 0);
    assert!(
        !lock.packages[0]
            .features
            .iter()
            .any(|f| f == "default" || f == "base"),
        "expected no default/base activation; got {:?}",
        lock.packages[0].features
    );
}

/// Build a per-package git registry hosting three flow packages:
///
/// - `flow:dispatcher` v0.1.0 with a `[target."context(stack:rust)"]`
///   conditional dep on `flow:rust-helper@^0.1`.
/// - `flow:rust-helper` v0.1.0 (the conditional target).
/// - `stack:rust-cli` v0.1.0 (a stack package that the project will
///   install to make the predicate match).
///
/// Returns the org root path and a `git+file://...` URL pointing at it.
fn make_conditional_deps_registry(root: &Path) -> (PathBuf, String) {
    let org = root.join("org");
    fs::create_dir_all(&org).unwrap();

    fn make_pkg(
        out_root: &Path,
        org_dir: &Path,
        kind: &str,
        name: &str,
        version: &str,
        manifest_extras: &str,
        files: &[(&str, &str)],
    ) {
        let src = out_root.join(format!("src-{}-{}", kind, name));
        fs::create_dir_all(&src).unwrap();
        run_git(&src, &["init", "--initial-branch=main"]);
        run_git(&src, &["config", "user.email", "t@example.com"]);
        run_git(&src, &["config", "user.name", "Test"]);
        let writes = files
            .iter()
            .map(|(p, _)| format!("    \"{p}\""))
            .collect::<Vec<_>>()
            .join(",\n");
        let manifest = format!(
            r#"[package]
name = "{name}"
kind = "{kind}"
version = "{version}"

[writes]
files = [
{writes}
]
{manifest_extras}
"#
        );
        fs::write(src.join("vibe-package.toml"), manifest).unwrap();
        for (path, content) in files {
            let target = src.join(path);
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(target, content).unwrap();
        }
        run_git(&src, &["add", "-A"]);
        run_git(&src, &["commit", "-m", &format!("v{version}")]);
        run_git(&src, &["tag", &format!("v{version}")]);

        let bare = org_dir.join(format!("{kind}-{name}.git"));
        run_git(out_root, &[
            "clone",
            "--bare",
            src.to_str().unwrap(),
            bare.to_str().unwrap(),
        ]);
        run_git(&bare, &["symbolic-ref", "HEAD", "refs/heads/main"]);
    }

    make_pkg(
        root,
        &org,
        "flow",
        "dispatcher",
        "0.1.0",
        r#"
[target."context(stack:rust-cli)".dependencies]
packages = ["flow:rust-helper@^0.1"]
"#,
        &[("spec/flows/dispatcher/CORE.md", "# dispatcher core")],
    );
    make_pkg(
        root,
        &org,
        "flow",
        "rust-helper",
        "0.1.0",
        "",
        &[("spec/flows/rust-helper/HINT.md", "# rust hint")],
    );
    make_pkg(
        root,
        &org,
        "stack",
        "rust-cli",
        "0.1.0",
        "",
        &[("spec/stacks/rust-cli/STACK.md", "# rust-cli stack")],
    );

    let abs_org = org
        .canonicalize()
        .unwrap()
        .to_string_lossy()
        .replace('\\', "/")
        .trim_start_matches("//?/")
        .to_string();
    let url = format!("git+file:///{abs_org}");
    (org, url)
}

#[test]
fn install_expands_conditional_dependencies_when_predicate_matches() {
    if !git_available() {
        eprintln!("skipping install_expands_conditional_dependencies: git not on PATH");
        return;
    }
    let outer = tempfile::tempdir().unwrap();
    let (_org, registry_url) = make_conditional_deps_registry(outer.path());

    let project = tempfile::tempdir().unwrap();
    init_project(project.path());
    write_project_with_per_package_registry(project.path(), &registry_url);
    let cache = outer.path().join("cache");
    fs::create_dir_all(&cache).unwrap();

    // Install stack:rust-cli first to make `stack:rust` present in the
    // graph before dispatcher's conditional predicate evaluates.
    vibe()
        .env("VIBE_REGISTRY_CACHE", &cache)
        .arg("install")
        .arg("stack:rust-cli")
        .arg("flow:dispatcher")
        .arg("--path")
        .arg(project.path())
        .arg("--assume-yes")
        .assert()
        .success();

    // The conditional dependency `flow:rust-helper` should have been
    // pulled in as well.
    let lock_text =
        fs::read_to_string(project.path().join("vibe.lock")).unwrap();
    let lock: vibe_core::manifest::Lockfile =
        toml::from_str(&lock_text).unwrap();
    let names: Vec<_> = lock
        .packages
        .iter()
        .map(|p| format!("{}:{}", p.kind, p.name))
        .collect();
    assert!(
        names.iter().any(|n| n == "stack:rust-cli"),
        "expected stack:rust-cli; got {:?}",
        names
    );
    assert!(
        names.iter().any(|n| n == "flow:dispatcher"),
        "expected flow:dispatcher; got {:?}",
        names
    );
    assert!(
        names.iter().any(|n| n == "flow:rust-helper"),
        "expected flow:rust-helper to be pulled in via conditional dep; got {:?}",
        names
    );
}

#[test]
fn conditional_dependencies_dormant_when_predicate_misses() {
    if !git_available() {
        eprintln!("skipping conditional_dependencies_dormant: git not on PATH");
        return;
    }
    let outer = tempfile::tempdir().unwrap();
    let (_org, registry_url) = make_conditional_deps_registry(outer.path());

    let project = tempfile::tempdir().unwrap();
    init_project(project.path());
    write_project_with_per_package_registry(project.path(), &registry_url);
    let cache = outer.path().join("cache");
    fs::create_dir_all(&cache).unwrap();

    // Install dispatcher WITHOUT stack:rust-cli. The conditional
    // predicate `context(stack:rust)` doesn't match → rust-helper
    // stays dormant.
    vibe()
        .env("VIBE_REGISTRY_CACHE", &cache)
        .arg("install")
        .arg("flow:dispatcher")
        .arg("--path")
        .arg(project.path())
        .arg("--assume-yes")
        .assert()
        .success();

    let lock: vibe_core::manifest::Lockfile = toml::from_str(
        &fs::read_to_string(project.path().join("vibe.lock")).unwrap(),
    )
    .unwrap();
    let names: Vec<_> = lock
        .packages
        .iter()
        .map(|p| format!("{}:{}", p.kind, p.name))
        .collect();
    assert!(
        names.iter().any(|n| n == "flow:dispatcher"),
        "got {:?}",
        names
    );
    assert!(
        !names.iter().any(|n| n == "flow:rust-helper"),
        "rust-helper should NOT be installed; got {:?}",
        names
    );
}

/// Build a per-package git registry with two tagged versions of
/// `flow:test-multi`: v0.1.0 (the older release) and v0.2.0 (newer).
/// Returns `(registry_org_root, file_url_for_org)` so tests can wire
/// the registry into a project's `vibe.toml`.
fn make_two_version_per_package_registry(root: &Path) -> (PathBuf, String) {
    let org = root.join("org");
    fs::create_dir_all(&org).unwrap();
    let src = root.join("src-flow-test-multi");
    fs::create_dir_all(src.join("spec/flows/test-multi")).unwrap();
    run_git(&src, &["init", "--initial-branch=main"]);
    run_git(&src, &["config", "user.email", "t@example.com"]);
    run_git(&src, &["config", "user.name", "Test"]);
    fs::write(
        src.join("vibe-package.toml"),
        r#"[package]
name = "test-multi"
kind = "flow"
version = "0.1.0"

[writes]
files = ["spec/flows/test-multi/PROTOCOL.md"]
"#,
    )
    .unwrap();
    fs::write(
        src.join("spec/flows/test-multi/PROTOCOL.md"),
        "# v0.1.0",
    )
    .unwrap();
    run_git(&src, &["add", "-A"]);
    run_git(&src, &["commit", "-m", "v0.1.0"]);
    run_git(&src, &["tag", "v0.1.0"]);

    // Bump to 0.2.0.
    fs::write(
        src.join("vibe-package.toml"),
        r#"[package]
name = "test-multi"
kind = "flow"
version = "0.2.0"

[writes]
files = ["spec/flows/test-multi/PROTOCOL.md"]
"#,
    )
    .unwrap();
    fs::write(
        src.join("spec/flows/test-multi/PROTOCOL.md"),
        "# v0.2.0",
    )
    .unwrap();
    run_git(&src, &["add", "-A"]);
    run_git(&src, &["commit", "-m", "v0.2.0"]);
    run_git(&src, &["tag", "v0.2.0"]);

    let bare = org.join("flow-test-multi.git");
    run_git(root, &[
        "clone",
        "--bare",
        src.to_str().unwrap(),
        bare.to_str().unwrap(),
    ]);
    run_git(&bare, &["symbolic-ref", "HEAD", "refs/heads/main"]);

    let abs_org = org
        .canonicalize()
        .unwrap()
        .to_string_lossy()
        .replace('\\', "/")
        .trim_start_matches("//?/")
        .to_string();
    let url = format!("git+file:///{abs_org}");
    (org, url)
}

#[test]
fn outdated_reports_newer_version_available() {
    if !git_available() {
        eprintln!("skipping outdated_reports_newer_version_available: git not on PATH");
        return;
    }
    let outer = tempfile::tempdir().unwrap();
    let (_org, registry_url) = make_two_version_per_package_registry(outer.path());

    let project = tempfile::tempdir().unwrap();
    init_project(project.path());
    write_project_with_per_package_registry(project.path(), &registry_url);

    let cache = outer.path().join("cache");
    fs::create_dir_all(&cache).unwrap();

    // Install pinned to v0.1.0 explicitly.
    vibe()
        .env("VIBE_REGISTRY_CACHE", &cache)
        .arg("install")
        .arg("flow:test-multi@=0.1.0")
        .arg("--path")
        .arg(project.path())
        .arg("--assume-yes")
        .assert()
        .success();

    let out = vibe()
        .env("VIBE_REGISTRY_CACHE", &cache)
        .arg("--json")
        .arg("outdated")
        .arg("--path")
        .arg(project.path())
        .output()
        .unwrap();
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let v: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("stdout must be JSON");
    assert_eq!(v["command"], "outdated");
    assert_eq!(v["update_available"], 1);
    let pkg = &v["packages"][0];
    assert_eq!(pkg["kind"], "flow");
    assert_eq!(pkg["name"], "test-multi");
    assert_eq!(pkg["installed"], "0.1.0");
    assert_eq!(pkg["latest"], "0.2.0");
    assert_eq!(pkg["status"], "update available");
}

#[test]
fn show_features_lists_active_features_after_install() {
    // After installing with --features, `vibe show features --json`
    // surfaces the activation set per package.
    let outer = tempfile::tempdir().unwrap();
    let registry = make_features_fixture_registry(outer.path());
    let project = tempfile::tempdir().unwrap();
    init_project(project.path());
    vibe()
        .arg("install")
        .arg("flow:feat-pkg")
        .arg("--registry")
        .arg(&registry)
        .arg("--path")
        .arg(project.path())
        .arg("--features")
        .arg("with-rust")
        .arg("--assume-yes")
        .assert()
        .success();
    let out = vibe()
        .arg("--json")
        .arg("show")
        .arg("features")
        .arg("--path")
        .arg(project.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    let v: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("stdout must be JSON");
    assert_eq!(v["command"], "show:features");
    let pkgs = v["packages"].as_array().unwrap();
    assert_eq!(pkgs.len(), 1);
    assert_eq!(pkgs[0]["package"], "flow:feat-pkg");
    let features: Vec<&str> = pkgs[0]["features"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert!(features.contains(&"with-rust"));
}

#[test]
fn show_subskills_and_purls_after_install() {
    let outer = tempfile::tempdir().unwrap();
    let registry = make_features_fixture_registry(outer.path());
    let project = tempfile::tempdir().unwrap();
    init_project(project.path());
    fs::write(project.path().join("Cargo.toml"), "[package]\nname=\"x\"\n")
        .unwrap();
    vibe()
        .arg("install")
        .arg("flow:feat-pkg")
        .arg("--registry")
        .arg(&registry)
        .arg("--path")
        .arg(project.path())
        .arg("--assume-yes")
        .assert()
        .success();

    // show subskills
    let out = vibe()
        .arg("--json")
        .arg("show")
        .arg("subskills")
        .arg("--path")
        .arg(project.path())
        .output()
        .unwrap();
    let v: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("JSON");
    assert_eq!(v["command"], "show:subskills");
    let subs: Vec<&str> = v["packages"][0]["subskills"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["path"].as_str().unwrap())
        .collect();
    assert!(subs.contains(&"doc/extra"));

    // show purls
    let out = vibe()
        .arg("--json")
        .arg("show")
        .arg("purls")
        .arg("--path")
        .arg(project.path())
        .output()
        .unwrap();
    let v: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("JSON");
    assert_eq!(v["command"], "show:purls");
    let bindings = v["bindings"].as_array().unwrap();
    assert!(
        bindings.iter().any(|b| b["purl"] == "pkg:cargo/sqlx@0.8.0"),
        "expected sqlx PURL in bindings; got {:?}",
        bindings
    );
}

#[test]
fn install_subskill_activates_via_if_files_glob() {
    // `doc/extra` subskill activates when project tree contains
    // `Cargo.toml`. Drop one in the project root, install, expect the
    // subskill's content to materialise without any feature flag.
    let outer = tempfile::tempdir().unwrap();
    let registry = make_features_fixture_registry(outer.path());
    let project = tempfile::tempdir().unwrap();
    init_project(project.path());
    fs::write(project.path().join("Cargo.toml"), "[package]\nname=\"x\"\n")
        .unwrap();

    vibe()
        .arg("install")
        .arg("flow:feat-pkg")
        .arg("--registry")
        .arg(&registry)
        .arg("--path")
        .arg(project.path())
        .arg("--assume-yes")
        .assert()
        .success();

    assert!(
        project
            .path()
            .join("spec/feats/feat-pkg/EXTRA.md")
            .is_file(),
        "expected EXTRA.md present from doc/extra subskill activated by if_files"
    );
    // Lockfile records the activation channel via subskills_active.
    let lock: vibe_core::manifest::Lockfile = toml::from_str(
        &fs::read_to_string(project.path().join("vibe.lock")).unwrap(),
    )
    .unwrap();
    let paths: Vec<_> = lock.packages[0]
        .subskills_active
        .iter()
        .map(|s| s.path.as_str())
        .collect();
    assert!(
        paths.contains(&"doc/extra"),
        "expected doc/extra in subskills_active; got {:?}",
        paths
    );
}

/// Build a self-contained local fixture registry that ships a flow
/// package whose `morning-routine.md` carries a Russian sidecar
/// (`morning-routine.ru.md`). Returns the registry root path and the
/// expected canonical / Russian text bytes for assertions.
fn make_i18n_fixture_registry(root: &Path) -> (PathBuf, &'static str, &'static str) {
    let registry = root.join("registry");
    let pkg_dir = registry.join("flow").join("hello-i18n").join("v0.1.0");
    fs::create_dir_all(pkg_dir.join("spec/flows/hello-i18n")).unwrap();
    fs::create_dir_all(pkg_dir.join("boot")).unwrap();

    fs::write(
        pkg_dir.join("vibe-package.toml"),
        r#"[package]
name = "hello-i18n"
kind = "flow"
version = "0.1.0"

[i18n]
canonical = "en"
available = ["en", "ru"]

[writes]
files = [
    "spec/flows/hello-i18n/PROTOCOL.md",
]

[boot_snippet]
filename = "50-hello-i18n.md"
source = "boot/50-hello-i18n.md"
"#,
    )
    .unwrap();

    fs::write(
        pkg_dir.join("spec/flows/hello-i18n/PROTOCOL.md"),
        "# Hello protocol (canonical English)\n",
    )
    .unwrap();
    fs::write(
        pkg_dir.join("spec/flows/hello-i18n/PROTOCOL.ru.md"),
        "# Привет, протокол (русская версия)\n",
    )
    .unwrap();
    fs::write(
        pkg_dir.join("boot/50-hello-i18n.md"),
        "Boot snippet (English).\n",
    )
    .unwrap();
    fs::write(
        pkg_dir.join("boot/50-hello-i18n.ru.md"),
        "Boot snippet (русская версия).\n",
    )
    .unwrap();

    (
        registry,
        "# Hello protocol (canonical English)\n",
        "# Привет, протокол (русская версия)\n",
    )
}

#[test]
fn install_with_language_flag_picks_localised_content() {
    // PROP-003 §2.7 i18n end-to-end. The fixture package ships canonical
    // English plus a Russian sidecar. `vibe install --language ru`
    // materialises the Russian content; `vibe install` (no flag)
    // materialises the canonical form.
    let outer = tempfile::tempdir().unwrap();
    let (registry, en_text, ru_text) = make_i18n_fixture_registry(outer.path());

    let project = tempfile::tempdir().unwrap();
    init_project(project.path());

    vibe()
        .arg("install")
        .arg("flow:hello-i18n")
        .arg("--registry")
        .arg(&registry)
        .arg("--path")
        .arg(project.path())
        .arg("--language")
        .arg("ru")
        .arg("--assume-yes")
        .assert()
        .success();

    // Canonical target path on disk — `.ru.` segment NOT preserved.
    let materialised = fs::read_to_string(
        project.path().join("spec/flows/hello-i18n/PROTOCOL.md"),
    )
    .unwrap();
    assert_eq!(
        materialised, ru_text,
        "expected Russian content, got:\n{materialised}"
    );

    // Boot snippet also localised.
    let boot = fs::read_to_string(
        project.path().join("spec/boot/50-hello-i18n.md"),
    )
    .unwrap();
    assert!(boot.contains("русская версия"));

    // Without --language, canonical English wins.
    let project2 = tempfile::tempdir().unwrap();
    init_project(project2.path());
    vibe()
        .arg("install")
        .arg("flow:hello-i18n")
        .arg("--registry")
        .arg(&registry)
        .arg("--path")
        .arg(project2.path())
        .arg("--assume-yes")
        .assert()
        .success();
    let materialised_en = fs::read_to_string(
        project2.path().join("spec/flows/hello-i18n/PROTOCOL.md"),
    )
    .unwrap();
    assert_eq!(materialised_en, en_text);
}

#[test]
fn install_with_language_falls_back_to_canonical_when_translation_missing() {
    // Project requests Japanese; package ships only English + Russian.
    // Per PROP-003 §2.7.2 the resolution chain falls through to the
    // canonical English content rather than failing.
    let outer = tempfile::tempdir().unwrap();
    let (registry, en_text, _ru_text) = make_i18n_fixture_registry(outer.path());

    let project = tempfile::tempdir().unwrap();
    init_project(project.path());
    vibe()
        .arg("install")
        .arg("flow:hello-i18n")
        .arg("--registry")
        .arg(&registry)
        .arg("--path")
        .arg(project.path())
        .arg("--language")
        .arg("ja")
        .arg("--assume-yes")
        .assert()
        .success();
    let materialised = fs::read_to_string(
        project.path().join("spec/flows/hello-i18n/PROTOCOL.md"),
    )
    .unwrap();
    assert_eq!(
        materialised, en_text,
        "fallback to canonical English failed: got\n{materialised}"
    );
}

#[test]
fn set_mirror_accepts_file_url_with_no_org_segment() {
    // Regression defence for the 2026-05-04 walk of
    // manual-tests/M1.6-mirror-vendor-smoke.md Scenario A4. Earlier
    // `run_set_mirror` ran `extract_host_segment` + `extract_org_segment`
    // on the mirror URL — the same gate that `[[registry]]` URLs go
    // through. That refused `file:///<dir>` URLs because they have no
    // host or org segment, even though `vibe registry vendor` produces
    // exactly that URL shape and recommends it as a `[[mirror]]`. This
    // test pins the post-fix shape: a `file:///` URL is accepted, the
    // manifest learns a new `[[mirror]]` block, and the URL is recorded
    // verbatim.
    let project = tempfile::tempdir().unwrap();
    init_project(project.path());

    let vendor_dir = tempfile::tempdir().unwrap();
    let abs_vendor = vendor_dir.path().to_string_lossy().replace('\\', "/");
    let mirror_url = format!("file:///{abs_vendor}");

    vibe()
        .arg("registry")
        .arg("set-mirror")
        .arg("vibespecs")
        .arg(&mirror_url)
        .arg("--path")
        .arg(project.path())
        .assert()
        .success();

    let manifest = fs::read_to_string(project.path().join("vibe.toml")).unwrap();
    assert!(
        manifest.contains("[[mirror]]"),
        "expected [[mirror]] block in manifest; got:\n{manifest}"
    );
    assert!(
        manifest.contains(&mirror_url),
        "expected mirror URL `{mirror_url}` in manifest; got:\n{manifest}"
    );
}

#[test]
fn set_mirror_rejects_empty_url() {
    // The pre-fix gate accidentally caught empty URLs as a side effect
    // of `extract_host_segment` failing on them. The post-fix gate is
    // intentionally narrow: non-empty after trim. Pin both the empty
    // and the whitespace-only case so a refactor that loosens the
    // check doesn't sneak through unnoticed.
    let project = tempfile::tempdir().unwrap();
    init_project(project.path());

    for bad in ["", "   "] {
        vibe()
            .arg("registry")
            .arg("set-mirror")
            .arg("vibespecs")
            .arg(bad)
            .arg("--path")
            .arg(project.path())
            .assert()
            .failure()
            .stderr(predicate::str::contains("mirror URL must be non-empty"));
    }
}

// ---------------------------------------------------------------------------
// Help-text smoke — every CLI subcommand renders `--help`
// ---------------------------------------------------------------------------
//
// Regression defence for the clap derive on `crates/vibe-cli/src/cli.rs`. A
// silently-empty help text on a subcommand has happened in other Rust CLIs
// when a `#[command]` attribute drifts (missing `about` on a fresh
// subcommand, mistyped `subcommand` attribute on a parent, etc.) — the CLI
// still parses and runs, but `--help` returns blank and confuses users.
// This test catches that the next time someone adds a subcommand without
// also adding its docstring.
//
// Each entry is a path to a help target. `--help` is appended; the subcommand
// itself never runs (clap short-circuits on `--help`), so no project setup
// or network access is needed.

#[test]
fn every_subcommand_renders_help() {
    // Path lists for every help surface clap exposes today. When a new
    // subcommand lands in `crates/vibe-cli/src/cli.rs`, add it here too —
    // the help-text contract is part of the user-facing surface.
    let subcommand_paths: &[&[&str]] = &[
        &[],                            // top-level: `vibe --help`
        &["init"],
        &["install"],
        &["list"],
        &["outdated"],
        &["uninstall"],
        &["update"],
        &["check"],
        &["show"],
        &["show", "effective"],
        &["show", "config"],
        &["show", "features"],
        &["show", "subskills"],
        &["show", "purls"],
        &["registry"],                  // shows the registry subcommand enum
        &["registry", "sync"],
        &["registry", "publish"],
        &["registry", "list"],
        &["registry", "add"],
        &["registry", "set-mirror"],
        &["registry", "remove"],
        &["registry", "vendor"],
        &["version"],
    ];

    for path in subcommand_paths {
        let mut cmd = vibe();
        for seg in *path {
            cmd.arg(seg);
        }
        cmd.arg("--help");
        let out = cmd.output().unwrap_or_else(|e| {
            panic!(
                "spawning `vibe {} --help` failed: {e}",
                path.join(" ")
            )
        });

        let label = if path.is_empty() {
            "vibe --help".to_string()
        } else {
            format!("vibe {} --help", path.join(" "))
        };

        assert!(
            out.status.success(),
            "`{label}` should exit 0 — got {:?}, stderr: {}",
            out.status.code(),
            String::from_utf8_lossy(&out.stderr)
        );

        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(
            !stdout.trim().is_empty(),
            "`{label}` produced empty stdout — clap derive likely lost the docstring"
        );
        // Cheap sanity: every help screen mentions usage. Catches the
        // "wrong binary got invoked" scenario without coupling to wording.
        assert!(
            stdout.to_lowercase().contains("usage"),
            "`{label}` stdout did not contain `Usage` — got:\n{stdout}"
        );
    }
}

#[test]
fn version_subcommand_matches_version_flag() {
    // `vibe version` and `vibe --version` are documented as identical
    // (see docs/commands/version.md). Drift between the two would confuse
    // any tooling that scrapes the version string.
    let sub = vibe().arg("version").output().expect("spawn vibe version");
    let flag = vibe().arg("--version").output().expect("spawn vibe --version");
    assert!(sub.status.success() && flag.status.success());
    let sub_out = String::from_utf8_lossy(&sub.stdout).trim().to_string();
    let flag_out = String::from_utf8_lossy(&flag.stdout).trim().to_string();
    assert_eq!(
        sub_out, flag_out,
        "`vibe version` and `vibe --version` must produce identical output"
    );
    assert!(
        sub_out.starts_with("vibe "),
        "version output should start with `vibe `, got: {sub_out}"
    );
}
