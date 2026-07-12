//! Shared helpers for the `vibe-cli` integration-test binaries.
//!
//! The fixture registry most tests run against is the hand-written
//! `fixtures/registry/` tree that ships in the vibevm repo itself (the
//! canonical `org.vibevm/wal` fixture per `VIBEVM-SPEC.md` §13). The git
//! builders below construct per-package bare registries, single-package
//! repos, and redirect stubs in temp dirs for the hermetic git-backed walks.
//!
//! Each test binary compiles its own copy of this module and uses only a
//! subset of the helpers, so dead-code analysis is silenced for the module
//! as a whole.
#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;

pub fn vibe() -> Command {
    Command::cargo_bin("vibe").expect("vibe binary built")
}

/// The `fixtures/registry/` directory at the repo root holds the
/// hermetic fixture registry the e2e tests run against. Layout is the
/// M0/M1.1 monorepo shape (`<kind>/<name>/v<ver>/…`); the directory
/// is intentionally separate from the future `packages/` tree (where
/// vibevm dogfoods its own packages — keeps test fixtures and live
/// artefacts visually distinct).
pub fn fixture_registry() -> PathBuf {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace = crate_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root");
    workspace.join("fixtures").join("registry")
}

pub fn init_project(dir: &Path) {
    vibe().arg("init").arg("--path").arg(dir).assert().success();
}

pub fn git_available() -> bool {
    std::process::Command::new("git")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

pub fn run_git(cwd: &Path, args: &[&str]) {
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
/// For this test we seed exactly one package: `org.vibevm/wal@0.1.0` →
/// `<root>/org.vibevm_wal.git`. The "registry" is then `<root>` itself —
/// `MultiRegistryResolver` composes per-package URLs by appending
/// `<group>_<name>.git` to the org URL (the `fqdn` naming convention,
/// PROP-008 §3).
///
/// Returns the org root path (not any single repo), since the install
/// flow points `[[registry]]` at the org URL.
pub fn make_per_package_registry(root: &Path) -> PathBuf {
    let src = root.join("src-flow-wal");
    fs::create_dir_all(&src).unwrap();
    run_git(&src, &["init", "--initial-branch=main"]);
    run_git(&src, &["config", "user.email", "t@example.com"]);
    run_git(&src, &["config", "user.name", "Test"]);

    // Per-package layout: package contents live AT THE ROOT of the repo,
    // not under `<group>/<name>/v<ver>/`.
    copy_tree(&fixture_registry().join("org.vibevm/wal/v0.1.0"), &src);
    run_git(&src, &["add", "-A"]);
    run_git(&src, &["commit", "-m", "org.vibevm/wal@0.1.0"]);
    run_git(&src, &["tag", "v0.1.0"]);

    let bare = root.join("org.vibevm_wal.git");
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

pub fn copy_tree(src: &Path, dst: &Path) {
    for entry in walkdir::WalkDir::new(src)
        .into_iter()
        .filter_map(|e| e.ok())
    {
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

pub fn write_project_with_per_package_registry(project_dir: &Path, registry_url: &str) {
    // [[registry]] in PROP-002 shape, pointing at the per-package org URL.
    // `naming` defaults to `fqdn` — repos resolve as `<group>_<name>.git`
    // (PROP-008 §3).
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

/// Build a single-package bare git repo (NOT under an org) usable
/// as a `vibe install --git ...` target. The repo's URL is the URL
/// of the bare clone itself; vibevm's M1.15 git-source path treats
/// it as a one-package "registry" without applying naming.
pub fn make_single_package_bare_repo(root: &Path) -> PathBuf {
    let src = root.join("src-flow-wal-direct");
    fs::create_dir_all(&src).unwrap();
    run_git(&src, &["init", "--initial-branch=main"]);
    run_git(&src, &["config", "user.email", "t@example.com"]);
    run_git(&src, &["config", "user.name", "Test"]);
    copy_tree(&fixture_registry().join("org.vibevm/wal/v0.1.0"), &src);
    run_git(&src, &["add", "-A"]);
    run_git(&src, &["commit", "-m", "org.vibevm/wal@0.1.0"]);
    run_git(&src, &["tag", "v0.1.0"]);
    let bare = root.join("flow-wal-direct.git");
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
    bare
}

/// Build a single-package bare repo carrying `vibe-redirect.toml` (NOT
/// `vibe.toml`). Used by the redirect-stub tests as the slot a
/// registry org's per-package walk lands on; the resolver detects the
/// marker and follows it to the target.
///
/// `repo_name` is the directory the bare clone lands in (which becomes
/// the `<kind>-<name>` slot under the org root once you place it there).
pub fn make_redirect_stub_bare_repo(
    root: &Path,
    repo_name: &str,
    target_url: &str,
    ref_policy: &str,
    pinned_ref: Option<&str>,
    tags: &[&str],
) -> PathBuf {
    let src = root.join(format!("src-stub-{repo_name}"));
    fs::create_dir_all(&src).unwrap();
    run_git(&src, &["init", "--initial-branch=main"]);
    run_git(&src, &["config", "user.email", "stub@example.com"]);
    run_git(&src, &["config", "user.name", "Stub"]);

    let mut marker = format!("[redirect]\ntarget_url = \"{target_url}\"\n");
    if ref_policy != "pass-through-tag" {
        marker.push_str(&format!("ref_policy = \"{ref_policy}\"\n"));
    }
    if let Some(r) = pinned_ref {
        marker.push_str(&format!("pinned_ref = \"{r}\"\n"));
    }
    fs::write(src.join("vibe-redirect.toml"), marker).unwrap();
    fs::write(
        src.join("README.md"),
        format!("# stub for {repo_name}\nDelegates to {target_url}\n"),
    )
    .unwrap();
    run_git(&src, &["add", "-A"]);
    run_git(&src, &["commit", "-m", &format!("stub: {repo_name}")]);
    for t in tags {
        run_git(&src, &["tag", t]);
    }

    let bare = root.join(format!("{repo_name}.git"));
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
    bare
}

/// Write a workspace root `vibe.toml` carrying `[project]` + `[workspace]`
/// plus a single `[[registry]]` (GitHub-shaped so URL parsing succeeds;
/// dry-run never calls the network).
pub fn write_workspace_root(dir: &Path, members: &[&str]) {
    let list = members
        .iter()
        .map(|m| format!("\"{m}\""))
        .collect::<Vec<_>>()
        .join(", ");
    fs::write(
        dir.join("vibe.toml"),
        format!(
            "[project]\nname = \"mono\"\nversion = \"0.0.1\"\n\n\
             [workspace]\nmembers = [{list}]\n\n\
             [[registry]]\nname = \"vibespecs\"\nurl = \"https://github.com/vibespecs\"\n"
        ),
    )
    .unwrap();
}

/// Write a member package `vibe.toml`. `publish` is the raw TOML value
/// for the `publish` field (`"true"`, `"false"`, `"[\"vibespecs\"]"`),
/// or empty to omit the field (default = published).
pub fn write_member_pkg(dir: &Path, rel: &str, name: &str, kind: &str, publish: &str) {
    let publish_line = if publish.is_empty() {
        String::new()
    } else {
        format!("publish = {publish}\n")
    };
    let path = dir.join(rel).join("vibe.toml");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(
        path,
        format!(
            "[package]\ngroup = \"org.vibevm\"\nname = \"{name}\"\nkind = \"{kind}\"\nversion = \"0.1.0\"\n{publish_line}"
        ),
    )
    .unwrap();
}
