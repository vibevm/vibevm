//! An opted-in install reports exactly one prompt status before its final
//! trailer; quiet mode folds the actual status into its single line.
#![cfg(unix)]

mod common;

use common::UserScratch;
use std::path::Path;

/// An explicitly opted-in synthetic project, with init isolated as well.
fn opted_in() -> (UserScratch, tempfile::TempDir) {
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    user.vibe()
        .env("HOME", &user.settings)
        .env("VIBE_NO_DEFAULT_REGISTRY", "1")
        .args([
            "init",
            "--group",
            "org.example",
            "--name",
            "prompt-order",
            "--path",
        ])
        .arg(project.path())
        .assert()
        .success();
    user.vibe()
        .env("HOME", &user.settings)
        .env("VIBE_NO_DEFAULT_REGISTRY", "1")
        .args(["prompt", "init", "--path"])
        .arg(project.path())
        .assert()
        .success();
    (user, project)
}

/// Exercise the public CLI with no registry, inherited home or network.
fn install(user: &UserScratch, project: &Path, quiet: bool) -> String {
    let mut command = user.vibe();
    command
        .env("HOME", &user.settings)
        .env("VIBE_NO_DEFAULT_REGISTRY", "1")
        .env("NO_COLOR", "1")
        .args(["install", "--offline", "--path"])
        .arg(project);
    if quiet {
        command.arg("--quiet");
    }
    let output = command.output().unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        output.status.success(),
        "{stdout}{}",
        String::from_utf8_lossy(&output.stderr)
    );
    println!("observed install (quiet={quiet}):\n{stdout}");
    stdout
}

/// Match words, not F13's visual treatment or optional leading status glyph.
fn semantic(line: &str) -> String {
    let mut plain = String::new();
    let mut chars = line.chars();
    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' && chars.next() == Some('[') {
            for escape in chars.by_ref() {
                if ('@'..='~').contains(&escape) {
                    break;
                }
            }
        } else {
            plain.push(ch);
        }
    }
    plain
        .trim_start_matches(|ch: char| !ch.is_alphanumeric())
        .replace("system prompt:", "system prompt")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[test]
fn install_prints_the_system_prompt_line_before_its_trailer() {
    let (user, project) = opted_in();
    for status in ["rebuilt", "unchanged"] {
        let stdout = install(&user, project.path(), false);
        let lines: Vec<_> = stdout
            .lines()
            .filter(|line| !line.trim().is_empty())
            .collect();
        let prompts: Vec<_> = lines
            .iter()
            .enumerate()
            .filter(|(_, line)| semantic(line).starts_with("system prompt "))
            .collect();
        assert_eq!(prompts.len(), 1, "exactly one prompt status: {stdout}");
        let (prompt, line) = prompts[0];
        assert!(
            semantic(line).starts_with(&format!("system prompt {status}")),
            "actual artifact status: {stdout}"
        );
        let trailers: Vec<_> = lines
            .iter()
            .enumerate()
            .filter(|(_, line)| line.trim_start().starts_with('✻'))
            .collect();
        assert_eq!(trailers.len(), 1, "exactly one trailer: {stdout}");
        let last = lines.len() - 1;
        assert_eq!(trailers[0].0, last, "trailer must be last: {stdout}");
        assert!(prompt < last, "prompt precedes final trailer: {stdout}");
    }
}

/// Missing remap input refuses after regeneration, outside the direct run.
fn require_missing_remap(project: &Path) {
    let path = project.join("vibe.toml");
    let manifest = std::fs::read_to_string(&path).unwrap();
    std::fs::write(
        path,
        format!("{manifest}\n[requires.remaps]\n'org.example/missing'='=1.0.0'\n"),
    )
    .unwrap();
}

/// Public post-install refusal using only synthetic local inputs.
fn refusing_install(user: &UserScratch, project: &Path, flags: &[&str]) -> std::process::Output {
    user.vibe()
        .env("HOME", &user.settings)
        .env("VIBE_NO_DEFAULT_REGISTRY", "1")
        .args(["install", "--offline", "--path"])
        .arg(project)
        .args(flags)
        .output()
        .unwrap()
}

#[test]
fn remap_pin_refusal_discards_only_the_deferred_success_output() {
    for quiet in [false, true] {
        let (user, project) = opted_in();
        require_missing_remap(project.path());
        let flags = if quiet { vec!["--quiet"] } else { vec![] };
        let output = refusing_install(&user, project.path(), &flags);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(!output.status.success(), "{stdout}{stderr}");
        assert!(stderr.contains("org.example/missing"), "{stderr}");
        assert!(project.path().join(common::boot_rel("SYSTEM.md")).is_file());
        assert!(!stdout.contains("system prompt"), "{stdout}");
        assert!(!stdout.contains('✻'), "no false final success: {stdout}");
        assert!(!stdout.contains("vibe install:"), "{stdout}");
    }
}

#[test]
fn json_root_is_emitted_before_a_later_remap_pin_refusal() {
    let (user, project) = opted_in();
    require_missing_remap(project.path());
    let output = refusing_install(&user, project.path(), &["--json"]);
    assert!(!output.status.success());
    let root = common::hosted_slot::sole_root(&output.stdout, "install");
    assert_eq!(
        root["ok"], true,
        "direct run completed before outer refusal"
    );
    assert_eq!(root["complete"], true);
    assert!(root["delegation"].is_null());
    assert!(String::from_utf8_lossy(&output.stderr).contains("org.example/missing"));
}

#[test]
fn a_parked_handoff_survives_later_remap_pin_refusal() {
    for json in [false, true] {
        let (user, project) = opted_in();
        let registry = tempfile::tempdir().unwrap();
        common::hosted_slot::publish_slot_agent(registry.path(), "slot:pre-install", "0.1.0");
        common::write_project_with_per_package_registry(
            project.path(),
            &common::hosted_slot::registry_url(registry.path()),
        );
        require_missing_remap(project.path());
        let mut flags = vec!["org.demo/tools", "--agent-mode", "agent", "--assume-yes"];
        if json {
            flags.push("--json");
        }
        let manifest = std::fs::read_to_string(project.path().join("vibe.toml")).unwrap();
        let registry_url = common::hosted_slot::registry_url(registry.path());
        assert!(registry_url.starts_with("git+file:///"));
        assert!(manifest.contains(&format!("url = \"{registry_url}\"")));
        assert!(!manifest.contains("https://") && !manifest.contains("http://"));
        // Initial acquisition uses the canonical hosted local-Git route, not
        // offline catalog lookup. No provider or remote registry is configured.
        let output = user
            .vibe()
            .env("HOME", &user.settings)
            .env("VIBE_NO_DEFAULT_REGISTRY", "1")
            .args(["install", "--path"])
            .arg(project.path())
            .args(&flags)
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(!output.status.success(), "{stdout}{stderr}");
        assert!(stderr.contains("org.example/missing"), "{stderr}");
        if json {
            let root = common::hosted_slot::sole_document(&output.stdout);
            assert_eq!(root["ok"], true);
            assert_eq!(root["complete"], false);
            assert!(
                root["delegation"]["resume"]
                    .as_str()
                    .unwrap()
                    .contains("install")
            );
            assert!(!root["delegation"]["tasks"].as_array().unwrap().is_empty());
        } else {
            assert!(stdout.contains("parked for the hosting agent"), "{stdout}");
            assert!(stdout.contains("resume with"), "{stdout}");
            assert!(!stdout.contains('✻'), "{stdout}");
        }
        assert!(project.path().join(".vibe/lifecycle.toml").is_file());
    }
}

#[test]
fn quiet_install_is_one_line_with_the_prompt_status() {
    let (user, project) = opted_in();
    for status in ["rebuilt", "unchanged"] {
        let stdout = install(&user, project.path(), true);
        assert_eq!(stdout.lines().count(), 1, "one quiet line: {stdout}");
        let line = semantic(stdout.trim_end());
        assert_eq!(line.matches("system prompt").count(), 1, "{stdout}");
        assert!(
            line.ends_with(&format!("· system prompt {status}")),
            "quiet reports the actual artifact status: {stdout}"
        );
    }
}
