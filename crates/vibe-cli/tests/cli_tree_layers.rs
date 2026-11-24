//! Opt-in layered tree keeps the frozen legacy tree projection unchanged.
mod common;
use common::UserScratch;
use std::{fs, path::Path};
fn tree(user: &UserScratch, root: &Path, layers: bool) -> serde_json::Value {
    let mut cmd = user.vibe();
    cmd.args(["tree", "--path"]).arg(root).arg("--json");
    if layers {
        cmd.arg("--layers");
    }
    let output = cmd.output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let mut report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    if layers {
        validate_layered(&report);
    }
    fn timestamps(value: &mut serde_json::Value) {
        match value {
            serde_json::Value::Object(map) => {
                map.remove("generated_at");
                for value in map.values_mut() {
                    timestamps(value);
                }
            }
            serde_json::Value::Array(rows) => {
                for row in rows {
                    timestamps(row);
                }
            }
            _ => {}
        }
    }
    timestamps(&mut report);
    report
}
fn validate_layered(report: &serde_json::Value) {
    let project: serde_json::Value =
        serde_json::from_str(include_str!("../resources/package-tree.schema.v1.json")).unwrap();
    let registry = jsonschema::Registry::new()
        .add(
            "https://vibespecs.org/schemas/package-tree.schema.v1.json",
            project,
        )
        .unwrap()
        .prepare()
        .unwrap();
    let schema: serde_json::Value =
        serde_json::from_str(include_str!("../resources/layered-tree.schema.v2.json")).unwrap();
    let validator = jsonschema::options()
        .with_base_uri("https://vibespecs.org/schemas/layered-tree.schema.v2.json")
        .with_registry(&registry)
        .build(&schema)
        .unwrap();
    let errors: Vec<_> = validator
        .iter_errors(report)
        .map(|error| error.to_string())
        .collect();
    assert!(errors.is_empty(), "{}", errors.join("\n"));
}
#[test]
fn layered_tree_reports_disabled_and_missing_layers_without_changing_project_projection() {
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    user.init_project(project.path());
    let legacy = tree(&user, project.path(), false);
    assert!(legacy.get("layers").is_none());
    let absent = tree(&user, project.path(), true);
    assert_eq!(absent["layers"][0]["id"], "project");
    assert_eq!(absent["layers"][1]["state"], "absent");
    assert_eq!(absent["layers"][2]["state"], "absent");
    assert!(!user.settings.join("override-bindings.toml").exists());
    let mut expected = legacy.clone();
    expected.as_object_mut().unwrap().remove("ok");
    expected.as_object_mut().unwrap().remove("command");
    assert_eq!(absent["project"], expected);
    user.vibe().args(["user", "init"]).assert().success();
    user.vibe()
        .args(["install", "-g", "--user-project", "--offline", "--json"])
        .assert()
        .success();
    let output = user
        .vibe()
        .args(["user", "override", "init", "--alias", "tree", "--path"])
        .arg(project.path())
        .arg("--json")
        .output()
        .unwrap();
    assert!(output.status.success());
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let root = std::path::PathBuf::from(report["root"].as_str().unwrap());
    user.vibe()
        .args(["install", "--path"])
        .arg(&root)
        .args(["--offline", "--json"])
        .assert()
        .success();
    user.vibe()
        .args(["user", "override", "disable", "--path"])
        .arg(project.path())
        .assert()
        .success();
    let before_mapping = fs::read(user.settings.join("override-bindings.toml")).unwrap();
    let before_manifest = fs::read(root.join("vibe.toml")).unwrap();
    let layered = tree(&user, project.path(), true);
    assert_eq!(
        fs::read(user.settings.join("override-bindings.toml")).unwrap(),
        before_mapping
    );
    assert_eq!(fs::read(root.join("vibe.toml")).unwrap(), before_manifest);
    assert_eq!(layered["layers"][1]["state"], "available");
    assert_eq!(layered["layers"][2]["state"], "available");
    assert_eq!(layered["layers"][2]["enabled"], false);
    assert!(layered["layers"][2]["tree"].is_object());
    assert_eq!(tree(&user, project.path(), false), legacy);
    fs::write(root.join("vibe.toml"), "invalid").unwrap();
    let broken = tree(&user, project.path(), true);
    assert_eq!(broken["layers"][2]["state"], "unavailable");
    assert_eq!(broken["layers"][1]["state"], "available");
    user.vibe()
        .args(["tree", "--layers", "--plain", "--path"])
        .arg(project.path())
        .assert()
        .success();
}

#[test]
fn layers_plain_has_rule_sections_and_one_legend() {
    let user = UserScratch::new();
    let scratch = tempfile::tempdir_in("/home/olegchir/.claude2/jobs/10bde8f1/tmp").unwrap();
    let home = scratch.path().join("home");
    let project = scratch.path().join("project");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(&project).unwrap();
    let vibe = || {
        let mut cmd = user.vibe();
        cmd.env("HOME", &home)
            .env("USERPROFILE", &home)
            .env("VIBE_SETTINGS", &user.settings);
        cmd
    };
    vibe()
        .current_dir(&project)
        .args(["init", "--name", "layer-plain"])
        .assert()
        .success();
    vibe().args(["user", "init"]).assert().success();
    let output = vibe()
        .args(["tree", "--layers", "--plain", "--path"])
        .arg(&project)
        .output()
        .unwrap();
    assert!(output.status.success(), "layered tree command succeeds");
    let stdout = String::from_utf8(output.stdout).unwrap();
    for name in ["project", "personal", "override"] {
        assert!(
            stdout
                .lines()
                .any(|line| line.starts_with(&format!("── {name} layer "))),
            "each layer has a named rule"
        );
    }
    assert_eq!(stdout.matches("columns: load").count(), 1);
    assert!(!stdout.lines().any(|line| line.starts_with("layer ")));
    assert!(
        stdout
            .lines()
            .any(|line| line.starts_with("  enabled · available"))
    );
    assert!(stdout.lines().any(|line| line == "  disabled · absent"));
}

#[test]
fn identical_package_coordinates_are_kept_in_independent_layer_payloads() {
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    user.init_project(project.path());
    let registry = tempfile::tempdir().unwrap();
    let package = registry.path().join("org.layers/shared/v1.0.0");
    fs::create_dir_all(&package).unwrap();
    fs::write(
        package.join("vibe.toml"),
        "[package]\ngroup='org.layers'\nname='shared'\nkind='flow'\nversion='1.0.0'\n",
    )
    .unwrap();
    user.vibe().args(["user", "init"]).assert().success();
    let output = user
        .vibe()
        .args(["user", "override", "init", "--alias", "layers", "--path"])
        .arg(project.path())
        .arg("--json")
        .output()
        .unwrap();
    assert!(output.status.success());
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let override_root = std::path::PathBuf::from(report["root"].as_str().unwrap());
    for root in [
        project.path().to_path_buf(),
        user.settings.join("user-project"),
        override_root,
    ] {
        user.vibe()
            .args(["install", "org.layers/shared", "--path"])
            .arg(root)
            .arg("--registry")
            .arg(registry.path())
            .args(["--offline", "--json"])
            .assert()
            .success();
    }
    let report = tree(&user, project.path(), true);
    assert_eq!(report["project"]["packages"][0]["id"], "org.layers/shared");
    assert_eq!(
        report["layers"][1]["tree"]["packages"][0]["id"],
        "org.layers/shared"
    );
    assert_eq!(
        report["layers"][2]["tree"]["packages"][0]["id"],
        "org.layers/shared"
    );
    assert_eq!(report["layers"].as_array().unwrap().len(), 3);
    assert_eq!(report["project"]["packages"].as_array().unwrap().len(), 1);
}

#[test]
fn layered_tree_does_not_create_absent_settings_or_binding_stores() {
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    user.init_project(project.path());
    let scope = tempfile::tempdir().unwrap();
    let missing = scope.path().join("missing/settings");
    let output = user
        .vibe()
        .env("VIBE_SETTINGS", &missing)
        .args(["tree", "--layers", "--json", "--path"])
        .arg(project.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    validate_layered(&report);
    assert_eq!(report["layers"][1]["state"], "absent");
    assert_eq!(report["layers"][2]["state"], "absent");
    assert!(!scope.path().join("missing").exists());
}
