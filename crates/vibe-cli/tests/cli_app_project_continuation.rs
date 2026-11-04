//! Real project/user work isolation and current lock admission on exact resume.
#![cfg(all(test, target_os = "linux"))]
mod common;
use common::app_registry::{AppRegistry, json, ok};
use common::fake_agent::refused;
use serde_json::Value;
use std::{fs, path::Path, process::Output};
const NAME: &str = "org.example/project-continued";
const KEY: &str = "org.example/project-continued@1.0.0";
fn fixture() -> AppRegistry {
    let r = AppRegistry::new();
    let p = r.package(
        "project-continued",
        "1.0.0",
        "project-continued",
        r#"
result='owned'
[[installer_options.steps]]
id='build'
title='Build'
kind='recipe'
script='build.sh'
[[installer_options.steps]]
id='park'
title='Park'
kind='agent'
phase='finish'
required=true
prompt='Complete this bounded project continuation.'
[[installer_options.configuration.features]]
id='core'
name='Core'
required=true
"#,
    );
    fs::write(p.join("build.sh"),format!("set -e\nmkdir -p \"$VIBE_OUT\"\necho built > \"$VIBE_OUT/receipt\"\necho run >> '{}'\n",r.root.path().join("runs").display())).unwrap();
    r
}
fn hosted(r: &AppRegistry, project: Option<&Path>) -> Output {
    let mut c = r.user.vibe();
    c.current_dir(r.root.path())
        .env("HOME", r.home())
        .env("VIBE_AGENT_COMMAND", "sh")
        .env("VIBE_AGENT_ARGS", "[]")
        .env_remove("VIBE_INVOKED_BY");
    c.args([
        "install",
        NAME,
        "--registry",
        r.registry().to_str().unwrap(),
        "--assume-yes",
        "--agent-mode",
        "agent",
        "--offline",
        "--unattended",
        "--json",
    ]);
    if let Some(p) = project {
        c.arg("--path").arg(p);
    } else {
        c.arg("-g");
    }
    c.output().unwrap()
}
fn row(r: &AppRegistry, project: Option<&Path>) -> Value {
    let out = if let Some(p) = project {
        r.vibe(&["app", "work", "--path", p.to_str().unwrap(), "--json"])
    } else {
        r.vibe(&["app", "work", "-g", "--json"])
    };
    json(&out)["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["key"] == KEY)
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
fn resume(r: &AppRegistry, p: &Path, target: &Value) -> Output {
    r.vibe(&[
        "app",
        "resume",
        KEY,
        "--operation",
        target["operation"]["target"]["operation"].as_str().unwrap(),
        "--path",
        p.to_str().unwrap(),
        "--json",
        "--offline",
        "--unattended",
    ])
}
#[test]
fn same_key_work_is_home_local_aliases_agree_and_lock_drift_refuses_before_effects() {
    let r = fixture();
    let a = r.root.path().join("a");
    let b = r.root.path().join("b");
    r.user.init_project(&a);
    r.user.init_project(&b);
    let (sa, ea) = refused(&hosted(&r, Some(&a)));
    assert_eq!(sa, Some(75), "{ea}");
    let (sb, eb) = refused(&hosted(&r, Some(&b)));
    assert_eq!(sb, Some(75), "{eb}");
    let (su, eu) = refused(&hosted(&r, None));
    assert_eq!(su, Some(75), "{eu}");
    let ar = row(&r, Some(&a));
    let br = row(&r, Some(&b));
    let ur = row(&r, None);
    assert_ne!(ar["path"], br["path"]);
    assert_ne!(ar["path"], ur["path"]);
    assert_ne!(br["path"], ur["path"]);
    assert!(
        ar["path"]
            .as_str()
            .unwrap()
            .starts_with(a.to_str().unwrap())
    );
    assert!(
        ur["path"]
            .as_str()
            .unwrap()
            .starts_with(r.settings().to_str().unwrap())
    );
    let alias = r.root.path().join("alias");
    std::os::unix::fs::symlink(&a, &alias).unwrap();
    assert_eq!(row(&r, Some(&alias)), ar);
    let checkpoint = Path::new(ar["path"].as_str().unwrap()).join("steps.json");
    let before = fs::read(&checkpoint).unwrap();
    let lock_path = a.join("vibe.lock");
    let old_lock = fs::read_to_string(&lock_path).unwrap();
    let mut lock: toml::Value = toml::from_str(&old_lock).unwrap();
    lock["package"]
        .as_array_mut()
        .unwrap()
        .retain(|p| p["name"].as_str() != Some("project-continued"));
    fs::write(&lock_path, toml::to_string(&lock).unwrap()).unwrap();
    answer(&ea);
    let output = resume(&r, &a, &ar);
    let (status, error) = refused(&output);
    assert_eq!(status, Some(1));
    assert_eq!(error["code"], "registry_failed");
    assert!(error["error"].as_str().unwrap().contains("lock"));
    assert_eq!(fs::read(&checkpoint).unwrap(), before);
    fs::write(&lock_path, &old_lock).unwrap();
    ok(&resume(&r, &alias, &ar));
    assert_eq!(fs::read_to_string(&lock_path).unwrap(), old_lock);
    assert_eq!(
        fs::read_to_string(r.root.path().join("runs"))
            .unwrap()
            .lines()
            .count(),
        3
    );
    // Explicit cleanup stays scoped to the selected physical home.
    ok(&r.vibe(&[
        "app",
        "work",
        "--clean",
        "--all",
        "--path",
        b.to_str().unwrap(),
        "--json",
    ]));
    assert_eq!(row(&r, None), ur);
    assert!(Path::new(ur["path"].as_str().unwrap()).exists());
    let wrong = resume(&r, &b, &ar);
    let (status, error) = refused(&wrong);
    assert_eq!(status, Some(2));
    assert_eq!(error["code"], "resume_mismatch");
}
