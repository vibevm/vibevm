//! Public Linux fixture for changing an installed choice and for updates
//! (PROP-065 `#modify`, `#outdated`, `#update-and-rollback`): `vibe app
//! modify` drops a feature and switches the installer option of the same
//! version, running the old recorded removal first and keeping the slot,
//! its default and its public command; `outdated` reports the newest
//! registry version, `hold` makes it report held and a bulk update skip
//! it; an update keeps the previous version hidden, `rollback` swaps them
//! back, `autoremove` and `--purge-previous` remove the kept version, and
//! the update's run record names the key it replaced. Inside a project,
//! `-g` reaches the user home and a miss names the home it searched.

#![cfg(target_os = "linux")]

mod common;

use std::fs;

use common::app_registry::{AppRegistry, failed, json, ok};

const V1: &str = "org.example/editor@1.0.0";
const V2: &str = "org.example/editor@2.0.0";
const NAME: &str = "org.example/editor";

fn editor(apps: &AppRegistry, version: &str) {
    let log = apps.root.path().join("choices.log");
    let log = log.display();
    let marker = format!("EDITOR{}", &version[..1]);
    let package = apps.package(
        "editor",
        version,
        "editor",
        &format!(
            r#"
[[installer_options.features]]
id = "docs"
name = "Documentation"
default = true

[[installer_options.steps]]
id = "place"
title = "Place the editor"
kind = "copy"
from = "payload"

[[installer_options.steps]]
id = "docs"
title = "Copy the documentation"
kind = "copy"
when = {{ feature = "docs" }}
from = "docs"
to = "share/doc"

[[installer_options.steps]]
id = "note"
title = "Note the choice"
kind = "shell"
phase = "finish"
command = "echo main >> '{log}'"
uninstall = "echo undo-main >> '{log}'"

[[installer_options.launchers]]
id = "main"
name = "Editor"
executable = "$app_bin/editor"
kind = "command"

[[installer_options.public_commands]]
name = "edit"
launcher = "main"

[[installer_options]]
id = "alt"
name = "Alternative build"
platform = "linux"
platform_name = "Linux"
arch = "{arch}"

[[installer_options.steps]]
id = "place"
title = "Place the alternative build"
kind = "copy"
from = "payload-alt"

[[installer_options.launchers]]
id = "main"
name = "Editor"
executable = "$app_bin/editor"
kind = "command"

[[installer_options.public_commands]]
name = "edit"
launcher = "main"
"#,
            arch = std::env::consts::ARCH
        ),
    );
    apps.script(&package, "editor", &marker);
    let alt = package.join("payload-alt/bin");
    fs::create_dir_all(&alt).unwrap();
    fs::copy(package.join("payload/bin/editor"), alt.join("editor")).unwrap();
    fs::write(package.join("payload-alt/ALT"), "alt").unwrap();
    fs::create_dir_all(package.join("docs")).unwrap();
    fs::write(package.join("docs/readme.txt"), "read me").unwrap();
}

fn fixture() -> AppRegistry {
    let apps = AppRegistry::new();
    editor(&apps, "1.0.0");
    editor(&apps, "2.0.0");
    apps
}

fn edit(apps: &AppRegistry) -> String {
    let output = std::process::Command::new(apps.settings().join("opt/bin/edit"))
        .env_remove("VIBE_APP_USE")
        .output()
        .unwrap();
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

fn log(apps: &AppRegistry) -> String {
    fs::read_to_string(apps.root.path().join("choices.log")).unwrap_or_default()
}

#[test]
fn modify_changes_features_and_installer_in_place() {
    let apps = fixture();
    ok(&apps.app(
        &["install", "-g"],
        "org.example/editor@=1.0.0",
        &["--installer", "main"],
    ));
    let payload = apps
        .settings()
        .join("opt/apps/editor/org.example.editor@1.0.0");
    assert!(payload.join("share/doc/readme.txt").is_file());
    assert_eq!(log(&apps), "main\n");

    let modified = apps.app(
        &["app", "modify", "-g"],
        NAME,
        &["--no-feature", "docs", "--json"],
    );
    let report = json(&modified);
    assert_eq!(report["command"], "modify");
    assert_eq!(report["installation"], V1);
    assert_eq!(report["slotDefault"], V1, "the slot keeps its default");
    let shown = String::from_utf8_lossy(&modified.stderr);
    assert!(
        shown.contains("modify first undoes note of the recorded choice: echo undo-main"),
        "{shown}"
    );
    assert!(!payload.join("share/doc").exists(), "the docs step is gone");
    assert_eq!(
        log(&apps),
        "main\nundo-main\nmain\n",
        "old removal, then the new choice"
    );
    let record = &apps.index()["installations"][V1];
    assert!(record.get("features").is_none(), "{record}");
    assert_eq!(edit(&apps), "EDITOR1", "the public command still works");

    let report = json(&apps.app(
        &["app", "modify", "-g"],
        NAME,
        &["--installer", "alt", "--json"],
    ));
    assert_eq!(report["installation"], V1);
    assert_eq!(apps.index()["installations"][V1]["option"], "alt");
    assert!(
        payload.join("ALT").is_file(),
        "the alternative payload is placed"
    );
    assert_eq!(log(&apps), "main\nundo-main\nmain\nundo-main\n");
    assert_eq!(edit(&apps), "EDITOR1");
    let runs = apps
        .settings()
        .join("cache/app-runs/org.example.editor@1.0.0");
    let commands: Vec<String> = fs::read_dir(&runs)
        .unwrap()
        .map(|entry| fs::read_to_string(entry.unwrap().path().join("run.json")).unwrap())
        .collect();
    assert!(
        commands.iter().any(|run| run.contains("\"modify\"")),
        "{commands:?}"
    );
}

#[test]
fn update_keeps_the_previous_version_hidden_and_rollback_swaps_them() {
    let apps = fixture();
    ok(&apps.app(
        &["install", "-g"],
        "org.example/editor@=1.0.0",
        &["--installer", "main"],
    ));
    let report = json(&apps.app(&["app", "outdated", "-g"], "--json", &[]));
    assert_eq!(report["protocol"], "vibe-app-outdated/1");
    let row = &report["installations"][0];
    assert_eq!(
        (
            row["key"].as_str(),
            row["latest"].as_str(),
            row["status"].as_str()
        ),
        (Some(V1), Some("2.0.0"), Some("outdated"))
    );

    ok(&apps.vibe(&["app", "hold", "-g", NAME]));
    let report = json(&apps.app(&["app", "outdated", "-g"], "--json", &[]));
    assert_eq!(report["installations"][0]["status"], "held");
    assert_eq!(report["installations"][0]["held"], true);
    let registry = apps.registry();
    let bulk = ok(&apps.vibe(&[
        "update",
        "-g",
        "--all",
        "--registry",
        registry.to_str().unwrap(),
    ]));
    assert!(
        bulk.contains("held and skipped: org.example/editor"),
        "{bulk}"
    );
    assert!(
        apps.listed(V2).is_null(),
        "a bulk update skips a held installation"
    );
    ok(&apps.vibe(&["app", "unhold", "-g", NAME]));

    ok(&apps.app(&["update", "-g"], NAME, &[]));
    assert_eq!(edit(&apps), "EDITOR2");
    let previous = apps.listed(V1);
    assert_eq!(previous["hidden"], true);
    assert_eq!(previous["superseded_by"], V2);
    assert_eq!(apps.listed(V2)["default"], true);
    assert_eq!(apps.index()["installations"][V2]["replaced"], V1);
    let runs = apps
        .settings()
        .join("cache/app-runs/org.example.editor@2.0.0");
    let run: Vec<String> = fs::read_dir(&runs)
        .unwrap()
        .map(|entry| fs::read_to_string(entry.unwrap().path().join("run.json")).unwrap())
        .collect();
    assert!(
        run.iter()
            .any(|run| run.contains(&format!("\"replaces\": \"{V1}\""))),
        "{run:?}"
    );
    let outdated = json(&apps.app(&["app", "outdated", "-g"], "--json", &[]));
    assert_eq!(
        outdated["installations"].as_array().unwrap().len(),
        1,
        "kept versions are left out"
    );

    let rolled = json(&apps.vibe(&["app", "rollback", "-g", NAME, "--json"]));
    assert_eq!(rolled["command"], "rollback");
    assert_eq!(rolled["slotDefault"], V1);
    assert_eq!(edit(&apps), "EDITOR1");
    assert_eq!(apps.listed(V2)["hidden"], true);
    assert_eq!(apps.listed(V2)["superseded_by"], V1);
    assert_ne!(apps.listed(V1)["hidden"], true);

    let report = json(&apps.vibe(&["app", "autoremove", "-g", "--json"]));
    assert_eq!(report["installations"][0]["key"], V2);
    assert_eq!(report["installations"][0]["reason"], "previous");
    assert!(apps.listed(V2).is_null());
    assert_eq!(edit(&apps), "EDITOR1");
}

#[test]
fn purge_previous_removes_the_replaced_version_at_once() {
    let apps = fixture();
    ok(&apps.app(
        &["install", "-g"],
        "org.example/editor@=1.0.0",
        &["--installer", "main"],
    ));
    let message = ok(&apps.app(&["update", "-g"], NAME, &["--purge-previous"]));
    assert!(
        message.contains(&format!("removed the previous {V1}")),
        "{message}"
    );
    assert!(apps.listed(V1).is_null());
    assert!(
        !apps
            .settings()
            .join("opt/apps/editor/org.example.editor@1.0.0")
            .exists()
    );
    assert_eq!(edit(&apps), "EDITOR2");
    let error =
        String::from_utf8_lossy(&apps.vibe(&["app", "rollback", "-g", NAME]).stderr).into_owned();
    assert!(error.contains("no kept previous installation"), "{error}");
}

/// Inside a project directory a verb acts on the project home; `-g` reaches
/// the user home, and a miss names the project home and points at `-g`
/// (B-225, PROP-059 `#project-scope`).
#[test]
fn inside_a_project_g_reaches_the_user_home_and_a_miss_names_the_home() {
    let apps = fixture();
    ok(&apps.app(
        &["install", "-g"],
        "org.example/editor@=1.0.0",
        &["--installer", "main"],
    ));
    let project = apps.root.path().join("project");
    fs::create_dir_all(&project).unwrap();
    fs::write(project.join("vibe.toml"), "[project]\nname = \"demo\"\n").unwrap();

    ok(&apps.vibe_in(&project, &["app", "hold", "-g", NAME]));
    assert_eq!(apps.listed(V1)["held"], true);
    ok(&apps.vibe_in(&project, &["app", "unhold", "--global", NAME]));
    assert_ne!(apps.listed(V1)["held"], true);

    let error = failed(&apps.vibe_in(&project, &["app", "hold", NAME]));
    assert!(error.contains("not_installed"), "{error}");
    assert!(
        error.contains("is not installed in the project home "),
        "{error}"
    );
    assert!(error.contains("vibeopt"), "{error}");
    assert!(error.contains("use -g for the user home"), "{error}");
    assert_ne!(apps.listed(V1)["held"], true, "nothing changed");

    let error = failed(&apps.vibe(&["app", "hold", "-g", "org.example/absent"]));
    assert!(
        error.contains("is not installed in the user home"),
        "{error}"
    );
    assert!(!error.contains("use -g"), "{error}");
}
