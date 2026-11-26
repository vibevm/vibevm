//! Local-only public coverage for registry probe rows and failure verdicts.

mod common;

use common::UserScratch;
use std::fs;
use std::path::Path;
use std::process::{Command, Output};

/// Every child, including init, receives an isolated home and settings root.
fn command(user: &UserScratch, home: &Path, project: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_vibe"));
    command
        .current_dir(project)
        .env("HOME", home)
        .env("USERPROFILE", home)
        .env("VIBE_SETTINGS", &user.settings)
        .env("VIBE_REGISTRY_CACHE", &user.cache)
        .env("VIBEVM_SEARCH_CACHE_DIR", &user.search_cache)
        .env("VIBE_NO_DEFAULT_REGISTRY", "1")
        .env("NO_COLOR", "1")
        .env("CI", "1")
        .env_remove("VIBEVM_INSTALL_ROOT");
    command
}

fn probe(user: &UserScratch, home: &Path, project: &Path, json: bool) -> Output {
    let mut child = command(user, home, project);
    if json {
        child.arg("--json");
    }
    child
        .args(["registry", "test", "--path"])
        .arg(project)
        .output()
        .expect("run local registry probe")
}

#[test]
fn a_missing_localdir_fails_the_registry_test() {
    let root = tempfile::tempdir().expect("fixture root");
    let user = UserScratch::new_in(root.path());
    let home = root.path().join("home");
    let project = root.path().join("project");
    let registry = root.path().join("registry");
    let missing = root.path().join("absent");
    for dir in [&home, &project, &registry] {
        fs::create_dir_all(dir).expect("fixture directory");
    }
    let init = command(&user, &home, &project)
        .args(["init", "--path"])
        .arg(&project)
        .output()
        .expect("initialize isolated project");
    assert!(init.status.success(), "{init:?}");

    let valid = format!(
        "[[registry]]\nname='local'\ntype='localdir'\nurl={}\n",
        toml::Value::String(format!("file://{}", registry.display()))
    );
    let broken = format!(
        "\n[[registry]]\nname='broken'\ntype='localdir'\nurl={}\n",
        toml::Value::String(format!("file://{}", missing.display()))
    );
    fs::write(
        user.settings.join("registry.toml"),
        format!("{valid}{broken}"),
    )
    .expect("two local registries");

    for json in [false, true] {
        let output = probe(&user, &home, &project, json);
        assert_eq!(output.status.code(), Some(1), "{output:?}");
        let stderr = String::from_utf8(output.stderr).expect("stderr UTF-8");
        assert!(stderr.contains("registry_test_failed"), "{stderr}");
        if json {
            // Parse the entire stdout: a second error document must fail this.
            let report: serde_json::Value = serde_json::from_slice(&output.stdout)
                .expect("exactly one registry report on stdout");
            assert_eq!(report["ok"], false);
            let rows = report["registries"].as_array().expect("registry rows");
            assert_eq!(rows.len(), 2);
            for (name, status) in [("local", "reachable"), ("broken", "missing")] {
                let row = rows.iter().find(|row| row["name"] == name).expect(name);
                assert_eq!(row["status"], status);
            }
        } else {
            let stdout = String::from_utf8(output.stdout).expect("stdout UTF-8");
            assert!(stdout.contains("local"), "{stdout}");
            assert!(stdout.contains("broken"), "{stdout}");
            assert!(stdout.contains("missing"), "{stdout}");
            assert!(stdout.contains("1 of 2 reachable"), "{stdout}");
            assert!(!stdout.contains("registry_test_failed"), "{stdout}");
        }
    }

    fs::write(user.settings.join("registry.toml"), valid).expect("only valid registry");
    for json in [false, true] {
        let output = probe(&user, &home, &project, json);
        assert_eq!(output.status.code(), Some(0), "{output:?}");
        assert!(!String::from_utf8_lossy(&output.stderr).contains("registry_test_failed"));
        if json {
            let report: serde_json::Value =
                serde_json::from_slice(&output.stdout).expect("one success report");
            assert_eq!(report["ok"], true);
            assert_eq!(report["registries"].as_array().expect("rows").len(), 1);
            assert_eq!(report["registries"][0]["name"], "local");
            assert_eq!(report["registries"][0]["status"], "reachable");
        } else {
            assert!(String::from_utf8_lossy(&output.stdout).contains("1 of 1 reachable"));
        }
    }
}
