//! `vibe prompt init` opts a project in by writing its authored template,
//! builds nothing, refuses an existing template, and is the command every
//! "not opted in" message names.

mod common;

use std::fs;
use std::path::PathBuf;

use common::UserScratch;

const FRAME: &str = "You are working inside org.example/demo, a vibevm-managed project. The\n\
project's own instructions and its compiled boot lane follow this frame;\n\
treat them as the project's standing rules.\n";

fn project(user: &UserScratch) -> tempfile::TempDir {
    let project = tempfile::tempdir().unwrap();
    user.vibe()
        .args(["init", "--group", "org.example", "--name", "demo", "--path"])
        .arg(project.path())
        .assert()
        .success();
    project
}

fn boot(project: &tempfile::TempDir, name: &str) -> PathBuf {
    project
        .path()
        .join(vibe_core::layout::current_boot_dir())
        .join(name)
}

#[test]
fn prompt_init_writes_the_template_and_builds_nothing() {
    let user = UserScratch::new();
    let project = project(&user);
    let output = user
        .vibe()
        .args(["prompt", "init", "--path"])
        .arg(project.path())
        .arg("--json")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let report: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(report["protocol"], "vibe-prompt-init/1");
    assert_eq!(report["node"], ".");
    assert_eq!(
        report["template"],
        "vibevm/vibespecs/boot/SYSTEM-TEMPLATE.md"
    );
    assert_eq!(report["created"], true);
    assert_eq!(
        fs::read_to_string(boot(&project, "SYSTEM-TEMPLATE.md")).unwrap(),
        FRAME
    );
    assert!(!boot(&project, "SYSTEM.md").exists());
    assert!(!boot(&project, "AGENT-LANE.md").exists());
}

#[test]
fn prompt_init_refuses_an_existing_template() {
    let user = UserScratch::new();
    let project = project(&user);
    user.vibe()
        .args(["prompt", "init", "--path"])
        .arg(project.path())
        .assert()
        .success();
    let template = boot(&project, "SYSTEM-TEMPLATE.md");
    fs::write(&template, "# My own frame\n").unwrap();
    let output = user
        .vibe()
        .args(["prompt", "init", "--path"])
        .arg(project.path())
        .assert()
        .failure()
        .get_output()
        .clone();
    let said = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(said.contains("template_exists"), "{said}");
    assert!(said.contains("already opted in"), "{said}");
    assert_eq!(fs::read_to_string(&template).unwrap(), "# My own frame\n");
}
