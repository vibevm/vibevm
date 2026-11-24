//! Recording-shim evidence that generated worker types precede Claude exec.
//! No real harness is started.

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
fn opted_out_launch_removes_the_generated_worker_type() {
    let machine = Machine::new();
    fs::remove_file(
        machine
            .project
            .path()
            .join(vibe_core::layout::current_boot_dir())
            .join("SYSTEM-TEMPLATE.md"),
    )
    .unwrap();
    let agents = machine.project.path().join(".claude/agents");
    fs::create_dir_all(&agents).unwrap();
    let worker = agents.join("vibe-worker.md");
    fs::write(
        &worker,
        vibe_workspace::system_prompt::agent_types::render_claude_agent_type(
            &vibe_workspace::system_prompt::agent_types::builtin_agent_types()[0],
            b"OLD LANE\n",
        ),
    )
    .unwrap();

    machine.agent(&["claude", "-p", "hi"]).assert().success();
    assert!(!worker.exists());
    assert_eq!(machine.argv().unwrap(), ["--no-telemetry", "-p", "hi"]);
}

#[test]
fn failed_build_soft_launch_removes_the_generated_worker_type() {
    let machine = Machine::new();
    machine.agent(&["claude", "-p", "hi"]).assert().success();
    let worker = machine.project.path().join(".claude/agents/vibe-worker.md");
    assert!(worker.exists());

    let template = machine
        .project
        .path()
        .join(vibe_core::layout::current_boot_dir())
        .join("SYSTEM-TEMPLATE.md");
    fs::remove_file(&template).unwrap();
    fs::create_dir(&template).unwrap();

    machine
        .agent(&["claude", "-p", "hi"])
        .assert()
        .success()
        .stderr(predicate::str::contains("could not be built"));
    assert!(!worker.exists());
    assert_eq!(machine.argv().unwrap(), ["--no-telemetry", "-p", "hi"]);
}

#[test]
fn claude_launch_writes_the_worker_type_before_exec() {
    let machine = Machine::new();
    machine
        .agent(&["claude", "-p", "hi"])
        .assert()
        .success()
        .stderr(predicate::str::contains(
            "✓ subagent type  created · .claude/agents/vibe-worker.md",
        ))
        .stderr(predicate::str::contains("note:").not());
    let worker = fs::read(machine.project.path().join(".claude/agents/vibe-worker.md")).unwrap();
    assert!(worker.starts_with(b"---\nname: vibe-worker\n"));
    assert!(worker.ends_with(&fs::read(machine.lane()).unwrap()));
    assert!(
        machine
            .project
            .path()
            .join(".claude/agents/.gitignore")
            .exists()
    );
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
fn foreign_worker_bytes_and_warning_survive_quiet() {
    for quiet in [false, true] {
        let machine = Machine::new();
        let worker = machine.project.path().join(".claude/agents/vibe-worker.md");
        fs::create_dir_all(worker.parent().unwrap()).unwrap();
        let foreign = b"---\nname: custom-worker\n---\nSynthetic foreign instructions.\n";
        fs::write(&worker, foreign).unwrap();
        let mut command = machine.user.vibe();
        machine.harnessed(&mut command);
        if quiet {
            command.arg("--quiet");
        }
        command.args(["agent", "claude", "-p", "hi"]).assert().success()
            .stderr(predicate::str::contains("! .claude/agents/vibe-worker.md was not generated by vibe, so this launch leaves it alone; rename or delete it to get the generated type"))
            .stderr(predicate::str::contains("warning:").not())
            .stderr(predicate::str::contains("note:").not());
        assert_eq!(fs::read(&worker).unwrap(), foreign);
        assert!(machine.argv().unwrap().contains(&"-p".to_owned()));
    }
}

#[test]
fn worker_rows_and_launch_hints_are_hidden_in_quiet_and_json() {
    for mode in ["--quiet", "--json"] {
        let machine = Machine::new();
        let mut command = machine.user.vibe();
        machine
            .harnessed(&mut command)
            .args([mode, "agent", "claude", "--resume", "synthetic-session"])
            .assert()
            .success()
            .stderr(predicate::str::contains("subagent type").not())
            .stderr(predicate::str::contains("moves cwd").not())
            .stderr(predicate::str::contains("a resumed Claude Code session").not())
            .stderr(predicate::str::contains("note:").not());
        assert!(
            machine
                .project
                .path()
                .join(".claude/agents/vibe-worker.md")
                .exists()
        );
        assert!(machine.argv().unwrap().contains(&"--resume".to_owned()));
    }
}

#[test]
fn worker_refresh_failure_warns_even_in_quiet() {
    for quiet in [false, true] {
        let machine = Machine::new();
        fs::write(machine.project.path().join(".claude"), "synthetic blocker").unwrap();
        let mut command = machine.user.vibe();
        machine.harnessed(&mut command);
        if quiet {
            command.arg("--quiet");
        }
        command
            .args(["agent", "claude", "-p", "hi"])
            .assert()
            .success()
            .stderr(predicate::str::contains(
                "! subagent types were not refreshed (",
            ))
            .stderr(predicate::str::contains(
                "the session starts without fresh vibe types",
            ))
            .stderr(predicate::str::contains("warning:").not());
        assert!(machine.argv().unwrap().contains(&"-p".to_owned()));
    }
}
