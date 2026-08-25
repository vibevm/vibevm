//! R2.2 commissioning tests for the lifecycle phase line.

mod common;

use std::fs;
use std::path::Path;

use common::{UserScratch, fixture_registry};
use specmark::verifies;
use vibe_wire::generated::lifecycle_report::LifecycleReport;
use vibe_wire::generated::lifecycle_state::LifecycleState;

fn init_project(user: &UserScratch) -> tempfile::TempDir {
    let project = tempfile::tempdir().unwrap();
    user.vibe()
        .args(["init", "--no-registry", "--author", "Lifecycle Test"])
        .arg("--path")
        .arg(project.path())
        .assert()
        .success();
    project
}

fn install_alpha(user: &UserScratch, project: &Path) {
    user.vibe()
        .args(["install", "flow:org.vibevm/integration-alpha"])
        .arg("--registry")
        .arg(fixture_registry())
        .arg("--path")
        .arg(project)
        .arg("--assume-yes")
        .assert()
        .success();
}

fn clean(user: &UserScratch, project: &Path) {
    user.vibe()
        .arg("clean")
        .arg("--path")
        .arg(project)
        .arg("--assume-yes")
        .assert()
        .success();
}

fn lifecycle_json(user: &UserScratch, project: &Path, phase: &str) -> LifecycleReport {
    let assert = user
        .vibe()
        .arg(phase)
        .arg("--json")
        .arg("--path")
        .arg(project)
        .arg("--registry")
        .arg(fixture_registry())
        .arg("--assume-yes")
        .assert()
        .success();
    lifecycle_document(&assert.get_output().stdout).unwrap_or_else(|error| {
        panic!(
            "generated lifecycle reader rejected stdout: {error}\n{}",
            String::from_utf8_lossy(&assert.get_output().stdout),
        )
    })
}

fn lifecycle_document(bytes: &[u8]) -> serde_json::Result<LifecycleReport> {
    let values = serde_json::Deserializer::from_slice(bytes)
        .into_iter::<serde_json::Value>()
        .collect::<Result<Vec<_>, _>>()?;
    serde_json::from_value(values.last().cloned().unwrap())
}

fn status<'a>(report: &'a LifecycleReport, phase: &str) -> &'a str {
    report
        .steps
        .iter()
        .find(|step| step.phase == phase)
        .unwrap_or_else(|| panic!("missing phase `{phase}` in {:?}", report.chain))
        .status
        .as_str()
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#LIFECYCLES")]
fn deploy_reports_all_nine_canonical_phases_through_the_generated_reader() {
    let user = UserScratch::new();
    let project = init_project(&user);

    let report = lifecycle_json(&user, project.path(), "deploy");
    assert!(report.ok);
    assert_eq!(report.command, "lifecycle");
    assert_eq!(report.requested, "deploy");
    assert_eq!(
        report.chain,
        [
            "validate", "install", "generate", "build", "test", "create", "verify", "package",
            "deploy",
        ]
    );
    assert_eq!(report.steps.len(), 9);
    assert_eq!(status(&report, "validate"), "ok");
    assert_eq!(status(&report, "install"), "fresh");
    for phase in [
        "generate", "build", "test", "create", "verify", "package", "deploy",
    ] {
        assert_eq!(status(&report, phase), "no-op");
    }
    let state: LifecycleState =
        toml::from_str(&fs::read_to_string(project.path().join(".vibe/lifecycle.toml")).unwrap())
            .unwrap();
    assert_eq!(state.run.requested, "deploy");
    assert_eq!(state.run.chain, report.chain);
    assert!(state.execution.is_empty());
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#INVOKE-RUNS-PRIORS")]
fn build_runs_generate_and_exactly_one_install_then_install_is_fresh() {
    let user = UserScratch::new();
    let project = init_project(&user);
    install_alpha(&user, project.path());
    clean(&user, project.path());

    let first = lifecycle_json(&user, project.path(), "build");
    assert_eq!(first.chain, ["validate", "install", "generate", "build"]);
    assert_eq!(
        first
            .chain
            .iter()
            .filter(|phase| *phase == "install")
            .count(),
        1
    );
    assert_eq!(status(&first, "install"), "ok");
    assert_eq!(status(&first, "generate"), "no-op");

    let second = lifecycle_json(&user, project.path(), "build");
    assert_eq!(status(&second, "install"), "fresh");
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#PHASE-INSTALL")]
fn json_parent_keeps_install_auto_approval_while_child_output_stays_silent() {
    let user = UserScratch::new();
    let project = init_project(&user);
    install_alpha(&user, project.path());
    clean(&user, project.path());

    let assert = user
        .vibe()
        .args(["build", "--json"])
        .arg("--path")
        .arg(project.path())
        .arg("--registry")
        .arg(fixture_registry())
        .assert()
        .success();
    let report = lifecycle_document(&assert.get_output().stdout)
        .expect("plan then lifecycle report, with no child install JSON");
    assert_eq!(status(&report, "install"), "ok");
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-000#JTD-SSOT")]
fn generated_reader_is_forward_compatible_with_future_step_statuses() {
    let report: LifecycleReport = serde_json::from_value(serde_json::json!({
        "ok": true,
        "command": "lifecycle",
        "requested": "create",
        "chain": ["create"],
        "contributions": [],
        "notices": [],
        "steps": [{"phase": "create", "status": "delegated"}],
    }))
    .expect("a newer normative status must not break an older reader");
    assert_eq!(report.steps[0].status, "delegated");
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-000#JTD-SSOT")]
fn lifecycle_json_keeps_global_invocation_stamps() {
    let user = UserScratch::new();
    let project = init_project(&user);

    let assert = user
        .vibe()
        .args([
            "deploy",
            "--json",
            "--unattended",
            "--invoked-by",
            "r2-test",
        ])
        .arg("--path")
        .arg(project.path())
        .assert()
        .success();
    let values = serde_json::Deserializer::from_slice(&assert.get_output().stdout)
        .into_iter::<serde_json::Value>()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let value = values.last().unwrap();
    assert_eq!(value["command"], "lifecycle");
    assert_eq!(value["invoked_by"], "r2-test");
    assert_eq!(value["unattended"], true);
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#PHASE-VALIDATE")]
fn validate_rejects_a_malformed_manifest_under_the_offline_posture() {
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    fs::write(
        project.path().join("vibe.toml"),
        "[project\nname = broken\n",
    )
    .unwrap();

    user.vibe()
        .args(["validate", "--offline"])
        .arg("--path")
        .arg(project.path())
        .assert()
        .failure()
        .stderr(predicates::str::contains("vibe.toml"));
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM")]
fn malformed_bootstrap_never_prepares_registry_inputs_before_validate() {
    for verb in ["validate", "build"] {
        let user = UserScratch::new();
        let project = tempfile::tempdir().unwrap();
        fs::write(
            project.path().join("vibe.toml"),
            "[project\nname = broken\n",
        )
        .unwrap();

        user.vibe()
            .env_remove("VIBE_NO_DEFAULT_REGISTRY")
            .arg(verb)
            .arg("--path")
            .arg(project.path())
            .assert()
            .failure();
        assert!(
            !user.settings.join("registry.toml").exists(),
            "{verb} prepared install registry inputs before validate refused the manifest",
        );
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM")]
fn quiet_lifecycle_is_exactly_one_summary_line() {
    let user = UserScratch::new();
    let project = init_project(&user);

    let assert = user
        .vibe()
        .args(["deploy", "--quiet"])
        .arg("--path")
        .arg(project.path())
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert_eq!(stdout.lines().count(), 1, "{stdout}");
    assert!(
        stdout.contains(
            "deploy completed (9 phases, 0 contribution(s) selected, 0 executed, 0 ok, 0 fresh, 0 notice(s))"
        ),
        "{stdout}"
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#CHAIN-GENERAL")]
fn clean_build_wipes_then_restores_the_world_and_keeps_lifecycle_state() {
    let user = UserScratch::new();
    let project = init_project(&user);
    install_alpha(&user, project.path());
    let state = project.path().join(".vibe/lifecycle.toml");
    let before = fs::read_to_string(&state).unwrap();

    let assert = user
        .vibe()
        .args(["clean", "build", "--json"])
        .arg("--path")
        .arg(project.path())
        .arg("--registry")
        .arg(fixture_registry())
        .arg("--assume-yes")
        .assert()
        .success();
    let report = lifecycle_document(&assert.get_output().stdout).unwrap();

    assert_eq!(
        report.chain,
        ["clean", "validate", "install", "generate", "build"]
    );
    assert!(
        project
            .path()
            .join(common::slot_dir("org.vibevm.integration-alpha", "0.1.0"))
            .is_dir(),
        "the prerequisite install restores dependency slots after clean",
    );
    assert!(project.path().join(common::index_rel()).is_file());
    let after = fs::read_to_string(&state).unwrap();
    assert_ne!(after, before, "default epoch must checkpoint its new run");
    let state: LifecycleState = toml::from_str(&after).unwrap();
    assert_eq!(state.run.requested, "build");
    assert_eq!(
        state.run.chain,
        ["validate", "install", "generate", "build"]
    );
}

#[test]
fn clean_only_never_fresh_skips_and_byte_preserves_invalid_state() {
    let user = UserScratch::new();
    let project = init_project(&user);
    let manifest = project.path().join("vibe.toml");
    let mut body = fs::read_to_string(&manifest).unwrap();
    body.push_str(
        r#"
[[extension]]
id = "host-clean"
point = "phase:clean"
handler = { kind = "builtin", name = "log" }
config = { message = "CLEAN-ALWAYS" }
"#,
    );
    fs::write(manifest, body).unwrap();
    let state = project.path().join(".vibe/lifecycle.toml");
    fs::create_dir_all(state.parent().unwrap()).unwrap();
    fs::write(&state, "arbitrary invalid sentinel bytes\n").unwrap();
    for _ in 0..2 {
        let output = user
            .vibe()
            .args(["clean", "--path"])
            .arg(project.path())
            .arg("--assume-yes")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(String::from_utf8_lossy(&output.stdout).contains("CLEAN-ALWAYS"));
        assert_eq!(
            fs::read_to_string(&state).unwrap(),
            "arbitrary invalid sentinel bytes\n"
        );
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#CHAIN-GENERAL")]
fn outer_clean_consent_does_not_silently_approve_the_child_install() {
    let user = UserScratch::new();
    let project = init_project(&user);

    user.vibe()
        .args([
            "clean",
            "--assume-yes",
            "install",
            "flow:org.vibevm/integration-alpha",
        ])
        .arg("--registry")
        .arg(fixture_registry())
        .arg("--path")
        .arg(project.path())
        .assert()
        .failure()
        .stderr(predicates::str::contains(
            "no TTY available for confirmation",
        ));
    assert!(
        !project
            .path()
            .join(common::slot_dir("org.vibevm.integration-alpha", "0.1.0"))
            .exists(),
        "the outer clean consent must not become install consent",
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#CHAIN-GENERAL")]
fn clean_path_precedence_matches_the_original_install_chain() {
    let user = UserScratch::new();
    let selected = init_project(&user);
    install_alpha(&user, selected.path());
    let non_project_cwd = tempfile::tempdir().unwrap();

    user.vibe()
        .current_dir(non_project_cwd.path())
        .args(["clean", "build", "--json"])
        .arg("--path")
        .arg(selected.path())
        .arg("--registry")
        .arg(fixture_registry())
        .arg("--assume-yes")
        .assert()
        .success();
    assert!(
        selected
            .path()
            .join(common::slot_dir("org.vibevm.integration-alpha", "0.1.0"))
            .is_dir(),
        "default outer `.` must yield to the child path",
    );

    let outer = init_project(&user);
    install_alpha(&user, outer.path());
    let child = init_project(&user);
    install_alpha(&user, child.path());
    let child_marker = child
        .path()
        .join(common::slot_dir("org.vibevm.integration-alpha", "0.1.0"))
        .join("child-must-stay.txt");
    fs::write(&child_marker, "untouched\n").unwrap();

    user.vibe()
        .arg("clean")
        .arg("--path")
        .arg(outer.path())
        .args(["build", "--json"])
        .arg("--path")
        .arg(child.path())
        .arg("--registry")
        .arg(fixture_registry())
        .arg("--assume-yes")
        .assert()
        .success();
    assert!(child_marker.is_file(), "explicit outer path must win");
    assert!(
        outer
            .path()
            .join(common::slot_dir("org.vibevm.integration-alpha", "0.1.0"))
            .is_dir(),
        "the selected outer project must be restored",
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#FAILURE-BY-PHASE")]
fn install_failure_after_clean_does_not_roll_the_wipe_back() {
    let user = UserScratch::new();
    let project = init_project(&user);
    install_alpha(&user, project.path());
    let manifest_path = project.path().join("vibe.toml");
    let mut manifest = vibe_core::manifest::Manifest::read(&manifest_path).unwrap();
    manifest
        .requires
        .packages
        .push(vibe_core::PackageRef::parse("org.vibevm/does-not-exist@^1.0").unwrap());
    manifest.write(&manifest_path).unwrap();

    let slot = project
        .path()
        .join(common::slot_dir("org.vibevm.integration-alpha", "0.1.0"));
    let index = project.path().join(common::index_rel());
    assert!(slot.is_dir() && index.is_file());

    let output = user
        .vibe()
        .args(["clean", "build", "--json", "--offline"])
        .arg("--path")
        .arg(project.path())
        .arg("--registry")
        .arg(fixture_registry())
        .arg("--assume-yes")
        .output()
        .unwrap();
    assert!(!output.status.success());
    let documents = serde_json::Deserializer::from_slice(&output.stdout)
        .into_iter::<serde_json::Value>()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert!(
        documents
            .iter()
            .all(|document| document["command"] == "lifecycle:plan"),
        "selection may be reported before failure, outcomes may not: {documents:?}"
    );
    let error: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(error["ok"], false);
    assert!(!slot.exists(), "failed install must not restore the slot");
    assert!(
        !index.exists(),
        "failed install must not restore generated boot"
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#FAILURE-BY-PHASE")]
fn failing_install_stops_before_generate_and_build() {
    let user = UserScratch::new();
    let project = init_project(&user);
    let manifest_path = project.path().join("vibe.toml");
    let mut manifest = fs::read_to_string(&manifest_path).unwrap();
    manifest.push_str("\n[requires]\npackages = { \"org.vibevm/does-not-exist\" = \"^1.0\" }\n");
    fs::write(&manifest_path, manifest).unwrap();

    let assert = user
        .vibe()
        .args(["build", "--offline"])
        .arg("--path")
        .arg(project.path())
        .arg("--registry")
        .arg(fixture_registry())
        .arg("--assume-yes")
        .assert()
        .failure();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(!stdout.contains("generate:"), "{stdout}");
    assert!(!stdout.contains("build:"), "{stdout}");
    assert!(!stdout.contains("completed"), "{stdout}");
}
