//! Integration tests for `vibe self` (PROP-019), driven through the real
//! binary. The install root is pinned inside a temp dir via
//! `VIBEVM_INSTALL_ROOT`, so nothing here ever touches the developer's
//! real `~/opt` (PROP-019 §2.4).

use std::path::Path;
use std::process::Command as Sys;

use assert_cmd::Command;
use tempfile::TempDir;

/// A `vibe` invocation with the install root pinned to `base` and no
/// ambient `VIBEVM_HOME` leaking in.
fn vibe(base: &Path) -> Command {
    let mut cmd = Command::cargo_bin("vibe").unwrap();
    cmd.env("VIBEVM_INSTALL_ROOT", base)
        .env_remove("VIBEVM_HOME");
    cmd
}

fn bin_name() -> &'static str {
    if cfg!(windows) { "vibe.exe" } else { "vibe" }
}

#[test]
fn ls_is_empty_on_a_fresh_root() {
    let base = TempDir::new().unwrap();
    vibe(base.path())
        .args(["self", "ls"])
        .assert()
        .success()
        .stdout(predicates::str::contains("no versions installed"));
}

#[test]
fn which_fails_without_an_active_version() {
    let base = TempDir::new().unwrap();
    vibe(base.path())
        .args(["self", "which"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("no active version"));
}

#[test]
fn install_builds_publishes_and_records_under_the_temp_root() {
    let base = TempDir::new().unwrap();
    let src = TempDir::new().unwrap();
    write_tiny_source(src.path());

    vibe(base.path())
        .current_dir(src.path())
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
    assert!(
        instance_dir.join(bin_name()).is_file(),
        "binary published in the instance dir"
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
fn update_builds_and_activates_latest_like_install() {
    // `self update` is `self install latest`: from a tiny in-tree source on
    // branch `main`, it builds, publishes, and flips `current` to it.
    let base = TempDir::new().unwrap();
    let src = TempDir::new().unwrap();
    write_tiny_source(src.path());

    vibe(base.path())
        .current_dir(src.path())
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
/// workspace with a `crates/vibe-cli` hello-world bin) under a git repo, so
/// the real `CargoBuilder` has something tiny and offline to compile.
fn write_tiny_source(dir: &Path) {
    use std::fs;
    fs::create_dir_all(dir.join("crates").join("vibe-cli").join("src")).unwrap();
    fs::write(
        dir.join("Cargo.toml"),
        "[workspace]\nmembers = [\"crates/vibe-cli\"]\nresolver = \"2\"\n",
    )
    .unwrap();
    fs::write(
        dir.join("crates").join("vibe-cli").join("Cargo.toml"),
        "[package]\nname = \"vibe-cli\"\nversion = \"0.0.0\"\nedition = \"2021\"\n\n\
         [[bin]]\nname = \"vibe\"\npath = \"src/main.rs\"\n",
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
