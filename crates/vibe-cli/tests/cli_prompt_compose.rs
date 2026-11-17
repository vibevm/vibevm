//! Explicit local prefix composition preserves the shared project leading bytes.
mod common;
use common::UserScratch;
use std::{fs, path::Path};
fn compose(user: &UserScratch, project: &Path) -> serde_json::Value {
    let output = user
        .vibe()
        .args(["prompt", "compose", "--path"])
        .arg(project)
        .args(["--session-target", "os=linux", "--json"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    common::wire::report::<vibe_wire::generated::prompt_compose::PromptComposeReport>(
        std::str::from_utf8(&output.stdout).unwrap(),
    )
}
#[test]
fn local_composition_keeps_exact_project_prefix_then_live_user_and_override() {
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    user.init_project(project.path());
    let boot = project.path().join(vibe_core::layout::current_boot_dir());
    fs::write(boot.join("SYSTEM-TEMPLATE.md"), "compose-project-frame\n").unwrap();
    fs::write(boot.join("live.md"), "compose-project-live\n").unwrap();
    user.vibe()
        .args(["prompt", "build", "--path"])
        .arg(project.path())
        .assert()
        .success();
    let prefix = fs::read(boot.join("SYSTEM.md")).unwrap();
    let absent = compose(&user, project.path());
    assert_eq!(absent["local"], false);
    assert!(!boot.join("local").exists());
    assert_eq!(fs::read(absent["path"].as_str().unwrap()).unwrap(), prefix);
    user.vibe().args(["user", "init"]).assert().success();
    user.vibe()
        .args(["install", "-g", "--user-project", "--offline", "--json"])
        .assert()
        .success();
    let personal = user
        .settings
        .join("user-project")
        .join(vibe_core::layout::current_boot_dir())
        .join("preferences.md");
    fs::write(&personal, "compose-personal-one\n").unwrap();
    user.vibe()
        .args(["install", "-g", "--user-project", "--offline", "--json"])
        .assert()
        .success();
    let output = user
        .vibe()
        .args(["user", "override", "init", "--alias", "compose", "--path"])
        .arg(project.path())
        .arg("--json")
        .output()
        .unwrap();
    assert!(output.status.success());
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let root = std::path::PathBuf::from(report["root"].as_str().unwrap());
    user.vibe()
        .args(["install", "--path"])
        .arg(&root)
        .args(["--offline", "--json"])
        .assert()
        .success();
    let over = root
        .join(vibe_core::layout::current_boot_dir())
        .join("preferences.md");
    fs::write(&over, "compose-override-one\n").unwrap();
    user.vibe()
        .args(["install", "--path"])
        .arg(&root)
        .args(["--offline", "--json"])
        .assert()
        .success();
    // Read-only consumers still must not publish a private composed artifact.
    user.vibe()
        .args(["prompt", "capture", "--path"])
        .arg(project.path())
        .args(["--session-target", "os=linux", "--json"])
        .assert()
        .success();
    user.vibe()
        .args(["prompt", "flags", "--harness", "claude", "--path"])
        .arg(project.path())
        .assert()
        .success();
    user.vibe()
        .args(["user", "status", "--json"])
        .assert()
        .success();
    assert!(!boot.join("local").exists());
    let first = compose(&user, project.path());
    let bytes = fs::read(first["path"].as_str().unwrap()).unwrap();
    assert!(bytes.starts_with(&prefix));
    let text = String::from_utf8(bytes).unwrap();
    assert!(
        text.find("compose-project-live").unwrap() < text.find("compose-personal-one").unwrap()
    );
    assert!(
        text.find("compose-personal-one").unwrap() < text.find("compose-override-one").unwrap()
    );
    assert!(first["path"].as_str().unwrap().contains("/local/SYSTEM-"));
    fs::write(&personal, "compose-personal-two\n").unwrap();
    fs::write(&over, "compose-override-two\n").unwrap();
    let second = compose(&user, project.path());
    assert_ne!(first["path"], second["path"]);
    let text = fs::read_to_string(second["path"].as_str().unwrap()).unwrap();
    assert!(text.contains("compose-personal-two") && text.contains("compose-override-two"));
    user.vibe()
        .args(["user", "override", "disable", "--path"])
        .arg(project.path())
        .assert()
        .success();
    let disabled = compose(&user, project.path());
    assert!(
        !fs::read_to_string(disabled["path"].as_str().unwrap())
            .unwrap()
            .contains("compose-override-two")
    );
    assert_eq!(fs::read(boot.join("SYSTEM.md")).unwrap(), prefix);
    assert_eq!(fs::read(boot.join("local/.gitignore")).unwrap(), b"*\n");
}
