//! Old3/old1 have unknown historical provision; reads never invent a receipt.
#![cfg(all(test, target_os = "linux"))]
mod common;
use common::app_registry::{AppRegistry, json, ok};
use common::fake_agent::refused;
use serde_json::Value;
use std::{fs, path::Path};
const KEY: &str = "org.example/history@1.0.0";
const NAME: &str = "org.example/history";
fn package(r: &AppRegistry, park: bool) {
    let tail = if park {
        "[[installer_options.steps]]\nid='finish'\ntitle='Finish'\nkind='agent'\nphase='finish'\nrequired=true\nprompt='Complete the bounded historical fixture.'\n"
    } else {
        ""
    };
    let p = r.package(
        "history",
        "1.0.0",
        "history",
        &format!(
            r#"
result='owned'
[[installer_options.parameters]]
id='token'
name='Token'
type='string'
secret=true
default='public-default'
[[installer_options.steps]]
id='build'
title='Build'
kind='recipe'
script='build.sh'
{tail}
"#
        ),
    );
    fs::write(p.join("build.sh"),format!("set -e\necho build >> '{}'\nmkdir -p \"$VIBE_OUT\"\necho done > \"$VIBE_OUT/receipt\"\n",r.root.path().join("runs").display())).unwrap();
}
fn historical_index(r: &AppRegistry) -> Vec<u8> {
    let path = r.settings().join("applications/index.json");
    let mut value = r.index();
    value["protocol"] = "vibe-user-applications/3".into();
    for row in value["installations"].as_object_mut().unwrap().values_mut() {
        let row = row.as_object_mut().unwrap();
        row.remove("pending");
        row.remove("secretOrigins");
        row.remove("source_evidence");
    }
    let bytes = serde_json::to_vec_pretty(&value).unwrap();
    fs::write(path, &bytes).unwrap();
    bytes
}
#[test]
fn old3_read_is_unchanged_and_new_full_operation_records_only_its_actual_default() {
    let r = AppRegistry::new();
    package(&r, false);
    ok(&r.app(
        &["install", "-g"],
        NAME,
        &[
            "--set",
            "token=HISTORICAL_EXPLICIT_286",
            "--json",
            "--offline",
            "--unattended",
        ],
    ));
    assert_eq!(
        r.index()["installations"][KEY]["secretOrigins"]["token"]["kind"],
        "explicit"
    );
    let old = historical_index(&r);
    ok(&r.vibe(&["app", "list", "-g", "--json"]));
    ok(&r.vibe(&[
        "app",
        "describe",
        NAME,
        "-g",
        "--registry",
        r.registry().to_str().unwrap(),
        "--json",
        "--offline",
        "--unattended",
    ]));
    assert_eq!(
        fs::read(r.settings().join("applications/index.json")).unwrap(),
        old
    );
    // This genuinely fresh repair binds the current default; it does not
    // assert that the earlier installation also used that default.
    ok(&r.app(
        &["install", "-g"],
        NAME,
        &["--force", "--json", "--offline", "--unattended"],
    ));
    assert_eq!(
        r.index()["installations"][KEY]["secretOrigins"]["token"]["kind"],
        "default"
    );
    assert_eq!(
        fs::read_to_string(r.root.path().join("runs"))
            .unwrap()
            .lines()
            .count(),
        2
    );
}
#[test]
fn old1_kept_prefix_does_not_become_a_whole_installation_default_receipt() {
    let r = AppRegistry::new();
    package(&r, true);
    let output = r
        .user
        .vibe()
        .current_dir(r.root.path())
        .env("HOME", r.home())
        .env("VIBE_AGENT_COMMAND", "sh")
        .env("VIBE_AGENT_ARGS", "[]")
        .env_remove("VIBE_INVOKED_BY")
        .args([
            "install",
            "-g",
            NAME,
            "--registry",
            r.registry().to_str().unwrap(),
            "--agent-mode",
            "agent",
            "--json",
            "--offline",
            "--unattended",
        ])
        .output()
        .unwrap();
    let (status, error) = refused(&output);
    assert_eq!(status, Some(75));
    let entries = json(&r.vibe(&["app", "work", "-g", "--json"]));
    let row = &entries["entries"][0];
    let checkpoint = Path::new(row["path"].as_str().unwrap()).join("steps.json");
    let mut old: Value = serde_json::from_slice(&fs::read(&checkpoint).unwrap()).unwrap();
    old["protocol"] = "vibe-app-steps/1".into();
    old.as_object_mut().unwrap().remove("operation");
    old.as_object_mut().unwrap().remove("source_evidence");
    // Recreate the actual protocol1 envelope layout, not a current file with a historical protocol label.
    let legacy = checkpoint.parent().unwrap().join("steps/finish");
    let generated = Path::new(error["run"].as_str().unwrap());
    fs::copy(generated.join("prompt.md"), legacy.join("prompt.md")).unwrap();
    let mut context: vibe_wire::generated::application::e1::agent_hook::AgentHook =
        serde_json::from_slice(&fs::read(generated.join("context.json")).unwrap()).unwrap();
    context.paths.run = legacy.to_str().unwrap().into();
    context.paths.prompt_file = legacy.join("prompt.md").to_str().unwrap().into();
    context.paths.context_file = legacy.join("context.json").to_str().unwrap().into();
    context.paths.result_file = legacy.join("result.json").to_str().unwrap().into();
    fs::write(
        legacy.join("context.json"),
        format!("{}\n", serde_json::to_string_pretty(&context).unwrap()),
    )
    .unwrap();
    for step in old["steps"].as_array_mut().unwrap() {
        if step["id"] == "finish" {
            step["agent"]["envelope"] = legacy.to_str().unwrap().into();
        }
    }
    let bytes = serde_json::to_vec_pretty(&old).unwrap();
    fs::write(&checkpoint, &bytes).unwrap();
    historical_index(&r);
    ok(&r.vibe(&["app", "work", "-g", "--json"]));
    assert_eq!(fs::read(&checkpoint).unwrap(), bytes);
    fs::write(
        legacy.join("result.json"),
        r#"{"protocol":"vibe-agent-hook-result/1","status":"ok","message":"done"}"#,
    )
    .unwrap();
    let output = r
        .user
        .vibe()
        .current_dir(r.root.path())
        .env("HOME", r.home())
        .env("VIBE_AGENT_COMMAND", "sh")
        .env("VIBE_AGENT_ARGS", "[]")
        .env_remove("VIBE_INVOKED_BY")
        .args([
            "install",
            "-g",
            NAME,
            "--registry",
            r.registry().to_str().unwrap(),
            "--resume",
            "--agent-mode",
            "agent",
            "--json",
            "--offline",
            "--unattended",
        ])
        .output()
        .unwrap();
    ok(&output);
    assert_eq!(
        fs::read_to_string(r.root.path().join("runs"))
            .unwrap()
            .lines()
            .count(),
        1
    );
    assert_eq!(
        r.index()["installations"][KEY]["secretOrigins"]["token"]["kind"],
        "legacy-effective"
    );
}
