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

/// One initialised project that has opted in, under an isolated home.
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
        self.dir
            .path()
            .join(vibe_core::layout::current_boot_dir())
            .join(name)
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
        .stdout(predicate::str::contains("system prompt: rebuilt → "))
        .stdout(predicate::str::contains("SYSTEM.md"))
        .stdout(predicate::str::contains("lane → "))
        .stdout(predicate::str::contains("AGENT-LANE.md"))
        .stdout(predicate::str::contains(" bytes)"));
    assert!(project.boot("SYSTEM.md").exists());
    assert!(project.boot("SYSTEM.meta.json").exists());
    assert!(project.boot("AGENT-LANE.md").exists());
    // The ignore file is written before the artifacts it covers, and it
    // ignores itself, so an opted-in project's `git status` stays clean.
    let ignore = fs::read_to_string(project.boot(".gitignore")).unwrap();
    for name in [
        "/SYSTEM.md",
        "/SYSTEM.meta.json",
        "/AGENT-LANE.md",
        "/.gitignore",
    ] {
        assert!(ignore.contains(name), "{ignore}");
    }

    // A second build with nothing changed writes nothing and says so.
    project
        .prompt("build")
        .assert()
        .success()
        .stdout(predicate::str::contains("system prompt: unchanged → "));
}

#[test]
fn a_project_that_has_not_opted_in_is_told_so_rather_than_left_silent() {
    let project = Project::fresh();
    project
        .prompt("build")
        .assert()
        .success()
        .stdout(predicate::str::contains("system prompt: not generated"))
        .stdout(predicate::str::contains("has not opted in"));
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
        .stdout(predicate::str::contains("system prompt: removed"));
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
    // The authored boot sources are untouched: only what vibe owns went.
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
        .stdout(predicate::str::contains("system prompt: equal"));
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
}

/// The assertion the band model rests on: a dynamic boot file's bytes are
/// witnessed nowhere, so editing one leaves the prefix current.
#[test]
fn check_stays_equal_when_a_dynamic_boot_file_changes() {
    let project = Project::opted_in();
    project.prompt("build").assert().success();
    fs::write(project.boot("90-user.md"), "# edited by me\n").unwrap();
    project
        .prompt("check")
        .assert()
        .success()
        .stdout(predicate::str::contains("system prompt: equal"));
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
        .stdout(predicate::str::contains("system prompt: stale"))
        .stdout(predicate::str::contains("vibe prompt build"));
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

/// Every file of the boot directory with its bytes, so "wrote nothing" is
/// asserted over content rather than over mtimes a filesystem may coarsen.
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
    // The breakpoints are offsets into one ordered sequence, and the last
    // one is the end of band U.
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

    // A missing artifact is a refusal naming the build, never an empty
    // line a shell alias turns silently into a broken launch.
    fs::remove_file(project.boot("AGENT-LANE.md")).unwrap();
    project
        .prompt("path")
        .assert()
        .failure()
        .stderr(predicate::str::contains("vibe prompt build"));
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
fn an_unknown_harness_is_a_refusal_listing_the_launchers() {
    let project = Project::opted_in();
    project.prompt("build").assert().success();
    project
        .prompt("flags")
        .args(["--harness", "qwen"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("is not an agent launcher"))
        .stderr(predicate::str::contains("`claude`"))
        .stderr(predicate::str::contains("`codex`"));
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
        .stdout(predicate::str::contains("system prompt: rebuilt → "))
        .stdout(predicate::str::contains("lane → "));
    // A second reinstall with nothing changed says `unchanged`, exactly as
    // `vibe prompt build` does — one sentence, spelled in one place.
    project
        .user
        .vibe()
        .args(["reinstall", "--assume-yes"])
        .arg(project.dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("system prompt: unchanged → "));
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
        !printed.contains("system prompt:"),
        "a project that never opted in was told about a prompt: {printed}"
    );
    assert!(!project.boot("SYSTEM.md").exists());
}
