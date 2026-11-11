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
        machine.shim("qwen");
        machine
    }

    /// A shim that records its argv and its carrier environment, and exits
    /// — never a real harness.
    ///
    /// It answers two questions **without** recording, because both are
    /// probes the carrier gates make and both must be answerable by a shim:
    /// a test that let a gate fall through to the real `codex` or
    /// `opencode` on `PATH` would be measuring this machine rather than the
    /// code.
    ///
    /// - `--version`, from `$SHIM_VERSION`. The Codex carrier is gated on
    ///   the installed version, and OpenCode's prompt-base set is recorded
    ///   per harness version.
    /// - any argv containing `debug`, from `$SHIM_DEBUG_CONFIG`. That is
    ///   OpenCode's own answer about its own configuration, which is where
    ///   a launch with no `-m` of its own learns the model.
    fn shim(&self, name: &str) {
        let path = self.bin.path().join(name);
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
             exit 0\n",
        )
        .unwrap();
        let mut permissions = fs::metadata(&path).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&path, permissions).unwrap();
    }

    fn argv_path(&self) -> PathBuf {
        self.bin.path().join("argv.txt")
    }

    fn env_path(&self) -> PathBuf {
        self.bin.path().join("carrier.txt")
    }

    /// What the launched harness found in `$OPENCODE_CONFIG_CONTENT`.
    fn carrier(&self) -> String {
        fs::read_to_string(self.env_path()).unwrap_or_default()
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
            .env("SHIM_ENV", self.env_path())
            .env("SHIM_VERSION", version)
            .env("SHIM_DEBUG_CONFIG", CONFIGURED)
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

    /// Write a prompt-base set shaped like a real one, into a directory the
    /// launch is pointed at with `VIBE_PROMPT_BASES_DIR`.
    ///
    /// **The fixture's text is nobody else's.** A test that asserted over
    /// vendored third-party prose would fail the day upstream edits a
    /// sentence, which is that package's business; what is pinned here is
    /// the mechanism — the lookup, the digest gate, the `verified` gate, and
    /// the shape of the document that reaches the harness.
    fn bases(&self, set: &Set) -> PathBuf {
        let root = self.bin.path().join("bases");
        let dir = root.join("prompt-bases").join(SHIM_OPENCODE_VERSION);
        fs::create_dir_all(&dir).unwrap();
        let base = "FIXTURE base head\n${TOKEN}\nFIXTURE base tail\n";
        fs::write(dir.join("system.txt"), base).unwrap();
        let digest = if set.digest_matches {
            format!(
                "{:x}",
                <sha2::Sha256 as sha2::Digest>::digest(base.as_bytes())
            )
        } else {
            "0".repeat(64)
        };
        fs::write(
            dir.join("PROVENANCE.toml"),
            format!(
                r#"schema = 1
harness = "opencode"
harness_version = "{SHIM_OPENCODE_VERSION}"

[[base]]
id = "generic"
file = "system.txt"
upstream_path = "packages/core/src/session/runner/prompt/system.txt"
bytes = {}
sha256 = "{digest}"
has_tool_guidance_token = true
default = true

[render]
token = "${{TOKEN}}"
replace = "first"
join = "\n"
trim = "end"

[[render.bullet]]
gate = "edit"
text = "- FIXTURE edit rule"

[carrier]
kind = "config-agent-system"
env = "OPENCODE_CONFIG_CONTENT"
flags = ["--standalone"]
agents_key = "agents"
system_key = "system"
mode_key = "mode"
default_agent_key = "default_agent"
agent_name = "vibe"
agent_mode = "primary"
gate_tools = ["edit"]
lane_token = "{{file:%LANE%}}"
lane_transform = "trim"
separator_between_base_and_lane = "\n\n"

[verification]
verified = {}
proof_id = "fixture"
proof_date = "2026-11-11"
"#,
                base.len(),
                set.verified
            ),
        )
        .unwrap();
        root
    }

    /// A launch whose prompt-base lookup is pointed at a fixture set, and
    /// whose harness answers as the named OpenCode.
    fn opencoded(&self, args: &[&str], set: &Set) -> assert_cmd::Command {
        let root = self.bases(set);
        let mut command = self.user.vibe();
        self.harnessed(&mut command, SHIM_OPENCODE_REPORTS);
        if set.present {
            command.env("VIBE_PROMPT_BASES_DIR", &root);
        } else {
            command.env_remove("VIBE_PROMPT_BASES_DIR");
        }
        if !set.model_known {
            command.env("SHIM_DEBUG_CONFIG", NO_MODEL);
        }
        command.arg("agent");
        for argument in args {
            command.arg(argument);
        }
        command
    }
}

/// Which way a fixture set is broken, if it is.
struct Set {
    present: bool,
    verified: bool,
    digest_matches: bool,
    model_known: bool,
}

impl Set {
    /// The set a launch may deliver.
    fn good() -> Self {
        Self {
            present: true,
            verified: true,
            digest_matches: true,
            model_known: true,
        }
    }
}

/// What the shims answer `opencode --version` with, and therefore which
/// version directory the lookup asks for.
const SHIM_OPENCODE_VERSION: &str = "9.9.9";
const SHIM_OPENCODE_REPORTS: &str = "opencode v9.9.9";

/// What the shim answers `opencode debug config` with: OpenCode's own
/// configuration documents, in its own resolution order. The last one that
/// states a model is the effective one, which is how a launch with no `-m`
/// of its own learns which base to supply.
const CONFIGURED: &str = r#"[
  { "type": "document", "path": "/h/.config/opencode/opencode.json",
    "info": { "model": "homelocal/qwen3.8-27b" } }
]"#;

/// The same answer from a machine that has configured no model at all.
const NO_MODEL: &str = r#"[
  { "type": "document", "path": "/h/.config/opencode/opencode.json",
    "info": { "theme": "dark" } }
]"#;

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
    // `gemini` rather than `qwen`: the latter is a kind of its own since
    // `#SYS-QWEN`, and a test whose fixture becomes valid stops testing.
    let machine = Machine::new("[launchers.mine]\nkind = \"gemini\"\ncommand = [\"x\"]\n");
    machine
        .agent(&["mine"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "[launchers.mine] kind = \"gemini\"",
        ))
        .stderr(predicate::str::contains("`claude`"))
        .stderr(predicate::str::contains("`qwen`"));
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

/// OpenCode — a launcher whose carrier is a per-launch configuration layer
/// declaring vibe's own primary agent (`#SYS-OPENCODE`,
/// `#SYS-SUPPLIED-BASE`).
///
/// The slot's cost is that it **replaces** OpenCode's own prompt, so the
/// launch supplies that prompt from versioned data with provenance in
/// OpenCode's own package. Everything these tests pin follows from that: the
/// lookup, the gates that refuse a set that has not been proved, and the one
/// argument of the user's that would silently bypass the whole thing.
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
fn opencode_launches_with_the_carrier_in_the_environment_and_one_flag() {
    // **The whole delivery, end to end through the shim.** `--standalone`
    // is the document's own flag, the document is one environment
    // assignment, and what it carries is the harness's own base plus the
    // lane's PATH — never the lane's bytes, in the environment as on a
    // command line.
    let machine = Machine::new(WRAPPER);
    machine
        .opencoded(&["opencode", "run", "hello"], &Set::good())
        .assert()
        .success()
        .stderr(predicate::str::contains("OPENCODE_CONFIG_CONTENT"))
        .stderr(predicate::str::contains("qwen3.8-27b"))
        .stderr(predicate::str::contains("warning: vibe cannot deliver").not());
    assert_eq!(machine.argv(), ["--standalone", "run", "hello"]);

    let carrier = machine.carrier();
    let document: serde_json::Value = serde_json::from_str(&carrier).expect("valid JSON");
    // The agent is declared AND selected, because the top-level command
    // takes no `--agent` argument: measured on 2.2.7, `opencode
    // --standalone --agent <name>` prints the usage and starts nothing.
    assert_eq!(document["default_agent"], "vibe");
    let system = document["agents"]["vibe"]["system"]
        .as_str()
        .expect("the agent carries the composed prompt");
    assert_eq!(document["agents"]["vibe"]["mode"], "primary");
    // The supplied base comes first, with the render rule applied and its
    // own trailing newline trimmed as the program's own is.
    assert!(system.starts_with("FIXTURE base head\n"), "{system}");
    assert!(system.contains("- FIXTURE edit rule"), "{system}");
    assert!(!system.contains("${TOKEN}"), "{system}");
    // Then one blank line, then the lane's path in the file token.
    assert!(
        system.ends_with(&format!(
            "FIXTURE base tail\n\n{{file:{}}}",
            machine.lane().display()
        )),
        "{system}"
    );
    // No lane byte anywhere in the assignment.
    assert!(!carrier.contains("My frame"), "{carrier}");
    assert!(machine.lane().exists());
}

#[test]
fn a_launch_whose_own_argument_names_the_model_does_not_ask_the_harness() {
    // The most specific thing anyone knows about a launch is what its user
    // wrote, so nothing else is consulted — and the note says which model
    // the base was chosen for, because that is the fact a reader checks.
    let machine = Machine::new(WRAPPER);
    machine
        .opencoded(
            &["opencode", "run", "-m", "home/gpt-6.1-sol", "hello"],
            &Set::good(),
        )
        .assert()
        .success()
        .stderr(predicate::str::contains("gpt-6.1-sol"))
        .stderr(predicate::str::contains("qwen3.8-27b").not());
}

#[test]
fn a_machine_with_no_set_for_its_opencode_launches_softly_and_names_both_places() {
    // There is deliberately no fallback to another version's base, so the
    // refusal names the two places the lookup knows and says there is no
    // search.
    let machine = Machine::new(WRAPPER);
    machine
        .opencoded(
            &["opencode"],
            &Set {
                present: false,
                ..Set::good()
            },
        )
        .assert()
        .success()
        .stderr(predicate::str::contains("warning: vibe cannot deliver"))
        .stderr(predicate::str::contains(SHIM_OPENCODE_VERSION))
        .stderr(predicate::str::contains("no fallback to another version"))
        .stderr(predicate::str::contains("org.vibevm.opencode/opencode"))
        .stderr(predicate::str::contains("VIBE_PROMPT_BASES_DIR"));
    assert_eq!(machine.argv(), [""], "a soft launch passed a flag");
    assert!(
        machine.carrier().is_empty(),
        "a soft launch set the carrier"
    );
}

#[test]
fn an_unverified_set_launches_softly_and_points_at_the_sync_tool() {
    // **The gate that matters most.** Digests prove the data is what it
    // claims; only the whole-request proof proves the claim matches the
    // installed program — and the only thing that may write the flag is the
    // tool that runs that proof.
    let machine = Machine::new(WRAPPER);
    machine
        .opencoded(
            &["opencode"],
            &Set {
                verified: false,
                ..Set::good()
            },
        )
        .assert()
        .success()
        .stderr(predicate::str::contains("warning: vibe cannot deliver"))
        .stderr(predicate::str::contains("is not verified"))
        .stderr(predicate::str::contains("opencode-bases"))
        .stderr(predicate::str::contains("never hand-edit"));
    assert_eq!(machine.argv(), [""]);
    assert!(machine.carrier().is_empty());
}

#[test]
fn a_set_that_is_not_what_its_provenance_claims_launches_softly() {
    let machine = Machine::new(WRAPPER);
    machine
        .opencoded(
            &["opencode"],
            &Set {
                digest_matches: false,
                ..Set::good()
            },
        )
        .assert()
        .success()
        .stderr(predicate::str::contains("warning: vibe cannot deliver"))
        .stderr(predicate::str::contains("recorded digest"))
        .stderr(predicate::str::contains("not what its provenance claims"));
    assert_eq!(machine.argv(), [""]);
    assert!(machine.carrier().is_empty());
}

#[test]
fn a_launch_whose_model_nobody_states_launches_softly_rather_than_guessing() {
    // Taking the generic base here would hand a model family with a
    // template of its own the wrong instructions, silently — which is the
    // failure the whole mechanism exists to prevent.
    let machine = Machine::new(WRAPPER);
    machine
        .opencoded(
            &["opencode"],
            &Set {
                model_known: false,
                ..Set::good()
            },
        )
        .assert()
        .success()
        .stderr(predicate::str::contains("warning: vibe cannot deliver"))
        .stderr(predicate::str::contains("model could not be determined"))
        .stderr(predicate::str::contains("silently"));
    assert_eq!(machine.argv(), [""]);
    assert!(machine.carrier().is_empty());
}

#[test]
fn a_users_own_agent_argument_is_refused_rather_than_silently_bypassing_the_lane() {
    // Not a second prompt but a bypass of the first: `--agent other`
    // selects an agent whose `system` is not the one carrying the lane, and
    // nothing reports it.
    let machine = Machine::new(WRAPPER);
    machine
        .opencoded(&["opencode", "run", "--agent", "plan"], &Set::good())
        .assert()
        .failure()
        .stderr(predicate::str::contains("is already in your own arguments"));
}

#[test]
fn a_read_delivery_opencode_starts_unchanged_and_names_the_carrier_anyway() {
    // `read` is the project's own choice to have the router do the work;
    // the note still says where the lane WOULD have gone, which is what a
    // reader choosing a tier needs.
    let machine = Machine::new(WRAPPER);
    machine.declare("[system_prompt.harness.opencode]\ndelivery = \"read\"\n");
    machine
        .opencoded(&["opencode", "run", "hello"], &Set::good())
        .assert()
        .success()
        .stderr(predicate::str::contains("no slot flag is passed"))
        .stderr(predicate::str::contains("OPENCODE_CONFIG_CONTENT"))
        .stderr(predicate::str::contains("warning: vibe cannot deliver").not());
    assert_eq!(machine.argv(), ["run", "hello"]);
    assert!(machine.carrier().is_empty());
}

#[test]
fn printing_flags_for_opencode_prints_the_assignment_first_and_the_flags_last() {
    // **The one-line contract holds where it can and is stated where it
    // cannot.** `NAME=value` only works as a shell *prefix*, with the
    // program between it and the flags, so no single line can be both; the
    // assignments come first, one per line, and the flags last.
    let machine = Machine::new(WRAPPER);
    let root = machine.bases(&Set::good());
    machine
        .user
        .vibe()
        .args(["prompt", "build", "--path"])
        .arg(machine.project.path())
        .assert()
        .success();
    let mut command = machine.user.vibe();
    machine.harnessed(&mut command, SHIM_OPENCODE_REPORTS);
    let printed = command
        .env("VIBE_PROMPT_BASES_DIR", &root)
        .args(["prompt", "flags", "--harness", "opencode", "--path"])
        .arg(machine.project.path())
        .assert()
        .success();
    let stdout = String::from_utf8(printed.get_output().stdout.clone()).unwrap();
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(*lines.last().unwrap(), "--standalone", "{stdout}");
    assert_eq!(lines.len(), 2, "one assignment, then the flags: {stdout}");
    assert!(lines[0].starts_with("OPENCODE_CONFIG_CONTENT="), "{stdout}");
    assert!(lines[0].contains("default_agent"), "{stdout}");
    // Printing writes nothing: the carrier is an environment value, so
    // there is no file for this command to have left behind.
    assert!(!machine.env_path().exists());
}

#[test]
fn printing_flags_for_an_unverified_opencode_refuses_where_a_launch_would_soften() {
    // The two surfaces answer the same facts differently on purpose: this
    // one prints and starts nothing, so there is no session to save, and
    // flags for a carrier that may not arrive are the one answer a shell
    // alias turns silently into a session with no lane in it.
    let machine = Machine::new(WRAPPER);
    let root = machine.bases(&Set {
        verified: false,
        ..Set::good()
    });
    machine
        .user
        .vibe()
        .args(["prompt", "build", "--path"])
        .arg(machine.project.path())
        .assert()
        .success();
    let mut command = machine.user.vibe();
    machine.harnessed(&mut command, SHIM_OPENCODE_REPORTS);
    command
        .env("VIBE_PROMPT_BASES_DIR", &root)
        .args(["prompt", "flags", "--harness", "opencode", "--path"])
        .arg(machine.project.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("is not verified"));
}

/// Qwen Code — a launcher and a dialect whose one additive slot takes the
/// instructions as **text**.
///
/// The kind exists so `vibe agent qwen` exists and
/// `[system_prompt.harness.qwen]` is declarable; the slot is named in the
/// note and never passed, because `--append-system-prompt` would put the
/// lane's own bytes on a command line, where one argument is capped at
/// 128 KiB (`#SYS-QWEN`).
#[test]
fn qwen_is_a_builtin_launcher() {
    let machine = Machine::new(WRAPPER);
    machine
        .agent(&["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "qwen (kind qwen) — qwen [built-in]",
        ));
}

#[test]
fn qwen_defaults_to_read_and_starts_unchanged_naming_its_slot() {
    let machine = Machine::new(WRAPPER);
    machine
        .agent(&["qwen", "-p", "hello"])
        .assert()
        .success()
        .stderr(predicate::str::contains("no slot flag is passed"))
        .stderr(predicate::str::contains("--append-system-prompt"))
        .stderr(predicate::str::contains("QWEN_SYSTEM_MD"))
        .stderr(predicate::str::contains("128 KiB"))
        // `read` is the default, not a warning: the router is doing the
        // work by design, so this is a note and not a soft launch.
        .stderr(predicate::str::contains("warning: vibe cannot deliver").not());
    assert_eq!(machine.argv(), ["-p", "hello"]);
}

#[test]
fn a_project_declaring_launch_for_qwen_launches_softly_with_the_reason() {
    let machine = Machine::new(WRAPPER);
    machine.declare("[system_prompt.harness.qwen]\ndelivery = \"launch\"\n");
    machine
        .agent(&["qwen"])
        .assert()
        .success()
        .stderr(predicate::str::contains("warning: vibe cannot deliver"))
        .stderr(predicate::str::contains("E2BIG"))
        .stderr(predicate::str::contains("0.27.2"))
        .stderr(predicate::str::contains("delivery = \"read\""))
        // Worker exclusion is satisfied here, unlike OpenCode's, and the
        // sentence says so rather than letting a reader carry that kind's
        // conclusion over.
        .stderr(predicate::str::contains("worker exclusion is satisfied"));
    // The shim records one empty line when it received no argument at all.
    assert_eq!(machine.argv(), [""], "a soft launch passed a flag");
}

#[test]
fn printing_flags_for_qwen_says_why_the_slot_is_not_used() {
    let machine = Machine::new(WRAPPER);
    machine
        .user
        .vibe()
        .args(["prompt", "build", "--path"])
        .arg(machine.project.path())
        .assert()
        .success();
    // The default, `read`: no flags, and the note names the argument the
    // slot would have taken.
    let mut command = machine.user.vibe();
    machine
        .harnessed(&mut command, VERIFIED)
        .args(["prompt", "flags", "--harness", "qwen", "--path"])
        .arg(machine.project.path())
        .assert()
        .success()
        .stderr(predicate::str::contains("--append-system-prompt"))
        .stderr(predicate::str::contains("on the command line"));
    // A declared `launch` is a refusal here, where `vibe agent` launches
    // softly: this command prints and starts nothing, so there is no
    // session to save.
    machine.declare("[system_prompt.harness.qwen]\ndelivery = \"launch\"\n");
    let mut command = machine.user.vibe();
    machine
        .harnessed(&mut command, VERIFIED)
        .args(["prompt", "flags", "--harness", "qwen", "--path"])
        .arg(machine.project.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("128 KiB"));
}

/// The real offline proof, as an **opt-in** test.
///
/// **Why it is gated and not simply a test.** It needs an installed OpenCode
/// binary, a published prompt-base set outside this repository, `python3`,
/// `bash`, a free loopback port and about five minutes; a suite that needed
/// any of those would stop being a suite. So the mechanism is pinned by the
/// shim tests above — over a fixture whose text nobody else owns — and *this*
/// one exists so the real thing can be run on demand, by the one command that
/// already knows how:
///
/// ```text
/// VIBE_OPENCODE_PROVE=1 \
/// VIBE_OPENCODE_SAMPLE=/tmp/sample \
/// VIBE_OPENCODE_BASES=/path/to/vibe-pkg-opencode \
///   cargo test -p vibe-cli --test cli_agent_launch -- --ignored opencode_prove
/// ```
///
/// The sample is built once by `campaigns/sys-prompt-2026-11/live/
/// make-sample.sh <dir> 202349`. The proof calls no model and reaches nothing
/// but `127.0.0.1`.
#[test]
#[ignore = "needs an installed OpenCode, a published prompt-base set and ~5 minutes"]
fn opencode_prove_runs_the_real_offline_capture() {
    if std::env::var_os("VIBE_OPENCODE_PROVE").is_none() {
        eprintln!("skipped: set VIBE_OPENCODE_PROVE=1 (and VIBE_OPENCODE_SAMPLE) to run it");
        return;
    }
    let sample = std::env::var("VIBE_OPENCODE_SAMPLE")
        .expect("VIBE_OPENCODE_SAMPLE must name a project built by make-sample.sh");
    let out = tempfile::tempdir().unwrap();
    let captures = out.path().join("captures");
    let script = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../campaigns/sys-prompt-2026-11/live/opencode-prove.sh");
    // `--from-vibe` is the point of running it from here: the carrier under
    // test is the one `vibe prompt flags` emits, so a pass says the bytes the
    // proof measured are the bytes the launcher sets.
    let mut command = std::process::Command::new("bash");
    command
        .arg(&script)
        .arg(&sample)
        .arg(&captures)
        .arg("--from-vibe")
        .env("VIBE", assert_cmd::cargo::cargo_bin("vibe"));
    if let Some(bases) = std::env::var_os("VIBE_OPENCODE_BASES") {
        command.env("VIBE_PROMPT_BASES_DIR", bases);
    }
    let status = command.status().expect("the proof script runs");
    let report = fs::read_to_string(captures.join("report.txt")).unwrap_or_default();
    assert!(
        status.success(),
        "the whole-request proof FAILED — every condition it failed is named \
         in its report:\n{report}"
    );
    assert!(report.contains("VERDICT: PASS"), "{report}");
}
