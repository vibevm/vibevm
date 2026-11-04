//! The real cohort target is independent of failed, retried and parked package rows.
#![cfg(all(test, target_os = "linux"))]
mod common;
use common::app_registry::{AppRegistry, json, ok};
use common::fake_agent::refused;
use serde_json::Value;
use std::{fs, path::Path};

fn package(r: &AppRegistry, name: &str, tail: &str) -> std::path::PathBuf {
    let root = r.package(
        name,
        "1.0.0",
        name,
        &format!(
            r#"
result='owned'
[[installer_options.steps]]
id='build'
title='Build'
kind='recipe'
script='build.sh'
{tail}
"#
        ),
    );
    fs::write(root.join("build.sh"),format!("set -e\necho '{name}' >> '{}'\nmkdir -p \"$VIBE_OUT/bin\"\nprintf '#!/bin/sh\\necho ok\\n' > \"$VIBE_OUT/bin/{name}\"\nchmod +x \"$VIBE_OUT/bin/{name}\"\n",r.root.path().join("executions").display())).unwrap();
    root
}
fn set(r: &AppRegistry, names: &[&str]) -> std::path::PathBuf {
    let path = r.root.path().join("apps.toml");
    let mut text="protocol='vibe-apps/1'\nexported='2026-11-01T00:00:00Z'\nhome='user'\nplatform='linux'\narch='x86_64'\n".to_string();
    for name in names {
        text.push_str(&format!("\n[[application]]\nkey='org.example/{name}@1.0.0'\ninstaller='main'\nmark='manual'\nheld={}\n",*name=="last"));
    }
    fs::write(&path, text).unwrap();
    path
}
fn executions(r: &AppRegistry) -> Vec<String> {
    fs::read_to_string(r.root.path().join("executions"))
        .unwrap_or_default()
        .lines()
        .map(str::to_owned)
        .collect()
}
fn hosted(r: &AppRegistry, file: &Path) -> std::process::Output {
    r.user
        .vibe()
        .current_dir(r.root.path())
        .env("HOME", r.home())
        .env("VIBE_AGENT_COMMAND", "sh")
        .env("VIBE_AGENT_ARGS", "[]")
        .env_remove("VIBE_INVOKED_BY")
        .args([
            "app",
            "import",
            file.to_str().unwrap(),
            "--registry",
            r.registry().to_str().unwrap(),
            "--agent-mode",
            "agent",
            "--json",
            "--offline",
            "--unattended",
        ])
        .output()
        .unwrap()
}
#[test]
fn failed_first_row_then_parked_row_resumes_the_remaining_cohort_and_reports_the_original_refusal()
{
    let r = AppRegistry::new();
    let first = package(&r, "first", "");
    fs::write(
        first.join("build.sh"),
        format!(
            "echo first >> '{}'\nexit 17\n",
            r.root.path().join("executions").display()
        ),
    )
    .unwrap();
    package(
        &r,
        "second",
        r#"
[[installer_options.steps]]
id='finish'
title='Finish'
kind='agent'
phase='finish'
required=true
prompt='Complete this bounded import fixture.'
"#,
    );
    package(
        &r,
        "last",
        r#"
[[installer_options.launchers]]
id='last'
name='Last'
kind='command'
executable='$app_bin/last'
"#,
    );
    let file = set(&r, &["first", "second", "last"]);
    let mut text = fs::read_to_string(&file).unwrap();
    text.push_str("\n[[collection]]\nname='Imported'\nlaunchers=['org.example/last@1.0.0#last']\n");
    fs::write(&file, text).unwrap();
    let (status, error) = refused(&hosted(&r, &file));
    assert_eq!(status, Some(75), "{error}");
    assert_eq!(executions(&r), ["first", "second"]);
    let work = json(&r.vibe(&["app", "work", "-g", "--json"]));
    let rows = work["entries"].as_array().unwrap();
    let root = &work["imports"][0];
    let failed = rows
        .iter()
        .find(|e| e["key"] == "org.example/first@1.0.0")
        .unwrap();
    assert_eq!(failed["state"], "failed");
    let second = rows
        .iter()
        .find(|e| e["key"] == "org.example/second@1.0.0")
        .unwrap();
    assert_ne!(second["operation"]["target"], root["target"]);
    assert!(
        !r.index()["installations"]
            .as_object()
            .unwrap()
            .contains_key("org.example/first@1.0.0")
    );
    assert!(error["resume"].as_str().unwrap().contains(&format!(
        "vibe app import --resume {}",
        root["target"]["operation"].as_str().unwrap()
    )));
    fs::remove_file(file).unwrap();
    fs::write(
        Path::new(error["run"].as_str().unwrap()).join("result.json"),
        r#"{"protocol":"vibe-agent-hook-result/1","status":"ok","message":"done"}"#,
    )
    .unwrap();
    let output = r.vibe(&[
        "app",
        "import",
        "--resume",
        root["target"]["operation"].as_str().unwrap(),
        "-g",
        "--json",
        "--offline",
        "--unattended",
    ]);
    assert_eq!(
        output.status.code(),
        Some(1),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["protocol"], "vibe-app-import/1");
    assert_eq!(
        report["installed"],
        serde_json::json!(["org.example/second@1.0.0", "org.example/last@1.0.0"])
    );
    assert_eq!(report["refused"][0]["key"], "org.example/first@1.0.0");
    let detail = report["refused"][0]["message"].as_str().unwrap();
    assert!(
        detail.contains("step `build`")
            && detail.contains("Historical application attempt")
            && detail.contains("vibe app runs org.example/first@1.0.0 -g"),
        "{detail}"
    );
    assert_eq!(executions(&r), ["first", "second", "last"]);
    assert_eq!(
        r.index()["installations"]["org.example/last@1.0.0"]["held"],
        true
    );
    assert!(
        !r.index()["installations"]
            .as_object()
            .unwrap()
            .contains_key("org.example/first@1.0.0")
    );
    let layer = fs::read_to_string(r.settings().join("applications/launchers.toml")).unwrap();
    assert!(layer.contains("Imported") && layer.contains("org.example/last@1.0.0#last"));
    let final_work = json(&r.vibe(&["app", "work", "-g", "--json"]));
    assert!(final_work.get("imports").is_none());
    assert_eq!(final_work["entries"].as_array().unwrap().len(), 1);
    assert_eq!(final_work["entries"][0]["key"], "org.example/first@1.0.0");
    assert_eq!(final_work["entries"][0]["state"], "failed");
}

#[test]
fn rich_portable_refusals_preserve_output_and_preflight_every_import_row() {
    let r = AppRegistry::new();
    package(&r, "first", "");
    package(
        &r,
        "rich",
        r#"
[[installer_options.configuration.features]]
id='extra'
name='Extra'
"#,
    );
    let file = set(&r, &["first", "rich"]);
    let output = r.vibe(&[
        "app",
        "import",
        file.to_str().unwrap(),
        "--registry",
        r.registry().to_str().unwrap(),
        "--offline",
        "--unattended",
        "--json",
    ]);
    let (_, error) = refused(&output);
    assert_eq!(error["code"], "configuration_portable_unsupported");
    assert!(executions(&r).is_empty());
    assert!(!r.settings().join("applications/index.json").exists());
    ok(&r.app(
        &["install", "-g"],
        "org.example/rich",
        &["--json", "--offline", "--unattended"],
    ));
    let output_file = r.root.path().join("preserve.toml");
    fs::write(&output_file, "KEEP EXACTLY\n").unwrap();
    let output = r.vibe(&[
        "app",
        "export",
        "--output",
        output_file.to_str().unwrap(),
        "--json",
    ]);
    let (_, error) = refused(&output);
    assert_eq!(error["code"], "configuration_portable_unsupported");
    assert_eq!(fs::read_to_string(output_file).unwrap(), "KEEP EXACTLY\n");
}

#[test]
fn ancestor_secret_resupply_is_not_broadcast_to_another_import_row() {
    scoped_secret_resupply(true);
}
#[test]
fn absent_secret_can_be_supplied_to_its_exact_child_and_must_be_resupplied_after_a_second_park() {
    scoped_secret_resupply(false);
}
fn scoped_secret_resupply(initial_default: bool) {
    let r = AppRegistry::new();
    package(
        &r,
        "first",
        r#"
[[installer_options.parameters]]
id='token'
name='Token'
type='string'
secret=true
default='root-public-default'
"#,
    );
    let child = package(
        &r,
        "second",
        &r#"
[[installer_options.parameters]]
id='token'
name='Token'
type='string'
secret=true
default='child-public-default'
[[installer_options.steps]]
id='finish'
title='Finish'
kind='agent'
phase='finish'
required=true
prompt='Complete the bounded import secret fixture.'
[[installer_options.steps]]
id='verify'
title='Verify scoped resupply'
kind='shell'
phase='finish'
command='sh "$VIBE_PACKAGE/verify.sh"'
[[installer_options.steps]]
id='second-park'
title='Second park'
kind='agent'
phase='finish'
required=true
prompt='Complete the second scoped-secret continuation.'
"#
        .replace(
            "default='child-public-default'",
            if initial_default {
                "default='child-public-default'"
            } else {
                ""
            },
        ),
    );
    let expected = r.root.path().join("expected-secret");
    let canary = "IMPORT_CHILD_ONLY_747a";
    fs::write(&expected, canary).unwrap();
    fs::write(
        child.join("verify.sh"),
        format!(
            "set -e\ntest \"$VIBE_PARAM_TOKEN\" = \"$(cat '{}')\"\necho verified >> '{}'\n",
            expected.display(),
            r.root.path().join("verified").display()
        ),
    )
    .unwrap();
    let file = set(&r, &["first", "second"]);
    let output = hosted(&r, &file);
    assert_eq!(
        output.status.code(),
        Some(75),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let (status, error) = refused(&output);
    assert_eq!(status, Some(75), "{error}");
    let initial_origin = if initial_default { "default" } else { "absent" };
    assert_eq!(
        r.index()["installations"]["org.example/second@1.0.0"]["secretOrigins"]["token"]["kind"],
        initial_origin
    );
    let view = json(&r.vibe(&["app", "work", "-g", "--json"]));
    let rows = view["entries"].as_array().unwrap();
    let root = &view["imports"][0];
    let row = rows
        .iter()
        .find(|e| e["key"] == "org.example/second@1.0.0")
        .unwrap();
    let root_before =
        fs::read(Path::new(root["path"].as_str().unwrap()).join("import.json")).unwrap();
    let child_before =
        fs::read(Path::new(row["path"].as_str().unwrap()).join("steps.json")).unwrap();
    let output = r.vibe(&[
        "app",
        "import",
        "--resume",
        root["target"]["operation"].as_str().unwrap(),
        "--set",
        "token=PARENT_MUST_NOT_BROADCAST",
        "-g",
        "--offline",
        "--unattended",
        "--json",
    ]);
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("--set"));
    assert_eq!(
        fs::read(Path::new(root["path"].as_str().unwrap()).join("import.json")).unwrap(),
        root_before
    );
    assert_eq!(
        fs::read(Path::new(row["path"].as_str().unwrap()).join("steps.json")).unwrap(),
        child_before
    );
    fs::write(
        Path::new(error["run"].as_str().unwrap()).join("result.json"),
        r#"{"protocol":"vibe-agent-hook-result/1","status":"ok","message":"done"}"#,
    )
    .unwrap();
    let resumed = r.vibe(&[
        "app",
        "resume",
        "org.example/second@1.0.0",
        "--operation",
        row["operation"]["target"]["operation"].as_str().unwrap(),
        "--set",
        &format!("token={canary}"),
        "-g",
        "--offline",
        "--unattended",
        "--json",
    ]);
    let (status, second_park) = refused(&resumed);
    assert_eq!(status, Some(75), "{second_park}");
    let checkpoint = Path::new(row["path"].as_str().unwrap()).join("steps.json");
    let prior = fs::read(&checkpoint).unwrap();
    let saved: Value = serde_json::from_slice(&prior).unwrap();
    assert_eq!(
        saved["operation"]["request"]["secretOrigins"]["token"]["kind"],
        initial_origin
    );
    assert!(
        saved["operation"]["secretRequired"]
            .as_array()
            .unwrap()
            .iter()
            .any(|id| id == "token")
    );

    let index = fs::read(r.settings().join("applications/index.json")).unwrap();
    let missing = r.vibe(&[
        "app",
        "resume",
        "org.example/second@1.0.0",
        "--operation",
        row["operation"]["target"]["operation"].as_str().unwrap(),
        "-g",
        "--offline",
        "--unattended",
        "--json",
    ]);
    let (status, error) = refused(&missing);
    assert_eq!(status, Some(2), "{error}");
    assert_eq!(error["code"], "parameter_invalid");
    assert_eq!(fs::read(&checkpoint).unwrap(), prior);
    assert_eq!(
        fs::read(r.settings().join("applications/index.json")).unwrap(),
        index
    );
    assert_eq!(
        fs::read_to_string(r.root.path().join("verified")).unwrap(),
        "verified\n"
    );
    fs::write(
        Path::new(second_park["run"].as_str().unwrap()).join("result.json"),
        r#"{"protocol":"vibe-agent-hook-result/1","status":"ok","message":"done"}"#,
    )
    .unwrap();
    ok(&r.vibe(&[
        "app",
        "resume",
        "org.example/second@1.0.0",
        "--operation",
        row["operation"]["target"]["operation"].as_str().unwrap(),
        "--set",
        &format!("token={canary}"),
        "-g",
        "--offline",
        "--unattended",
        "--json",
    ]));
    let report = json(&r.vibe(&[
        "app",
        "import",
        "--resume",
        root["target"]["operation"].as_str().unwrap(),
        "-g",
        "--offline",
        "--unattended",
        "--json",
    ]));
    assert_eq!(report["protocol"], "vibe-app-import/1");
    assert_eq!(executions(&r), ["first", "second"]);
    assert_eq!(
        fs::read_to_string(r.root.path().join("verified")).unwrap(),
        "verified\n"
    );
    assert!(
        !fs::read_to_string(r.settings().join("applications/index.json"))
            .unwrap()
            .contains(canary)
    );
}

#[test]
fn failed_first_row_can_later_be_required_and_retried_without_overwriting_the_cohort() {
    let r = AppRegistry::new();
    package(&r, "dependency", "");
    let first = package(&r, "first", "");
    let first_manifest = first.join("vibe.toml");
    let text = fs::read_to_string(&first_manifest).unwrap().replace(
        "result='owned'",
        "result='owned'\nrequires=['org.example/dependency@=1.0.0']",
    );
    fs::write(&first_manifest, text).unwrap();
    let marker = r.root.path().join("first-attempt");
    fs::write(first.join("build.sh"),format!("set -e\necho first >> '{}'\nif [ ! -e '{}' ]; then touch '{}'; exit 17; fi\nmkdir -p \"$VIBE_OUT\"\necho built > \"$VIBE_OUT/receipt\"\n",r.root.path().join("executions").display(),marker.display(),marker.display())).unwrap();
    let last = package(&r, "last", "");
    let manifest = last.join("vibe.toml");
    let text = fs::read_to_string(&manifest).unwrap().replace(
        "result='owned'",
        "result='owned'\nrequires=['org.example/first@=1.0.0']",
    );
    fs::write(&manifest, text).unwrap();
    let file = set(&r, &["first", "last"]);
    let output = hosted(&r, &file);
    assert_eq!(
        output.status.code(),
        Some(1),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        report["installed"],
        serde_json::json!(["org.example/last@1.0.0"]),
        "{report}"
    );
    assert_eq!(report["refused"].as_array().unwrap().len(), 1, "{report}");
    assert_eq!(report["refused"][0]["key"], "org.example/first@1.0.0");
    assert_eq!(executions(&r), ["dependency", "first", "first", "last"]);
    let kept = json(&r.vibe(&["app", "work", "-g", "--json"]));
    assert!(kept["entries"].as_array().unwrap().is_empty(), "{kept}");
    let (status, error) = refused(&r.vibe(&[
        "app",
        "work",
        "org.example/dependency",
        "--clean",
        "-g",
        "--json",
    ]));
    assert_eq!(status, Some(2));
    assert_eq!(error["code"], "work_missing");
    assert!(r.index()["installations"]["org.example/first@1.0.0"].is_object());
    assert!(r.index()["installations"]["org.example/last@1.0.0"].is_object());
}

#[cfg(test)]
#[path = "cli_app_import_continuation/cohort_controls.rs"]
mod cohort_controls;
