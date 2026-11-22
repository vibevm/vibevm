//! Recording-shim evidence for opt-in delivery of the parent's lane to workers.
//! No real harness is started; the assertions describe only the argument vector.

#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use predicates::prelude::*;

mod common;

use common::UserScratch;

const LAUNCHERS: &str = "[launchers.claude]\n\
                         kind = \"claude\"\n\
                         command = [\"my-claude\", \"--no-telemetry\"]\n";

struct Machine {
    user: UserScratch,
    project: tempfile::TempDir,
    bin: tempfile::TempDir,
}

impl Machine {
    fn new() -> Self {
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
            "# My frame\n\nPermanent instructions.\n",
        )
        .unwrap();
        fs::write(user.settings.join("agents.toml"), LAUNCHERS).unwrap();
        let machine = Self { user, project, bin };
        machine.shim();
        machine
    }

    fn shim(&self) {
        let path = self.bin.path().join("my-claude");
        fs::write(
            &path,
            "#!/bin/sh\n\
             if [ \"$1\" = \"--version\" ]; then printf '%s\\n' \"$SHIM_VERSION\"; exit 0; fi\n\
             for argument in \"$@\"; do\n\
             \x20 if [ \"$argument\" = \"debug\" ]; then \
             printf '%s\\n' \"$SHIM_DEBUG_CONFIG\"; exit 0; fi\n\
             done\n\
             printf '%s\\n' \"$@\" > \"$SHIM_ARGV\"\n\
             printf '%s' \"${OPENCODE_CONFIG_CONTENT-}\" > \"$SHIM_ENV\"\n\
             printf '%s' \"${QWEN_SYSTEM_IDENTITY_MD-}\" > \"$SHIM_ENV.qwen\"\n\
             exit 0\n",
        )
        .unwrap();
        let mut permissions = fs::metadata(&path).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&path, permissions).unwrap();
    }

    fn agent(&self, args: &[&str]) -> assert_cmd::Command {
        let mut command = self.user.vibe();
        self.harnessed(&mut command).arg("agent").args(args);
        command
    }

    fn harnessed<'a>(&self, command: &'a mut assert_cmd::Command) -> &'a mut assert_cmd::Command {
        let existing = std::env::var("PATH").unwrap_or_default();
        command
            .current_dir(self.project.path())
            .env("PATH", format!("{}:{existing}", self.bin.path().display()))
            .env("SHIM_ARGV", self.bin.path().join("argv.txt"))
            .env("SHIM_ENV", self.bin.path().join("carrier.txt"))
            .env("SHIM_VERSION", "codex-cli 0.167.0")
            .env("SHIM_DEBUG_CONFIG", "[]")
            .env("CODEX_HOME", self.bin.path().join("codex-home"))
    }

    fn argv(&self) -> Option<Vec<String>> {
        fs::read_to_string(self.bin.path().join("argv.txt"))
            .ok()
            .map(|argv| argv.lines().map(str::to_owned).collect())
    }

    fn lane(&self) -> PathBuf {
        self.project
            .path()
            .join(vibe_core::layout::current_boot_dir())
            .join("AGENT-LANE.md")
    }
}

#[test]
fn worker_prefix_appends_the_subagent_flag_naming_the_parent_lane() {
    let machine = Machine::new();
    machine
        .agent(&["claude", "--worker-prefix", "-p", "hi"])
        .assert()
        .success();
    assert_eq!(
        machine.argv().unwrap(),
        [
            "--no-telemetry".to_owned(),
            "--append-system-prompt-file".to_owned(),
            machine.lane().display().to_string(),
            "--exclude-dynamic-system-prompt-sections".to_owned(),
            "--append-subagent-system-prompt-file".to_owned(),
            machine.lane().display().to_string(),
            "-p".to_owned(),
            "hi".to_owned(),
        ]
    );
}

#[test]
fn without_worker_prefix_the_argv_is_unchanged() {
    let machine = Machine::new();
    machine.agent(&["claude", "-p", "hi"]).assert().success();
    assert_eq!(
        machine.argv().unwrap(),
        [
            "--no-telemetry".to_owned(),
            "--append-system-prompt-file".to_owned(),
            machine.lane().display().to_string(),
            "--exclude-dynamic-system-prompt-sections".to_owned(),
            "-p".to_owned(),
            "hi".to_owned(),
        ]
    );
}

#[test]
fn worker_prefix_without_print_mode_is_refused() {
    let machine = Machine::new();
    machine
        .agent(&["claude", "--worker-prefix"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("needs print mode"));
    assert!(machine.argv().is_none());
}

#[test]
fn worker_prefix_for_another_kind_is_refused() {
    let machine = Machine::new();
    machine
        .agent(&["codex", "--worker-prefix", "exec"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "supported only for launchers of kind `claude`",
        ));
}

#[test]
fn worker_prefix_refuses_the_users_own_subagent_flag() {
    let machine = Machine::new();
    machine
        .agent(&[
            "claude",
            "--worker-prefix",
            "-p",
            "--append-subagent-system-prompt-file",
            "x.md",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("is already in your own arguments"));
    assert!(machine.argv().is_none());
}

#[test]
fn worker_prefix_is_dropped_when_the_project_has_not_opted_in() {
    let machine = Machine::new();
    fs::remove_file(
        machine
            .project
            .path()
            .join(vibe_core::layout::current_boot_dir())
            .join("SYSTEM-TEMPLATE.md"),
    )
    .unwrap();
    machine
        .agent(&["claude", "--worker-prefix", "-p", "hi"])
        .assert()
        .success();
    assert_eq!(machine.argv().unwrap(), ["--no-telemetry", "-p", "hi"]);
}
