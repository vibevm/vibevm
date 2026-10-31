//! Public Linux fixture for the `env` step kind (PROP-065 `#env-step`): an
//! installation sets, unsets and prepends to PATH through vibe's managed
//! `env.sh` / `env.fish` (one line per entry tagged with the installation
//! key) and one managed block in the user's startup files; the applied
//! entries are recorded and reported by `app list --json`, and uninstall
//! removes exactly them and the block, leaving every other byte in place.

#![cfg(target_os = "linux")]

mod common;

use std::fs;

use common::app_registry::{AppRegistry, json, ok};
use serde_json::json as value;

const KEY: &str = "org.example/tool@1.0.0";
const BLOCK: &str = "# >>> vibe applications environment (managed by vibe, do not edit) >>>";

fn fixture() -> AppRegistry {
    let apps = AppRegistry::new();
    let package = apps.package(
        "tool",
        "1.0.0",
        "tool",
        r#"
[[installer_options.steps]]
id = "place"
title = "Place the tool"
kind = "copy"
from = "payload"

[[installer_options.steps]]
id = "register"
title = "Register the tool in the user environment"
kind = "env"
set = { TOOL_HOME = "$app_root" }
unset = ["TOOL_DEBUG"]
path_prepend = ["$app_bin"]

[[installer_options.launchers]]
id = "main"
name = "Tool"
executable = "$app_bin/tool"
"#,
    );
    apps.script(&package, "tool", "TOOL");
    apps
}

#[test]
fn an_env_step_writes_managed_lines_and_blocks_and_uninstall_removes_exactly_them() {
    let apps = fixture();
    let bashrc = apps.home().join(".bashrc");
    fs::write(&bashrc, "alias ll='ls -l'\n").unwrap();

    let plan = json(&apps.app(&["app", "plan"], "org.example/tool", &["--json"]));
    assert_eq!(plan["steps"][1]["kind"], "env");
    assert_eq!(plan["steps"][1]["phase"], "finish", "always a finish step");

    ok(&apps.app(&["install", "-g"], "org.example/tool", &[]));
    let root = apps.settings().join("opt/apps/tool/org.example.tool@1.0.0");
    let script = fs::read_to_string(apps.settings().join("env.sh")).unwrap();
    assert!(
        script.contains(&format!(
            "export TOOL_HOME='{}' # vibe-app {KEY} register set TOOL_HOME",
            root.display()
        )),
        "{script}"
    );
    assert!(
        script.contains(&format!(
            "unset TOOL_DEBUG # vibe-app {KEY} register unset TOOL_DEBUG"
        )),
        "{script}"
    );
    assert!(
        script.contains(&format!(
            "export PATH='{}'\"${{PATH:+:$PATH}}\"",
            root.join("bin").display()
        )),
        "{script}"
    );
    assert!(apps.settings().join("env.fish").is_file());
    let profile = fs::read_to_string(apps.home().join(".profile")).unwrap();
    assert!(profile.starts_with(BLOCK), "{profile}");
    assert!(profile.contains(&apps.settings().join("env.sh").display().to_string()));
    let rc = fs::read_to_string(&bashrc).unwrap();
    assert!(rc.starts_with("alias ll='ls -l'\n# >>> vibe"), "{rc}");
    assert_eq!(rc.matches(BLOCK).count(), 1);
    assert!(
        !apps.home().join(".zshrc").exists(),
        "absent startup files stay absent"
    );

    // The script sourced by a POSIX shell applies the entries.
    let shell = std::process::Command::new("/bin/sh")
        .arg("-c")
        .arg(format!(
            ". '{}'; printf '%s|%s|%s' \"$TOOL_HOME\" \"${{TOOL_DEBUG-unset}}\" \"$(command -v tool)\"",
            apps.settings().join("env.sh").display()
        ))
        .env("TOOL_DEBUG", "1")
        .output()
        .unwrap();
    assert_eq!(
        String::from_utf8_lossy(&shell.stdout),
        format!(
            "{}|unset|{}",
            root.display(),
            root.join("bin/tool").display()
        )
    );

    let row = apps.listed(KEY);
    assert_eq!(
        row["env"],
        value!([
            { "step": "register", "op": "set", "name": "TOOL_HOME", "value": root.display().to_string() },
            { "step": "register", "op": "unset", "name": "TOOL_DEBUG" },
            { "step": "register", "op": "path_prepend", "name": "PATH", "value": root.join("bin").display().to_string() }
        ])
    );
    assert_eq!(
        apps.index()["installations"][KEY]["env"][0]["name"],
        "TOOL_HOME"
    );

    // A reinstall does not duplicate a line.
    ok(&apps.app(&["install", "-g"], "org.example/tool", &[]));
    let script = fs::read_to_string(apps.settings().join("env.sh")).unwrap();
    assert_eq!(
        script
            .lines()
            .filter(|line| line.starts_with("export TOOL_HOME="))
            .count(),
        1,
        "{script}"
    );

    let plan = json(&apps.vibe(&[
        "uninstall",
        "-g",
        "org.example/tool",
        "--plan-only",
        "--json",
    ]));
    let actions: Vec<(&str, &str)> = plan["installations"][0]["actions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|action| {
            (
                action["kind"].as_str().unwrap(),
                action["step"].as_str().unwrap_or(""),
            )
        })
        .collect();
    assert_eq!(
        actions,
        [("env", "register"), ("payload", ""), ("record", "")],
        "the env rollback comes first"
    );

    ok(&apps.vibe(&["uninstall", "-g", "org.example/tool"]));
    assert!(!apps.settings().join("env.sh").exists());
    assert!(!apps.settings().join("env.fish").exists());
    assert_eq!(fs::read_to_string(&bashrc).unwrap(), "alias ll='ls -l'\n");
    assert_eq!(
        fs::read_to_string(apps.home().join(".profile")).unwrap(),
        ""
    );
    assert!(!root.exists());
}
