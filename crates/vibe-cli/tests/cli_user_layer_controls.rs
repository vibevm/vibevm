//! User loading is per-project, independent from override and read-only view hiding.
mod common;
use common::UserScratch;
use std::{fs, path::Path};
fn control(user: &UserScratch, project: &Path, enabled: bool) {
    user.vibe()
        .args(["user", if enabled { "enable" } else { "disable" }, "--path"])
        .arg(project)
        .arg("--json")
        .assert()
        .success();
}
fn capture(user: &UserScratch, project: &Path) -> String {
    let o = user
        .vibe()
        .current_dir(project)
        .args([
            "prompt",
            "capture",
            "--session-target",
            "os=linux",
            "--with-text",
            "--json",
        ])
        .output()
        .unwrap();
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    String::from_utf8(o.stdout).unwrap()
}
#[test]
fn disabling_personal_without_override_changes_next_capture_only_for_selected_project() {
    let user = UserScratch::new();
    let a = tempfile::tempdir().unwrap();
    let b = tempfile::tempdir().unwrap();
    for project in [a.path(), b.path()] {
        user.init_project(project);
        fs::write(
            project
                .join(vibe_core::layout::current_boot_dir())
                .join("SYSTEM-TEMPLATE.md"),
            "CONTROL_PROJECT\n",
        )
        .unwrap();
        user.vibe()
            .args(["prompt", "build", "--path"])
            .arg(project)
            .assert()
            .success();
    }
    user.vibe().args(["user", "init"]).assert().success();
    let personal = user.settings.join("user-project");
    let boot = personal.join(vibe_core::layout::current_boot_dir());
    fs::create_dir_all(&boot).unwrap();
    fs::write(boot.join("preferences.md"), "USER_LAYER_CONTROL_CANARY\n").unwrap();
    user.vibe()
        .args(["install", "-g", "--user-project", "--offline", "--json"])
        .assert()
        .success();
    assert!(capture(&user, a.path()).contains("USER_LAYER_CONTROL_CANARY"));
    control(&user, a.path(), false);
    assert!(!capture(&user, a.path()).contains("USER_LAYER_CONTROL_CANARY"));
    assert!(capture(&user, b.path()).contains("USER_LAYER_CONTROL_CANARY"));
    assert!(!user.settings.join("project-overrides").exists());
    let mapping = user.settings.join("override-bindings.toml");
    let bytes = fs::read(&mapping).unwrap();
    let m: toml::Value = toml::from_str(std::str::from_utf8(&bytes).unwrap()).unwrap();
    assert_eq!(m["schema"].as_integer(), Some(2));
    control(&user, a.path(), true);
    assert!(capture(&user, a.path()).contains("USER_LAYER_CONTROL_CANARY"));
    assert!(boot.join("preferences.md").exists());
    let _ = m;
}
#[test]
fn schema1_observation_never_migrates_and_explicit_control_upgrades_to2() {
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    user.init_project(project.path());
    let mapping = user.settings.join("override-bindings.toml");
    fs::write(&mapping, "schema = 1\nbindings = []\n").unwrap();
    let before = fs::read(&mapping).unwrap();
    user.vibe()
        .args(["tree", "--layers", "--plain", "--path"])
        .arg(project.path())
        .assert()
        .success();
    assert_eq!(fs::read(&mapping).unwrap(), before);
    control(&user, project.path(), false);
    let text = fs::read_to_string(&mapping).unwrap();
    let m: toml::Value = toml::from_str(&text).unwrap();
    assert_eq!(m["schema"].as_integer(), Some(2));
    assert_eq!(m["user_controls"][0]["user_enabled"].as_bool(), Some(false));
}
