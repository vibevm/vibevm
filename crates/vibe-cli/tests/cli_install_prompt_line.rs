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
