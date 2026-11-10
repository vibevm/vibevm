//! `vibe agent <launcher>` — resolution by name, the argv prefix, the
//! overrides and the refusals.
//!
//! **Nothing real is ever started.** The launcher's `command` points at a
//! shim written into this test's own directory and reached through `PATH`,
//! which records the argv it received and exits. The claim under test is
//! about an argument vector, so the evidence is an argument vector
//! (PROP-076 `#SYS-HARNESS-EXECUTABLE`).

#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use predicates::prelude::*;

mod common;

use common::UserScratch;

const TEMPLATE: &str = "# My frame\n\nPermanent instructions.\n";

/// A project that has opted in, plus a `bin/` of shims on `PATH`.
struct Machine {
    user: UserScratch,
    project: tempfile::TempDir,
    bin: tempfile::TempDir,
}

impl Machine {
    fn new(launchers: &str) -> Self {
        let user = UserScratch::new();
        let project = tempfile::tempdir().unwrap();
        let bin = tempfile::tempdir().unwrap();
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
            TEMPLATE,
        )
        .unwrap();
        fs::write(user.settings.join("agents.toml"), launchers).unwrap();
        let machine = Self { user, project, bin };
        machine.shim("my-claude");
        machine.shim("codex");
        machine
    }

    /// A shim that records its argv and exits — never a real harness.
    fn shim(&self, name: &str) {
        let path = self.bin.path().join(name);
        fs::write(
            &path,
            "#!/bin/sh\nprintf '%s\\n' \"$@\" > \"$SHIM_ARGV\"\nexit 0\n",
        )
        .unwrap();
        let mut permissions = fs::metadata(&path).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&path, permissions).unwrap();
    }

    fn argv_path(&self) -> PathBuf {
        self.bin.path().join("argv.txt")
    }

    fn agent(&self, args: &[&str]) -> assert_cmd::Command {
        let mut command = self.user.vibe();
        command
            .current_dir(self.project.path())
            .env("PATH", self.path_env())
            .env("SHIM_ARGV", self.argv_path())
            .env("CODEX_HOME", self.bin.path().join("codex-home"))
            .arg("agent");
        for argument in args {
            command.arg(argument);
        }
        command
    }

    fn path_env(&self) -> String {
        let existing = std::env::var("PATH").unwrap_or_default();
        format!("{}:{existing}", self.bin.path().display())
    }

    fn argv(&self) -> Vec<String> {
        fs::read_to_string(self.argv_path())
            .unwrap()
            .lines()
            .map(str::to_owned)
            .collect()
    }

    fn lane(&self) -> PathBuf {
        self.project
            .path()
            .join(vibe_core::layout::current_boot_dir())
            .join("AGENT-LANE.md")
    }
}

const WRAPPER: &str = "[launchers.claude]\n\
                       kind = \"claude\"\n\
                       command = [\"my-claude\", \"--no-telemetry\"]\n\
                       config_dir = \"/tmp/claude-logs\"\n\
                       \n\
                       [launchers.work-codex]\n\
                       kind = \"codex\"\n\
                       command = [\"codex\"]\n";

#[test]
fn a_user_launcher_overrides_the_builtin_of_the_same_name() {
    let machine = Machine::new(WRAPPER);
    machine
        .agent(&["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "claude (kind claude) — my-claude --no-telemetry [",
        ))
        // The built-in `codex` survives beside it, and a second launcher of
        // the same kind is ordinary configuration.
        .stdout(predicate::str::contains(
            "codex (kind codex) — codex [built-in]",
        ))
        .stdout(predicate::str::contains("work-codex (kind codex)"))
        .stdout(predicate::str::contains("config dir: /tmp/claude-logs"));
}

#[test]
fn a_launch_execs_the_argv_prefix_then_the_flags_then_the_users_own_arguments() {
    let machine = Machine::new(WRAPPER);
    machine
        .agent(&["claude", "--model", "opus"])
        .assert()
        .success();
    assert_eq!(
        machine.argv(),
        [
            // The wrapper's own standing argument, because `command` is a
            // prefix rather than a path.
            "--no-telemetry".to_owned(),
            "--append-system-prompt-file".to_owned(),
            machine.lane().display().to_string(),
            "--exclude-dynamic-system-prompt-sections".to_owned(),
            "--model".to_owned(),
            "opus".to_owned(),
        ]
    );
    // The lane was recomposed before the launch, so the slot points at a
    // file that exists.
    assert!(machine.lane().exists());
}

#[test]
fn a_launch_passes_exactly_one_file_flag_and_no_lane_bytes() {
    let machine = Machine::new(WRAPPER);
    machine.agent(&["claude"]).assert().success();
    let argv = machine.argv();
    assert_eq!(
        argv.iter()
            .filter(|flag| *flag == "--append-system-prompt-file")
            .count(),
        1
    );
    for argument in &argv {
        assert!(
            !argument.contains("My frame"),
            "lane bytes reached the command line: {argument}"
        );
        assert!(argument.len() < 4096, "an argument carried a body");
    }
}

#[test]
fn codex_gets_its_profile_file_written_at_launch_and_the_declared_doc_limit() {
    let machine = Machine::new(WRAPPER);
    machine.agent(&["work-codex"]).assert().success();
    let argv = machine.argv();
    assert_eq!(argv[0], "-p");
    assert!(argv[1].starts_with("vibe-"), "{argv:?}");
    // A profile name with a dot in it would be a nested TOML table header.
    assert!(!argv[1].contains('.'), "{argv:?}");
    assert_eq!(argv[2], "-c");
    assert_eq!(argv[3], "project_doc_max_bytes=32768");

    let home = machine.bin.path().join("codex-home");
    let profile = home.join(format!("{}.config.toml", argv[1]));
    assert!(profile.exists(), "the profile file is written at launch");
    let body = fs::read_to_string(&profile).unwrap();
    let parsed: toml::Table = body.parse().expect("the carrier is valid TOML");
    let delivered = parsed["profiles"][argv[1].as_str()]["developer_instructions"]
        .as_str()
        .expect("the lane arrives as a string value");
    assert_eq!(delivered, fs::read_to_string(machine.lane()).unwrap());
    // A basic string with escapes, never a `'''` literal block.
    assert!(!body.contains("'''"));
}

#[test]
fn an_install_never_writes_the_codex_profile() {
    let machine = Machine::new(WRAPPER);
    let home = machine.bin.path().join("codex-home");
    machine
        .user
        .vibe()
        .args(["prompt", "build", "--path"])
        .arg(machine.project.path())
        .env("CODEX_HOME", &home)
        .assert()
        .success();
    assert!(
        !home.exists(),
        "a build wrote outside the project: {}",
        home.display()
    );
}

#[test]
fn flags_never_write_the_codex_profile_either() {
    let machine = Machine::new(WRAPPER);
    machine
        .user
        .vibe()
        .args(["prompt", "build", "--path"])
        .arg(machine.project.path())
        .assert()
        .success();
    let home = machine.bin.path().join("codex-home");
    machine
        .user
        .vibe()
        .args(["prompt", "flags", "--harness", "codex", "--path"])
        .arg(machine.project.path())
        .env("CODEX_HOME", &home)
        .assert()
        .success();
    assert!(!home.exists(), "`vibe prompt flags` wrote a file");
}

#[test]
fn an_unknown_launcher_name_is_a_refusal_listing_the_launchers() {
    let machine = Machine::new(WRAPPER);
    machine
        .agent(&["nope"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("is not an agent launcher"))
        .stderr(predicate::str::contains("`claude`"))
        .stderr(predicate::str::contains("`work-codex`"));
}

#[test]
fn an_unknown_kind_is_a_refusal_naming_the_field() {
    let machine = Machine::new("[launchers.mine]\nkind = \"qwen\"\ncommand = [\"x\"]\n");
    machine
        .agent(&["mine"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("[launchers.mine] kind = \"qwen\""))
        .stderr(predicate::str::contains("`claude`"));
}

#[test]
fn a_conflicting_user_flag_is_refused_rather_than_passed_beside_ours() {
    let machine = Machine::new(WRAPPER);
    for flag in [
        "--append-system-prompt-file",
        "--system-prompt",
        "--system-prompt-file",
    ] {
        machine
            .agent(&["claude", flag, "whatever"])
            .assert()
            .failure()
            .stderr(predicate::str::contains("is already in your own arguments"));
    }
}

#[test]
fn a_project_manifest_that_names_a_launcher_is_a_refusal() {
    let machine = Machine::new(WRAPPER);
    let manifest = machine.project.path().join("vibe.toml");
    let mut text = fs::read_to_string(&manifest).unwrap();
    text.push_str("\n[agent.launchers.theirs]\nkind = \"claude\"\ncommand = [\"evil\"]\n");
    fs::write(&manifest, text).unwrap();
    machine
        .agent(&["claude"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("[agent.launchers.*]"))
        .stderr(predicate::str::contains("`theirs`"))
        .stderr(predicate::str::contains("may not name an agent launcher"));
    assert!(
        !Path::new(&machine.argv_path()).exists(),
        "a refused launch started something"
    );
}
