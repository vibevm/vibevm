//! Help-text smoke test — every subcommand the clap derive knows
//! renders `--help` cleanly and `--version` round-trips. Mirrors the
//! `every_subcommand_renders_help` invariant the main `vibe-cli` crate
//! holds; here it is the regression gate that every later slice's CLI
//! addition must keep green.

use assert_cmd::Command;
use predicates::prelude::*;
extern crate tempfile;

use vibe_index::cli;

// The subcommand list is not written in this file: it is read off the
// clap tree (`cli::command()`), the same object the binary renders
// `--help` from. Both directions hold by construction — a verb added
// to the `Command` enum joins the smoke by itself, and a verb that
// stops appearing in `--help`, or stops rendering its own help, turns
// the gate red. The hand-maintained `const` this replaces ran in ONE
// direction only and could see just the names someone remembered to
// re-type: `yank` shipped in `--help` with the list none the wiser
// (BACKLOG B-094 — the tree is now the source of truth).
fn visible_subcommands() -> Vec<String> {
    let root = cli::command();
    root.get_subcommands()
        // A `hide = true` verb is out of `--help` by design; the only
        // exclusion allowed here is one the derive itself computes —
        // never a spelled-out name.
        .filter(|sub| !sub.is_hide_set())
        .map(|sub| sub.get_name().to_owned())
        .collect()
}

fn cmd() -> Command {
    vibe_test_support::cargo_bin("vibe-index")
}

#[test]
fn root_help_lists_every_subcommand() {
    let out = cmd().arg("--help").assert().success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    let visible = visible_subcommands();
    assert!(
        !visible.is_empty(),
        "the clap tree knows no subcommands — the derive wiring broke"
    );
    for sub in &visible {
        assert!(
            stdout.contains(sub.as_str()),
            "root --help is missing subcommand `{sub}` (present in the clap tree); \
             output was:\n{stdout}"
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
    for sub in visible_subcommands() {
        cmd()
            .args([sub.as_str(), "--help"])
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
