//! Help-text smoke test — every documented subcommand renders `--help`
//! cleanly and `--version` round-trips. Mirrors the
//! `every_subcommand_renders_help` invariant the main `vibe-cli` crate
//! holds; here it is the regression gate that every later slice's CLI
//! addition must keep green.

use assert_cmd::Command;
use predicates::prelude::*;
extern crate tempfile;

// Hand-maintained, and the assertions below run in ONE direction only:
// every name here must appear in `--help`, but a subcommand present in
// `--help` and absent here is invisible to this gate. So a new verb can
// be added without the smoke test noticing, which is a norm with no
// checker — filed as `BACKLOG.md` B-094 with the measurement that found
// it (the `yank` line below was added by hand, after the fact, exactly
// because nothing failed without it).
const SUBCOMMANDS: &[&str] = &[
    "init",
    "reindex",
    "rescan-org",
    "get",
    "list",
    "search",
    "capabilities",
    "purls",
    "outdated",
    "add",
    "remove",
    "yank",
    "bury",
    "verify",
    "dump",
    "serve",
    "stop",
];

fn cmd() -> Command {
    vibe_test_support::cargo_bin("vibe-index")
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
fn root_help_shows_the_global_log_level_flag() {
    let out = cmd().arg("--help").assert().success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    assert!(
        stdout.contains("--log-level"),
        "root --help is missing the global `--log-level` flag; output was:\n{stdout}"
    );
}

#[test]
fn log_level_flag_is_accepted_after_the_subcommand() {
    // The operator's form is `vibe-index <sub> … --log-level off`;
    // `global = true` on the argument is what makes that parse. The
    // trailing `--help` keeps the subcommand side-effect free.
    cmd()
        .args(["dump", "--log-level", "off", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty().not());
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
    cmd().arg("definitely-not-a-subcommand").assert().failure();
}

#[test]
fn reindex_from_gitverse_emits_stub_envelope() {
    // `--from-gitverse` waits on GitVerse exposing org-scoped repo
    // enumeration in their public API. Until then the dispatcher
    // emits a structured `stub: true` envelope (mirrors the
    // `vibe registry publish` GitVerse stub) so consumers detect
    // the limitation programmatically without scraping stderr.
    let dir = tempfile::tempdir().unwrap();
    cmd()
        .args([
            "init",
            dir.path().to_str().unwrap(),
            "--registry",
            "vibespecs-gitverse",
            "--registry-url",
            "https://gitverse.ru/vibespecs",
        ])
        .assert()
        .success();
    let out = cmd()
        .args([
            "reindex",
            dir.path().to_str().unwrap(),
            "--from-gitverse",
            "vibespecs",
            "--json",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    let envelope: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(envelope["ok"], false);
    assert_eq!(envelope["stub"], true);
    assert_eq!(envelope["host"], "gitverse.ru");
    assert_eq!(envelope["org"], "vibespecs");
    assert_eq!(envelope["command"], "registry:reindex");
    let reason = envelope["reason"].as_str().unwrap();
    assert!(reason.contains("not implemented"));
}

#[test]
fn reindex_from_gitverse_text_form_shows_reason() {
    let dir = tempfile::tempdir().unwrap();
    cmd()
        .args([
            "init",
            dir.path().to_str().unwrap(),
            "--registry",
            "vibespecs-gitverse",
            "--registry-url",
            "https://gitverse.ru/vibespecs",
        ])
        .assert()
        .success();
    cmd()
        .args([
            "reindex",
            dir.path().to_str().unwrap(),
            "--from-gitverse",
            "vibespecs",
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("not implemented"));
}
