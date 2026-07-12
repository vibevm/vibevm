//! Shared helpers for the `vibe-cli` integration-test binaries.
//!
//! The `wal` integration tests dogfood the real `org.vibevm.world/wal`
//! package that ships in this repo at `packages/org.vibevm.world/wal/`
//! (the loading model installs the actual product, not a stale mini-copy
//! fixture). The non-`wal` tests still run against the hand-written
//! `fixtures/registry/` tree. The git builders below construct per-package
//! bare registries, single-package repos, and redirect stubs in temp dirs
//! for the hermetic git-backed walks.
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
/// hermetic fixture registry the non-`wal` e2e tests run against.
/// Layout is the monorepo shape (`<group>/<name>/v<ver>/…`). The `wal`
/// tests instead dogfood the real `org.vibevm.world/wal` package from
/// `packages/` — see [`real_wal_dir`] / [`make_wal_dir_registry`].
pub fn fixture_registry() -> PathBuf {
    workspace_root().join("fixtures").join("registry")
}

/// The vibevm workspace root: two `parent()`s up from this crate's
/// manifest dir (`crates/vibe-cli` → workspace). Same computation
/// [`fixture_registry`] builds on.
pub fn workspace_root() -> PathBuf {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    crate_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root")
        .to_path_buf()
}

/// The real `org.vibevm.world/wal@0.2.0` package as it ships in this
/// repo — the tree the `wal` e2e tests dogfood rather than a fixture.
fn real_wal_dir() -> PathBuf {
    workspace_root().join("packages/org.vibevm.world/wal/v0.2.0")
}

/// Build a directory registry under `<root>/wal-registry/` carrying the
/// real `org.vibevm.world/wal@0.2.0` package, copied verbatim from
/// [`real_wal_dir`]. Returns the registry dir (`<root>/wal-registry`) so
/// it can be passed straight to `vibe install --registry <dir>`.
pub fn make_wal_dir_registry(root: &Path) -> PathBuf {
    let registry = root.join("wal-registry");
    let pkg = registry.join("org.vibevm.world").join("wal").join("v0.2.0");
    copy_tree(&real_wal_dir(), &pkg);
    registry
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
/// For this test we seed exactly one package: `org.vibevm.world/wal@0.2.0`
/// → `<root>/org.vibevm.world_wal.git`. The "registry" is then `<root>`
/// itself — `MultiRegistryResolver` composes per-package URLs by
/// appending `<group>_<name>.git` to the org URL (the `fqdn` naming
/// convention, PROP-008 §3).
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
    // not under `<group>/<name>/v<ver>/`. Seed it from the real
    // `org.vibevm.world/wal@0.2.0` package (dogfood, not a fixture).
    copy_tree(&real_wal_dir(), &src);
    run_git(&src, &["add", "-A"]);
    run_git(&src, &["commit", "-m", "org.vibevm.world/wal@0.2.0"]);
    run_git(&src, &["tag", "v0.2.0"]);

    let bare = root.join("org.vibevm.world_wal.git");
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
    copy_tree(&real_wal_dir(), &src);
    run_git(&src, &["add", "-A"]);
    run_git(&src, &["commit", "-m", "org.vibevm.world/wal@0.2.0"]);
    run_git(&src, &["tag", "v0.2.0"]);
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
