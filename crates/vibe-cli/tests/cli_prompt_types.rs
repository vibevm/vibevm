//! Generated subagent types use the published lane and report unavailable bodies.

mod common;

use std::fs;

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
