//! `vibe prompt` and `vibe agent <launcher>`, offline and against shims.
//!
//! **No real agent is ever started and no model is ever called.** Every
//! launch here goes to a shim on `PATH` that records the argv it received
//! and exits, which is the only honest way to assert what a launch passes:
//! the claim is about an argument vector, so the test reads an argument
//! vector (PROP-076 `#SYS-HARNESS-EXECUTABLE`).

use std::fs;

use predicates::prelude::*;

mod common;

use common::UserScratch;

const TEMPLATE: &str = "# My frame\n\nPermanent instructions.\n";

struct Project {
    user: UserScratch,
    dir: tempfile::TempDir,
}

impl Project {
    fn opted_in() -> Self {
        let project = Self::fresh();
        fs::write(
            project
                .dir
                .path()
                .join(common::boot_rel("SYSTEM-TEMPLATE.md")),
            TEMPLATE,
        )
        .unwrap();
        project
    }

    fn fresh() -> Self {
        let user = UserScratch::new();
        let dir = tempfile::tempdir().unwrap();
        user.vibe()
            .arg("init")
            .arg("--path")
            .arg(dir.path())
            .assert()
            .success();
        Self { user, dir }
    }

    fn prompt(&self, verb: &str) -> assert_cmd::Command {
        let mut command = self.user.vibe();
        command
            .arg("prompt")
            .arg(verb)
            .arg("--path")
            .arg(self.dir.path());
        command
    }

    fn boot(&self, name: &str) -> std::path::PathBuf {
        self.boot_dir().join(name)
    }

    fn boot_dir(&self) -> std::path::PathBuf {
        self.dir.path().join(vibe_core::layout::current_boot_dir())
    }
}
#[test]
fn build_reports_what_it_did_and_where_then_reports_unchanged() {
    let project = Project::opted_in();
    project
        .prompt("build")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "✻ system prompt rebuilt · SYSTEM.md ",
        ))
        .stdout(predicate::str::contains("SYSTEM.md"))
        .stdout(predicate::str::contains("AGENT-LANE.md "))
        .stdout(predicate::str::contains("AGENT-LANE.md"))
        .stdout(predicate::str::contains(" bytes"));
    assert!(project.boot("SYSTEM.md").exists());
    assert!(project.boot("SYSTEM.meta.json").exists());
    assert!(project.boot("AGENT-LANE.md").exists());
    let ignore = fs::read_to_string(project.boot(".gitignore")).unwrap();
    for name in [
        "/SYSTEM.md",
        "/SYSTEM.meta.json",
        "/AGENT-LANE.md",
        "/.gitignore",
    ] {
        assert!(ignore.contains(name), "{ignore}");
    }

    project
        .prompt("build")
        .assert()
        .success()
        .stdout(predicate::str::contains("✻ system prompt unchanged"));
}
#[test]
fn a_project_that_has_not_opted_in_is_told_so_rather_than_left_silent() {
    let project = Project::fresh();
    project
        .prompt("build")
        .assert()
        .success()
        .stdout(predicate::str::contains("○ system prompt  not opted in"))
        .stdout(predicate::str::contains(
            "vibe prompt init opts this project in",
        ));
    assert!(!project.boot("SYSTEM.md").exists());
}
#[test]
fn deleting_the_template_removes_exactly_the_four_owned_paths() {
    let project = Project::opted_in();
    project.prompt("build").assert().success();
    fs::remove_file(project.boot("SYSTEM-TEMPLATE.md")).unwrap();
    project
        .prompt("build")
        .assert()
        .success()
        .stdout(predicate::str::contains("✻ system prompt removed"));
    for name in [
        "SYSTEM.md",
        "SYSTEM.meta.json",
        "AGENT-LANE.md",
        ".gitignore",
    ] {
        assert!(
            !project.boot(name).exists(),
            "`{name}` survived the opt-out"
        );
    }
    assert!(project.boot("00-core.md").exists());
}
#[test]
fn check_is_equal_after_a_build_and_json_carries_the_same_facts() {
    let project = Project::opted_in();
    project.prompt("build").assert().success();
    project
        .prompt("check")
        .assert()
        .success()
        .stdout(predicate::str::contains("✓ system prompt  equal"));
    let output = project
        .prompt("check")
        .arg("--json")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let report: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(report["protocol"], "vibe-prompt-check/1");
    assert_eq!(report["verdict"], "equal");
    assert_eq!(report["opted_in"], true);
    assert_eq!(report["artifacts"].as_array().unwrap().len(), 3);
    project
        .prompt("check")
        .arg("--quiet")
        .assert()
        .success()
        .stdout("system prompt equal · .\n")
        .stderr("");
    let fresh = Project::fresh();
    fresh
        .prompt("check")
        .assert()
        .success()
        .stdout(predicate::str::contains("○ system prompt  not opted in"))
        .stdout(predicate::str::contains(
            "vibe prompt init opts this project in",
        ));
    fresh
        .prompt("check")
        .arg("--quiet")
        .assert()
        .success()
        .stdout("")
        .stderr("");
}
#[test]
fn check_reports_c1_and_c4_for_a_divergent_instruction_file() {
    let project = Project::opted_in();
    project.prompt("build").assert().success();
    project
        .prompt("check")
        .assert()
        .success()
        .stdout(predicate::str::contains("C1:").not());

    let claude = project.dir.path().join("CLAUDE.md");
    let text = fs::read_to_string(&claude).unwrap();
    fs::write(&claude, text.replace("<vibevm>", "<vibevm>\nHand-edited.")).unwrap();
    project
        .prompt("check")
        .assert()
        .success()
        .stdout(predicate::str::contains("✓ system prompt  equal"))
        .stdout(predicate::str::contains("C1:"))
        .stdout(predicate::str::contains("C4:"))
        .stdout(predicate::str::contains("CLAUDE.md"));
}
#[test]
fn check_stays_equal_when_a_dynamic_boot_file_changes() {
    let project = Project::opted_in();
    project.prompt("build").assert().success();
    fs::write(project.boot("90-user.md"), "# edited by me\n").unwrap();
    project
        .prompt("check")
        .assert()
        .success()
        .stdout(predicate::str::contains("✓ system prompt  equal"));
}
#[test]
fn check_is_stale_and_names_the_input_when_the_template_changes() {
    let project = Project::opted_in();
    project.prompt("build").assert().success();
    fs::write(project.boot("SYSTEM-TEMPLATE.md"), "# another frame\n").unwrap();
    project
        .prompt("check")
        .assert()
        .failure()
        .stderr(predicate::str::contains("prompt_stale"))
        .stderr(predicate::str::contains("vibe prompt build"));
    project
        .prompt("check")
        .arg("--quiet")
        .assert()
        .failure()
        .stdout("")
        .stderr(predicate::str::contains("prompt_stale"));
    let output = project
        .prompt("path")
        .args(["--part", "system"])
        .assert()
        .success()
        .stdout(format!("{}\n", project.boot("SYSTEM.md").display()))
        .get_output()
        .clone();
    let warning = String::from_utf8(output.stderr).unwrap();
    assert!(warning.starts_with("! SYSTEM.md is stale: "), "{warning}");
    assert!(warning.contains("the template input"), "{warning}");
    assert!(warning.ends_with("\n  vibe prompt build\n"), "{warning}");
    assert!(!warning.contains(&project.dir.path().display().to_string()));
}
#[test]
fn check_writes_nothing_and_leaves_every_byte_of_the_boot_directory_alone() {
    let project = Project::opted_in();
    project.prompt("build").assert().success();
    let before = listing(&project);
    project.prompt("check").assert().success();
    project.prompt("path").assert().success();
    project.prompt("prepare").arg("--json").assert().success();
    assert_eq!(before, listing(&project), "a read-only verb wrote");
}

fn listing(project: &Project) -> Vec<(String, Vec<u8>)> {
    let dir = project.boot_dir();
    let mut rows: Vec<(String, Vec<u8>)> = fs::read_dir(&dir)
        .unwrap()
        .filter_map(Result::ok)
        .filter(|entry| entry.path().is_file())
        .map(|entry| {
            (
                entry.file_name().to_string_lossy().into_owned(),
                fs::read(entry.path()).unwrap(),
            )
        })
        .collect();
    rows.sort();
    rows
}
#[test]
fn prepare_returns_the_bands_separately_and_never_concatenated() {
    let project = Project::opted_in();
    project.prompt("build").assert().success();
    let output = project
        .prompt("prepare")
        .arg("--json")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let report: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(report["protocol"], "vibe-prompt-prepare/1");
    assert!(report["prefix"]["len"].as_u64().unwrap() > 0);
    assert!(report["prefix"]["sha256"].as_str().unwrap().len() == 64);
    let band_u = report["unconditional"].as_array().unwrap();
    assert!(!band_u.is_empty(), "a fresh project has dynamic boot rows");
    for member in band_u {
        assert!(member["sha256"].as_str().unwrap().len() == 64);
    }
    let boundaries = report["cache_boundaries"].as_array().unwrap();
    let band_u_total: u64 = band_u
        .iter()
        .map(|member| member["len"].as_u64().unwrap())
        .sum();
    assert_eq!(
        boundaries.last().unwrap().as_u64().unwrap(),
        report["prefix"]["len"].as_u64().unwrap() + band_u_total
    );
}
#[test]
fn prepare_refuses_a_stale_prefix_with_the_build_that_fixes_it() {
    let project = Project::opted_in();
    project.prompt("build").assert().success();
    fs::write(project.boot("SYSTEM-TEMPLATE.md"), "# another frame\n").unwrap();
    project
        .prompt("prepare")
        .arg("--json")
        .assert()
        .failure()
        .stderr(predicate::str::contains("vibe prompt build"));
}
#[test]
fn path_prints_an_absolute_path_alone_and_never_an_empty_line() {
    let project = Project::opted_in();
    project.prompt("build").assert().success();
    let output = project
        .prompt("path")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let printed = String::from_utf8(output).unwrap();
    let printed = printed.trim();
    assert!(printed.ends_with("AGENT-LANE.md"), "{printed}");
    assert_eq!(
        std::path::Path::new(printed),
        project.boot("AGENT-LANE.md").canonicalize().unwrap()
    );

    fs::remove_file(project.boot("AGENT-LANE.md")).unwrap();
    project
        .prompt("path")
        .assert()
        .failure()
        .stderr(predicate::str::contains("vibe prompt build"))
        .stderr(predicate::str::contains("prompt_not_generated"))
        .stderr(predicate::str::contains(
            project.boot("AGENT-LANE.md").display().to_string(),
        ))
        .stderr(predicate::str::contains("SYS-OPERATIONS-PATH-AND-FLAGS"));
}
#[test]
fn flags_print_flags_only_and_never_a_program_or_lane_bytes() {
    let project = Project::opted_in();
    project.prompt("build").assert().success();
    let printed = String::from_utf8(
        project
            .prompt("flags")
            .args(["--harness", "claude"])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone(),
    )
    .unwrap();
    assert!(
        printed.starts_with("--append-system-prompt-file "),
        "{printed}"
    );
    assert!(printed.contains("--exclude-dynamic-system-prompt-sections"));
    assert!(!printed.contains("claude "), "a program leaked: {printed}");
    assert!(!printed.contains("My frame"), "lane bytes leaked");

    let output = project
        .prompt("flags")
        .args(["--harness", "codex", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let report: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(report["kind"], "codex");
    assert_eq!(report["delivery"], "launch");
    let flags: Vec<String> = report["flags"]
        .as_array()
        .unwrap()
        .iter()
        .map(|flag| flag.as_str().unwrap().to_owned())
        .collect();
    assert!(flags.contains(&"-p".to_owned()));
    assert!(
        flags
            .iter()
            .any(|flag| flag.starts_with("project_doc_max_bytes=")),
        "{flags:?}"
    );
}
#[test]
fn flags_with_workers_add_the_subagent_flag_naming_the_same_lane() {
    let project = Project::opted_in();
    project.prompt("build").assert().success();
    let output = project
        .prompt("flags")
        .args(["--harness", "claude", "--workers", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let report: serde_json::Value = serde_json::from_slice(&output).unwrap();
    let flags: Vec<String> = report["flags"]
        .as_array()
        .unwrap()
        .iter()
        .map(|flag| flag.as_str().unwrap().to_owned())
        .collect();
    let lane = project.boot("AGENT-LANE.md").display().to_string();
    assert_eq!(
        flags,
        [
            "--append-system-prompt-file".to_owned(),
            lane.clone(),
            "--exclude-dynamic-system-prompt-sections".to_owned(),
            "--append-subagent-system-prompt-file".to_owned(),
            lane,
        ]
    );
}
#[test]
fn flags_with_workers_for_codex_print_no_worker_flag_and_say_so() {
    let project = Project::opted_in();
    project.prompt("build").assert().success();
    let output = project
        .prompt("flags")
        .args(["--harness", "codex", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let usual: serde_json::Value = serde_json::from_slice(&output).unwrap();
    let output = project
        .prompt("flags")
        .args(["--harness", "codex", "--workers", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let workers: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(usual["flags"], workers["flags"]);
    assert!(workers["notes"].as_array().unwrap().iter().any(|note| {
        note.as_str()
            .unwrap()
            .contains("has no flag that appends a file to its subagents' system prompt")
    }));
}
#[test]
fn an_unknown_harness_is_a_refusal_listing_the_launchers() {
    let project = Project::opted_in();
    project.prompt("build").assert().success();
    project
        .prompt("flags")
        .args(["--harness", "gemini"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("is not an agent launcher"))
        .stderr(predicate::str::contains("launcher_unknown"))
        .stderr(predicate::str::contains("codex"))
        .stderr(predicate::str::contains("qwen"));
}
#[test]
fn an_unknown_node_is_a_refusal_listing_the_nodes() {
    let project = Project::opted_in();
    project
        .prompt("check")
        .args(["--node", "members/nope"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("is not a node of this workspace"));
}
#[test]
fn an_install_ends_with_the_same_summary_line_as_a_build() {
    let project = Project::opted_in();
    project
        .user
        .vibe()
        .args(["reinstall", "--assume-yes"])
        .arg(project.dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "system prompt  rebuilt · SYSTEM.md ",
        ))
        .stdout(predicate::str::contains("AGENT-LANE.md "));
    project
        .user
        .vibe()
        .args(["reinstall", "--assume-yes"])
        .arg(project.dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("✓ system prompt  unchanged"));
}
#[test]
fn a_failed_reinstall_prints_no_prompt_summary() {
    let project = Project::opted_in();
    project.prompt("build").assert().success();
    fs::write(project.boot("SYSTEM-TEMPLATE.md"), [0xff]).unwrap();
    project
        .user
        .vibe()
        .args(["reinstall", "--assume-yes"])
        .arg(project.dir.path())
        .assert()
        .failure()
        .stdout(predicate::str::contains("system prompt").not())
        .stderr(predicate::str::contains("SYSTEM-TEMPLATE.md"));
}
#[test]
fn an_install_of_a_project_without_the_opt_in_says_nothing_about_a_prompt() {
    let project = Project::fresh();
    let output = project
        .user
        .vibe()
        .args(["reinstall", "--assume-yes"])
        .arg(project.dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let printed = String::from_utf8(output).unwrap();
    assert!(
        !printed.contains("system prompt"),
        "a project that never opted in was told about a prompt: {printed}"
    );
    assert!(!project.boot("SYSTEM.md").exists());
}
#[test]
fn agent_edits_reach_their_own_trailers() {
    let user = UserScratch::new();
    let path = user.settings.join("agents.toml");
    for mode in ["human", "--quiet", "--json"] {
        for (args, done) in [
            (vec!["add", "myag", "--preset", "codex"], "added agent myag"),
            (vec!["use", "myag"], "default agent set to myag"),
            (vec!["remove", "myag"], "removed agent myag"),
        ] {
            let mut command = user.vibe();
            command
                .env("HOME", &user.settings)
                .env("USERPROFILE", &user.settings);
            command
                .env_remove("VIBE_AGENT_COMMAND")
                .arg("agent")
                .args(args);
            if mode != "human" {
                command.arg(mode);
            }
            let output = command.assert().success().get_output().clone();
            assert!(output.stderr.is_empty(), "{output:?}");
            let text = String::from_utf8(output.stdout).unwrap();
            if mode == "--json" {
                let doc: serde_json::Value = serde_json::from_str(&text).unwrap();
                assert_eq!(doc["protocol"], "vibe-agent-list/1");
                assert_eq!(doc["path"], path.display().to_string());
                assert_eq!(
                    doc["agents"].as_array().unwrap().len(),
                    usize::from(!done.starts_with("removed"))
                );
                assert_eq!(
                    doc["default"].as_str(),
                    done.starts_with("default").then_some("myag")
                );
                continue;
            }
            let trailer = format!("✻ {done} · {}", path.display());
            assert_eq!(text.lines().last(), Some(trailer.as_str()), "{text}");
            assert_eq!(text.matches('✻').count(), 1, "{text}");
            if mode == "--quiet" {
                assert_eq!(text, format!("{trailer}\n"));
            } else {
                let positions = ["launchers", "agent definitions", "trust"]
                    .map(|name| text.find(&format!("── {name} ─")).expect(&text));
                assert!(positions.windows(2).all(|pair| pair[0] < pair[1]), "{text}");
                assert!(text.starts_with("── launchers ─"), "{text}");
                assert!(!text.contains("vibe agent:"), "{text}");
            }
        }
    }
}
