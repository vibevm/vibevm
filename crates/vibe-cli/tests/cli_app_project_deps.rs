//! Public Linux fixture for requirements between application packages in a
//! project (PROP-065 `#app-dependencies`, `#uninstall-tree`; PROP-059
//! `#project-scope`): a project depends on the fixture "app" whose build
//! step requires the fixture "toolchain". The lockfile records the
//! toolchain as a transitive row naming its requirer and scope, a second
//! checkout materialises both offline from the lockfile alone, and
//! deleting the dependency row removes the app and its orphaned toolchain
//! from the project home.

#![cfg(target_os = "linux")]

mod common;

use std::fs;
use std::path::Path;
use std::process::Output;

use common::app_registry::{AppRegistry, ok};
use serde_json::Value;

const APP: &str = "org.example/app@1.0.0";
const TOOLCHAIN: &str = "org.example/toolchain@1.0.0";

/// The toolchain (`vibe-test-tcc`) and an app whose build step requires it.
fn fixture() -> AppRegistry {
    let apps = AppRegistry::new();
    let toolchain = apps.package(
        "toolchain",
        "1.0.0",
        "toolchain",
        r#"
[[installer_options.steps]]
id = "place"
title = "Place the toolchain"
kind = "copy"
from = "payload"

[[installer_options.launchers]]
id = "tcc"
name = "Test compiler"
executable = "$app_bin/vibe-test-tcc"
kind = "command"

[[installer_options.public_commands]]
name = "vibe-test-tcc"
launcher = "tcc"
"#,
    );
    apps.script(&toolchain, "vibe-test-tcc", "TCC_RAN");
    let app = apps.package(
        "app",
        "1.0.0",
        "app",
        r#"
[[installer_options.steps]]
id = "build"
title = "Build with the toolchain"
kind = "recipe"
interpreter = "posix_sh"
script = "build.sh"
requires = ["org.example/toolchain@1"]
needs = ["command:vibe-test-tcc"]

[[installer_options.launchers]]
id = "main"
name = "App"
executable = "$app_bin/app"
kind = "command"
"#,
    );
    fs::write(
        app.join("build.sh"),
        "set -e\nmkdir -p \"$VIBE_OUT/bin\"\nvibe-test-tcc compiled > \"$VIBE_OUT/built.txt\"\n\
         printf '#!/bin/sh\\necho APP\\n' > \"$VIBE_OUT/bin/app\"\nchmod 755 \"$VIBE_OUT/bin/app\"\n",
    )
    .unwrap();
    apps
}

fn vibe_in(apps: &AppRegistry, dir: &Path, args: &[&str]) -> Output {
    let mut command = apps.user.vibe();
    command
        .current_dir(dir)
        .env("HOME", apps.home())
        .env("VIBE_NO_DEFAULT_REGISTRY", "1")
        .args(args);
    for variable in [
        "VIBE_UNATTENDED",
        "VIBE_AGENT_COMMAND",
        "VIBE_AGENT_ARGS",
        "VIBE_INVOKED_BY",
        "VIBE_APP_USE",
    ] {
        command.env_remove(variable);
    }
    command.output().unwrap()
}

/// The `[[package]]` row of `name` in the project's `vibe.lock`.
fn lock_row(project: &Path, name: &str) -> toml::Value {
    let lock: toml::Value =
        toml::from_str(&fs::read_to_string(project.join("vibe.lock")).unwrap()).unwrap();
    lock["package"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["name"].as_str() == Some(name))
        .cloned()
        .unwrap_or_else(|| panic!("no `{name}` row in {lock}"))
}

/// The project home's index records by key.
fn home_index(project: &Path) -> Value {
    let path = project.join("vibevm/vibeopt/applications/index.json");
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}

#[test]
fn a_required_application_is_locked_materialised_offline_and_removed_with_its_requirer() {
    let apps = fixture();
    let registry = apps.registry();
    let registry = registry.to_str().unwrap();
    let project = apps.root.path().join("project");
    apps.user.init_project(&project);
    let project = project.canonicalize().unwrap();

    ok(&vibe_in(
        &apps,
        &project,
        &[
            "install",
            "org.example/app",
            "--registry",
            registry,
            "--assume-yes",
        ],
    ));

    // The toolchain is a transitive lock row naming its requirer and scope.
    let app_row = lock_row(&project, "app");
    let dependencies: Vec<&str> = app_row["dependencies"]
        .as_array()
        .unwrap()
        .iter()
        .map(|dependency| dependency.as_str().unwrap())
        .collect();
    assert_eq!(dependencies, ["org.example/toolchain@=1.0.0"]);
    let toolchain_row = lock_row(&project, "toolchain");
    assert_eq!(toolchain_row["kind"].as_str(), Some("app"));
    let required_by = toolchain_row["required_by"].as_array().unwrap();
    assert_eq!(required_by.len(), 1, "{toolchain_row}");
    assert_eq!(
        required_by[0]["package"].as_str(),
        Some("app:org.example/app@=1.0.0")
    );
    assert_eq!(required_by[0]["scope"].as_str(), Some("build"));
    assert!(app_row.get("required_by").is_none());
    let manifest = fs::read_to_string(project.join("vibe.toml")).unwrap();
    assert!(
        !manifest.contains("org.example/toolchain"),
        "a requirement is not a direct dependency: {manifest}"
    );

    // The project home holds the app and its hidden, auto-marked toolchain.
    let index = home_index(&project);
    let toolchain = &index["installations"][TOOLCHAIN];
    assert_eq!(toolchain["hidden"], true, "{index}");
    assert_eq!(toolchain["mark"], "auto", "{index}");
    let built = project.join("vibevm/vibeopt/opt/apps/app/org.example.app@1.0.0/built.txt");
    assert_eq!(
        fs::read_to_string(&built).unwrap().trim(),
        "TCC_RAN compiled"
    );
    assert!(
        !project
            .join("vibevm/vibeopt/opt/bin/vibe-test-tcc")
            .exists()
    );

    // A second checkout materialises both from the lockfile alone: offline,
    // with no registry at all.
    let second = apps.root.path().join("second");
    fs::create_dir_all(&second).unwrap();
    for file in ["vibe.toml", "vibe.lock"] {
        fs::copy(project.join(file), second.join(file)).unwrap();
    }
    copy_tree(
        &project.join("vibevm/vibedeps"),
        &second.join("vibevm/vibedeps"),
    );
    let reinstalled = ok(&vibe_in(
        &apps,
        &second,
        &["install", "--offline", "--assume-yes"],
    ));
    assert!(
        reinstalled.contains(&format!("installed {APP}")),
        "{reinstalled}"
    );
    let index = home_index(&second);
    assert_eq!(index["installations"][TOOLCHAIN]["mark"], "auto");
    assert!(
        second
            .join("vibevm/vibeopt/opt/apps/app/org.example.app@1.0.0/built.txt")
            .is_file()
    );

    // Deleting the dependency row removes the app and the orphaned
    // toolchain from the project home.
    let removed = ok(&vibe_in(
        &apps,
        &project,
        &["uninstall", "org.example/app", "--assume-yes"],
    ));
    assert!(removed.contains(&format!("removed {APP}")), "{removed}");
    assert!(
        removed.contains(&format!("removed {TOOLCHAIN}")),
        "{removed}"
    );
    let lock = fs::read_to_string(project.join("vibe.lock")).unwrap_or_default();
    assert!(!lock.contains("name = \"toolchain\""), "{lock}");
    assert!(!lock.contains("name = \"app\""), "{lock}");
    let index = home_index(&project);
    assert_eq!(
        index["installations"]
            .as_object()
            .map_or(0, serde_json::Map::len),
        0,
        "{index}"
    );
    assert!(
        !project
            .join("vibevm/vibeopt/opt/apps/toolchain/org.example.toolchain@1.0.0")
            .exists()
    );
}

fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), &target).unwrap();
        }
    }
}
