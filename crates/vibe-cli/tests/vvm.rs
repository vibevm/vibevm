//! Integration tests for `vibe self` (PROP-019), driven through the real
//! binary. The install root is pinned inside a temp dir via
//! `VIBEVM_INSTALL_ROOT`, so nothing here ever touches the developer's
//! real `~/opt` (PROP-019 §2.4), and the settings home is pinned away
//! from `~/.vibe` by linking `vibe-test-support` (DRIFT-020).

use std::path::Path;
use std::process::Command as Sys;
use std::sync::{Mutex, MutexGuard};

use assert_cmd::Command;
use tempfile::TempDir;

static FIXTURE_PROCESS_LOCK: Mutex<()> = Mutex::new(());

// Serialize every fixture's copies AND process launches. A concurrent fork
// can inherit a write descriptor during fs::copy and retain it until exec,
// causing ETXTBSY even after the copying thread closed its descriptor:
// https://github.com/rust-lang/rust/issues/114554
fn fixture_process_guard() -> MutexGuard<'static, ()> {
    FIXTURE_PROCESS_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// A `vibe` invocation with the install root pinned to `base` and no
/// ambient `VIBEVM_HOME` leaking in.
fn vibe(base: &Path) -> Command {
    let mut cmd = vibe_test_support::vibe();
    cmd.env("VIBEVM_INSTALL_ROOT", base)
        .env_remove("VIBEVM_HOME");
    cmd
}

/// Copy the real test binary below `source/target/` so current_exe provenance
/// identifies that worktree rather than relying on (and accidentally trusting)
/// `current_dir`.
fn source_vibe(base: &Path, source: &Path) -> Command {
    let probe = vibe_test_support::vibe();
    let destination = source.join("target").join("bootstrap").join(bin_name());
    std::fs::create_dir_all(destination.parent().unwrap()).unwrap();
    std::fs::copy(probe.get_program(), &destination).unwrap();
    let mut cmd = Command::new(destination);
    cmd.env("VIBEVM_INSTALL_ROOT", base)
        .env_remove("VIBEVM_HOME")
        .current_dir(source);
    cmd
}

fn bin_name() -> &'static str {
    if cfg!(windows) { "vibe.exe" } else { "vibe" }
}

#[cfg(not(windows))]
fn index_bin_name() -> &'static str {
    if cfg!(windows) {
        "vibe-index.exe"
    } else {
        "vibe-index"
    }
}

#[cfg(not(windows))]
fn proxy_bin_name() -> &'static str {
    "vibe-app-proxy"
}

#[test]
fn ls_on_a_fresh_root_still_identifies_the_direct_source_execution() {
    let _fixture_guard = fixture_process_guard();
    let base = TempDir::new().unwrap();
    let src = TempDir::new().unwrap();
    write_tiny_source(src.path());
    source_vibe(base.path(), src.path())
        .args(["self", "ls"])
        .assert()
        .success()
        .stdout(predicates::str::contains("> source origin=external"))
        .stdout(predicates::str::contains("0 instance(s) installed"));
}

#[test]
fn which_reports_the_direct_source_executable_without_an_active_version() {
    let _fixture_guard = fixture_process_guard();
    let base = TempDir::new().unwrap();
    let src = TempDir::new().unwrap();
    write_tiny_source(src.path());
    source_vibe(base.path(), src.path())
        .args(["self", "which"])
        .assert()
        .success()
        .stdout(predicates::str::contains(bin_name()));
}

#[test]
fn remove_selector_conflicts_with_all() {
    let _fixture_guard = fixture_process_guard();
    let base = TempDir::new().unwrap();
    vibe(base.path())
        .args(["self", "remove", "tag:1.0.0", "--all"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("cannot be used with"));
    vibe(base.path())
        .args(["self", "remove", "--all", "--tag"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("cannot be used with"));
}

#[test]
fn doctor_exits_nonzero_when_reported_problems_remain() {
    let _fixture_guard = fixture_process_guard();
    let base = TempDir::new().unwrap();
    vibe(base.path())
        .args(["--json", "self", "doctor"])
        .assert()
        .failure()
        .stdout(predicates::str::contains("\"ok\": false"))
        .stdout(predicates::str::contains("\"problems\":"));
}

#[test]
#[cfg(not(windows))]
fn install_builds_publishes_and_records_under_the_temp_root() {
    let _fixture_guard = fixture_process_guard();
    let base = TempDir::new().unwrap();
    let src = TempDir::new().unwrap();
    write_tiny_source(src.path());

    source_vibe(base.path(), src.path())
        .args(["self", "install"])
        .assert()
        .success()
        .stdout(predicates::str::contains("installed branch:main"));

    // The active binary landed under the temp root, in an instance dir named
    // by the live `current` pointer — not the real ~/opt.
    let current =
        std::fs::read_to_string(base.path().join("opt").join("vibevm").join("current")).unwrap();
    let instance_dir = std::path::PathBuf::from(current.trim());
    assert!(
        instance_dir.starts_with(base.path()),
        "instance is under the temp root"
    );
    let bin = instance_dir.join("bin");
    assert!(bin.join(bin_name()).is_file(), "vibe published under bin/");
    assert!(
        bin.join(index_bin_name()).is_file(),
        "vibe-index published under bin/"
    );
    assert!(
        bin.join(proxy_bin_name()).is_file(),
        "vibe-app-proxy published under bin/"
    );

    let state =
        std::fs::read_to_string(base.path().join("opt").join("vibevm").join("state.toml")).unwrap();
    assert!(state.contains("kind = \"branch\""), "records the kind");
    assert!(state.contains("id = \"main\""), "records the id");

    // install flipped `current`, so ls marks it active with no extra env.
    vibe(base.path())
        .args(["self", "ls"])
        .assert()
        .success()
        .stdout(predicates::str::contains("* branch:main"));
}

#[test]
#[cfg(not(windows))]
fn update_builds_and_activates_latest_like_install() {
    let _fixture_guard = fixture_process_guard();
    // A source execution updates `latest`: it builds all essential binaries,
    // publishes, and flips `current` to the new immutable instance.
    let base = TempDir::new().unwrap();
    let src = TempDir::new().unwrap();
    write_tiny_source(src.path());

    source_vibe(base.path(), src.path())
        .args(["self", "update"])
        .assert()
        .success()
        .stdout(predicates::str::contains("installed branch:main"));

    vibe(base.path())
        .args(["self", "ls"])
        .assert()
        .success()
        .stdout(predicates::str::contains("* branch:main"));
}

/// Write a minimal, dependency-free vibevm-shaped source tree (a cargo
/// workspace with tiny `vibe`, `vibe-index`, and `vibe-app-proxy` bins) under a git repo, so the
/// real `CargoBuilder` has something tiny and offline to compile.
fn write_tiny_source(dir: &Path) {
    use std::fs;
    fs::create_dir_all(dir.join("crates").join("vibe-cli").join("src")).unwrap();
    fs::create_dir_all(dir.join("crates").join("vibe-cli").join("src").join("bin")).unwrap();
    fs::create_dir_all(dir.join("crates").join("vibe-index").join("src")).unwrap();
    fs::write(
        dir.join("Cargo.toml"),
        "[workspace]\nmembers = [\"crates/vibe-cli\", \"crates/vibe-index\"]\nresolver = \"2\"\n",
    )
    .unwrap();
    fs::write(
        dir.join("crates").join("vibe-cli").join("Cargo.toml"),
        "[package]\nname = \"vibe-cli\"\nversion = \"0.0.0\"\nedition = \"2021\"\n\n\
         [[bin]]\nname = \"vibe\"\npath = \"src/main.rs\"\n\n\
         [[bin]]\nname = \"vibe-app-proxy\"\npath = \"src/bin/vibe-app-proxy.rs\"\n",
    )
    .unwrap();
    fs::write(
        dir.join("crates").join("vibe-index").join("Cargo.toml"),
        "[package]\nname = \"vibe-index\"\nversion = \"0.0.0\"\nedition = \"2021\"\n\n\
         [[bin]]\nname = \"vibe-index\"\npath = \"src/main.rs\"\n",
    )
    .unwrap();
    fs::write(
        dir.join("crates")
            .join("vibe-index")
            .join("src")
            .join("main.rs"),
        "fn main() { println!(\"tiny vibe-index\"); }\n",
    )
    .unwrap();
    fs::write(
        dir.join("crates")
            .join("vibe-cli")
            .join("src")
            .join("main.rs"),
        "fn main() { println!(\"tiny vibe\"); }\n",
    )
    .unwrap();
    fs::write(
        dir.join("crates")
            .join("vibe-cli")
            .join("src")
            .join("bin")
            .join("vibe-app-proxy.rs"),
        "fn main() { println!(\"tiny vibe-app-proxy\"); }\n",
    )
    .unwrap();
    // These workspace packages have no dependencies; this complete lock
    // keeps the real builder's production `cargo build --locked` contract.
    fs::write(
        dir.join("Cargo.lock"),
        "# This file is automatically @generated by Cargo.\n\
         # It is not intended for manual editing.\n\
         version = 4\n\n\
         [[package]]\nname = \"vibe-cli\"\nversion = \"0.0.0\"\n\n\
         [[package]]\nname = \"vibe-index\"\nversion = \"0.0.0\"\n",
    )
    .unwrap();
    git(dir, &["init", "-q", "-b", "main"]);
    git(dir, &["add", "."]);
    git(
        dir,
        &[
            "-c",
            "user.email=t@example.com",
            "-c",
            "user.name=tester",
            "commit",
            "-qm",
            "init",
        ],
    );
}

fn git(dir: &Path, args: &[&str]) {
    let ok = Sys::new("git")
        .current_dir(dir)
        .args(args)
        .status()
        .unwrap()
        .success();
    assert!(ok, "git {args:?} failed");
}
