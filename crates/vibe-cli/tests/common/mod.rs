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
//! subset of the helpers, so dead-code analysis — and, for the
//! re-exported `vibe-test-support` names, unused-import analysis — is
//! silenced for the module as a whole.
#![allow(dead_code, unused_imports)]

use std::fs;
use std::path::{Path, PathBuf};

/// `vibe()` and `UserScratch` moved into `vibe-test-support` (DRIFT-020) so
/// `vibe-index`'s test binaries can reach them too, and — the point of the
/// move — so that *linking* that crate isolates this test process's settings
/// home before the first `#[test]` runs. Re-exported under their old names:
/// the callers here are unchanged, and the `use common::UserScratch` sites
/// keep working.
pub use vibe_test_support::{UserScratch, vibe};

/// The `fixtures/registry/` directory at the repo root holds the
/// hermetic fixture registry the non-`wal` e2e tests run against.
/// Layout is the monorepo shape (`<group>/<name>/v<ver>/…`). The `wal`
/// tests instead dogfood the real `org.vibevm.world/wal` package from
/// the repo's packages root — see [`real_wal_dir`] /
/// [`make_wal_dir_registry`].
pub fn fixture_registry() -> PathBuf {
    workspace_root().join("fixtures").join("registry")
}

// ---- Layout helpers (PROP-052 L2) ---------------------------------------
//
// Every scaffold path these test binaries write or assert on routes
// through `vibe_core::layout`, so the R4 flip (one `USE_NEW_LAYOUT`
// edit) carries the whole integration suite without touching it.

/// The live specs root as a `PathBuf` (`spec/` today).
pub fn specs_root() -> PathBuf {
    vibe_core::layout::current_specs_root()
}

/// The live packages root as a `PathBuf` (`packages/` today).
pub fn packages_root() -> PathBuf {
    vibe_core::layout::current_packages_root()
}

/// The live dependency-slot root as a `PathBuf` (`vibedeps/` today).
pub fn deps_root() -> PathBuf {
    vibe_core::layout::current_vibedeps_root()
}

/// The live boot-lane directory as a `PathBuf` (`spec/boot/` today).
pub fn boot_dir() -> PathBuf {
    vibe_core::layout::current_boot_dir()
}

/// The live boot manifest, forward-slashed (`spec/boot/INDEX.md` today).
pub fn index_rel() -> String {
    vibe_core::machine_json_path(&vibe_core::layout::current_boot_index())
}

/// The live Markdown WAL path as a `PathBuf` (`spec/WAL.md` today).
pub fn wal_md() -> PathBuf {
    vibe_core::layout::current_wal_md()
}

/// The live facts root as a `PathBuf` (`vibefacts/` today).
pub fn facts_root() -> PathBuf {
    vibe_core::layout::current_vibefacts_root()
}

/// One file under the live facts root, forward-slashed
/// (`vibefacts/<tail>` today).
pub fn facts_rel(tail: &str) -> String {
    vibe_core::machine_json_path(&facts_root().join(tail))
}

/// The generated static lane, Markdown spelling, forward-slashed
/// (`spec/boot/STATIC.md` today).
pub fn static_md_rel() -> String {
    vibe_core::machine_json_path(&vibe_core::layout::current_boot_static_md())
}

/// The generated static lane, XML spelling, forward-slashed
/// (`spec/boot/STATIC.xml` today).
pub fn static_xml_rel() -> String {
    vibe_core::machine_json_path(&vibe_core::layout::current_boot_static_xml())
}

/// The live specs root as a forward-slashed string (`spec` today) —
/// for include-globs and other `format!`-built fixture bodies.
pub fn specs_str() -> String {
    vibe_core::machine_json_path(&specs_root())
}

/// The live boot lane as a forward-slashed string (`spec/boot` today).
pub fn boot_str() -> String {
    vibe_core::machine_json_path(&boot_dir())
}

/// One project-relative path under the live specs root,
/// forward-slashed (`spec/<tail>` today).
pub fn spec_rel(tail: &str) -> String {
    vibe_core::machine_json_path(&specs_root().join(tail))
}

/// One boot-lane file, forward-slashed (`spec/boot/<file>` today).
pub fn boot_rel(file: &str) -> String {
    vibe_core::machine_json_path(&boot_dir().join(file))
}

/// One project-relative path under the live packages root,
/// forward-slashed (`packages/<tail>` today).
pub fn pack_rel(tail: &str) -> String {
    vibe_core::machine_json_path(&packages_root().join(tail))
}

/// A materialised slot directory, forward-slashed
/// (`vibedeps/<slot>/<version>` today).
pub fn slot_dir(slot: &str, version: &str) -> String {
    vibe_core::machine_json_path(&deps_root().join(slot).join(version))
}

/// A file inside a materialised slot, forward-slashed
/// (`vibedeps/<slot>/<version>/<tail>` today).
pub fn slot_rel(slot: &str, version: &str, tail: impl AsRef<str>) -> String {
    vibe_core::machine_json_path(&deps_root().join(slot).join(version).join(tail.as_ref()))
}

/// A package's boot-lane file inside its materialised slot,
/// forward-slashed (`vibedeps/<slot>/<version>/spec/boot/<file>` today).
pub fn slot_boot_rel(slot: &str, version: &str, file: &str) -> String {
    vibe_core::machine_json_path(
        &deps_root()
            .join(slot)
            .join(version)
            .join(boot_dir())
            .join(file),
    )
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
/// Routed through the layout module so the R4 flip keeps the dogfood
/// pointing at the moved tree.
fn real_wal_dir() -> PathBuf {
    workspace_root()
        .join(packages_root())
        .join("org.vibevm.world/wal/v0.2.0")
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

/// Run `git <args>` in `cwd`, panicking with the FULL failure picture
/// on a non-zero exit (B-075): the rendered argv, the raw exit status
/// (Windows abnormal-termination codes decoded — a bare `-1073741819`
/// read as a git exit code is how a flake turns undiagnosable), git's
/// ENTIRE stdout and stderr, the cwd, and — for `clone`, where a
/// half-written destination is the decisive evidence a silent
/// `exit 1` leaves behind — whether the destination exists and a
/// listing of what is inside it. Every fixture-building git call in
/// these test binaries goes through here, so one loud message covers
/// them all.
pub fn run_git(cwd: &Path, args: &[&str]) {
    let out = run_git_output(cwd, args);
    assert!(
        out.status.success(),
        "{}",
        render_git_failure(cwd, args, &out)
    );
}

/// [`run_git`] for callers that need the successful output back (tag
/// listings, `ls-remote`, verification clones). Panishes identically
/// on failure; on success returns git's `Output` verbatim.
pub fn run_git_output(cwd: &Path, args: &[&str]) -> std::process::Output {
    let out = std::process::Command::new("git")
        .args(args)
        .current_dir(cwd)
        .env("LC_ALL", "C")
        .env("LANG", "C")
        .output()
        .unwrap_or_else(|e| {
            panic!(
                "failed to spawn `git {}` in {}: {e}",
                args.join(" "),
                cwd.display()
            )
        });
    if !out.status.success() {
        panic!("{}", render_git_failure(cwd, args, &out));
    }
    out
}

/// Render the full B-075 failure picture for a git invocation that
/// exited non-zero. Everything the reporter of a one-in-a-workspace-run
/// flake needs, in one panic message: what ran, where, how it died,
/// both streams in full, and (for clones) the state of the destination.
fn render_git_failure(cwd: &Path, args: &[&str], out: &std::process::Output) -> String {
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    let mut msg = format!(
        "git invocation failed.\n\
         \x20 command : git {}\n\
         \x20 cwd     : {}\n\
         \x20 exit    : {}\n\
         \x20 stdout  : <{} bytes>\n{}\n\
         \x20 stderr  : <{} bytes>\n{}",
        args.join(" "),
        cwd.display(),
        render_exit_status(&out.status),
        stdout.len(),
        indent_stream(&stdout),
        stderr.len(),
        indent_stream(&stderr),
    );
    // For `clone` the destination directory is the one artifact a
    // silent failure leaves: a half-populated tree, a stray `.git`,
    // or nothing at all each point at a different failure family.
    if args.first().is_some_and(|a| *a == "clone")
        && !args.last().is_some_and(|a| a.starts_with('-'))
    {
        let dest = Path::new(args.last().expect("clone argv tail"));
        let dest = if dest.is_absolute() {
            dest.to_path_buf()
        } else {
            cwd.join(dest)
        };
        msg.push_str(&format!(
            "\n\x20 clone destination `{}`: {}",
            dest.display(),
            describe_path(&dest)
        ));
    }
    msg
}

/// The exit status as a diagnostic, not just a number: NTSTATUS-style
/// abnormal terminations arrive as large negative `i32`s and are named
/// here, because `git` itself never exits with those codes.
fn render_exit_status(status: &std::process::ExitStatus) -> String {
    match status.code() {
        Some(code) => {
            let named = match code as u32 {
                0xC000_0005 => " (0xC0000005 — access violation)",
                0xC000_0409 => " (0xC0000409 — stack buffer overrun / fail-fast)",
                0x4000_0015 => " (0x40000015 — fatal application exit)",
                0xC000_0135 => " (0xC0000135 — DLL initialization failure)",
                0xC000_0142 => " (0xC0000142 — DLL initialization failed)",
                _ => "",
            };
            format!("{code}{named}")
        }
        None => format!("{status} (terminated by signal; no exit code)"),
    }
}

/// Prefix every line of a captured stream so it reads as a block
/// inside the panic message; an empty stream says so explicitly
/// instead of printing nothing (an empty stderr IS the symptom B-075
/// chased — it must be visible as empty, not absent).
fn indent_stream(text: &str) -> String {
    let trimmed = text.trim_end();
    if trimmed.is_empty() {
        "<empty>".to_string()
    } else {
        trimmed
            .lines()
            .map(|l| format!("    | {l}"))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// One line on whether `path` exists and what it is; a directory gets
/// a short sorted listing so a half-written clone destination shows
/// its contents (`.git` only? packed but unchecked out? empty?).
fn describe_path(path: &Path) -> String {
    match fs::metadata(path) {
        Err(e) => format!("absent (lookup error: {e})"),
        Ok(m) if m.is_dir() => {
            let mut names: Vec<String> = fs::read_dir(path)
                .map(|rd| {
                    rd.filter_map(|e| e.ok())
                        .map(|e| e.file_name().to_string_lossy().into_owned())
                        .collect()
                })
                .unwrap_or_default();
            names.sort();
            let total = names.len();
            let noun = if total == 1 { "entry" } else { "entries" };
            let listing = names.into_iter().take(25).collect::<Vec<_>>().join(", ");
            if listing.is_empty() {
                format!("directory, {total} {noun}")
            } else {
                format!("directory, {total} {noun}: {listing}")
            }
        }
        Ok(m) if m.is_file() => format!("file, {} bytes", m.len()),
        Ok(_) => "exists (neither file nor directory)".to_string(),
    }
}

/// Build a per-package bare git registry under `root/`: one bare repo
/// per package, content at the repo root, tagged `v<semver>`.
///
/// For this test we seed exactly one package: `org.vibevm.world/wal@0.2.0`
/// → `<root>/org.vibevm.world.wal.git`. The "registry" is then `<root>`
/// itself — `MultiRegistryResolver` composes per-package URLs by
/// appending `<group>.<name>.git` to the org URL (the `fqdn` naming
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

    let bare = root.join("org.vibevm.world.wal.git");
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
    // `naming` defaults to `fqdn` — repos resolve as `<group>.<name>.git`
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
