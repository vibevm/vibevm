//! Help-text smoke test — every documented subcommand renders `--help`
//! cleanly and `--version` round-trips. Mirrors the
//! `every_subcommand_renders_help` invariant the main `vibe-cli` crate
//! holds; here it is the regression gate that every later slice's CLI
//! addition must keep green.

use assert_cmd::Command;
use predicates::prelude::*;

const SUBCOMMANDS: &[&str] = &[
    "init",
    "reindex",
    "get",
    "list",
    "search",
    "capabilities",
    "purls",
    "outdated",
    "add",
    "remove",
    "verify",
    "dump",
    "serve",
    "stop",
];

fn cmd() -> Command {
    Command::cargo_bin("vibe-index").expect("vibe-index binary built")
}

#[test]
fn root_help_lists_every_subcommand() {
    let out = cmd().arg("--help").assert().success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    for sub in SUBCOMMANDS {
        assert!(
            stdout.contains(sub),
            "root --help is missing subcommand `{sub}`; output was:\n{stdout}"
        );
    }
}

#[test]
fn version_flag_works() {
    cmd()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("vibe-index"));
}

#[test]
fn every_subcommand_renders_help() {
    for sub in SUBCOMMANDS {
        cmd()
            .args([sub, "--help"])
            .assert()
            .success()
            .stdout(predicate::str::is_empty().not());
    }
}

#[test]
fn unknown_subcommand_fails_clean() {
    cmd()
        .arg("definitely-not-a-subcommand")
        .assert()
        .failure();
}

#[test]
fn stub_subcommands_emit_not_yet_implemented() {
    // `add` is still a stub in slice 2 — `init` / `dump` / `verify`
    // got real bodies. Supply enough args so clap parses; the
    // dispatcher's NotYetImplemented branch fires.
    let out = cmd()
        .args([
            "add",
            "/tmp/does-not-matter-stub",
            "--manifest",
            "/tmp/nope.toml",
        ])
        .assert()
        .failure();
    let stderr = String::from_utf8(out.get_output().stderr.clone()).unwrap();
    assert!(
        stderr.contains("not yet implemented"),
        "expected NotYetImplemented stderr, got: {stderr}"
    );
}
