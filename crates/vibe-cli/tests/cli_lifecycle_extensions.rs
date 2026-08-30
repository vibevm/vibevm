//! R2.3 CLI adapter tests: durable-world reload, planning order, and clean safety.

mod common;

use std::fs;
use std::path::Path;

use common::{UserScratch, fixture_registry};
use vibe_core::PackageRef;
use vibe_core::manifest::{ActiveSection, ExtensionKey, ExtensionUse, Manifest};
use vibe_wire::generated::install_report::InstallReport;
use vibe_wire::generated::lifecycle_report::LifecycleReport;

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

fn extension_registry() -> tempfile::TempDir {
    let registry = tempfile::tempdir().unwrap();
    let source = fixture_registry().join("org.vibevm/integration-alpha/v0.1.0");
    let package = registry.path().join("org.vibevm/integration-alpha/v0.1.0");
    common::copy_tree(&source, &package);
    let manifest = package.join("vibe.toml");
    let mut body = fs::read_to_string(&manifest).unwrap();
    body.push_str(
        r#"

[[extension]]
id = "install-plan"
point = "phase:install"
handler = { kind = "builtin", name = "log" }
config = { message = "install {phase} {package}" }
[[extension]]
id = "build-default"
point = "phase:build"
handler = { kind = "builtin", name = "log" }
config = { message = "build default {phase} {package}" }
[[extension]]
id = "build-activated"
point = "phase:build"
handler = { kind = "builtin", name = "log" }
config = { message = "build activated {phase} {package}" }
[[extension]]
id = "test-plan"
point = "phase:test"
handler = { kind = "builtin", name = "log" }
config = { message = "test {phase} {package}" }
[[extension]]
id = "clean-guard"
point = "phase:clean"
handler = { kind = "script", base = "hooks/clean" }
[[extension]]
id = "compile-auto-ignored"
point = "compile:emitted"
handler = { kind = "builtin", name = "xml-minify" }
config = { message = "compile {phase} {package}" }
auto = true
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

fn append_host_controls(project: &Path, disable: &[&str]) {
    let manifest = project.join("vibe.toml");
    let mut body = fs::read_to_string(&manifest).unwrap();
    body.push_str(
        r#"

[[extension]]
id = "host-build"
point = "phase:build"
handler = { kind = "builtin", name = "log" }
config = { message = "host {phase} {package}" }

[extensions]
"#,
    );
    body.push_str(&format!(
        "disable = [{}]\n",
        disable
            .iter()
            .map(|key| format!("\"{key}\""))
            .collect::<Vec<_>>()
            .join(", ")
    ));
    body.push_str(
        r#"
[[extensions.use]]
ref = "org.vibevm/integration-alpha#build-activated"
"#,
    );
    fs::write(manifest, body).unwrap();
}

fn json_documents(bytes: &[u8]) -> Vec<serde_json::Value> {
    serde_json::Deserializer::from_slice(bytes)
        .into_iter::<serde_json::Value>()
        .collect::<Result<_, _>>()
        .unwrap()
}

fn last_lifecycle(bytes: &[u8]) -> LifecycleReport {
    serde_json::from_value(json_documents(bytes).pop().unwrap()).unwrap()
}
fn status<'a>(report: &'a LifecycleReport, phase: &str) -> &'a str {
    report
        .steps
        .iter()
        .find(|step| step.phase == phase)
        .unwrap()
        .status
        .as_str()
}

#[test]
fn direct_install_reloads_the_new_world_and_keeps_its_final_json_document_last() {
    let user = UserScratch::new();
    let project = init_project(&user);
    let registry = extension_registry();
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
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let documents = json_documents(&output.stdout);
    assert_eq!(documents.last().unwrap()["command"], "install");
    assert_eq!(
        documents
            .iter()
            .filter(|document| document["command"] == "install")
            .count(),
        1,
        "the post-durability callback must be consumed exactly once"
    );
    let ritual = documents
        .iter()
        .find(|document| document["command"] == "install")
        .expect("ritual document before final install report");
    // `vibe install` is the outermost command: the ritual rows it ran are its
    // own report's typed `contributions`, and the step vocabulary lives on the
    // lifecycle report a PHASE VERB emits, not here.
    let ritual: InstallReport = serde_json::from_value(ritual.clone()).unwrap();
    assert!(ritual.ok);
    assert!(!ritual.unchanged, "an applied install is not unchanged");
    assert!(ritual.contributions.iter().any(|row| {
        row.key == "org.vibevm/integration-alpha#install-plan"
            && row.phase == "install"
            && row.status == "ok"
    }));
    assert_eq!(ritual.notices.len(), 1);
    assert!(ritual.notices[0].contains("compile-auto-ignored"));
}

#[test]
fn direct_fresh_install_keeps_builtin_status_separate_from_the_planned_ritual() {
    let user = UserScratch::new();
    let project = init_project(&user);
    let registry = extension_registry();
    install_from(&user, project.path(), registry.path());

    for force in [false, true] {
        let mut command = user.vibe();
        command.args(["install", "--json"]);
        if force {
            command.arg("--force");
        }
        let output = command
            .arg("--registry")
            .arg(registry.path())
            .arg("--path")
            .arg(project.path())
            .arg("--assume-yes")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let documents = json_documents(&output.stdout);
        assert_eq!(documents.last().unwrap()["command"], "install");
        assert_eq!(documents.last().unwrap()["unchanged"], true);
        let ritual: InstallReport = serde_json::from_value(
            documents
                .iter()
                .find(|document| document["command"] == "install")
                .unwrap()
                .clone(),
        )
        .unwrap();
        assert!(ritual.unchanged, "the lock was fresh");
        assert_eq!(
            ritual.contributions[0].status,
            if force { "ok" } else { "fresh" }
        );
        if !force {
            let raw = fs::read_to_string(project.path().join(".vibe/lifecycle.toml")).unwrap();
            let state: vibe_wire::generated::lifecycle_state::LifecycleState =
                toml::from_str(&raw).unwrap();
            assert!(state.execution.values().all(|row| row.duration_ms == 0));
        }
    }
}

#[test]
fn direct_empty_world_callback_plans_host_install_once_before_the_final_report() {
    let user = UserScratch::new();
    let project = init_project(&user);
    let manifest = project.path().join("vibe.toml");
    let mut body = fs::read_to_string(&manifest).unwrap();
    body.push_str(
        r#"

[[extension]]
id = "host-install"
point = "phase:install"
handler = { kind = "builtin", name = "log" }
config = { message = "host install {phase} {package}" }
"#,
    );
    fs::write(&manifest, body).unwrap();

    let output = user
        .vibe()
        .args(["install", "--json", "--path"])
        .arg(project.path())
        .arg("--assume-yes")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let documents = json_documents(&output.stdout);
    assert_eq!(documents.last().unwrap()["command"], "install");
    assert_eq!(documents.last().unwrap()["unchanged"], true);
    let rituals: Vec<_> = documents
        .iter()
        .filter(|document| document["command"] == "install")
        .collect();
    assert_eq!(rituals.len(), 1, "callback must be consumed once");
    let ritual: InstallReport = serde_json::from_value((*rituals[0]).clone()).unwrap();
    assert!(ritual.unchanged, "the empty world's lock was fresh");
    assert_eq!(ritual.contributions.len(), 1);
    assert!(ritual.contributions[0].key.ends_with("#host-install"));
    assert_eq!(ritual.contributions[0].tier, "host-declaration");
}

#[test]
fn lifecycle_plan_keeps_phase_lock_declaration_and_control_tier_order() {
    let user = UserScratch::new();
    let project = init_project(&user);
    let registry = extension_registry();
    install_from(&user, project.path(), registry.path());
    append_host_controls(
        project.path(),
        &["org.vibevm/integration-alpha#build-default"],
    );
    let assert = user
        .vibe()
        .args(["test", "--json"])
        .arg("--registry")
        .arg(registry.path())
        .arg("--path")
        .arg(project.path())
        .arg("--assume-yes")
        .assert()
        .success();
    let report = last_lifecycle(&assert.get_output().stdout);
    let host_name = vibe_core::manifest::Manifest::read(project.path().join("vibe.toml"))
        .unwrap()
        .project
        .unwrap()
        .name;
    let host_key = format!("__host__/{host_name}#host-build");
    let keys: Vec<_> = report
        .contributions
        .iter()
        .map(|row| (row.phase.as_str(), row.key.as_str(), row.tier.as_str()))
        .collect();
    assert_eq!(
        keys,
        [
            (
                "install",
                "org.vibevm/integration-alpha#install-plan",
                "dependency"
            ),
            ("build", host_key.as_str(), "host-declaration"),
            (
                "build",
                "org.vibevm/integration-alpha#build-activated",
                "host-activation"
            ),
            (
                "test",
                "org.vibevm/integration-alpha#test-plan",
                "dependency"
            ),
        ]
    );
    assert_eq!(status(&report, "build"), "ok");
    assert_eq!(status(&report, "test"), "ok");
    assert_eq!(report.notices.len(), 1);
    assert!(report.notices[0].contains("compile-auto-ignored"));
    assert!(
        !report
            .contributions
            .iter()
            .any(|row| row.key.ends_with("#build-default"))
    );
}

#[test]
fn direct_install_quiet_is_one_line_and_names_the_planned_count() {
    let user = UserScratch::new();
    let project = init_project(&user);
    let registry = extension_registry();
    let assert = user
        .vibe()
        .args(["install", "flow:org.vibevm/integration-alpha", "--quiet"])
        .arg("--registry")
        .arg(registry.path())
        .arg("--path")
        .arg(project.path())
        .arg("--assume-yes")
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert_eq!(stdout.lines().count(), 1, "{stdout}");
    assert!(
        stdout.contains("1 lifecycle contribution(s) selected, 1 executed, 1 ok"),
        "{stdout}"
    );
    assert!(stdout.contains("1 lifecycle notice(s)"), "{stdout}");
}

#[test]
fn standalone_and_chained_clean_refuse_a_plan_before_the_wipe() {
    for chained in [false, true] {
        let user = UserScratch::new();
        let project = init_project(&user);
        let registry = extension_registry();
        install_from(&user, project.path(), registry.path());
        let slot = project
            .path()
            .join(common::slot_dir("org.vibevm.integration-alpha", "0.1.0"));
        let mut command = user.vibe();
        command.arg("clean").arg("--path").arg(project.path());
        if chained {
            command.args(["build", "--registry"]);
            command.arg(registry.path());
        }
        command
            .arg("--assume-yes")
            .assert()
            .failure()
            .stderr(predicates::str::contains("cannot select a usable script"));
        assert!(
            slot.is_dir(),
            "clean wiped before refusing (chained={chained})"
        );
    }
}

#[test]
fn disabling_clean_allows_wipe_reinstall_and_preserves_lifecycle_state() {
    let user = UserScratch::new();
    let project = init_project(&user);
    let registry = extension_registry();
    install_from(&user, project.path(), registry.path());
    let manifest = project.path().join("vibe.toml");
    let mut body = fs::read_to_string(&manifest).unwrap();
    body.push_str(
        r#"

[extensions]
disable = ["org.vibevm/integration-alpha#clean-guard"]
"#,
    );
    fs::write(&manifest, body).unwrap();
    let state = project.path().join(".vibe/lifecycle.toml");
    let before = fs::read_to_string(&state).unwrap();
    user.vibe()
        .args(["clean", "build", "--json"])
        .arg("--path")
        .arg(project.path())
        .arg("--registry")
        .arg(registry.path())
        .arg("--assume-yes")
        .assert()
        .success();
    let after = fs::read_to_string(state).unwrap();
    assert_ne!(after, before);
    let state: vibe_wire::generated::lifecycle_state::LifecycleState =
        toml::from_str(&after).unwrap();
    assert_eq!(state.run.requested, "build");
    assert!(
        project
            .path()
            .join(common::slot_dir("org.vibevm.integration-alpha", "0.1.0"))
            .is_dir()
    );
}

#[test]
fn disabled_clean_remains_idempotent_after_the_installed_world_is_gone() {
    let user = UserScratch::new();
    let project = init_project(&user);
    let registry = extension_registry();
    install_from(&user, project.path(), registry.path());
    let manifest = project.path().join("vibe.toml");
    let mut body = fs::read_to_string(&manifest).unwrap();
    body.push_str(
        r#"

[extensions]
disable = ["org.vibevm/integration-alpha#clean-guard"]
"#,
    );
    fs::write(&manifest, body).unwrap();
    let state = project.path().join(".vibe/lifecycle.toml");
    fs::write(&state, "arbitrary invalid sentinel bytes\n").unwrap();

    for _ in 0..2 {
        user.vibe()
            .args(["clean", "--path"])
            .arg(project.path())
            .arg("--assume-yes")
            .assert()
            .success();
        assert_eq!(
            fs::read_to_string(&state).unwrap(),
            "arbitrary invalid sentinel bytes\n"
        );
    }
}

#[test]
fn active_stack_does_not_break_a_second_clean_after_the_world_is_gone() {
    let user = UserScratch::new();
    let project = init_project(&user);
    let registry = extension_registry();
    let package_manifest = registry
        .path()
        .join("org.vibevm/integration-alpha/v0.1.0/vibe.toml");
    let body = fs::read_to_string(&package_manifest).unwrap().replacen(
        "kind = \"flow\"",
        "kind = \"stack\"",
        1,
    );
    fs::write(package_manifest, body).unwrap();
    user.vibe()
        .args(["install", "stack:org.vibevm/integration-alpha"])
        .arg("--registry")
        .arg(registry.path())
        .arg("--path")
        .arg(project.path())
        .arg("--assume-yes")
        .assert()
        .success();
    let manifest = project.path().join("vibe.toml");
    let mut body = fs::read_to_string(&manifest).unwrap();
    body.push_str(
        r#"

[active]
stack = "integration-alpha"

[extensions]
disable = ["org.vibevm/integration-alpha#clean-guard"]
"#,
    );
    fs::write(manifest, body).unwrap();

    for _ in 0..2 {
        user.vibe()
            .args(["clean", "--path"])
            .arg(project.path())
            .arg("--assume-yes")
            .assert()
            .success();
    }
}

#[test]
fn clean_build_defers_future_stack_and_controls_until_after_install() {
    let user = UserScratch::new();
    let project = init_project(&user);
    let registry = extension_registry();
    let beta_source = fixture_registry().join("org.vibevm/integration-beta/v0.1.0");
    let beta = registry.path().join("org.vibevm/integration-beta/v0.1.0");
    common::copy_tree(&beta_source, &beta);
    let beta_manifest = beta.join("vibe.toml");
    let mut beta_body = fs::read_to_string(&beta_manifest).unwrap().replacen(
        "kind = \"flow\"",
        "kind = \"stack\"",
        1,
    );
    beta_body.push_str(
        r#"

[[extension]]
id = "future-build"
point = "phase:build"
handler = { kind = "builtin", name = "log" }
config = { message = "future {phase} {package}" }
"#,
    );
    fs::write(beta_manifest, beta_body).unwrap();

    install_from(&user, project.path(), registry.path());
    let manifest_path = project.path().join("vibe.toml");
    let mut manifest = Manifest::read(&manifest_path).unwrap();
    manifest
        .requires
        .packages
        .push(PackageRef::parse("stack:org.vibevm/integration-beta@^0.1.0").unwrap());
    manifest.active = Some(ActiveSection {
        stack: Some("integration-beta".to_string()),
    });
    manifest.extension_controls.uses.push(ExtensionUse {
        reference: ExtensionKey::authored("org.vibevm/integration-beta#future-build"),
        config: None,
    });
    manifest
        .extension_controls
        .disable
        .push(ExtensionKey::authored(
            "org.vibevm/integration-alpha#clean-guard",
        ));
    manifest.write(&manifest_path).unwrap();

    user.vibe()
        .args(["validate", "--json", "--path"])
        .arg(project.path())
        .assert()
        .failure()
        .stderr(predicates::str::contains(
            "absent from effective-world lock",
        ));

    let assert = user
        .vibe()
        .args(["clean", "build", "--json"])
        .arg("--path")
        .arg(project.path())
        .arg("--registry")
        .arg(registry.path())
        .arg("--assume-yes")
        .assert()
        .success();
    let report = last_lifecycle(&assert.get_output().stdout);
    assert!(report.contributions.iter().any(|row| {
        row.key == "org.vibevm/integration-beta#future-build"
            && row.phase == "build"
            && row.tier == "host-activation"
    }));
}
