//! Differential CLI/library oracles for standalone package-skill projection.

mod common;

use std::fs;

use common::UserScratch;
use serde_json::{Value, json};
use vibe_mcp::agents::Scope;
use vibe_mcp::pkgskill::{DeclaredSkillFilter, prepare_declared_skill_projection};

fn project() -> tempfile::TempDir {
    let project = tempfile::tempdir().unwrap();
    fs::write(
        project.path().join("vibe.toml"),
        r#"[package]
group = "org.example"
name = "skill-cli"
kind = "tool"
version = "0.1.0"
authors = ["Fixture"]
license = "EULA"
description = "fixture"
keywords = ["fixture"]

[[skill]]
name = "demo"
path = "skills/demo"
description = "Demo skill"
include = ["SKILL.md", "references/**"]
"#,
    )
    .unwrap();
    let source = project.path().join("skills/demo");
    fs::create_dir_all(source.join("references")).unwrap();
    fs::write(source.join("SKILL.md"), "demo body").unwrap();
    fs::write(source.join("references/guide.md"), "guide").unwrap();
    fs::write(source.join("noise.txt"), "noise").unwrap();
    project
}

fn run_json(user: &UserScratch, args: &[&str], project: &std::path::Path) -> Value {
    let output = user
        .vibe()
        .arg("--json")
        .arg("skill")
        .args(args)
        .arg("--path")
        .arg(project)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stderr.is_empty(),
        "successful JSON command leaked stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn list_and_project_dry_run_keep_the_exact_json_contract_and_library_parity() {
    let user = UserScratch::new();
    let project = project();
    let source = project.path().join("skills/demo");

    let listed = run_json(&user, &["list"], project.path());
    assert_eq!(
        listed,
        json!({
            "ok": true,
            "command": "skill:list",
            "project": project.path().display().to_string(),
            "count": 1,
            "skills": [{
                "name": "demo",
                "origin": "project",
                "source": vibe_core::machine_json_path(&source),
                "description": "Demo skill",
                "agents": [],
            }],
        })
    );

    let cli = run_json(
        &user,
        &[
            "install",
            "--agent",
            "claude",
            "--scope",
            "project",
            "--dry-run",
        ],
        project.path(),
    );
    let direct = prepare_declared_skill_projection(
        project.path(),
        &DeclaredSkillFilter::new(&[], Some("claude")),
        Scope::Project,
    )
    .unwrap()
    .install(true)
    .unwrap();
    assert_eq!(cli["results"], serde_json::to_value(&direct).unwrap());
    assert_eq!(
        cli,
        json!({
            "ok": true,
            "command": "skill:install",
            "project": project.path().display().to_string(),
            "count": 1,
            "results": [{
                "skill": "demo",
                "agent": "claude",
                "scope": "project",
                "path": vibe_core::machine_json_path(
                    &project.path().join(".claude/skills/demo")
                ),
                "status": "would-create",
                "note": null,
            }],
        })
    );
    assert!(!project.path().join(".claude").exists());
}

#[test]
fn apply_is_idempotent_and_uninstall_keeps_the_standalone_contract() {
    let user = UserScratch::new();
    let project = project();
    let args = [
        "install", "--agent", "claude", "--scope", "project", "--yes",
    ];
    let created = run_json(&user, &args, project.path());
    assert_eq!(created["results"][0]["status"], "created");
    let target = project.path().join(".claude/skills/demo");
    assert_eq!(
        fs::read_to_string(target.join("SKILL.md")).unwrap(),
        "demo body"
    );
    assert!(target.join("references/guide.md").is_file());
    assert!(!target.join("noise.txt").exists());

    let unchanged = run_json(&user, &args, project.path());
    assert_eq!(unchanged["results"][0]["status"], "unchanged");

    let removed = run_json(
        &user,
        &[
            "uninstall",
            "--agent",
            "claude",
            "--scope",
            "project",
            "--yes",
        ],
        project.path(),
    );
    assert_eq!(removed["results"][0]["status"], "removed");
    assert!(!target.exists());
}

#[test]
fn explicit_user_and_both_scopes_remain_available_without_global_writes_in_dry_run() {
    let user = UserScratch::new();
    let project = project();

    let user_only = run_json(
        &user,
        &[
            "install",
            "--agent",
            "cursor",
            "--scope",
            "user",
            "--dry-run",
        ],
        project.path(),
    );
    assert_eq!(user_only["count"], 1);
    assert_eq!(user_only["results"][0]["scope"], "user");
    assert_eq!(user_only["results"][0]["status"], "skipped");

    let both = run_json(
        &user,
        &[
            "install",
            "--agent",
            "cursor",
            "--scope",
            "both",
            "--dry-run",
        ],
        project.path(),
    );
    assert_eq!(both["count"], 2);
    assert_eq!(both["results"][0]["scope"], "project");
    assert_eq!(both["results"][1]["scope"], "user");
    assert!(
        both["results"]
            .as_array()
            .unwrap()
            .iter()
            .all(|row| row["status"] == "skipped" && row["path"].is_null())
    );
    assert!(!project.path().join(".cursor").exists());
}
