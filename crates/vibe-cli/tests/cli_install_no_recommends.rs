//! `--no-recommends` on the project plane (`PROP-077#NO-RECOMMENDS-FLAG`):
//! the greedy weak expansion is not run at all, and the flag changes a
//! **solve**, never a replay — a lock already records whatever the solve
//! accepted, so a lock-replaying install materialises the same tree
//! whichever way the flag is passed.
//!
//! What this file deliberately does **not** assert is that a project install
//! installs a `[recommends]` reference, because measurably it does not: the
//! resolver's greedy expansion pulls the recommended package into the solved
//! graph (proved at the solver seam by `vibe-resolver`'s `recommends` suite),
//! and the PROP-050 `#EFFECTIVE-SET` visibility projection then prunes it
//! again, since no manifest declares an edge to it and it is not a root. The
//! last assertion below pins that measured behaviour so the day someone
//! changes it is visible; the finding itself is recorded in PROP-077
//! `#P7D-AS-BUILT`.

mod common;

use std::fs;
use std::path::{Path, PathBuf};

use common::UserScratch;
use predicates::prelude::*;

/// A directory registry holding one package that recommends another, both
/// the smallest manifest a `flow` package can be.
fn registry(root: &Path) -> PathBuf {
    let registry = root.join("weak-registry");
    write(
        &registry,
        "recommender",
        "\n[recommends]\npackages = [\"org.vibevm.fixture/companion\"]\n",
    );
    write(&registry, "companion", "");
    registry
}

fn write(registry: &Path, name: &str, extra: &str) {
    let dir = registry
        .join("org.vibevm.fixture")
        .join(name)
        .join("v1.0.0");
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("vibe.toml"),
        format!(
            r#"[package]
name = "{name}"
group = "org.vibevm.fixture"
kind = "flow"
version = "1.0.0"
authors = ["vibevm test fixtures"]
license = "UPL-1.0"
description = "PROP-077 weak-dependency fixture"
{extra}"#
        ),
    )
    .unwrap();
}

fn lock(project: &Path) -> String {
    fs::read_to_string(project.join("vibe.lock")).unwrap()
}

/// `vibe install <pkg>` with and without the flag; both must succeed.
fn install(
    user: &UserScratch,
    project: &Path,
    registry: &Path,
    package: Option<&str>,
    decline: bool,
) {
    let mut command = user.vibe();
    command.arg("install");
    if decline {
        command.arg("--no-recommends");
    }
    if let Some(package) = package {
        command.arg(package);
    }
    command
        .arg("--path")
        .arg(project)
        .arg("--registry")
        .arg(registry)
        .arg("--assume-yes")
        .assert()
        .success();
}

#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-077#NO-RECOMMENDS-FLAG")]
fn the_flag_is_admitted_by_the_solving_commands_and_never_changes_a_replay() {
    let user = UserScratch::new();
    let scratch = tempfile::tempdir().unwrap();
    let registry = registry(scratch.path());
    let project = tempfile::tempdir().unwrap();
    common::init_project(project.path());

    install(
        &user,
        project.path(),
        &registry,
        Some("org.vibevm.fixture/recommender"),
        true,
    );
    let declined = lock(project.path());
    assert!(declined.contains("recommender"));

    // It changes a solve, never a replay: a bare `vibe install` over the
    // same lock writes the same bytes whichever way the flag is passed, so
    // no user can produce the flapping a recorded preference would cause.
    install(&user, project.path(), &registry, None, false);
    assert_eq!(declined, lock(project.path()));
    install(&user, project.path(), &registry, None, true);
    assert_eq!(declined, lock(project.path()));

    // `vibe update` and `vibe reinstall` admit it too — the same spelling on
    // every command of the install family. `vibe update` re-solves and so
    // the flag decides there; `vibe reinstall` replays the lock, so what it
    // owes the flag is admission rather than an effect.
    user.vibe()
        .arg("update")
        .arg("--no-recommends")
        .arg("--path")
        .arg(project.path())
        .arg("--registry")
        .arg(&registry)
        .arg("--assume-yes")
        .assert()
        .success();
    user.vibe()
        .arg("reinstall")
        .arg("--no-recommends")
        .arg(project.path())
        .arg("--assume-yes")
        .assert()
        .success();

    // The measured project-plane state, pinned rather than assumed: an
    // accepted recommendation does not survive the effective-set projection,
    // so it reaches neither the lock nor `vibedeps/` — with the flag or
    // without it (PROP-077 `#P7D-AS-BUILT`).
    let fresh = tempfile::tempdir().unwrap();
    common::init_project(fresh.path());
    install(
        &user,
        fresh.path(),
        &registry,
        Some("org.vibevm.fixture/recommender"),
        false,
    );
    assert!(
        !lock(fresh.path()).contains("companion"),
        "the project plane's visibility projection still prunes an accepted recommendation"
    );
}

/// A flag silently ignored is worse than a flag refused
/// (`PROP-077#NO-RECOMMENDS-FLAG`).
#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-077#NO-RECOMMENDS-FLAG")]
fn a_command_that_neither_solves_nor_installs_refuses_the_flag_by_name() {
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    common::init_project(project.path());
    user.vibe()
        .arg("list")
        .arg("--no-recommends")
        .arg("--path")
        .arg(project.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("--no-recommends"));
}
