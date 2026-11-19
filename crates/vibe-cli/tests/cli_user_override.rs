//! Explicit binding and ordinary override project lifecycle, isolated from real settings.
mod common;
use common::UserScratch;
use std::{fs, path::Path};
fn command(
    user: &UserScratch,
    project: &Path,
    verb: &str,
    alias: Option<&str>,
) -> serde_json::Value {
    let mut cmd = user.vibe();
    cmd.args(["user", "override", verb, "--path"])
        .arg(project)
        .arg("--json");
    if let Some(alias) = alias {
        cmd.args(["--alias", alias]);
    }
    let result = cmd.output().unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    serde_json::from_slice(&result.stdout).unwrap()
}
#[test]
fn explicit_override_binding_is_idempotent_conflicts_refuse_and_reclone_requires_bind() {
    let user = UserScratch::new();
    let first = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    user.init_project(first.path());
    user.init_project(second.path());
    assert_eq!(
        command(&user, first.path(), "status", None)["state"],
        "absent"
    );
    assert!(!user.settings.join("override-bindings.toml").exists());
    let created = command(&user, first.path(), "init", Some("work"));
    let root = std::path::PathBuf::from(created["root"].as_str().unwrap());
    assert!(root.starts_with(user.settings.join("project-overrides")));
    assert!(created["enabled"].as_bool().unwrap());
    let before = fs::read(user.settings.join("override-bindings.toml")).unwrap();
    assert_eq!(
        command(&user, first.path(), "init", Some("work"))["root"],
        created["root"]
    );
    assert_eq!(
        fs::read(user.settings.join("override-bindings.toml")).unwrap(),
        before
    );
    for (project, alias) in [(first.path(), "other"), (second.path(), "work")] {
        user.vibe()
            .args(["user", "override", "init", "--path"])
            .arg(project)
            .args(["--alias", alias, "--json"])
            .assert()
            .failure();
        assert_eq!(
            fs::read(user.settings.join("override-bindings.toml")).unwrap(),
            before
        );
    }
    assert_eq!(
        command(&user, second.path(), "status", None)["state"],
        "absent"
    );
    command(&user, first.path(), "disable", None);
    assert_eq!(
        command(&user, second.path(), "bind", Some("work"))["enabled"],
        false
    );
    assert_eq!(
        command(&user, second.path(), "enable", None)["root"],
        created["root"]
    );
    assert_eq!(
        command(&user, first.path(), "status", None)["enabled"],
        true
    );
    assert!(root.join("vibe.toml").exists());
    assert!(!first.path().join("project-overrides").exists());
    assert!(!user.settings.join("user-project").exists());
}

#[test]
fn override_boot_is_last_and_disable_preserves_files_but_removes_next_request_layer() {
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    user.init_project(project.path());
    let boot = project.path().join(vibe_core::layout::current_boot_dir());
    fs::write(boot.join("SYSTEM-TEMPLATE.md"), "project-frame\n").unwrap();
    fs::write(boot.join("project.md"), "project-order-marker\n").unwrap();
    user.vibe()
        .args(["prompt", "build", "--path"])
        .arg(project.path())
        .assert()
        .success();
    let prefix = fs::read(boot.join("SYSTEM.md")).unwrap();
    user.vibe().args(["user", "init"]).assert().success();
    let personal = user.settings.join("user-project");
    user.vibe()
        .args(["install", "-g", "--user-project", "--offline", "--json"])
        .assert()
        .success();
    fs::write(
        personal
            .join(vibe_core::layout::current_boot_dir())
            .join("preferences.md"),
        "personal-order-marker\n",
    )
    .unwrap();
    user.vibe()
        .args(["install", "-g", "--user-project", "--offline", "--json"])
        .assert()
        .success();
    let selected = command(&user, project.path(), "init", Some("project-choice"));
    let root = std::path::PathBuf::from(selected["root"].as_str().unwrap());
    user.vibe()
        .args(["install", "--path"])
        .arg(&root)
        .args(["--offline", "--json"])
        .assert()
        .success();
    let preferences = root
        .join(vibe_core::layout::current_boot_dir())
        .join("preferences.md");
    fs::write(&preferences, "override-order-marker\n").unwrap();
    user.vibe()
        .args(["install", "--path"])
        .arg(&root)
        .args(["--offline", "--json"])
        .assert()
        .success();
    let capture = || {
        let result = user
            .vibe()
            .args(["prompt", "capture", "--path"])
            .arg(project.path())
            .args(["--session-target", "os=linux", "--with-text", "--json"])
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        let report: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
        report["instruction_blocks"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| row["text"].as_str().unwrap())
            .collect::<Vec<_>>()
            .join("\n")
    };
    let text = capture();
    assert!(
        text.find("project-order-marker").unwrap() < text.find("personal-order-marker").unwrap()
    );
    assert!(
        text.find("personal-order-marker").unwrap() < text.find("override-order-marker").unwrap()
    );
    assert_eq!(fs::read(boot.join("SYSTEM.md")).unwrap(), prefix);
    command(&user, project.path(), "disable", None);
    assert!(!capture().contains("override-order-marker"));
    assert_eq!(
        fs::read_to_string(&preferences).unwrap(),
        "override-order-marker\n"
    );
    fs::write(root.join("vibe.toml"), "invalid").unwrap();
    let disabled = command(&user, project.path(), "disable", None);
    assert_eq!(disabled["state"], "unavailable");
    assert_eq!(disabled["enabled"], false);
    assert!(capture().contains("personal-order-marker"));
}

#[test]
fn override_prestop_and_invalid_alias_never_create_mapping_or_root() {
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    user.init_project(project.path());
    let stop = project.path().join("stop");
    fs::write(&stop, "stop").unwrap();
    user.vibe()
        .args(["user", "override", "init", "--alias", "work", "--path"])
        .arg(project.path())
        .arg("--stop-file")
        .arg(stop)
        .arg("--json")
        .assert()
        .code(130);
    user.vibe()
        .args(["user", "override", "init", "--alias", "../bad", "--path"])
        .arg(project.path())
        .arg("--json")
        .assert()
        .failure();
    assert!(!user.settings.join("override-bindings.toml").exists());
    assert!(!user.settings.join("project-overrides").exists());
}

#[test]
fn git_common_directory_binding_is_shared_by_linked_worktrees_without_remote_matching() {
    let user = UserScratch::new();
    let scope = tempfile::tempdir().unwrap();
    let first = scope.path().join("first");
    fs::create_dir(&first).unwrap();
    user.init_project(&first);
    let git = |cwd: &Path, args: &[&str]| {
        let output = std::process::Command::new("git")
            .current_dir(cwd)
            .args(args)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    };
    git(&first, &["init"]);
    git(&first, &["add", "."]);
    git(
        &first,
        &[
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "-m",
            "fixture",
        ],
    );
    let initial = command(&user, &first, "init", Some("shared"));
    let sibling = scope.path().join("sibling");
    git(
        &first,
        &[
            "worktree",
            "add",
            "-b",
            "fixture-sibling",
            sibling.to_str().unwrap(),
        ],
    );
    assert_eq!(
        command(&user, &sibling, "status", None)["root"],
        initial["root"]
    );
    command(&user, &sibling, "disable", None);
    assert_eq!(command(&user, &first, "status", None)["enabled"], false);
}

#[cfg(unix)]
#[test]
fn override_root_symlink_is_unavailable_and_never_mutates_its_target() {
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    user.init_project(project.path());
    let report = command(&user, project.path(), "init", Some("unsafe-root"));
    let root = std::path::PathBuf::from(report["root"].as_str().unwrap());
    let preserved = root.with_extension("preserved");
    fs::rename(&root, &preserved).unwrap();
    let target = tempfile::tempdir().unwrap();
    fs::write(target.path().join("sentinel"), "untouched").unwrap();
    std::os::unix::fs::symlink(target.path(), &root).unwrap();
    user.vibe()
        .args(["user", "override", "enable", "--path"])
        .arg(project.path())
        .arg("--json")
        .assert()
        .failure();
    let disabled = command(&user, project.path(), "disable", None);
    assert_eq!(disabled["enabled"], false);
    assert_eq!(
        fs::read_to_string(target.path().join("sentinel")).unwrap(),
        "untouched"
    );
    assert!(!target.path().join("vibe.toml").exists());
}
