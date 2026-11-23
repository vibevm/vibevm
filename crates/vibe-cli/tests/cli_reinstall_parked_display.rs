//! Human reinstall sections and trailers, exercised through real hosted parking.

mod common;

use assert_cmd::Command;
use common::UserScratch;
use common::hosted_slot::{
    assert_ok, lifecycle_state, publish_slot_agent, registry_url, sole_document, sole_root,
    write_declared_output,
};
use std::path::Path;
use vibe_wire::generated::lifecycle_state::ExecutionRecordStatus;
use vibe_wire::generated::reinstall_report::ReinstallReport;

// Keep every invocation, including fixture initialization, inside UserScratch's
// temporary settings; explicitly pin HOME as well. No provider is configured.
fn command(user: &UserScratch) -> Command {
    let mut command = user.vibe();
    command.env("HOME", &user.settings);
    command.env("NO_COLOR", "1");
    command
}

fn project_at(user: &UserScratch, registry: &Path) -> tempfile::TempDir {
    let project = tempfile::tempdir().unwrap();
    let initialized = command(user)
        .args(["init", "--path"])
        .arg(project.path())
        .output()
        .unwrap();
    assert_ok(&initialized);
    common::write_project_with_per_package_registry(project.path(), &registry_url(registry));
    project
}

fn seed(user: &UserScratch, project: &Path) {
    let parked = command(user)
        .args(["install", "org.demo/tools", "--assume-yes", "--json"])
        .args(["--agent-mode", "agent", "--path"])
        .arg(project)
        .output()
        .unwrap();
    assert_ok(&parked);
    assert!(sole_document(&parked.stdout)["delegation"].is_object());
    write_declared_output(project);
    let resumed = command(user)
        .args(["install", "--assume-yes", "--json"])
        .args(["--agent-mode", "agent", "--path"])
        .arg(project)
        .output()
        .unwrap();
    assert_ok(&resumed);
    assert!(lifecycle_state(project).run.slot_continuation.is_none());
}

#[test]
fn reinstall_parked_output_ends_with_resume_command() {
    assert!(common::git_available(), "hosted fixture requires git");
    let registry = tempfile::tempdir().unwrap();
    let published = publish_slot_agent(registry.path(), "slot:post-install", "0.1.0");
    let user = UserScratch::new();
    let project = project_at(&user, &published.registry);
    seed(&user, project.path());
    std::fs::write(
        project
            .path()
            .join(vibe_core::layout::current_vibedeps_root())
            .join("org.demo.tools/0.1.0/payload.txt"),
        "corrupted\n",
    )
    .unwrap();

    let parked = command(&user)
        .args(["reinstall", "--force", "--assume-yes"])
        .args(["--agent-mode", "agent"])
        .arg(project.path())
        .output()
        .unwrap();
    assert_ok(&parked);
    let stdout = String::from_utf8(parked.stdout).unwrap();
    let lines: Vec<_> = stdout.lines().filter(|line| !line.is_empty()).collect();
    assert_eq!(stdout.matches('✻').count(), 1, "{stdout}");
    assert_eq!(
        &lines[lines.len() - 2..],
        [
            "✻ parked at agent tasks · 1 task awaits the hosting agent · resume with",
            "  vibe reinstall",
        ],
        "{stdout}",
    );
    let heading = stdout
        .find("── reinstall --force · re-fetch 1 package")
        .expect("forced reinstall opens with its refetch rule");
    let row = stdout
        .find("  org.demo/tools@0.1.0")
        .expect("full package row");
    assert!(heading < row && row < stdout.find('✻').unwrap(), "{stdout}");
    assert!(!stdout.contains("✻ reinstalled"), "{stdout}");
    let state = lifecycle_state(project.path());
    let continuation = state
        .run
        .slot_continuation
        .as_ref()
        .expect("parked slot run");
    assert_eq!(continuation.targets.len(), 1);
    assert_eq!(continuation.targets[0].group, "org.demo");
    assert_eq!(continuation.targets[0].name, "tools");
    assert_eq!(continuation.targets[0].version, "0.1.0");
    let delegated: Vec<_> = state
        .execution
        .values()
        .filter(|row| row.status == ExecutionRecordStatus::Delegated)
        .collect();
    assert_eq!(delegated.len(), 1, "{state:?}");
    assert_eq!(
        delegated[0].scope,
        Some(vibe_wire::generated::lifecycle_state::ExecutionRecordScope::Slot),
    );
    let tasks = &delegated[0].tasks;
    assert_eq!(tasks.len(), 1);
    assert!(project.path().join(&tasks[0]).is_file());
    let run_id = state.run.run_id.clone();

    // Obtain the typed protocol comparison in an independent fresh fixture.
    // A command on the human subject here would service, not observe, its park.
    let json_user = UserScratch::new();
    let json_project = project_at(&json_user, &published.registry);
    seed(&json_user, json_project.path());
    std::fs::remove_file(json_project.path().join("docs/slot.md")).unwrap();
    std::fs::write(
        json_project
            .path()
            .join(vibe_core::layout::current_vibedeps_root())
            .join("org.demo.tools/0.1.0/payload.txt"),
        "corrupted\n",
    )
    .unwrap();
    let json_parked = command(&json_user)
        .args([
            "reinstall",
            "--force",
            "--json",
            "--assume-yes",
            "--agent-mode",
            "agent",
        ])
        .arg(json_project.path())
        .output()
        .unwrap();
    assert_ok(&json_parked);
    let protocol: ReinstallReport =
        serde_json::from_value(sole_document(&json_parked.stdout)).unwrap();
    let handoff = protocol
        .delegation
        .expect("independent forced reinstall parked");
    assert_eq!(handoff.resume, lines[lines.len() - 1].trim());
    assert_eq!(handoff.tasks.len(), tasks.len());
    assert!(json_project.path().join(&handoff.tasks[0]).is_file());
    assert!(
        lifecycle_state(json_project.path())
            .run
            .slot_continuation
            .is_some()
    );
    write_declared_output(project.path());
    let resumed = command(&user)
        .args(lines[lines.len() - 1].split_whitespace().skip(1))
        .args(["--assume-yes", "--json", "--agent-mode", "agent"])
        .arg(project.path())
        .output()
        .unwrap();
    assert_ok(&resumed);
    let report: ReinstallReport =
        serde_json::from_value(sole_root(&resumed.stdout, "reinstall")).unwrap();
    assert!(report.delegation.is_none());
    assert!(!report.forced);
    let settled = lifecycle_state(project.path());
    assert_eq!(settled.run.run_id, run_id);
    assert!(settled.run.slot_continuation.is_none());
    assert!(
        settled
            .execution
            .values()
            .all(|row| row.status != ExecutionRecordStatus::Delegated)
    );
    assert!(!project.path().join(&tasks[0]).exists());
}

#[test]
fn ordinary_reinstall_rule_precedes_the_only_success_trailer() {
    assert!(common::git_available(), "hosted fixture requires git");
    let registry = tempfile::tempdir().unwrap();
    let published = publish_slot_agent(registry.path(), "slot:post-install", "0.1.0");
    let user = UserScratch::new();
    let project = project_at(&user, &published.registry);
    seed(&user, project.path());
    let output = command(&user)
        .args(["reinstall", "--assume-yes", "--agent-mode", "agent"])
        .arg(project.path())
        .output()
        .unwrap();
    assert_ok(&output);
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(stdout.matches('✻').count(), 1, "{stdout}");
    let heading = stdout
        .find("── reinstall · regenerate boot for 1 node from vibedeps/")
        .expect("ordinary reinstall opens with its regenerate rule");
    let trailer = stdout
        .find("✻ regenerated boot for 1 node · ")
        .expect("success trailer");
    assert!(heading < trailer, "{stdout}");
    assert!(
        stdout
            .lines()
            .rfind(|line| !line.is_empty())
            .unwrap()
            .starts_with("✻ regenerated boot for 1 node · "),
        "{stdout}"
    );
}
