//! Actual parent/child continuation, including an indexed unfinished finish step.
#![cfg(all(test, target_os = "linux"))]
mod common;
use common::app_registry::{AppRegistry, json, ok};
use common::fake_agent::refused;
use serde_json::{Value, json};
use std::{fs, path::Path, process::Output};

fn fixture(r: &AppRegistry) {
    let child = r.package(
        "continued-child",
        "1.0.0",
        "continued-child",
        r#"
result='owned'
[[installer_options.steps]]
id='build'
title='Build child'
kind='recipe'
script='build.sh'
[[installer_options.steps]]
id='finish'
title='Finish child'
kind='agent'
phase='finish'
required=true
prompt='Finish this bounded child task.'
"#,
    );
    fs::write(child.join("build.sh"),format!("set -e\necho child >> '{}'\nmkdir -p \"$VIBE_OUT\"\necho child > \"$VIBE_OUT/receipt\"\n",r.root.path().join("child-count").display())).unwrap();
    let parent = r.package(
        "continued-parent",
        "1.0.0",
        "continued-parent",
        r#"
result='owned'
requires=['org.example/continued-child@=1.0.0']
[[installer_options.steps]]
id='build'
title='Build parent'
kind='recipe'
script='build.sh'
[[installer_options.configuration.parameters]]
id='mode'
name='Mode'
type='string'
default='current-default'
"#,
    );
    fs::write(parent.join("build.sh"),format!("set -e\necho \"$VIBE_PARAM_MODE\" >> '{}'\nmkdir -p \"$VIBE_OUT\"\necho parent > \"$VIBE_OUT/receipt\"\n",r.root.path().join("parent-values").display())).unwrap();
}
fn hosted(r: &AppRegistry, args: &[&str]) -> Output {
    r.user
        .vibe()
        .current_dir(r.root.path())
        .env("HOME", r.home())
        .env("VIBE_AGENT_COMMAND", "sh")
        .env("VIBE_AGENT_ARGS", "[]")
        .env_remove("VIBE_INVOKED_BY")
        .args(args)
        .args([
            "--agent-mode",
            "agent",
            "--json",
            "--unattended",
            "--offline",
        ])
        .output()
        .unwrap()
}
fn row(r: &AppRegistry, key: &str) -> Value {
    let listing = json(&r.vibe(&["app", "work", "-g", "--json"]));
    listing["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["key"] == key)
        .unwrap()
        .clone()
}
#[test]
fn parent_resume_finishes_indexed_child_and_keeps_original_public_input_after_file_removal() {
    let r = AppRegistry::new();
    fixture(&r);
    let input = r.root.path().join("parent-input.json");
    fs::write(&input,serde_json::to_vec(&json!({"protocol":"vibe-configuration-input/1","input":{"intent":{"policy":"require-selection","features":[],"defaultFeatures":false,"selections":{},"parameters":{"mode":{"kind":"text","value":"kept-original"}}},"disabledFeatures":[]}})).unwrap()).unwrap();
    let output = hosted(
        &r,
        &[
            "install",
            "-g",
            "org.example/continued-parent",
            "--registry",
            r.registry().to_str().unwrap(),
            "--configuration-input",
            input.to_str().unwrap(),
        ],
    );
    let (status, parked) = refused(&output);
    assert_eq!(status, Some(75));
    assert_eq!(parked["code"], "delegated");
    let parent = row(&r, "org.example/continued-parent@1.0.0");
    let child = row(&r, "org.example/continued-child@1.0.0");
    assert_eq!(parent["operation"]["phase"], "dependencies");
    assert_eq!(child["operation"]["phase"], "published");
    assert!(
        r.index()["installations"]
            .get("org.example/continued-child@1.0.0")
            .is_some()
    );
    assert!(
        r.index()["installations"]
            .get("org.example/continued-parent@1.0.0")
            .is_none()
    );
    assert!(!r.root.path().join("parent-values").exists());
    fs::remove_file(input).unwrap();
    fs::write(
        Path::new(parked["run"].as_str().unwrap()).join("result.json"),
        r#"{"protocol":"vibe-agent-hook-result/1","status":"ok","message":"done"}"#,
    )
    .unwrap();
    // No replay of hosted flag, registry argument or input filename.
    ok(&r.vibe(&[
        "app",
        "resume",
        "org.example/continued-parent@1.0.0",
        "--operation",
        parent["operation"]["target"]["operation"].as_str().unwrap(),
        "-g",
        "--json",
        "--unattended",
        "--offline",
    ]));
    assert_eq!(
        fs::read_to_string(r.root.path().join("child-count")).unwrap(),
        "child\n"
    );
    assert_eq!(
        fs::read_to_string(r.root.path().join("parent-values")).unwrap(),
        "kept-original\n"
    );
    let index = r.index();
    assert_eq!(
        index["installations"]["org.example/continued-parent@1.0.0"]["configuration"]["evaluation"]
            ["input"]["intent"]["parameters"]["mode"]["value"],
        "kept-original"
    );
    assert!(
        index["installations"]["org.example/continued-child@1.0.0"]
            .get("pending")
            .is_none()
    );
}
#[test]
fn a_new_parent_does_not_adopt_or_resume_an_unrelated_pending_child() {
    let r = AppRegistry::new();
    fixture(&r);
    let output = hosted(
        &r,
        &[
            "install",
            "-g",
            "org.example/continued-child",
            "--registry",
            r.registry().to_str().unwrap(),
        ],
    );
    let (status, _) = refused(&output);
    assert_eq!(status, Some(75));
    let child = row(&r, "org.example/continued-child@1.0.0");
    let before = fs::read(Path::new(child["path"].as_str().unwrap()).join("steps.json")).unwrap();
    let refused_parent = hosted(
        &r,
        &[
            "install",
            "-g",
            "org.example/continued-parent",
            "--registry",
            r.registry().to_str().unwrap(),
        ],
    );
    let (status, error) = refused(&refused_parent);
    assert_eq!(status, Some(2));
    assert_eq!(error["code"], "requirement_unresolved");
    assert!(!r.root.path().join("parent-values").exists());
    let listing = json(&r.vibe(&["app", "work", "-g", "--json"]));
    assert_eq!(listing["entries"].as_array().unwrap().len(), 1);
    assert_eq!(
        fs::read(Path::new(child["path"].as_str().unwrap()).join("steps.json")).unwrap(),
        before
    );
}
