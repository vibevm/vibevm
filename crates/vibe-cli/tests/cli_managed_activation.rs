//! Real command contracts under isolated temporary homes and client configurations.
#![cfg(target_os = "linux")]
mod common;
use common::UserScratch;
use serde_json::Value;
use std::{fs, path::Path};
#[cfg(test)]
fn isolated(user: &UserScratch, home: &Path) -> assert_cmd::Command {
    let bin = home.join("bin");
    fs::create_dir_all(&bin).unwrap();
    for name in ["claude", "codex", "opencode"] {
        let p = bin.join(name);
        fs::write(&p, "#!/bin/sh\nexit 0\n").unwrap();
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(p, fs::Permissions::from_mode(0o755)).unwrap();
    }
    let mut command = user.vibe();
    command
        .env_remove("VIBE_INVOKED_BY")
        .env_remove("VIBE_AGENT_COMMAND")
        .env_remove("VIBE_AGENT_ARGS");
    command
        .env("HOME", home)
        .env("XDG_CONFIG_HOME", home.join(".config"))
        .env("CODEX_HOME", home.join(".codex"))
        .env("CLAUDE_CONFIG_DIR", home.join(".claude"))
        .env("GIT_CONFIG_GLOBAL", home.join(".gitconfig"))
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env(
            "PATH",
            format!(
                "{}:{}",
                bin.display(),
                std::env::var("PATH").unwrap_or_default()
            ),
        );
    command
}
#[cfg(test)]
fn init(user: &UserScratch, home: &Path, project: &Path) {
    isolated(user, home)
        .arg("init")
        .arg("--path")
        .arg(project)
        .assert()
        .success();
}
#[cfg(test)]
fn run(user: &UserScratch, home: &Path, args: &[&str]) -> Value {
    let output = isolated(user, home).args(args).output().unwrap();
    assert!(
        output.status.success(),
        "argv={args:?} stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}
#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-073#MANAGED-ACTIVATION")]
fn builtin_mcp_choice_survives_new_process_install_upgrade_and_reenable() {
    let user = UserScratch::new();
    let root = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    init(&user, home.path(), root.path());
    let path = root.path().to_str().unwrap();
    run(
        &user,
        home.path(),
        &[
            "mcp", "install", "--path", path, "--agent", "claude", "--scope", "project", "--what",
            "mcp", "--force", "--yes", "--json",
        ],
    );
    let off = run(
        &user,
        home.path(),
        &[
            "mcp", "disable", "vibevm", "--path", path, "--agent", "claude", "--scope", "project",
            "--actor", "operator", "--json",
        ],
    );
    assert_eq!(off["results"][0]["observed"], "disabled");
    run(
        &user,
        home.path(),
        &[
            "mcp", "install", "--path", path, "--agent", "claude", "--scope", "project", "--what",
            "mcp", "--force", "--yes", "--json",
        ],
    );
    run(
        &user,
        home.path(),
        &[
            "mcp",
            "upgrade",
            "--path",
            path,
            "--agent",
            "claude",
            "--scope",
            "project",
            "--config-only",
            "--yes",
            "--json",
        ],
    );
    let config: Value =
        serde_json::from_slice(&fs::read(root.path().join(".mcp.json")).unwrap()).unwrap();
    assert!(config["mcpServers"].get("vibevm").is_none());
    let on = run(
        &user,
        home.path(),
        &[
            "mcp", "enable", "vibevm", "--path", path, "--agent", "claude", "--scope", "project",
            "--json",
        ],
    );
    assert_eq!(on["results"][0]["observed"], "registered");
    assert!(root.path().join("vibe.toml").exists());
}
#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-073#MANAGED-ACTIVATION")]
fn user_and_project_builtin_skills_keep_independent_intent() {
    let user = UserScratch::new();
    let root = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    init(&user, home.path(), root.path());
    let path = root.path().to_str().unwrap();
    let installed = run(
        &user,
        home.path(),
        &[
            "mcp", "install", "--path", path, "--agent", "claude", "--scope", "both", "--what",
            "skill", "--force", "--yes", "--json",
        ],
    );
    assert!(
        home.path()
            .join(".claude/skills/.vibevm.vibe-skill-receipt.toml")
            .exists(),
        "initial user receipt missing; report={installed}"
    );
    let disabled = run(
        &user,
        home.path(),
        &[
            "skill",
            "disable",
            "vibevm",
            "--package",
            "org.vibevm.core/vibevm",
            "--path",
            path,
            "--agent",
            "claude",
            "--scope",
            "user",
            "--actor",
            "operator",
            "--json",
        ],
    );
    assert_eq!(disabled["results"][0]["observed"], "disabled");
    assert!(root.path().join(".claude/skills/vibevm/SKILL.md").exists());
    assert!(!home.path().join(".claude/skills/vibevm/SKILL.md").exists());
    run(
        &user,
        home.path(),
        &[
            "mcp", "install", "--path", path, "--agent", "claude", "--scope", "both", "--what",
            "skill", "--force", "--yes", "--json",
        ],
    );
    assert!(!home.path().join(".claude/skills/vibevm/SKILL.md").exists());
    let enabled = run(
        &user,
        home.path(),
        &[
            "skill",
            "enable",
            "vibevm",
            "--package",
            "org.vibevm.core/vibevm",
            "--path",
            path,
            "--agent",
            "claude",
            "--scope",
            "user",
            "--json",
        ],
    );
    assert_eq!(enabled["results"][0]["observed"], "enabled");
}

#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-073#MANAGED-ACTIVATION")]
fn builtin_skill_migration_requires_real_explicit_register_command() {
    let user = UserScratch::new();
    let root = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    init(&user, home.path(), root.path());
    let path = root.path().to_str().unwrap();
    let target = root.path().join(".claude/skills/vibevm/SKILL.md");
    fs::create_dir_all(target.parent().unwrap()).unwrap();
    fs::write(&target, vibe_mcp::install::SKILL_TEMPLATE).unwrap();
    let receipt = root
        .path()
        .join(".claude/skills/.vibevm.vibe-skill-receipt.toml");
    let args = [
        "skill",
        "disable",
        "vibevm",
        "--package",
        "org.vibevm.core/vibevm",
        "--path",
        path,
        "--agent",
        "claude",
        "--scope",
        "project",
        "--json",
    ];
    let refused = isolated(&user, home.path()).args(args).output().unwrap();
    assert!(!refused.status.success());
    assert!(!receipt.exists());
    assert_eq!(
        fs::read_to_string(&target).unwrap(),
        vibe_mcp::install::SKILL_TEMPLATE
    );
    let status = run(
        &user,
        home.path(),
        &[
            "skill",
            "status",
            "vibevm",
            "--package",
            "org.vibevm.core/vibevm",
            "--path",
            path,
            "--agent",
            "claude",
            "--scope",
            "project",
            "--json",
        ],
    );
    assert_eq!(status["results"][0]["observed"], "foreign");
    assert!(!receipt.exists());
    let registered = run(
        &user,
        home.path(),
        &[
            "mcp", "install", "--path", path, "--agent", "claude", "--scope", "project", "--what",
            "skill", "--force", "--yes", "--json",
        ],
    );
    assert_eq!(registered["skill_results"][0]["status"], "adopted");
    let status = run(
        &user,
        home.path(),
        &[
            "skill",
            "status",
            "vibevm",
            "--package",
            "org.vibevm.core/vibevm",
            "--path",
            path,
            "--agent",
            "claude",
            "--scope",
            "project",
            "--json",
        ],
    );
    assert_eq!(
        status["results"][0]["activation"]["provenance"],
        "vibe mcp install:explicit-adoption"
    );
    assert!(status["results"][0]["activation"].get("actor").is_none());
    let disabled = run(&user, home.path(), &args);
    assert_eq!(disabled["results"][0]["observed"], "disabled");
}

#[cfg(test)]
fn refused(user: &UserScratch, home: &Path, args: &[&str]) -> String {
    let output = isolated(user, home).args(args).output().unwrap();
    assert!(!output.status.success(), "unexpected success argv={args:?}");
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-073#MANAGED-ACTIVATION")]
fn actual_cli_mcp_and_builtin_skill_retain_off_intent_on_foreign_edit() {
    let user = UserScratch::new();
    let root = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    init(&user, home.path(), root.path());
    let path = root.path().to_str().unwrap();
    run(
        &user,
        home.path(),
        &[
            "mcp", "install", "--path", path, "--agent", "claude", "--scope", "project", "--what",
            "both", "--force", "--yes", "--json",
        ],
    );
    let config = root.path().join(".mcp.json");
    let foreign = r#"{"mcpServers":{"vibevm":{"command":"foreign"}}}"#;
    fs::write(&config, foreign).unwrap();
    let error = refused(
        &user,
        home.path(),
        &[
            "mcp", "disable", "vibevm", "--path", path, "--agent", "claude", "--scope", "project",
            "--json",
        ],
    );
    assert!(error.contains("intent retained"));
    let saved: Value =
        serde_json::from_slice(&fs::read(root.path().join(".mcp.json.vibevm-mcp.json")).unwrap())
            .unwrap();
    assert_eq!(saved["entries"]["vibevm"]["activation"]["enabled"], false);
    assert_eq!(fs::read_to_string(config).unwrap(), foreign);
    let mcp_status = run(
        &user,
        home.path(),
        &["mcp", "status", "--path", path, "--json"],
    );
    let native = mcp_status["managed_builtin"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["agent"] == "claude" && r["scope"] == "project")
        .unwrap();
    assert_eq!(native["desired_enabled"], false);
    assert_eq!(native["state"], "changed");
    let skill = root.path().join(".claude/skills/vibevm/SKILL.md");
    fs::write(&skill, "foreign skill").unwrap();
    let error = refused(
        &user,
        home.path(),
        &[
            "skill",
            "disable",
            "vibevm",
            "--package",
            "org.vibevm.core/vibevm",
            "--path",
            path,
            "--agent",
            "claude",
            "--scope",
            "project",
            "--json",
        ],
    );
    assert!(error.contains("intent retained"));
    let status = run(
        &user,
        home.path(),
        &[
            "skill",
            "status",
            "vibevm",
            "--package",
            "org.vibevm.core/vibevm",
            "--path",
            path,
            "--agent",
            "claude",
            "--scope",
            "project",
            "--json",
        ],
    );
    assert_eq!(status["results"][0]["desired_enabled"], false);
    assert_eq!(status["results"][0]["observed"], "changed");
    assert!(status["results"][0].get("representation").is_none());
    assert_eq!(fs::read_to_string(skill).unwrap(), "foreign skill");
}

#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-073#MANAGED-ACTIVATION")]
fn actual_cli_declared_skill_retains_intent_before_unsafe_projection_lowering() {
    let user = UserScratch::new();
    let root = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    fs::write(root.path().join("vibe.toml"),"[package]\ngroup='org.example'\nname='tool'\nversion='1.0.0'\nkind='tool'\n[[skill]]\nname='demo'\npath='skills/demo'\nagents=['claude']\n").unwrap();
    fs::create_dir_all(root.path().join("skills/demo")).unwrap();
    fs::write(root.path().join("skills/demo/SKILL.md"), "owned").unwrap();
    let path = root.path().to_str().unwrap();
    run(
        &user,
        home.path(),
        &[
            "skill", "install", "--path", path, "--skill", "demo", "--agent", "claude", "--scope",
            "project", "--yes", "--json",
        ],
    );
    let target = root.path().join(".claude/skills/demo");
    fs::remove_file(target.join("SKILL.md")).unwrap();
    fs::remove_dir(&target).unwrap();
    fs::write(outside.path().join("SKILL.md"), "foreign").unwrap();
    std::os::unix::fs::symlink(outside.path(), &target).unwrap();
    let error = refused(
        &user,
        home.path(),
        &[
            "skill",
            "disable",
            "demo",
            "--package",
            "org.example/tool",
            "--path",
            path,
            "--agent",
            "claude",
            "--scope",
            "project",
            "--json",
        ],
    );
    assert!(error.contains("intent retained"));
    let saved: toml::Value =
        toml::from_str(&fs::read_to_string(root.path().join(".vibe/package-skills.toml")).unwrap())
            .unwrap();
    assert_eq!(
        saved["binding"][0]["activation"]["claude"]["enabled"].as_bool(),
        Some(false)
    );
    assert!(
        fs::symlink_metadata(&target)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert_eq!(
        fs::read_to_string(outside.path().join("SKILL.md")).unwrap(),
        "foreign"
    );
}

#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-073#MANAGED-ACTIVATION")]
fn builtin_multi_scope_admits_all_then_retains_all_before_projection() {
    let user = UserScratch::new();
    let root = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    init(&user, home.path(), root.path());
    let path = root.path().to_str().unwrap();
    run(
        &user,
        home.path(),
        &[
            "mcp", "install", "--path", path, "--agent", "claude", "--scope", "project", "--what",
            "skill", "--force", "--yes", "--json",
        ],
    );
    let receipt = root
        .path()
        .join(".claude/skills/.vibevm.vibe-skill-receipt.toml");
    let before = fs::read(&receipt).unwrap();
    let args = [
        "skill",
        "disable",
        "vibevm",
        "--package",
        "org.vibevm.core/vibevm",
        "--path",
        path,
        "--agent",
        "claude",
        "--scope",
        "both",
        "--json",
    ];
    refused(&user, home.path(), &args);
    assert_eq!(fs::read(&receipt).unwrap(), before);
    assert!(root.path().join(".claude/skills/vibevm/SKILL.md").exists());
    run(
        &user,
        home.path(),
        &[
            "mcp", "install", "--path", path, "--agent", "claude", "--scope", "both", "--what",
            "skill", "--force", "--yes", "--json",
        ],
    );
    fs::write(
        root.path().join(".claude/skills/vibevm/SKILL.md"),
        "foreign",
    )
    .unwrap();
    let error = refused(&user, home.path(), &args);
    assert!(error.contains("intent retained"));
    for receipt in [
        receipt,
        home.path()
            .join(".claude/skills/.vibevm.vibe-skill-receipt.toml"),
    ] {
        let saved: toml::Value = toml::from_str(&fs::read_to_string(receipt).unwrap()).unwrap();
        assert_eq!(saved["activation"]["enabled"].as_bool(), Some(false));
    }
    assert_eq!(
        fs::read_to_string(home.path().join(".claude/skills/vibevm/SKILL.md")).unwrap(),
        vibe_mcp::install::SKILL_TEMPLATE
    );
    assert_eq!(
        fs::read_to_string(root.path().join(".claude/skills/vibevm/SKILL.md")).unwrap(),
        "foreign"
    );
    let states = run(
        &user,
        home.path(),
        &[
            "skill",
            "status",
            "vibevm",
            "--package",
            "org.vibevm.core/vibevm",
            "--path",
            path,
            "--agent",
            "claude",
            "--scope",
            "both",
            "--json",
        ],
    );
    for (scope, expected) in [("project", "changed"), ("user", "pending")] {
        let row = states["results"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["scope"] == scope)
            .unwrap();
        assert_eq!(row["desired_enabled"], false);
        assert_eq!(row["observed"], expected);
        assert!(row.get("representation").is_none());
    }
}
