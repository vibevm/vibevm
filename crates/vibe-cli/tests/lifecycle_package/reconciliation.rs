use std::fs;

use crate::common::UserScratch;

use super::support::{
    SKILL_KEY, SWEEP_KEY, command, json_documents, lifecycle_state, project, report_status,
    run_json,
};

fn run_offline(
    user: &UserScratch,
    project: &std::path::Path,
    home: &std::path::Path,
) -> vibe_wire::generated::lifecycle_report::LifecycleReport {
    let output = command(user, project, home, true)
        .arg("--offline")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_value(json_documents(&output.stdout)[1].clone()).unwrap()
}

#[test]
fn target_only_delete_and_tamper_force_reconciliation_before_fresh() {
    let user = UserScratch::new();
    let project = project(true, &["claude"], false);
    let home = tempfile::tempdir().unwrap();
    let first = run_offline(&user, project.path(), home.path());
    assert_eq!(report_status(&first, SKILL_KEY), "ok");
    let target = project.path().join(".claude/skills/demo");

    fs::remove_file(target.join("SKILL.md")).unwrap();
    let (_, deleted) = run_json(&user, project.path(), home.path());
    assert_eq!(report_status(&deleted, SKILL_KEY), "ok");
    assert_eq!(
        fs::read_to_string(target.join("SKILL.md")).unwrap(),
        "first\n"
    );

    fs::write(target.join("SKILL.md"), "tampered\n").unwrap();
    let (_, drifted) = run_json(&user, project.path(), home.path());
    assert_eq!(report_status(&drifted, SKILL_KEY), "ok");
    assert_eq!(
        fs::read_to_string(target.join("SKILL.md")).unwrap(),
        "first\n"
    );

    fs::remove_dir_all(&target).unwrap();
    let removed = run_offline(&user, project.path(), home.path());
    assert_eq!(report_status(&removed, SKILL_KEY), "ok");
    assert_eq!(
        fs::read_to_string(target.join("SKILL.md")).unwrap(),
        "first\n"
    );
}

#[test]
fn unowned_target_refuses_but_first_run_missing_source_deletes_nothing() {
    let user = UserScratch::new();
    let occupied = project(true, &["claude"], false);
    let home = tempfile::tempdir().unwrap();
    let target = occupied.path().join(".claude/skills/demo");
    fs::create_dir_all(&target).unwrap();
    fs::write(target.join("HUMAN.md"), "foreign\n").unwrap();
    let output = command(&user, occupied.path(), home.path(), false)
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("unowned pre-existing"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        fs::read_to_string(target.join("HUMAN.md")).unwrap(),
        "foreign\n"
    );
    assert!(!target.join("SKILL.md").exists());

    let missing = project(true, &["claude"], false);
    let missing_target = missing.path().join(".claude/skills/demo");
    fs::create_dir_all(&missing_target).unwrap();
    fs::write(missing_target.join("HUMAN.md"), "foreign\n").unwrap();
    fs::remove_dir_all(missing.path().join("skills/demo")).unwrap();
    let (_, report) = run_json(&user, missing.path(), home.path());
    assert_eq!(report_status(&report, SKILL_KEY), "ok");
    assert_eq!(
        fs::read_to_string(missing_target.join("HUMAN.md")).unwrap(),
        "foreign\n"
    );
    assert!(!missing_target.join("SKILL.md").exists());
}

#[test]
fn agent_shrink_rename_and_removal_reconcile_owned_set_and_prune_state() {
    let user = UserScratch::new();
    let project = project(true, &["claude", "codex"], false);
    let home = tempfile::tempdir().unwrap();
    run_json(&user, project.path(), home.path());
    let manifest_path = project.path().join("vibe.toml");
    let manifest = fs::read_to_string(&manifest_path).unwrap();

    fs::write(
        &manifest_path,
        manifest.replace("agents = [\"claude\", \"codex\"]", "agents = [\"claude\"]"),
    )
    .unwrap();
    let (_, shrunk) = run_json(&user, project.path(), home.path());
    assert_eq!(report_status(&shrunk, SKILL_KEY), "ok");
    assert!(
        project
            .path()
            .join(".claude/skills/demo/SKILL.md")
            .is_file()
    );
    assert!(!project.path().join(".agents/skills/demo").exists());
    assert_eq!(
        lifecycle_state(project.path()).execution[SKILL_KEY]
            .artifacts
            .len(),
        1
    );

    let renamed_manifest = fs::read_to_string(&manifest_path).unwrap().replace(
        "[[skill]]\nname = \"demo\"",
        "[[skill]]\nname = \"renamed\"",
    );
    fs::write(&manifest_path, renamed_manifest).unwrap();
    let (_, renamed) = run_json(&user, project.path(), home.path());
    let renamed_key = "@vibe/package/skill/org.example/lifecycle-skills/renamed";
    assert_eq!(report_status(&renamed, renamed_key), "ok");
    assert!(!project.path().join(".claude/skills/demo").exists());
    assert!(
        project
            .path()
            .join(".claude/skills/renamed/SKILL.md")
            .is_file()
    );
    let state = lifecycle_state(project.path());
    assert!(!state.execution.contains_key(SKILL_KEY));
    assert!(state.execution.contains_key(renamed_key));

    fs::write(
        &manifest_path,
        r#"[package]
group = "org.example"
name = "lifecycle-skills"
kind = "tool"
version = "0.1.0"
authors = ["Fixture"]
license = "EULA"
description = "fixture"
keywords = ["fixture"]
"#,
    )
    .unwrap();
    let (_, removed) = run_json(&user, project.path(), home.path());
    assert_eq!(report_status(&removed, SWEEP_KEY), "ok");
    assert!(!project.path().join(".claude/skills/renamed").exists());
    let state = lifecycle_state(project.path());
    assert!(!state.execution.contains_key(SKILL_KEY));
    assert!(!state.execution.contains_key(renamed_key));
    assert!(state.execution.contains_key(SWEEP_KEY));
}

#[test]
fn missing_or_malformed_receipt_never_authorizes_deletion() {
    let user = UserScratch::new();
    let home = tempfile::tempdir().unwrap();
    let bare_manifest = r#"[package]
group = "org.example"
name = "lifecycle-skills"
kind = "tool"
version = "0.1.0"
authors = ["Fixture"]
license = "EULA"
description = "fixture"
keywords = ["fixture"]
"#;

    let missing = project(true, &["claude"], false);
    run_json(&user, missing.path(), home.path());
    let target = missing.path().join(".claude/skills/demo/SKILL.md");
    fs::remove_file(missing.path().join(".vibe/package-skills.toml")).unwrap();
    fs::write(missing.path().join("vibe.toml"), bare_manifest).unwrap();
    let output = command(&user, missing.path(), home.path(), false)
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(fs::read_to_string(&target).unwrap(), "first\n");

    let malformed = project(true, &["claude"], false);
    run_json(&user, malformed.path(), home.path());
    let target = malformed.path().join(".claude/skills/demo/SKILL.md");
    fs::write(
        malformed.path().join(".vibe/package-skills.toml"),
        "schema = 1\n[[binding]]\nunknown = true\n",
    )
    .unwrap();
    fs::write(malformed.path().join("vibe.toml"), bare_manifest).unwrap();
    let output = command(&user, malformed.path(), home.path(), false)
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert_eq!(fs::read_to_string(&target).unwrap(), "first\n");
}
