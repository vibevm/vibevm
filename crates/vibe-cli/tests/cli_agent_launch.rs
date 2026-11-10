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
        machine.shim("opencode");
        machine
    }

    /// A shim that records its argv and exits — never a real harness.
    ///
    /// It answers `--version` from `$SHIM_VERSION` **without** recording,
    /// because the Codex carrier is gated on the installed version and the
    /// probe must be answerable by a shim: a test that let the gate fall
    /// through to the real `codex` on `PATH` would be measuring this
    /// machine rather than the code.
    fn shim(&self, name: &str) {
        let path = self.bin.path().join(name);
        fs::write(
            &path,
            "#!/bin/sh\n\
             if [ \"$1\" = \"--version\" ]; then printf '%s\\n' \"$SHIM_VERSION\"; exit 0; fi\n\
             printf '%s\\n' \"$@\" > \"$SHIM_ARGV\"\nexit 0\n",
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
        self.agent_reporting(args, VERIFIED)
    }

    /// A launch against a harness that reports `version` for `--version`.
    fn agent_reporting(&self, args: &[&str], version: &str) -> assert_cmd::Command {
        let mut command = self.user.vibe();
        self.harnessed(&mut command, version).arg("agent");
        for argument in args {
            command.arg(argument);
        }
        command
    }

    /// The shim environment: `PATH`, the argv sink, `$CODEX_HOME` and the
    /// version the shims answer with.
    fn harnessed<'a>(
        &self,
        command: &'a mut assert_cmd::Command,
        version: &str,
    ) -> &'a mut assert_cmd::Command {
        command
            .current_dir(self.project.path())
            .env("PATH", self.path_env())
            .env("SHIM_ARGV", self.argv_path())
            .env("SHIM_VERSION", version)
            .env("CODEX_HOME", self.bin.path().join("codex-home"))
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

    /// Append a `[system_prompt.*]` declaration to the project manifest.
    fn declare(&self, table: &str) {
        let manifest = self.project.path().join("vibe.toml");
        let mut text = fs::read_to_string(&manifest).unwrap();
        text.push('\n');
        text.push_str(table);
        fs::write(&manifest, text).unwrap();
    }

    fn lane(&self) -> PathBuf {
        self.project
            .path()
            .join(vibe_core::layout::current_boot_dir())
            .join("AGENT-LANE.md")
    }
}

/// What the shims answer `codex --version` with by default: the one
/// version whose profile carrier is verified.
const VERIFIED: &str = "codex-cli 0.167.0";

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
    // **Top-level, not `[profiles.<name>]`.** `-p` layers this file on top
    // of the base user config, so a table header would bury the key in a
    // nested table codex-cli loads and never reads — measured on 0.167.0.
    let delivered = parsed["developer_instructions"]
        .as_str()
        .expect("the lane arrives as a top-level string value");
    assert_eq!(delivered, fs::read_to_string(machine.lane()).unwrap());
    assert!(
        !body.contains("[profiles."),
        "the carrier was buried in a table header: {body}"
    );
    // A basic string with escapes, never a `'''` literal block.
    assert!(!body.contains("'''"));
}

#[test]
fn an_unverified_codex_version_launches_without_the_carrier_and_warns() {
    // **The soft launch.** A carrier vibe will not risk is not a reason to
    // leave a developer without a session: the harness starts bare, which
    // is the classic boot the router already handles, and the warning names
    // the reason and the fix (`#SYS-SOFT-LAUNCH`).
    let machine = Machine::new(WRAPPER);
    machine
        .agent_reporting(&["work-codex"], "codex-cli 0.168.0")
        .assert()
        .success()
        .stderr(predicate::str::contains("warning: vibe cannot deliver"))
        .stderr(predicate::str::contains("0.168.0"))
        .stderr(predicate::str::contains("0.167.0"))
        .stderr(predicate::str::contains("harness_version"))
        .stderr(predicate::str::contains("codex debug prompt-input"));
    // The shim records one empty line when it received no argument at all.
    assert_eq!(machine.argv(), [""], "a soft launch passed a flag");
    assert!(
        !machine.bin.path().join("codex-home").exists(),
        "a soft launch wrote a profile file"
    );
}

#[test]
fn the_soft_launch_warning_survives_quiet() {
    // `--quiet` asks for less narration about what vibe did; this sentence
    // is about what the *agent* receives, so it is not vibe's to suppress.
    let machine = Machine::new(WRAPPER);
    let mut command = machine.user.vibe();
    machine
        .harnessed(&mut command, "codex-cli 0.168.0")
        .args(["--quiet", "agent", "work-codex"])
        .assert()
        .success()
        .stderr(predicate::str::contains("warning: vibe cannot deliver"))
        .stderr(predicate::str::contains("`--quiet` does not suppress this"));
    assert_eq!(machine.argv(), [""]);
}

#[test]
fn a_failed_build_launches_softly_and_never_delivers_the_older_lane() {
    let machine = Machine::new(WRAPPER);
    // One good launch, so a lane exists on disk to be wrongly delivered.
    machine.agent(&["claude"]).assert().success();
    assert!(machine.lane().exists());
    let delivered = fs::read_to_string(machine.lane()).unwrap();

    // Break the build without touching the generated lane: the authored
    // template becomes a directory, which the read path refuses as not a
    // regular file.
    let template = machine
        .project
        .path()
        .join(vibe_core::layout::current_boot_dir())
        .join("SYSTEM-TEMPLATE.md");
    fs::remove_file(&template).unwrap();
    fs::create_dir(&template).unwrap();

    machine
        .agent(&["claude"])
        .assert()
        .success()
        .stderr(predicate::str::contains("warning: vibe cannot deliver"))
        .stderr(predicate::str::contains("could not be built"))
        .stderr(predicate::str::contains("vibe prompt build"));
    assert_eq!(
        machine.argv(),
        ["--no-telemetry"],
        "a failed build still delivered a lane"
    );
    assert_eq!(
        fs::read_to_string(machine.lane()).unwrap(),
        delivered,
        "the stale lane is still on disk — the claim is that it was not passed"
    );
}

#[test]
fn a_directory_that_is_not_a_project_launches_softly() {
    let machine = Machine::new(WRAPPER);
    let elsewhere = tempfile::tempdir().unwrap();
    let mut command = machine.user.vibe();
    machine
        .harnessed(&mut command, VERIFIED)
        .current_dir(elsewhere.path())
        .args(["agent", "claude", "--model", "opus"])
        .assert()
        .success()
        .stderr(predicate::str::contains("warning: vibe cannot deliver"))
        .stderr(predicate::str::contains("not a vibe project"));
    assert_eq!(machine.argv(), ["--no-telemetry", "--model", "opus"]);
}

#[test]
fn printing_flags_is_gated_on_the_same_version_as_launching() {
    let machine = Machine::new(WRAPPER);
    machine
        .user
        .vibe()
        .args(["prompt", "build", "--path"])
        .arg(machine.project.path())
        .assert()
        .success();
    let mut command = machine.user.vibe();
    machine
        .harnessed(&mut command, "codex-cli 0.168.0")
        .args(["prompt", "flags", "--harness", "codex", "--path"])
        .arg(machine.project.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("0.168.0"));
}

#[test]
fn a_user_who_verified_a_newer_codex_states_the_version_by_number() {
    // The override is a measurement, not a switch: it names the version the
    // user checked, so the claim is attributable.
    let machine = Machine::new(
        "[launchers.new-codex]\n\
         kind = \"codex\"\n\
         command = [\"codex\"]\n\
         harness_version = \"0.168.0\"\n",
    );
    machine
        .agent_reporting(&["new-codex"], "codex-cli 0.168.0")
        .assert()
        .success()
        .stderr(predicate::str::contains("the claim is yours"));
    assert_eq!(machine.argv()[0], "-p");
}

#[test]
fn a_codex_that_will_not_say_its_version_launches_with_a_note_and_not_a_refusal() {
    // "Cannot tell" is not "wrong": refusing here would break every machine
    // whose harness is reached through something that does not answer
    // `--version`, where nothing is actually broken.
    let machine = Machine::new(WRAPPER);
    machine
        .agent_reporting(&["work-codex"], "")
        .assert()
        .success()
        .stderr(predicate::str::contains("did not say which version"));
    assert_eq!(machine.argv()[0], "-p");
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
    let mut command = machine.user.vibe();
    machine
        .harnessed(&mut command, VERIFIED)
        .args(["prompt", "flags", "--harness", "codex", "--path"])
        .arg(machine.project.path())
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

#[test]
fn a_project_that_has_not_opted_in_starts_the_harness_unchanged() {
    // The launcher must stay usable in every project. With no template there
    // is no lane, so the harness gets no slot flag and boots the classic way,
    // and a user's own slot flag is then no conflict at all.
    let machine = Machine::new(WRAPPER);
    fs::remove_file(
        machine
            .project
            .path()
            .join(vibe_core::layout::current_boot_dir())
            .join("SYSTEM-TEMPLATE.md"),
    )
    .unwrap();
    machine
        .agent(&["claude", "--append-system-prompt-file", "mine.md"])
        .assert()
        .success()
        .stderr(predicate::str::contains("has not opted in"))
        .stderr(predicate::str::contains("unchanged"));
    assert_eq!(
        machine.argv(),
        ["--no-telemetry", "--append-system-prompt-file", "mine.md"]
    );
    assert!(
        !machine.lane().exists(),
        "an opted-out launch composed a lane"
    );
    // Printing flags for such a project is still a refusal: there is nothing
    // to print.
    let mut flags = machine.user.vibe();
    machine
        .harnessed(&mut flags, VERIFIED)
        .args(["prompt", "flags", "--harness", "claude"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("has not opted in"));
}

/// C2 — `vibe agent` injects only for a harness in `launch` delivery.
///
/// The positive half of the constraint: a `read`-delivery harness is
/// started unchanged, with no slot flag, and the user's own slot flag then
/// conflicts with nothing — refusing it would refuse a launch over a
/// delivery vibe was never going to perform (`HARNESS-DELIVERY.md` §4).
#[test]
fn c2_read_delivery_injects_nothing_and_conflicts_with_nothing() {
    let machine = Machine::new(WRAPPER);
    machine.declare("[system_prompt.harness.claude]\ndelivery = \"read\"\n");
    machine
        .agent(&["claude", "--append-system-prompt-file", "mine.md"])
        .assert()
        .success()
        .stderr(predicate::str::contains("delivery = \"read\""))
        .stderr(predicate::str::contains("no slot flag is passed"));
    assert_eq!(
        machine.argv(),
        ["--no-telemetry", "--append-system-prompt-file", "mine.md"]
    );
}

/// C2's refusal half, on the same project with the same user argument: in
/// `launch` delivery vibe does pass the flag, so two of them is refused.
#[test]
fn c2_launch_delivery_still_refuses_the_users_own_slot_flag() {
    let machine = Machine::new(WRAPPER);
    machine.declare("[system_prompt.harness.claude]\ndelivery = \"launch\"\n");
    machine
        .agent(&["claude", "--append-system-prompt-file", "mine.md"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("is already in your own arguments"));
}

/// C2 for Codex: a `read`-delivery launch writes no profile file and is not
/// gated on the harness version, because no carrier is delivered.
#[test]
fn c2_read_delivery_writes_no_codex_carrier_and_is_not_version_gated() {
    let machine = Machine::new(WRAPPER);
    machine.declare("[system_prompt.harness.codex]\ndelivery = \"read\"\n");
    machine
        .agent_reporting(&["work-codex"], "codex-cli 0.168.0")
        .assert()
        .success()
        .stderr(predicate::str::contains("no slot flag is passed"));
    // The shim records one empty line when it received no argument at all.
    assert_eq!(machine.argv(), [""], "a read-delivery launch passed a flag");
    assert!(
        !machine.bin.path().join("codex-home").exists(),
        "a read-delivery launch wrote a carrier"
    );
}

/// C2 through the printing path, which `vibe agent` is a thin layer over:
/// one code path decides what a kind takes, so `read` prints no flag there
/// too — and does so without a generated lane on disk.
#[test]
fn c2_printing_flags_for_a_read_harness_prints_none_and_needs_no_lane() {
    let machine = Machine::new(WRAPPER);
    machine.declare("[system_prompt.harness.claude]\ndelivery = \"read\"\n");
    let mut command = machine.user.vibe();
    machine
        .harnessed(&mut command, VERIFIED)
        .args(["prompt", "flags", "--harness", "claude"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty().or(predicate::str::contains("\n")))
        .stderr(predicate::str::contains("no slot flag is passed"));
    assert!(
        !machine.lane().exists(),
        "printing read-delivery flags composed a lane"
    );
}

#[test]
fn a_resumed_claude_session_is_told_it_keeps_its_old_lane() {
    let machine = Machine::new(WRAPPER);
    machine
        .agent(&["claude", "--resume", "some-id"])
        .assert()
        .success()
        .stderr(predicate::str::contains(
            "keeps the boot lane it started with",
        ));
    machine
        .agent(&["claude"])
        .assert()
        .success()
        .stderr(predicate::str::contains("keeps the boot lane").not());
}

/// OpenCode — a launcher and a dialect with **no deliverable slot**.
///
/// The kind exists so `vibe agent opencode` exists and
/// `[system_prompt.harness.opencode]` is declarable; the carrier is named
/// in the note and never passed, because OpenCode injects a configuration
/// `instructions` array into every session its process serves, `task`
/// subagents included (`#SYS-OPENCODE`, `#SYS-WORKER-EXCLUSION`).
#[test]
fn opencode_is_a_builtin_launcher() {
    let machine = Machine::new(WRAPPER);
    machine
        .agent(&["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "opencode (kind opencode) — opencode [built-in]",
        ));
}

#[test]
fn opencode_defaults_to_read_and_starts_unchanged_naming_its_carrier() {
    let machine = Machine::new(WRAPPER);
    machine
        .agent(&["opencode", "run", "hello"])
        .assert()
        .success()
        .stderr(predicate::str::contains("no slot flag is passed"))
        .stderr(predicate::str::contains("OPENCODE_CONFIG_CONTENT"))
        .stderr(predicate::str::contains("subagents included"))
        // `read` is the default, not a warning: the router is doing the
        // work by design, so this is a note and not a soft launch.
        .stderr(predicate::str::contains("warning: vibe cannot deliver").not());
    assert_eq!(machine.argv(), ["run", "hello"]);
}

#[test]
fn a_project_declaring_launch_for_opencode_launches_softly_with_the_reason() {
    let machine = Machine::new(WRAPPER);
    machine.declare("[system_prompt.harness.opencode]\ndelivery = \"launch\"\n");
    machine
        .agent(&["opencode"])
        .assert()
        .success()
        .stderr(predicate::str::contains("warning: vibe cannot deliver"))
        .stderr(predicate::str::contains("SYS-WORKER-EXCLUSION"))
        .stderr(predicate::str::contains("2.2.7"))
        .stderr(predicate::str::contains("delivery = \"read\""));
    // The shim records one empty line when it received no argument at all.
    assert_eq!(machine.argv(), [""], "a soft launch passed a flag");
}

#[test]
fn printing_flags_for_opencode_says_why_the_carrier_is_not_a_flag() {
    let machine = Machine::new(WRAPPER);
    machine
        .user
        .vibe()
        .args(["prompt", "build", "--path"])
        .arg(machine.project.path())
        .assert()
        .success();
    // The default, `read`: no flags, and the note names the environment
    // assignment the carrier would have been.
    let mut command = machine.user.vibe();
    machine
        .harnessed(&mut command, VERIFIED)
        .args(["prompt", "flags", "--harness", "opencode", "--path"])
        .arg(machine.project.path())
        .assert()
        .success()
        .stderr(predicate::str::contains("OPENCODE_CONFIG_CONTENT"))
        .stderr(predicate::str::contains(
            "takes no append-instructions argument",
        ));
    // A declared `launch` is a refusal here, where `vibe agent` launches
    // softly: this command prints and starts nothing, so there is no
    // session to save.
    machine.declare("[system_prompt.harness.opencode]\ndelivery = \"launch\"\n");
    let mut command = machine.user.vibe();
    machine
        .harnessed(&mut command, VERIFIED)
        .args(["prompt", "flags", "--harness", "opencode", "--path"])
        .arg(machine.project.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("SYS-WORKER-EXCLUSION"));
}
