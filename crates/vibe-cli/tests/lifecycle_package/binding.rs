use std::fs;

use specmark::verifies;

use crate::common::UserScratch;

use super::support::{
    AFTER_KEY, RECOVER_KEY, SKILL_KEY, SWEEP_KEY, command, lifecycle_state, project, report_status,
    run_json,
};

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#PRESET-LAW")]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ORDER-LAW")]
fn reserved_project_only_preset_is_queryable_ordered_and_narrated() {
    let user = UserScratch::new();
    let project = project(true, &["claude", "codex"], true);
    let fake_home = tempfile::tempdir().unwrap();
    let user_canary = fake_home.path().join(".claude/skills/demo/SKILL.md");
    fs::create_dir_all(user_canary.parent().unwrap()).unwrap();
    fs::write(&user_canary, "USER-CANARY\n").unwrap();

    // Seed one successful package run first so the ownership receipt exists:
    // only then do the engine-owned recovery and reconcile rows join the
    // plan alongside the ordinary binding.
    let (_, first) = run_json(&user, project.path(), fake_home.path());
    assert_eq!(report_status(&first, SKILL_KEY), "ok");

    let inspect = user
        .vibe()
        .args(["extensions", "--json", "--path"])
        .arg(project.path())
        .env("HOME", fake_home.path())
        .env("USERPROFILE", fake_home.path())
        .output()
        .unwrap();
    assert!(inspect.status.success());
    let inspect: serde_json::Value = serde_json::from_slice(&inspect.stdout).unwrap();
    let declarations = inspect["declarations"].as_array().unwrap();
    let row = declarations
        .iter()
        .find(|row| row["key"] == SKILL_KEY)
        .unwrap();
    assert_eq!(row["point"], "phase:package");
    assert_eq!(row["handler"]["kind"], "builtin");
    assert_eq!(row["handler"]["name"], "package-skill-project");
    assert_eq!(row["provider"]["identity"], "org.example/lifecycle-skills");
    assert_eq!(row["tier"], "preset");
    assert_eq!(
        row["effective_config"]["include"],
        serde_json::json!(["SKILL.md", "references/**"])
    );
    assert_eq!(
        row["effective_config"]["target_agents"],
        serde_json::json!(["claude", "codex"])
    );
    assert_eq!(
        row["effective_config"]["target_paths"],
        serde_json::json!([
            vibe_core::machine_json_path(&project.path().join(".claude/skills/demo")),
            vibe_core::machine_json_path(&project.path().join(".agents/skills/demo")),
        ])
    );
    assert!(
        row["effective_config"]["source_snapshot"]
            .as_str()
            .is_some_and(|value| value.starts_with("sha256:"))
    );
    // Both engine rows are present and queryable once the receipt exists.
    for key in [RECOVER_KEY, SWEEP_KEY] {
        let engine_row = declarations
            .iter()
            .find(|row| row["key"] == key)
            .unwrap_or_else(|| panic!("missing engine row `{key}`: {declarations:?}"));
        assert_eq!(engine_row["point"], "phase:package");
        assert_eq!(engine_row["tier"], "preset");
    }

    let (plan, report) = run_json(&user, project.path(), fake_home.path());
    let recover_at = plan
        .contributions
        .iter()
        .position(|row| row.key == RECOVER_KEY)
        .unwrap();
    let reconcile_at = plan
        .contributions
        .iter()
        .position(|row| row.key == SWEEP_KEY)
        .unwrap();
    let skill_at = plan
        .contributions
        .iter()
        .position(|row| row.key == SKILL_KEY)
        .unwrap();
    let after_at = plan
        .contributions
        .iter()
        .position(|row| row.key == AFTER_KEY)
        .unwrap();
    // Exact engine order: recovery first, then the vanished-binding sweep,
    // then the ordinary binding, then the host contribution.
    assert!(recover_at < reconcile_at, "{plan:?}");
    assert!(reconcile_at < skill_at, "{plan:?}");
    assert!(skill_at < after_at, "{plan:?}");
    assert_eq!(plan.contributions[recover_at].key, RECOVER_KEY);
    assert_eq!(plan.contributions[reconcile_at].key, SWEEP_KEY);
    let planned = &plan.contributions[skill_at];
    assert_eq!(planned.phase, "package");
    assert_eq!(planned.point, "phase:package");
    assert_eq!(planned.handler, "builtin");
    assert_eq!(planned.provider, "org.example/lifecycle-skills");
    assert_eq!(planned.tier, "preset");
    // The synthetic rows first become plannable after the first receipt exists,
    // while the already-observed binding and host row remain fresh.
    assert_eq!(report_status(&report, RECOVER_KEY), "ok");
    assert_eq!(report_status(&report, SWEEP_KEY), "ok");
    assert_eq!(report_status(&report, SKILL_KEY), "fresh");
    assert_eq!(report_status(&report, AFTER_KEY), "fresh");

    let claude = project.path().join(".claude/skills/demo");
    let codex = project.path().join(".agents/skills/demo");
    assert_eq!(
        fs::read_to_string(claude.join("SKILL.md")).unwrap(),
        "first\n"
    );
    assert_eq!(
        fs::read_to_string(codex.join("SKILL.md")).unwrap(),
        "first\n"
    );
    assert!(!project.path().join(".opencode/skills/demo").exists());
    assert_eq!(fs::read_to_string(&user_canary).unwrap(), "USER-CANARY\n");

    let state = lifecycle_state(project.path());
    let artifacts = &state.execution[SKILL_KEY].artifacts;
    assert_eq!(artifacts.len(), 2);
    assert_eq!(
        artifacts[0].id,
        "org.example/lifecycle-skills#skill:demo:claude"
    );
    assert_eq!(artifacts[0].kind, "agent-skill");
    assert_eq!(artifacts[0].path, vibe_core::machine_json_path(&claude));
    assert_eq!(
        artifacts[1].id,
        "org.example/lifecycle-skills#skill:demo:codex"
    );
    assert_eq!(artifacts[1].path, vibe_core::machine_json_path(&codex));

    let mut human = command(&user, project.path(), fake_home.path(), false);
    let human = human.arg("--force").output().unwrap();
    assert!(human.status.success());
    let stdout = String::from_utf8(human.stdout).unwrap();
    assert!(stdout.contains(SKILL_KEY), "{stdout}");
    assert!(
        stdout.contains("package binding [org.example/lifecycle-skills]: projected skill `demo`"),
        "{stdout}"
    );
    // The engine rows narrate through the same package-binding channel.
    assert!(
        stdout.contains("package binding [org.example/lifecycle-skills]: recovered 0 pending"),
        "{stdout}"
    );
    assert!(
        stdout.contains("package binding [org.example/lifecycle-skills]: reconciled 0 vanished"),
        "{stdout}"
    );
    assert_eq!(fs::read_to_string(&user_canary).unwrap(), "USER-CANARY\n");
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#PHASE-FINGERPRINT")]
fn source_changes_and_deletion_reconcile_only_the_owned_skill_directory() {
    let user = UserScratch::new();
    let project = project(true, &["claude"], false);
    let fake_home = tempfile::tempdir().unwrap();
    let neighbor = project.path().join(".claude/skills/foreign/sentinel.txt");
    fs::create_dir_all(neighbor.parent().unwrap()).unwrap();
    fs::write(&neighbor, "neighbor\n").unwrap();

    let (_, first) = run_json(&user, project.path(), fake_home.path());
    assert_eq!(report_status(&first, SKILL_KEY), "ok");
    let target = project.path().join(".claude/skills/demo");
    assert!(!target.join("noise.txt").exists());
    assert_eq!(fs::read_to_string(&neighbor).unwrap(), "neighbor\n");

    let (_, second) = run_json(&user, project.path(), fake_home.path());
    assert_eq!(report_status(&second, SKILL_KEY), "fresh");

    fs::write(target.join("stale.txt"), "owned stale\n").unwrap();
    fs::write(project.path().join("skills/demo/SKILL.md"), "second\n").unwrap();
    let (_, changed) = run_json(&user, project.path(), fake_home.path());
    assert_eq!(report_status(&changed, SKILL_KEY), "ok");
    assert_eq!(
        fs::read_to_string(target.join("SKILL.md")).unwrap(),
        "second\n"
    );
    assert_eq!(
        fs::read_to_string(target.join("stale.txt")).unwrap(),
        "owned stale\n",
        "unrecorded neighbors are not owned by the receipt"
    );
    assert_eq!(fs::read_to_string(&neighbor).unwrap(), "neighbor\n");

    fs::remove_file(project.path().join("skills/demo/references/guide.md")).unwrap();
    let (_, stale_owned) = run_json(&user, project.path(), fake_home.path());
    assert_eq!(report_status(&stale_owned, SKILL_KEY), "ok");
    assert!(!target.join("references/guide.md").exists());
    assert!(target.join("stale.txt").exists());

    fs::remove_dir_all(project.path().join("skills/demo")).unwrap();
    let (_, deleted) = run_json(&user, project.path(), fake_home.path());
    assert_eq!(report_status(&deleted, SKILL_KEY), "ok");
    assert!(
        target.exists(),
        "the unrecorded neighbor keeps the target alive"
    );
    assert!(!target.join("SKILL.md").exists());
    assert!(target.join("stale.txt").exists());
    assert_eq!(fs::read_to_string(&neighbor).unwrap(), "neighbor\n");
    assert!(
        lifecycle_state(project.path()).execution[SKILL_KEY]
            .artifacts
            .is_empty()
    );
}

#[test]
fn no_declared_skills_leave_package_as_an_algorithmic_no_op() {
    let user = UserScratch::new();
    let project = project(false, &[], false);
    let fake_home = tempfile::tempdir().unwrap();
    let (plan, report) = run_json(&user, project.path(), fake_home.path());
    assert!(plan.contributions.is_empty(), "{plan:?}");
    assert!(report.contributions.is_empty(), "{report:?}");
    assert_eq!(
        report.steps.last().map(|step| step.status.as_str()),
        Some("no-op")
    );
    assert!(!project.path().join(".claude").exists());
    assert!(fs::read_dir(fake_home.path()).unwrap().next().is_none());
}
