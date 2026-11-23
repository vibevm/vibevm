//! Generated subagent types use the published lane and report unavailable bodies.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-076#SYS-WORKER-TYPES");

mod common;

use std::fs;
use std::io::Write;

use common::UserScratch;

fn project(user: &UserScratch) -> tempfile::TempDir {
    let project = tempfile::tempdir().unwrap();
    user.vibe()
        .arg("init")
        .arg("--path")
        .arg(project.path())
        .assert()
        .success();
    fs::write(
        project
            .path()
            .join(vibe_core::layout::current_boot_dir())
            .join("SYSTEM-TEMPLATE.md"),
        "# My frame\n\nPermanent instructions.\n",
    )
    .unwrap();
    user.vibe()
        .args(["prompt", "build", "--path"])
        .arg(project.path())
        .assert()
        .success();
    project
}

/// Synthetic opted-in projects exercise the human contract without a real home or provider.
#[test]
fn prompt_prepare_and_capture_quiet_emit_only_the_headline() {
    let user = UserScratch::new();
    let project = project(&user);
    for (verb, words) in [("prepare", "prepared ."), ("capture", "envelope for .")] {
        let report = user
            .vibe()
            .args(["prompt", verb, "--path"])
            .arg(project.path())
            .arg("--json")
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();
        let report: serde_json::Value = serde_json::from_slice(&report).unwrap();
        let boundaries = report["cache_boundaries"]
            .as_array()
            .unwrap()
            .iter()
            .map(|offset| offset.as_u64().unwrap().to_string())
            .collect::<Vec<_>>()
            .join(", ");
        let output = user
            .vibe()
            .args(["prompt", verb, "--path"])
            .arg(project.path())
            .arg("--quiet")
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();
        let output = String::from_utf8(output).unwrap();
        assert_eq!(output.lines().count(), 1, "{verb}: {output}");
        assert!(
            output.starts_with(&format!("system prompt {words} · ")),
            "{output}"
        );
        assert!(
            output.ends_with(&format!("breakpoints {boundaries}\n")),
            "{output}"
        );
        assert!(!output.contains('✓'));
    }
}

#[test]
fn prompt_prepare_and_capture_draw_one_headline_and_measured_materials() {
    let user = UserScratch::new();
    let project = project(&user);
    for (verb, words) in [("prepare", "prepared ."), ("capture", "envelope for .")] {
        let mut command = user.vibe();
        command
            .args(["prompt", verb, "--path"])
            .arg(project.path())
            .env("COLUMNS", "500");
        if verb == "capture" {
            command.args(["--task", "synthetic task"]);
        }
        let output = command.assert().success().get_output().stdout.clone();
        let output = String::from_utf8(output).unwrap();
        assert_eq!(
            output
                .lines()
                .filter(|line| line.starts_with("✓ system prompt"))
                .count(),
            1,
            "{output}"
        );
        assert!(
            output.starts_with(&format!("✓ system prompt  {words} · ")),
            "{output}"
        );
        assert!(output.contains("breakpoints "), "{output}");
        assert!(
            output
                .lines()
                .any(|line| line.starts_with("  P  SYSTEM.md") && line.contains(" bytes")),
            "{output}"
        );
        if verb == "capture" {
            assert!(output.contains("0 tools before_instructions"), "{output}");
            assert!(output.contains("1 task message ·"), "{output}");
            assert!(
                output
                    .lines()
                    .any(|line| line.starts_with("  P  ") && line.ends_with("offset 0")),
                "{output}"
            );
            assert!(output.lines().any(|line| line.starts_with("  T  caller's own") && line.ends_with("14 bytes")), "{output}");
        } else {
            assert!(output.contains("0 unresolved"), "{output}");
        }
    }
}

#[test]
fn prompt_types_human_trailer_counts_only_writes_and_check_has_none() {
    let user = UserScratch::new();
    let project = project(&user);
    for expected in ["wrote 1 subagent type", "subagent types current"] {
        let output = user
            .vibe()
            .args(["prompt", "types", "--path"])
            .arg(project.path())
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();
        let output = String::from_utf8(output).unwrap();
        assert!(
            output
                .lines()
                .last()
                .unwrap()
                .starts_with(&format!("✻ {expected} · .claude/agents/ · ")),
            "{output}"
        );
        assert!(output.contains("[unavailable]"), "{output}");
        assert!(!output.contains("wrote 2"));
    }
    let output = user
        .vibe()
        .args(["prompt", "types", "--check", "--path"])
        .arg(project.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    assert!(!String::from_utf8(output).unwrap().contains('✻'));
}

#[test]
fn prompt_types_foreign_refusal_names_the_file_without_a_success_trailer() {
    let user = UserScratch::new();
    let project = project(&user);
    let worker = project.path().join(".claude/agents/vibe-worker.md");
    fs::create_dir_all(worker.parent().unwrap()).unwrap();
    fs::write(&worker, "synthetic authored worker\n").unwrap();
    let output = user
        .vibe()
        .args(["prompt", "types", "--path"])
        .arg(project.path())
        .assert()
        .failure()
        .get_output()
        .clone();
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stdout.contains("[foreign]"), "{stdout}");
    assert!(!stdout.contains('✻'), "{stdout}");
    assert!(stderr.contains("subagent_type_foreign"), "{stderr}");
    assert!(stderr.contains(".claude/agents/vibe-worker.md"), "{stderr}");
    assert_eq!(
        fs::read_to_string(worker).unwrap(),
        "synthetic authored worker\n"
    );
}

#[test]
fn prompt_capture_usage_hint_keeps_the_saved_record_words() {
    let user = UserScratch::new();
    let project = project(&user);
    let usage = project.path().join("synthetic-usage.json");
    fs::write(&usage, r#"{"usage":{"input_tokens":12,"cache_creation_input_tokens":3,"cache_read_input_tokens":4}}"#).unwrap();
    let output = user
        .vibe()
        .args(["prompt", "capture", "--path"])
        .arg(project.path())
        .arg("--usage")
        .arg(&usage)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let output = String::from_utf8(output).unwrap();
    assert!(
        output
            .contains("  provider usage (anthropic-messages-usage): write 3, read 4, uncached 12"),
        "{output}"
    );
    fs::write(
        &usage,
        r#"{"usage":{"prompt_tokens":12,"completion_tokens":2,"total_tokens":14}}"#,
    )
    .unwrap();
    let output = user
        .vibe()
        .args(["prompt", "capture", "--path"])
        .arg(project.path())
        .arg("--usage")
        .arg(&usage)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let output = String::from_utf8(output).unwrap();
    assert!(output.contains("`prompt_tokens`"), "{output}");
    assert!(
        output.contains("no cache counter this build knows"),
        "{output}"
    );
    assert!(output.contains("absence is not zero"), "{output}");
}

#[test]
fn prompt_types_check_refusal_counts_missing_and_foreign_types() {
    let user = UserScratch::new();
    let project = project(&user);
    let output = user
        .vibe()
        .args(["prompt", "types", "--check", "--path"])
        .arg(project.path())
        .assert()
        .failure()
        .get_output()
        .clone();
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("types_not_current"), "{stderr}");
    assert!(
        stderr.contains("1 subagent type is not current"),
        "{stderr}"
    );
    assert!(
        stderr.contains(".claude/agents/vibe-worker.md (missing)"),
        "{stderr}"
    );
    assert!(stderr.contains("vibe prompt types"), "{stderr}");
}

#[test]
fn prompt_types_writes_the_worker_type_from_the_lane() {
    let user = UserScratch::new();
    let project = project(&user);
    let output = user
        .vibe()
        .args(["prompt", "types", "--path"])
        .arg(project.path())
        .arg("--json")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let report: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(report["protocol"], "vibe-prompt-types/1");
    assert_eq!(report["mode"], "write");
    assert_eq!(report["types"][0]["name"], "vibe-worker");
    assert_eq!(report["types"][0]["file"], ".claude/agents/vibe-worker.md");
    assert_eq!(report["types"][0]["status"], "created");
    assert_eq!(report["types"][1]["name"], "vibe-executor");
    assert_eq!(report["types"][1]["status"], "unavailable");
    assert!(!report["types"][1]["reason"].as_str().unwrap().is_empty());
    let worker = fs::read(project.path().join(".claude/agents/vibe-worker.md")).unwrap();
    let lane = fs::read(
        project
            .path()
            .join(vibe_core::layout::current_boot_dir())
            .join("AGENT-LANE.md"),
    )
    .unwrap();
    assert!(worker.starts_with(b"---\nname: vibe-worker\n"));
    assert!(worker.ends_with(&lane));
    assert!(project.path().join(".claude/agents/.gitignore").exists());
}

#[test]
fn prompt_types_check_reports_a_stale_type() {
    let user = UserScratch::new();
    let project = project(&user);
    user.vibe()
        .args(["prompt", "types", "--path"])
        .arg(project.path())
        .assert()
        .success();
    let output = user
        .vibe()
        .args(["prompt", "types", "--check", "--path"])
        .arg(project.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    assert!(String::from_utf8(output).unwrap().lines().any(|line| {
        line.starts_with("✓ .claude/agents/vibe-worker.md") && line.ends_with("[current]")
    }));

    let worker_path = project.path().join(".claude/agents/vibe-worker.md");
    fs::OpenOptions::new()
        .append(true)
        .open(&worker_path)
        .unwrap()
        .write_all(b"edited\n")
        .unwrap();
    let edited = fs::read(&worker_path).unwrap();
    let output = user
        .vibe()
        .args(["prompt", "types", "--check", "--path"])
        .arg(project.path())
        .arg("--json")
        .assert()
        .failure()
        .get_output()
        .clone();
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["mode"], "check");
    let worker = report["types"]
        .as_array()
        .unwrap()
        .iter()
        .find(|agent_type| agent_type["name"] == "vibe-worker")
        .unwrap();
    assert_eq!(worker["status"], "stale");
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("vibe prompt types")
    );
    let unchanged = fs::read(&worker_path).unwrap();
    assert_eq!(unchanged, edited);
    assert!(unchanged.ends_with(b"edited\n"));
}
