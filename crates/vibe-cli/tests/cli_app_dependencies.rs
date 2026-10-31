//! Public Linux fixture for requirements between application packages and
//! the removal of a tree (PROP-065 `#app-dependencies`, `#uninstall-tree`):
//! a "toolchain" package whose public command a second package's recipe
//! step needs. The plan shows the requirement ahead of the step; install
//! puts the toolchain in hidden (build scope), exposes its command on the
//! step's PATH only and records the requirement both ways; uninstall lists
//! the orphan and autoremove takes it; a runtime requirement is installed
//! exposed and blocks the toolchain's removal unless `--cascade`, whose
//! `--plan-only` shows dependents first; a manual mark survives autoremove.

#![cfg(target_os = "linux")]

mod common;

use std::path::PathBuf;

use common::app_registry::{AppRegistry, failed, json, ok};
use common::fake_agent::{Record, script};
use serde_json::json as value;

const TOOLCHAIN: &str = "org.example/toolchain@1.0.0";
const CONSUMER: &str = "org.example/consumer@1.0.0";
const RUNNER: &str = "org.example/runner@1.0.0";

/// The toolchain (`vibe-test-tcc`); `extra` precedes its steps (a license).
fn add_toolchain(apps: &AppRegistry, extra: &str) -> PathBuf {
    let toolchain = apps.package(
        "toolchain",
        "1.0.0",
        "toolchain",
        &format!(
            r#"{extra}
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
"#
        ),
    );
    apps.script(&toolchain, "vibe-test-tcc", "TCC_RAN");
    toolchain
}

/// A consumer whose build step requires the toolchain (build scope).
fn add_consumer(apps: &AppRegistry) {
    let consumer = apps.package(
        "consumer",
        "1.0.0",
        "consumer",
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
name = "Consumer"
executable = "$app_bin/consumer"
kind = "command"
"#,
    );
    std::fs::write(
        consumer.join("build.sh"),
        "set -e\nmkdir -p \"$VIBE_OUT/bin\"\nvibe-test-tcc compiled > \"$VIBE_OUT/built.txt\"\n\
         printf '%s\\n' \"$VIBE_APP_USE\" > \"$VIBE_OUT/use.txt\"\n\
         printf '#!/bin/sh\\necho CONSUMER\\n' > \"$VIBE_OUT/bin/consumer\"\nchmod 755 \"$VIBE_OUT/bin/consumer\"\n",
    )
    .unwrap();
}

/// The toolchain, a consumer whose build step requires it (build scope),
/// and a runner whose option requires it (runtime scope).
fn fixture() -> AppRegistry {
    let apps = AppRegistry::new();
    add_toolchain(&apps, "");
    add_consumer(&apps);
    let runner = apps.package(
        "runner",
        "1.0.0",
        "runner",
        r#"
requires = [{ package = "org.example/toolchain@1", scope = "runtime" }]

[[installer_options.steps]]
id = "place"
title = "Place the runner"
kind = "copy"
from = "payload"
"#,
    );
    apps.script(&runner, "runner", "RUNNER");
    apps
}

#[test]
fn a_build_requirement_is_installed_hidden_exposed_to_its_step_and_autoremoved() {
    let apps = fixture();

    let plan = json(&apps.app(&["app", "plan"], "org.example/consumer", &["--json"]));
    assert_eq!(
        plan["dependencies"],
        value!([{
            "package": "org.example/toolchain@1",
            "key": TOOLCHAIN,
            "scope": "build",
            "step": "build",
            "action": "install",
            "hidden": true,
            "commands": ["vibe-test-tcc"]
        }]),
        "the requirement is resolved and met the step's command need"
    );
    let text = ok(&apps.app(&["app", "plan"], "org.example/consumer", &[]));
    let dependency = text
        .find("dependency: install org.example/toolchain@1.0.0 (build, hidden)")
        .unwrap_or_else(|| panic!("{text}"));
    assert!(dependency < text.find("1. run build").unwrap(), "{text}");

    ok(&apps.app(&["install", "-g"], "org.example/consumer", &[]));
    let payload = apps
        .settings()
        .join("opt/apps/consumer/org.example.consumer@1.0.0");
    assert_eq!(
        std::fs::read_to_string(payload.join("built.txt"))
            .unwrap()
            .trim(),
        "TCC_RAN compiled",
        "the toolchain's public command was on the step's PATH"
    );
    assert_eq!(
        std::fs::read_to_string(payload.join("use.txt"))
            .unwrap()
            .trim(),
        format!("toolchain={TOOLCHAIN}")
    );
    assert!(
        !apps.settings().join("opt/bin/vibe-test-tcc").exists(),
        "a build requirement leaves nothing on the user's PATH"
    );
    let toolchain = apps.listed(TOOLCHAIN);
    assert_eq!(toolchain["hidden"], true);
    assert_eq!(toolchain["mark"], "auto");
    assert_eq!(toolchain["default"], false);
    assert_eq!(toolchain["required_by"], value!([CONSUMER]));
    let consumer = apps.listed(CONSUMER);
    assert_eq!(consumer["requires"][0]["key"], TOOLCHAIN);
    assert_eq!(consumer["requires"][0]["scope"], "build");
    assert_eq!(consumer["requires"][0]["step"], "build");
    assert!(consumer.get("mark").is_none(), "named by the user: manual");
    let list = ok(&apps.vibe(&["app", "list", "-g"]));
    let section = list.find("hidden installation(s):").unwrap();
    assert!(
        list[section..].contains(&format!(
            "{TOOLCHAIN} (steps, ready) [auto] required by {CONSUMER}"
        )),
        "{list}"
    );

    let removed = ok(&apps.vibe(&["uninstall", "-g", "org.example/consumer"]));
    assert!(
        removed.contains(&format!("orphans nothing requires any more: {TOOLCHAIN}")),
        "{removed}"
    );
    assert_eq!(
        apps.listed(TOOLCHAIN)["hidden"],
        true,
        "never removed silently"
    );
    let plan = json(&apps.vibe(&["app", "autoremove", "-g", "--plan-only", "--json"]));
    assert_eq!(plan["planOnly"], true);
    assert_eq!(plan["installations"][0]["key"], TOOLCHAIN);
    assert_eq!(plan["installations"][0]["reason"], "orphan");
    let kinds: Vec<&str> = plan["installations"][0]["actions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|action| action["kind"].as_str().unwrap())
        .collect();
    assert_eq!(
        kinds,
        ["payload", "record"],
        "hidden: no proxies to release"
    );
    let report = json(&apps.vibe(&["app", "autoremove", "-g", "--json"]));
    assert_eq!(report["protocol"], "vibe-app-removal/1");
    assert_eq!(report["installations"][0]["key"], TOOLCHAIN);
    assert!(apps.listed(TOOLCHAIN).is_null());
    let runs = apps
        .settings()
        .join("cache/app-runs/org.example.toolchain@1.0.0");
    let records: Vec<String> = std::fs::read_dir(&runs)
        .unwrap()
        .map(|entry| std::fs::read_to_string(entry.unwrap().path().join("run.json")).unwrap())
        .collect();
    assert!(
        records.iter().any(|run| run.contains("\"uninstall\"")),
        "a removal writes a run record: {records:?}"
    );

    // A manual mark survives autoremove.
    ok(&apps.app(&["install", "-g"], "org.example/consumer", &[]));
    ok(&apps.vibe(&["app", "mark", "-g", "manual", "org.example/toolchain"]));
    let removed = ok(&apps.vibe(&["uninstall", "-g", "org.example/consumer"]));
    assert!(!removed.contains("orphans"), "{removed}");
    let report = json(&apps.vibe(&["app", "autoremove", "-g", "--json"]));
    assert_eq!(report["installations"], value!([]));
    assert!(!apps.listed(TOOLCHAIN).is_null());
}

#[test]
fn a_runtime_requirement_is_exposed_and_blocks_its_removal_unless_cascaded() {
    let apps = fixture();
    ok(&apps.app(&["install", "-g"], "org.example/runner", &[]));
    let toolchain = apps.listed(TOOLCHAIN);
    assert_ne!(
        toolchain["hidden"], true,
        "runtime scope is installed exposed"
    );
    assert_eq!(toolchain["mark"], "auto");
    assert_eq!(toolchain["default"], true);
    assert!(apps.settings().join("opt/bin/vibe-test-tcc").is_file());
    assert_eq!(apps.listed(RUNNER)["requires"][0]["scope"], "runtime");

    let error = failed(&apps.vibe(&["uninstall", "-g", "org.example/toolchain", "--json"]));
    assert!(error.contains("dependents_block"), "{error}");
    assert!(error.contains(RUNNER), "{error}");
    assert!(!apps.listed(TOOLCHAIN).is_null(), "nothing was removed");

    let plan = json(&apps.vibe(&[
        "uninstall",
        "-g",
        "org.example/toolchain",
        "--cascade",
        "--plan-only",
        "--json",
    ]));
    let order: Vec<(&str, &str)> = plan["installations"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| {
            (
                row["key"].as_str().unwrap(),
                row["reason"].as_str().unwrap(),
            )
        })
        .collect();
    assert_eq!(order, [(RUNNER, "dependent"), (TOOLCHAIN, "requested")]);
    let kinds: Vec<&str> = plan["installations"][1]["actions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|action| action["kind"].as_str().unwrap())
        .collect();
    assert_eq!(kinds, ["proxies", "payload", "record"]);
    assert!(!apps.listed(RUNNER).is_null(), "a plan changes nothing");

    let removed = ok(&apps.vibe(&["uninstall", "-g", "org.example/toolchain", "--cascade"]));
    assert!(
        removed.contains(&format!("also removed {RUNNER}")),
        "{removed}"
    );
    assert!(apps.listed(RUNNER).is_null());
    assert!(apps.listed(TOOLCHAIN).is_null());
    assert!(!apps.settings().join("opt/bin/vibe-test-tcc").exists());
}

#[test]
fn a_licensed_requirement_is_accepted_like_the_package_that_requires_it() {
    let apps = AppRegistry::new();
    let toolchain = add_toolchain(
        &apps,
        "[installer_options.license]\nfile = \"LICENSE.txt\"\nname = \"Toolchain License 1.0\"\n",
    );
    std::fs::write(toolchain.join("LICENSE.txt"), "Compile kindly.\n").unwrap();
    add_consumer(&apps);

    // `--assume-yes` alone accepts no license: the plan of the requiring
    // package refuses before anything is fetched or run.
    for (verb, extra) in [
        (&["install", "-g"][..], &["--assume-yes", "--json"][..]),
        (&["app", "plan"][..], &["--json"][..]),
    ] {
        let output = apps.app(verb, "org.example/consumer", extra);
        let error = failed(&output);
        assert_eq!(output.status.code(), Some(2), "{error}");
        assert!(error.contains("\"code\":\"license_required\""), "{error}");
        assert!(
            error.contains(TOOLCHAIN),
            "the requirement is named: {error}"
        );
    }
    assert!(!apps.settings().join("applications/index.json").exists());
    assert!(
        !apps
            .settings()
            .join("cache/app-work/org.example.consumer@1.0.0")
            .exists(),
        "nothing ran"
    );

    // `--accept-license` accepts it exactly as it accepts the requirer's.
    ok(&apps.app(
        &["install", "-g"],
        "org.example/consumer",
        &["--accept-license"],
    ));
    let index = apps.index();
    assert_eq!(
        index["installations"][TOOLCHAIN]["license"]["name"],
        "Toolchain License 1.0"
    );
    assert!(!apps.listed(CONSUMER).is_null());
}

#[test]
fn an_agent_step_sees_the_requirement_path_a_recipe_step_sees() {
    let apps = AppRegistry::new();
    add_toolchain(&apps, "");
    let agented = apps.package(
        "agented",
        "1.0.0",
        "agented",
        r#"
requires = [{ package = "org.example/toolchain@1", scope = "build" }]

[[installer_options.steps]]
id = "build"
title = "Build and record the PATH"
kind = "recipe"
interpreter = "posix_sh"
script = "build.sh"

[[installer_options.steps]]
id = "fix"
title = "Let an agent adapt the build"
kind = "agent"
prompt = "Adapt the build in $out."
"#,
    );
    std::fs::write(
        agented.join("build.sh"),
        "set -e\nmkdir -p \"$VIBE_OUT/bin\"\nprintf '%s' \"$PATH\" > \"$VIBE_OUT/path.txt\"\n",
    )
    .unwrap();
    let record = Record::new(&apps.root.path().join("agent-record"));
    let registry = apps.registry();
    let mut command = apps.user.vibe();
    command
        .current_dir(apps.root.path())
        .env("HOME", apps.home())
        .env("VIBE_AGENT_COMMAND", "sh")
        .env(
            "VIBE_AGENT_ARGS",
            serde_json::to_string(&[script().display().to_string()]).unwrap(),
        )
        .env("FAKE_AGENT_RECORD", &record.dir)
        .env_remove("VIBE_INVOKED_BY")
        .env_remove("VIBE_APP_USE")
        .args([
            "install",
            "-g",
            "org.example/agented",
            "--registry",
            registry.to_str().unwrap(),
            "--assume-yes",
        ]);
    ok(&command.output().unwrap());
    assert_eq!(record.calls(), 1);

    let recipe_path = std::fs::read_to_string(
        apps.settings()
            .join("opt/apps/agented/org.example.agented@1.0.0/path.txt"),
    )
    .unwrap();
    let agent_path = record.env("PATH");
    assert_eq!(
        agent_path, recipe_path,
        "one PATH for every step of the option"
    );
    let entries: Vec<&str> = agent_path.split(':').collect();
    let proxies = apps
        .settings()
        .join("cache/app-proxies/org.example.toolchain@1.0.0/bin");
    let bin = apps
        .settings()
        .join("opt/apps/toolchain/org.example.toolchain@1.0.0/bin");
    assert_eq!(
        entries[..2],
        [proxies.to_str().unwrap(), bin.to_str().unwrap()],
        "the requirement comes first: {agent_path}"
    );
    assert_eq!(record.env("VIBE_APP_USE"), format!("toolchain={TOOLCHAIN}"));
}
