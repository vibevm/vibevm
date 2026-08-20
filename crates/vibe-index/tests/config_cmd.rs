//! `vibe-index config` — the visible source of the four-rung ladder
//! (PROP-005 §3.5 / `BACKLOG.md` B-086), end to end: the verb's own
//! output, the pairwise precedence of the rungs, and the loud refusal
//! on a file this build does not understand. The flag > env > file
//! ordering is exercised on `dump --format` — the one member whose
//! effect a read-only verb can show without network or git. With an
//! empty index the two formats are totally distinct: JSONL prints
//! nothing, the JSON document prints a multi-line pretty object.

use std::path::Path;

use assert_cmd::Command;

fn cmd() -> Command {
    vibe_test_support::cargo_bin("vibe-index")
}

fn init_dir(dir: &Path) {
    cmd()
        .args([
            "init",
            dir.to_str().unwrap(),
            "--registry",
            "vibespecs",
            "--registry-url",
            "https://example.invalid/vibespecs",
        ])
        .assert()
        .success();
}

fn write_config(dir: &Path, body: &str) {
    let state = dir.join("state");
    std::fs::create_dir_all(&state).unwrap();
    std::fs::write(state.join("config.toml"), body).unwrap();
}

fn dump(dir: &Path) -> Command {
    let mut c = cmd();
    c.arg("dump").arg(dir.to_str().unwrap());
    c
}

#[test]
fn config_reports_defaults_when_the_file_is_absent() {
    let dir = tempfile::tempdir().unwrap();
    init_dir(dir.path());

    let out = cmd()
        .args(["config", dir.path().to_str().unwrap()])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    assert!(stdout.contains("Config file:"), "{stdout}");
    assert!(stdout.contains("absent"), "{stdout}");
    for (key, value) in [
        ("log-level", "warn"),
        ("git", "git"),
        ("api-base", "https://api.github.com"),
        ("dump-format", "jsonl"),
    ] {
        assert!(stdout.contains(key), "member {key} missing:\n{stdout}");
        assert!(
            stdout.contains(&format!("= {value}")),
            "member {key} default {value} missing:\n{stdout}"
        );
    }
    // The visible source: all four members name the default rung.
    assert_eq!(stdout.matches("[source: default]").count(), 4, "{stdout}");
}

#[test]
fn config_names_the_config_file_as_the_source() {
    let dir = tempfile::tempdir().unwrap();
    init_dir(dir.path());
    write_config(dir.path(), "dump-format = \"json\"\n");

    let out = cmd()
        .args(["config", dir.path().to_str().unwrap()])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    assert!(stdout.contains("present"), "{stdout}");
    assert!(stdout.contains("dump-format = json"), "{stdout}");
    assert!(stdout.contains("[source: config file"), "{stdout}");
    // The other members still name their own rungs.
    assert!(stdout.contains("[source: default]"), "{stdout}");
}

#[test]
fn config_names_env_and_flag_sources() {
    let dir = tempfile::tempdir().unwrap();
    init_dir(dir.path());

    let out = cmd()
        .args(["config", dir.path().to_str().unwrap()])
        .env("VIBE_INDEX_GIT", "C:/tools/git.exe")
        .env("VIBE_INDEX_LOG", "debug")
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    assert!(stdout.contains("[source: env VIBE_INDEX_GIT]"), "{stdout}");
    assert!(stdout.contains("[source: env VIBE_INDEX_LOG]"), "{stdout}");
    assert!(
        stdout.contains("git         = C:/tools/git.exe"),
        "{stdout}"
    );

    // The invocation's own global flag is the top rung and shows as
    // such.
    let out = cmd()
        .args([
            "config",
            dir.path().to_str().unwrap(),
            "--log-level",
            "trace",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    assert!(stdout.contains("[source: flag --log-level]"), "{stdout}");
    assert!(stdout.contains("log-level   = trace"), "{stdout}");
}

#[test]
fn config_json_envelope_carries_every_member() {
    let dir = tempfile::tempdir().unwrap();
    init_dir(dir.path());

    let out = cmd()
        .args(["config", dir.path().to_str().unwrap(), "--json"])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    let payload: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(payload["ok"], true);
    assert_eq!(payload["command"], "config");
    assert_eq!(payload["config_file"]["present"], false);
    let members = payload["members"].as_array().unwrap();
    assert_eq!(members.len(), 4, "{members:?}");
    let keys: Vec<&str> = members.iter().map(|m| m["key"].as_str().unwrap()).collect();
    assert_eq!(keys, ["log-level", "git", "api-base", "dump-format"]);
    assert_eq!(payload["precedence"][0], "flag");
    assert_eq!(payload["precedence"][3], "default");
}

// ---- the rungs, pairwise, on the one member a read verb can show ----

#[test]
fn dump_honours_the_config_file_rung() {
    let dir = tempfile::tempdir().unwrap();
    init_dir(dir.path());
    write_config(dir.path(), "dump-format = \"json\"\n");

    let out = dump(dir.path()).assert().success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    assert!(
        stdout.lines().count() > 1,
        "the file rung's `dump-format = \"json\"` must be honoured; got:\n{stdout}"
    );
    assert!(stdout.contains("\"schema_version\""), "{stdout}");
}

#[test]
fn flag_beats_env_and_file() {
    let dir = tempfile::tempdir().unwrap();
    init_dir(dir.path());
    write_config(dir.path(), "dump-format = \"json\"\n");

    let out = cmd()
        .args(["dump", dir.path().to_str().unwrap(), "--format", "jsonl"])
        .env("VIBE_INDEX_DUMP_FORMAT", "json")
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    assert!(
        stdout.lines().count() <= 1,
        "the explicit flag must beat env and file; got:\n{stdout}"
    );
}

#[test]
fn env_beats_file() {
    let dir = tempfile::tempdir().unwrap();
    init_dir(dir.path());
    write_config(dir.path(), "dump-format = \"json\"\n");

    let out = cmd()
        .args(["dump", dir.path().to_str().unwrap()])
        .env("VIBE_INDEX_DUMP_FORMAT", "jsonl")
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    assert!(
        stdout.lines().count() <= 1,
        "the env rung must beat the file rung; got:\n{stdout}"
    );
}

// ---- strictness, loudly ----

#[test]
fn unknown_config_key_refuses_loudly() {
    let dir = tempfile::tempdir().unwrap();
    init_dir(dir.path());
    write_config(dir.path(), "limit = 10\n");

    for verb in ["config", "dump", "verify"] {
        let out = cmd()
            .args([verb, dir.path().to_str().unwrap()])
            .assert()
            .failure();
        let stderr = String::from_utf8(out.get_output().stderr.clone()).unwrap();
        assert!(
            stderr.contains("limit") && stderr.contains("unknown key"),
            "`{verb}` must refuse loudly on the unknown key; stderr:\n{stderr}"
        );
    }
}

#[test]
fn env_log_value_outside_the_closed_set_refuses() {
    let dir = tempfile::tempdir().unwrap();
    init_dir(dir.path());

    let out = cmd()
        .args(["dump", dir.path().to_str().unwrap()])
        .env("VIBE_INDEX_LOG", "loud")
        .assert()
        .failure();
    let stderr = String::from_utf8(out.get_output().stderr.clone()).unwrap();
    assert!(
        stderr.contains("off, error, warn, info, debug, trace"),
        "the refusal must name the valid set; stderr:\n{stderr}"
    );
}
