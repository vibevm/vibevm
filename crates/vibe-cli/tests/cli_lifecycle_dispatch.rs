//! R2.4 commissioning tests for the canonical envelope and builtin dispatch.

mod common;

use std::fs;
use std::path::Path;

use common::{UserScratch, fixture_registry};
use specmark::verifies;
use vibe_wire::generated::lifecycle_report::LifecycleReport;
use vibe_wire::generated::lifecycle_state::{ExecutionRecordStatus, LifecycleState};

fn init_project(user: &UserScratch) -> tempfile::TempDir {
    let project = tempfile::tempdir().unwrap();
    user.vibe()
        .args(["init", "--no-registry", "--author", "Lifecycle Dispatch"])
        .arg("--path")
        .arg(project.path())
        .assert()
        .success();
    project
}

fn log_registry() -> tempfile::TempDir {
    let registry = tempfile::tempdir().unwrap();
    let source = fixture_registry().join("org.vibevm/integration-alpha/v0.1.0");
    let package = registry.path().join("org.vibevm/integration-alpha/v0.1.0");
    common::copy_tree(&source, &package);
    let manifest = package.join("vibe.toml");
    let mut body = fs::read_to_string(&manifest).unwrap();
    body.push_str(
        r#"

[[extension]]
id = "install-log"
point = "phase:install"
handler = { kind = "builtin", name = "log" }
config = { message = "INSTALL|{phase}|{project}|{package}" }

[[extension]]
id = "build-log"
point = "phase:build"
handler = { kind = "builtin", name = "log" }
config = { message = "BUILD|{phase}|{project}|{package}|{future}" }

[[extension]]
id = "test-log"
point = "phase:test"
handler = { kind = "builtin", name = "log" }
config = { message = "TEST|{phase}|{project}|{package}" }

[[extension]]
id = "clean-log"
point = "phase:clean"
handler = { kind = "builtin", name = "log" }
config = { message = "CLEAN|{phase}|{package}" }
"#,
    );
    fs::write(manifest, body).unwrap();
    registry
}

fn install_from(user: &UserScratch, project: &Path, registry: &Path) {
    user.vibe()
        .args(["install", "flow:org.vibevm/integration-alpha"])
        .arg("--registry")
        .arg(registry)
        .arg("--path")
        .arg(project)
        .arg("--assume-yes")
        .assert()
        .success();
}

fn append(project: &Path, text: &str) {
    let manifest = project.join("vibe.toml");
    let mut body = fs::read_to_string(&manifest).unwrap();
    body.push_str(text);
    fs::write(manifest, body).unwrap();
}

fn project_name(project: &Path) -> String {
    vibe_core::manifest::Manifest::read(project.join("vibe.toml"))
        .unwrap()
        .project
        .unwrap()
        .name
}

fn json_documents(bytes: &[u8]) -> Vec<serde_json::Value> {
    serde_json::Deserializer::from_slice(bytes)
        .into_iter::<serde_json::Value>()
        .collect::<Result<_, _>>()
        .unwrap()
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TEST-LOG-PLUGIN")]
fn dependency_logs_run_in_phase_order_with_placeholders_and_provider_attribution() {
    let user = UserScratch::new();
    let project = init_project(&user);
    let registry = log_registry();
    install_from(&user, project.path(), registry.path());
    let name = project_name(project.path());

    let assert = user
        .vibe()
        .args(["test", "--path"])
        .arg(project.path())
        .arg("--registry")
        .arg(registry.path())
        .arg("--assume-yes")
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let install = format!("INSTALL|install|{name}|org.vibevm/integration-alpha");
    let build = format!("BUILD|build|{name}|org.vibevm/integration-alpha|{{future}}");
    let test = format!("TEST|test|{name}|org.vibevm/integration-alpha");
    let install_at = stdout.find(&install).unwrap_or_else(|| panic!("{stdout}"));
    let build_at = stdout.find(&build).unwrap_or_else(|| panic!("{stdout}"));
    let test_at = stdout.find(&test).unwrap_or_else(|| panic!("{stdout}"));
    assert!(install_at < build_at && build_at < test_at, "{stdout}");
    assert!(
        stdout.contains("log [org.vibevm/integration-alpha]"),
        "{stdout}"
    );
    assert!(
        stdout.find("will run").unwrap() < install_at,
        "ritual narration must precede execution:\n{stdout}"
    );
}

#[test]
fn host_activation_config_override_reaches_the_same_envelope() {
    let user = UserScratch::new();
    let project = init_project(&user);
    let registry = log_registry();
    install_from(&user, project.path(), registry.path());
    append(
        project.path(),
        r#"

[[extensions.use]]
ref = "org.vibevm/integration-alpha#build-log"
config = { message = "OVERRIDE|{phase}|{package}" }
"#,
    );

    let assert = user
        .vibe()
        .args(["build", "--path"])
        .arg(project.path())
        .arg("--registry")
        .arg(registry.path())
        .arg("--assume-yes")
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(
        stdout.contains("OVERRIDE|build|org.vibevm/integration-alpha"),
        "{stdout}"
    );
    assert!(!stdout.contains("BUILD|build|"), "{stdout}");
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#FAILURE-BY-PHASE")]
fn unknown_builtin_stops_before_a_later_log_and_names_the_contribution() {
    let user = UserScratch::new();
    let project = init_project(&user);
    append(
        project.path(),
        r#"

[[extension]]
id = "first"
point = "phase:build"
handler = { kind = "builtin", name = "log" }
config = { message = "FIRST-RAN" }

[[extension]]
id = "stop"
point = "phase:build"
handler = { kind = "builtin", name = "unknown" }
config = { message = "STOP" }

[[extension]]
id = "never"
point = "phase:build"
handler = { kind = "builtin", name = "log" }
config = { message = "NEVER-RAN" }
"#,
    );

    let output = user
        .vibe()
        .args(["build", "--path"])
        .arg(project.path())
        .arg("--assume-yes")
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stdout.contains("FIRST-RAN"), "{stdout}");
    assert!(!stdout.contains("NEVER-RAN"), "{stdout}");
    assert!(stderr.contains("#stop"), "{stderr}");
    assert!(stderr.contains("unknown builtin `unknown`"), "{stderr}");
    let state: LifecycleState =
        toml::from_str(&fs::read_to_string(project.path().join(".vibe/lifecycle.toml")).unwrap())
            .unwrap();
    assert_eq!(
        state
            .execution
            .values()
            .filter(|row| row.status == ExecutionRecordStatus::Ok)
            .count(),
        1
    );
    assert_eq!(
        state
            .execution
            .values()
            .filter(|row| row.status == ExecutionRecordStatus::Fail)
            .count(),
        1
    );
}

#[test]
fn missing_and_wrong_log_message_fail_loudly_in_human_and_json_modes() {
    for (config, json) in [("", false), ("config = { message = 7 }", true)] {
        let user = UserScratch::new();
        let project = init_project(&user);
        append(
            project.path(),
            &format!(
                r#"

[[extension]]
id = "bad-config"
point = "phase:build"
handler = {{ kind = "builtin", name = "log" }}
{config}
"#,
            ),
        );
        let mut command = user.vibe();
        command.arg("build").arg("--path").arg(project.path());
        if json {
            command.arg("--json");
        }
        let output = command.output().unwrap();
        assert!(!output.status.success());
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(error.contains("#bad-config"), "{error}");
        assert!(error.contains("message` must be a string"), "{error}");
        if json {
            serde_json::from_slice::<serde_json::Value>(&output.stderr).unwrap();
        }
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#LIFECYCLES")]
fn builtin_clean_runs_before_terminal_wipe() {
    let user = UserScratch::new();
    let project = init_project(&user);
    let registry = log_registry();
    install_from(&user, project.path(), registry.path());
    let slot = project
        .path()
        .join(common::slot_dir("org.vibevm.integration-alpha", "0.1.0"));

    let assert = user
        .vibe()
        .args(["clean", "--path"])
        .arg(project.path())
        .arg("--assume-yes")
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let log_at = stdout
        .find("CLEAN|clean|org.vibevm/integration-alpha")
        .unwrap_or_else(|| panic!("{stdout}"));
    let wipe_at = stdout.find("cleaned 1 dependency slot").unwrap();
    assert!(log_at < wipe_at, "{stdout}");
    assert!(!slot.exists(), "clean handler did not precede/permit wipe");
}

#[test]
fn clean_refusal_runs_no_handler_and_preserves_the_world() {
    let user = UserScratch::new();
    let project = init_project(&user);
    let registry = log_registry();
    install_from(&user, project.path(), registry.path());
    let slot = project
        .path()
        .join(common::slot_dir("org.vibevm.integration-alpha", "0.1.0"));

    let output = user
        .vibe()
        .args(["clean", "--path"])
        .arg(project.path())
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stdout.contains("CLEAN|clean|"), "{stdout}");
    assert!(stderr.contains("no TTY available"), "{stderr}");
    assert!(slot.is_dir());
}

#[test]
fn direct_install_applied_fresh_and_empty_world_callbacks_are_once_and_json_last() {
    let user = UserScratch::new();
    let project = init_project(&user);
    let registry = log_registry();

    for packages in [true, false] {
        let mut command = user.vibe();
        command.arg("install");
        if packages {
            command.arg("flow:org.vibevm/integration-alpha");
        }
        command
            .arg("--json")
            .arg("--registry")
            .arg(registry.path())
            .arg("--path")
            .arg(project.path())
            .arg("--assume-yes");
        let output = command.output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let documents = json_documents(&output.stdout);
        assert_eq!(documents.last().unwrap()["command"], "install");
        let plan_at = documents
            .iter()
            .position(|document| document["command"] == "lifecycle:plan")
            .unwrap();
        // `vibe install` is the OUTERMOST command here, so it — and only it —
        // emits a root document. The `phase:install` ritual rows it ran travel
        // as that report's typed `contributions`; the separate `lifecycle`
        // echo was removed so a parked run can emit exactly one document.
        let outcome_at = documents.len() - 1;
        assert!(plan_at < outcome_at, "the plan precedes the report");
        assert!(
            documents[plan_at]["contributions"][0]
                .get("status")
                .is_none()
        );
        assert!(
            documents[plan_at]["contributions"][0]
                .get("message")
                .is_none()
        );
        assert_eq!(
            documents
                .iter()
                .filter(|document| document["command"] == "install")
                .count(),
            1,
            "exactly one root report",
        );
        let rows = documents[outcome_at]["contributions"].as_array().unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["status"], if packages { "ok" } else { "fresh" });
        assert_eq!(rows[0].get("message").is_some(), packages);
    }

    let empty = init_project(&user);
    append(
        empty.path(),
        r#"

[[extension]]
id = "host-install"
point = "phase:install"
handler = { kind = "builtin", name = "log" }
config = { message = "EMPTY|{project}|{package}" }
"#,
    );
    let output = user
        .vibe()
        .args(["install", "--json", "--path"])
        .arg(empty.path())
        .arg("--assume-yes")
        .output()
        .unwrap();
    assert!(output.status.success());
    let documents = json_documents(&output.stdout);
    assert_eq!(documents.last().unwrap()["command"], "install");
    // The empty world still runs its host `phase:install` row, and that row
    // reaches the outermost command's ONE report rather than a separate
    // `lifecycle` echo.
    assert_eq!(
        documents
            .iter()
            .filter(|document| document["command"] == "install")
            .count(),
        1,
        "exactly one root report",
    );
    assert_eq!(
        documents.last().unwrap()["contributions"]
            .as_array()
            .map_or(0, Vec::len),
        1,
        "and it carries the host install row: {documents:#?}",
    );
}

#[test]
fn lifecycle_json_is_a_structured_plan_and_outcome_while_quiet_is_one_line() {
    let user = UserScratch::new();
    let project = init_project(&user);
    let registry = log_registry();
    install_from(&user, project.path(), registry.path());

    let json = user
        .vibe()
        .args(["test", "--json", "--path"])
        .arg(project.path())
        .arg("--registry")
        .arg(registry.path())
        .arg("--assume-yes")
        .output()
        .unwrap();
    assert!(json.status.success());
    let documents = json_documents(&json.stdout);
    assert_eq!(documents.len(), 2);
    assert_eq!(documents[0]["command"], "lifecycle:plan");
    assert!(documents[0]["contributions"][0].get("status").is_none());
    assert!(documents[0]["contributions"][0].get("message").is_none());
    let report: LifecycleReport = serde_json::from_value(documents[1].clone()).unwrap();
    assert!(report.contributions.iter().all(|row| row.status == "ok"));

    let quiet = user
        .vibe()
        .args(["test", "--quiet", "--path"])
        .arg(project.path())
        .arg("--registry")
        .arg(registry.path())
        .arg("--assume-yes")
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&quiet.get_output().stdout);
    assert_eq!(stdout.lines().count(), 1, "{stdout}");
    assert!(
        stdout.contains("3 contribution(s) selected, 0 executed, 0 ok, 3 fresh"),
        "{stdout}"
    );
    assert!(!stdout.contains("BUILD|"), "{stdout}");
}

#[test]
fn second_build_is_fresh_and_force_executes_every_selected_log() {
    let user = UserScratch::new();
    let project = init_project(&user);
    let registry = log_registry();
    install_from(&user, project.path(), registry.path());

    for (force, expected_fresh) in [(false, false), (false, true), (true, false)] {
        let assert = user
            .vibe()
            .args(["build", "--path"])
            .arg(project.path())
            .arg("--registry")
            .arg(registry.path())
            .arg("--assume-yes")
            .args(force.then_some("--force"))
            .assert()
            .success();
        let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
        assert_eq!(stdout.contains("BUILD|build|"), !expected_fresh, "{stdout}");
        assert_eq!(stdout.contains("fresh `"), expected_fresh, "{stdout}");
    }
}

#[test]
fn editing_one_declared_input_reruns_only_its_contribution() {
    let user = UserScratch::new();
    let project = init_project(&user);
    fs::write(project.path().join("a.txt"), "a1").unwrap();
    fs::write(project.path().join("b.txt"), "b1").unwrap();
    append(
        project.path(),
        r#"
[[extension]]
id = "input-a"
point = "phase:build"
handler = { kind = "builtin", name = "log" }
config = { message = "INPUT-A" }
inputs = ["a.txt"]
[[extension]]
id = "input-b"
point = "phase:build"
handler = { kind = "builtin", name = "log" }
config = { message = "INPUT-B" }
inputs = ["b.txt"]
"#,
    );
    let run = || {
        let output = user
            .vibe()
            .args(["build", "--json", "--path"])
            .arg(project.path())
            .arg("--assume-yes")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let docs = json_documents(&output.stdout);
        serde_json::from_value::<LifecycleReport>(docs.last().unwrap().clone()).unwrap()
    };
    assert!(run().contributions.iter().all(|row| row.status == "ok"));
    assert!(run().contributions.iter().all(|row| row.status == "fresh"));
    fs::write(project.path().join("a.txt"), "a2").unwrap();
    let report = run();
    assert_eq!(
        report
            .contributions
            .iter()
            .find(|row| row.key.ends_with("#input-a"))
            .unwrap()
            .status,
        "ok"
    );
    assert_eq!(
        report
            .contributions
            .iter()
            .find(|row| row.key.ends_with("#input-b"))
            .unwrap()
            .status,
        "fresh"
    );
}

#[test]
fn direct_callback_failure_keeps_the_durable_world_and_emits_no_install_success() {
    let user = UserScratch::new();
    let project = init_project(&user);
    let registry = log_registry();
    let package_manifest = registry
        .path()
        .join("org.vibevm/integration-alpha/v0.1.0/vibe.toml");
    let body = fs::read_to_string(&package_manifest)
        .unwrap()
        .replace(
            "handler = { kind = \"builtin\", name = \"log\" }\nconfig = { message = \"INSTALL|{phase}|{project}|{package}\" }",
            "handler = { kind = \"builtin\", name = \"unknown\" }\nconfig = { message = \"INSTALL-FAIL\" }",
        );
    fs::write(package_manifest, body).unwrap();

    let output = user
        .vibe()
        .args(["install", "flow:org.vibevm/integration-alpha", "--json"])
        .arg("--registry")
        .arg(registry.path())
        .arg("--path")
        .arg(project.path())
        .arg("--assume-yes")
        .output()
        .unwrap();
    assert!(!output.status.success());
    let documents = json_documents(&output.stdout);
    assert_eq!(
        documents
            .iter()
            .filter(|document| document["command"] == "lifecycle:plan")
            .count(),
        1,
    );
    assert!(
        documents
            .iter()
            .all(|document| document["command"] != "install")
    );
    assert!(project.path().join("vibe.lock").is_file());
    assert!(
        project
            .path()
            .join(common::slot_dir("org.vibevm.integration-alpha", "0.1.0"))
            .is_dir()
    );
}
