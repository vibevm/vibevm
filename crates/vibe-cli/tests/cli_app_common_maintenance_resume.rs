//! Actual maintenance continuation keeps removal/predecessor/publication ownership.
#![cfg(all(test, target_os = "linux"))]
mod common;
use common::app_registry::{AppRegistry, json, ok};
use common::fake_agent::refused;
use serde_json::Value;
use std::{fs, path::Path, process::Output};
const NAME: &str = "org.example/maintained";
fn package(r: &AppRegistry, version: &str, phase: &str, broken_removal: bool) {
    let removal = if broken_removal { "; exit 7" } else { "" };
    let p = r.package(
        "maintained",
        version,
        "maintained",
        &format!(
            r#"
result='owned'
[[installer_options.steps]]
id='note'
title='Record install'
kind='shell'
command='true'
uninstall="echo remove >> '{}'{}"
[[installer_options.steps]]
id='build'
title='Build'
kind='recipe'
script='build.sh'
[[installer_options.steps]]
id='extra'
title='Additional setup'
kind='agent'
phase='{phase}'
when={{feature='extra'}}
required=true
prompt='Complete the bounded maintenance fixture.'
[[installer_options.configuration.features]]
id='extra'
name='Additional setup'
"#,
            r.root.path().join("removals").display(),
            removal
        ),
    );
    fs::write(p.join("build.sh"),format!("set -e\necho '{version}' >> '{}'\nmkdir -p \"$VIBE_OUT\"\necho '{version}' > \"$VIBE_OUT/receipt\"\n",r.root.path().join("builds").display())).unwrap();
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
fn count(r: &AppRegistry, path: &str) -> usize {
    fs::read_to_string(r.root.path().join(path))
        .unwrap_or_default()
        .lines()
        .count()
}
fn work(r: &AppRegistry, key: &str) -> Value {
    json(&r.vibe(&["app", "work", "-g", "--json"]))["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["key"] == key)
        .unwrap()
        .clone()
}
fn answer(error: &Value) {
    fs::write(
        Path::new(error["run"].as_str().unwrap()).join("result.json"),
        r#"{"protocol":"vibe-agent-hook-result/1","status":"ok","message":"done"}"#,
    )
    .unwrap();
}
fn resume(r: &AppRegistry, row: &Value) -> Output {
    r.vibe(&[
        "app",
        "resume",
        row["key"].as_str().unwrap(),
        "--operation",
        row["operation"]["target"]["operation"].as_str().unwrap(),
        "-g",
        "--json",
        "--unattended",
        "--offline",
    ])
}
#[test]
fn modify_park_after_removal_resumes_without_repeating_removal() {
    let r = AppRegistry::new();
    package(&r, "1.0.0", "prepare", false);
    ok(&r.app(
        &["install", "-g"],
        NAME,
        &["--offline", "--unattended", "--json"],
    ));
    let output = hosted(
        &r,
        &[
            "app",
            "modify",
            NAME,
            "-g",
            "--registry",
            r.registry().to_str().unwrap(),
            "--feature",
            "extra",
        ],
    );
    let (status, error) = refused(&output);
    assert_eq!(status, Some(75));
    let row = work(&r, "org.example/maintained@1.0.0");
    assert_eq!(count(&r, "removals"), 1);
    assert_eq!(count(&r, "builds"), 2);
    answer(&error);
    ok(&resume(&r, &row));
    assert_eq!(count(&r, "removals"), 1);
    assert_eq!(count(&r, "builds"), 2);
    assert_eq!(
        r.index()["installations"]["org.example/maintained@1.0.0"]["features"],
        serde_json::json!(["extra"])
    );
}
#[test]
fn uncertain_removal_refuses_continuation_without_repeating_the_command() {
    let r = AppRegistry::new();
    package(&r, "1.0.0", "prepare", true);
    ok(&r.app(
        &["install", "-g"],
        NAME,
        &["--offline", "--unattended", "--json"],
    ));
    assert!(
        !hosted(
            &r,
            &[
                "app",
                "modify",
                NAME,
                "-g",
                "--registry",
                r.registry().to_str().unwrap(),
                "--feature",
                "extra"
            ]
        )
        .status
        .success()
    );
    let row = work(&r, "org.example/maintained@1.0.0");
    assert_eq!(row["operation"]["phase"], "removing");
    assert_eq!(count(&r, "removals"), 1);
    let checkpoint = fs::read(Path::new(row["path"].as_str().unwrap()).join("steps.json")).unwrap();
    let (status, error) = refused(&resume(&r, &row));
    assert_eq!(status, Some(2));
    assert_eq!(error["code"], "resume_mismatch");
    assert_eq!(count(&r, "removals"), 1);
    assert_eq!(
        fs::read(Path::new(row["path"].as_str().unwrap()).join("steps.json")).unwrap(),
        checkpoint
    );
}
#[test]
fn update_finish_resume_keeps_the_original_predecessor_and_rollback() {
    let r = AppRegistry::new();
    package(&r, "1.0.0", "finish", false);
    ok(&r.app(
        &["install", "-g"],
        NAME,
        &["--offline", "--unattended", "--json"],
    ));
    package(&r, "2.0.0", "finish", false);
    let output = hosted(
        &r,
        &[
            "update",
            "-g",
            NAME,
            "--registry",
            r.registry().to_str().unwrap(),
            "--feature",
            "extra",
        ],
    );
    let (status, error) = refused(&output);
    assert_eq!(status, Some(75));
    let row = work(&r, "org.example/maintained@2.0.0");
    assert_eq!(row["operation"]["phase"], "published");
    assert_eq!(count(&r, "builds"), 2);
    answer(&error);
    ok(&resume(&r, &row));
    let index = r.index();
    let next = &index["installations"]["org.example/maintained@2.0.0"];
    assert_eq!(next["replaced"], "org.example/maintained@1.0.0");
    assert_ne!(next["hidden"], true);
    assert_eq!(count(&r, "builds"), 2);
    ok(&r.vibe(&[
        "app",
        "rollback",
        NAME,
        "-g",
        "--offline",
        "--unattended",
        "--json",
    ]));
    assert_eq!(
        r.index()["slots"]["maintained"]["default"],
        "org.example/maintained@1.0.0"
    );
}
#[test]
fn indexed_pending_record_drift_refuses_before_finish_or_checkpoint_rewrite() {
    let r = AppRegistry::new();
    package(&r, "1.0.0", "finish", false);
    let output = hosted(
        &r,
        &[
            "install",
            "-g",
            NAME,
            "--registry",
            r.registry().to_str().unwrap(),
            "--feature",
            "extra",
        ],
    );
    let (status, error) = refused(&output);
    assert_eq!(status, Some(75));
    let row = work(&r, "org.example/maintained@1.0.0");
    answer(&error);
    ok(&r.vibe(&[
        "app",
        "hide",
        NAME,
        "-g",
        "--offline",
        "--unattended",
        "--json",
    ]));
    let checkpoint = fs::read(Path::new(row["path"].as_str().unwrap()).join("steps.json")).unwrap();
    let index = fs::read(r.settings().join("applications/index.json")).unwrap();
    let (status, error) = refused(&resume(&r, &row));
    assert_eq!(status, Some(2));
    assert_eq!(error["code"], "resume_mismatch");
    assert_eq!(
        fs::read(Path::new(row["path"].as_str().unwrap()).join("steps.json")).unwrap(),
        checkpoint
    );
    assert_eq!(
        fs::read(r.settings().join("applications/index.json")).unwrap(),
        index
    );
    assert_eq!(count(&r, "builds"), 1);
}

#[test]
fn published_update_cannot_silently_drop_a_failed_requested_purge() {
    let r = AppRegistry::new();
    package(&r, "1.0.0", "finish", true);
    ok(&r.app(
        &["install", "-g"],
        NAME,
        &["--offline", "--unattended", "--json"],
    ));
    package(&r, "2.0.0", "finish", false);
    let output = r.app(
        &["update", "-g"],
        NAME,
        &["--purge-previous", "--offline", "--unattended", "--json"],
    );
    assert_eq!(
        output.status.code(),
        Some(1),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(count(&r, "removals"), 1);
    assert_eq!(count(&r, "builds"), 2);
    assert!(r.index()["installations"]["org.example/maintained@1.0.0"].is_object());
    assert!(r.index()["installations"]["org.example/maintained@2.0.0"].is_object());
    let row = work(&r, "org.example/maintained@2.0.0");
    let checkpoint = Path::new(row["path"].as_str().unwrap()).join("steps.json");
    let before = fs::read(&checkpoint).unwrap();
    let index = fs::read(r.settings().join("applications/index.json")).unwrap();
    let output = resume(&r, &row);
    assert_eq!(
        output.status.code(),
        Some(2),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let (_, error) = refused(&output);
    assert_eq!(error["code"], "resume_mismatch");
    assert_eq!(fs::read(&checkpoint).unwrap(), before);
    assert_eq!(
        fs::read(r.settings().join("applications/index.json")).unwrap(),
        index
    );
    assert_eq!(count(&r, "removals"), 1);
    assert_eq!(count(&r, "builds"), 2);
}

#[test]
fn parked_replacement_keeps_previous_until_success_then_purges_once() {
    let r = AppRegistry::new();
    package(&r, "1.0.0", "finish", false);
    ok(&r.app(
        &["install", "-g"],
        NAME,
        &["--offline", "--unattended", "--json"],
    ));
    package(&r, "2.0.0", "finish", false);
    let output = hosted(
        &r,
        &[
            "update",
            "-g",
            NAME,
            "--registry",
            r.registry().to_str().unwrap(),
            "--feature",
            "extra",
            "--purge-previous",
        ],
    );
    let (status, error) = refused(&output);
    assert_eq!(status, Some(75));
    assert_eq!(count(&r, "removals"), 0);
    assert!(r.index()["installations"]["org.example/maintained@1.0.0"].is_object());
    let row = work(&r, "org.example/maintained@2.0.0");
    let pending: Value = serde_json::from_slice(
        &fs::read(Path::new(row["path"].as_str().unwrap()).join("steps.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(pending["operation"]["retirements"][0]["state"], "queued");
    assert!(r.index()["installations"]["org.example/maintained@2.0.0"]["pending"].is_object());
    answer(&error);
    ok(&resume(&r, &row));
    assert_eq!(count(&r, "removals"), 1);
    assert_eq!(count(&r, "builds"), 2);
    assert!(
        r.index()["installations"]
            .get("org.example/maintained@1.0.0")
            .is_none()
    );
    assert!(
        r.index()["installations"]["org.example/maintained@2.0.0"]
            .get("pending")
            .is_none()
    );
    assert!(!resume(&r, &row).status.success());
    assert_eq!(count(&r, "removals"), 1);
}
#[test]
fn terminal_required_finish_failure_keeps_previous_and_never_claims_purge() {
    let r = AppRegistry::new();
    package(&r, "1.0.0", "finish", false);
    ok(&r.app(
        &["install", "-g"],
        NAME,
        &["--offline", "--unattended", "--json"],
    ));
    package(&r, "2.0.0", "finish", false);
    let manifest = r.registry().join("org.example/maintained/v2.0.0/vibe.toml");
    let text = fs::read_to_string(&manifest).unwrap();
    let text = text
        .replace(
            "kind='agent'\nphase='finish'",
            "kind='shell'\nphase='finish'",
        )
        .replace(
            "required=true\nprompt='Complete the bounded maintenance fixture.'",
            "command='exit 19'",
        );
    fs::write(&manifest, text).unwrap();
    let output = r.app(
        &["update", "-g"],
        NAME,
        &[
            "--feature",
            "extra",
            "--purge-previous",
            "--offline",
            "--unattended",
            "--json",
        ],
    );
    let (status, error) = refused(&output);
    assert_eq!(status, Some(1));
    assert_eq!(error["code"], "finish_failed");
    assert_eq!(count(&r, "removals"), 0);
    assert_eq!(count(&r, "builds"), 2);
    assert!(r.index()["installations"]["org.example/maintained@1.0.0"].is_object());
    assert!(
        r.index()["installations"]["org.example/maintained@2.0.0"]
            .get("pending")
            .is_none()
    );
}

#[test]
fn fresh_replacement_cannot_erase_its_own_uncertain_purge() {
    let r = AppRegistry::new();
    package(&r, "1.0.0", "finish", true);
    ok(&r.app(
        &["install", "-g"],
        NAME,
        &["--offline", "--unattended", "--json"],
    ));
    package(&r, "2.0.0", "finish", false);
    assert_eq!(
        r.app(
            &["update", "-g"],
            NAME,
            &["--purge-previous", "--offline", "--unattended", "--json"]
        )
        .status
        .code(),
        Some(1)
    );
    let row = work(&r, "org.example/maintained@2.0.0");
    let path = Path::new(row["path"].as_str().unwrap()).join("steps.json");
    let before = fs::read(&path).unwrap();
    let index = fs::read(r.settings().join("applications/index.json")).unwrap();
    for flags in [vec!["--force"], vec!["--from-step", "build"]] {
        let mut args = flags;
        args.extend(["--offline", "--unattended", "--json"]);
        let output = r.app(&["install", "-g"], "org.example/maintained@2.0.0", &args);
        assert_eq!(
            output.status.code(),
            Some(2),
            "{}",
            String::from_utf8_lossy(&output.stdout)
        );
        assert_eq!(fs::read(&path).unwrap(), before);
        assert_eq!(
            fs::read(r.settings().join("applications/index.json")).unwrap(),
            index
        );
        assert_eq!(count(&r, "builds"), 2);
        assert_eq!(count(&r, "removals"), 1);
    }
}
#[test]
fn final_published_retirement_completion_needs_no_recipe_source_or_secret() {
    let r = AppRegistry::new();
    package(&r, "1.0.0", "finish", false);
    ok(&r.app(
        &["install", "-g"],
        NAME,
        &["--offline", "--unattended", "--json"],
    ));
    package(&r, "2.0.0", "finish", false);
    ok(&r.app(
        &["update", "-g"],
        NAME,
        &[
            "--purge-previous",
            "--keep-work",
            "--offline",
            "--unattended",
            "--json",
        ],
    ));
    let row = work(&r, "org.example/maintained@2.0.0");
    let path = Path::new(row["path"].as_str().unwrap()).join("steps.json");
    let mut saved: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    saved["operation"]["phase"] = "published".into();
    saved["operation"]["request"]["agent"] = "deleted-executor".into();
    saved["operation"]["request"]["agentExternal"] = false.into();
    fs::write(&path, serde_json::to_vec_pretty(&saved).unwrap()).unwrap();
    fs::remove_dir_all(r.registry()).unwrap();
    let index = fs::read(r.settings().join("applications/index.json")).unwrap();
    for phase in ["published", "retiring"] {
        saved["operation"]["phase"] = phase.into();
        fs::write(&path, serde_json::to_vec_pretty(&saved).unwrap()).unwrap();
        ok(&resume(&r, &row));
        assert_eq!(
            fs::read(r.settings().join("applications/index.json")).unwrap(),
            index
        );
        assert_eq!(count(&r, "builds"), 2);
        assert_eq!(count(&r, "removals"), 1);
    }
}
