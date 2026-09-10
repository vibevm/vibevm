//! Process-boundary coverage for `vibe-index rebuild --check`.

use std::fs;
use std::path::Path;

use assert_cmd::Command;
use predicates::prelude::*;

fn cmd() -> Command {
    vibe_test_support::cargo_bin("vibe-index")
}

fn init(data_dir: &Path) {
    cmd()
        .args([
            "init",
            data_dir.to_str().expect("fixture path is UTF-8"),
            "--registry",
            "vibespecs",
            "--registry-url",
            "https://example.invalid/vibespecs",
        ])
        .assert()
        .success();
}

#[test]
fn projected_catalog_passes_through_the_shipped_binary() {
    let temp = tempfile::tempdir().expect("tempdir");
    let data_dir = temp.path().join("index");
    init(&data_dir);

    cmd()
        .args([
            "rebuild",
            data_dir.to_str().expect("fixture path is UTF-8"),
            "--check",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("byte-identical"));
}

#[test]
fn drift_is_named_and_fails_through_the_shipped_binary() {
    let temp = tempfile::tempdir().expect("tempdir");
    let data_dir = temp.path().join("index");
    init(&data_dir);
    let planted = "not the projection\n";
    fs::write(data_dir.join("primary.jsonl"), planted).expect("plant drift");

    cmd()
        .args([
            "rebuild",
            data_dir.to_str().expect("fixture path is UTF-8"),
            "--check",
        ])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("rebuild: differs")
                .and(predicate::str::contains("primary.jsonl"))
                .and(predicate::str::contains("drift item")),
        );
    assert_eq!(
        fs::read_to_string(data_dir.join("primary.jsonl")).expect("read planted drift"),
        planted,
        "the check must report drift without repairing it"
    );
}

#[test]
fn command_has_no_implicit_repair_mode() {
    let temp = tempfile::tempdir().expect("tempdir");
    let data_dir = temp.path().join("index");
    init(&data_dir);
    let before = fs::read(data_dir.join("repomd.json")).expect("read catalog before refusal");

    cmd()
        .args(["rebuild", data_dir.to_str().expect("fixture path is UTF-8")])
        .assert()
        .failure()
        .stderr(predicate::str::contains("only `--check` exists"));
    assert_eq!(
        fs::read(data_dir.join("repomd.json")).expect("read catalog after refusal"),
        before,
        "omitting --check must not become an implicit repair mode"
    );
}
