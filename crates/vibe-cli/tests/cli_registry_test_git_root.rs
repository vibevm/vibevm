//! Missing local Git roots must not be reported as reachable or as localdirs.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};
use vibe_test_support::UserScratch;

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
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", home.join("empty-gitconfig"))
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_ALLOW_PROTOCOL", "file")
        .env("NO_COLOR", "1")
        .env("CI", "1")
        .env_remove("GIT_CONFIG")
        .env_remove("GIT_CONFIG_COUNT")
        .env_remove("GIT_CONFIG_PARAMETERS")
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
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
        .expect("run isolated registry probe")
}

#[test]
fn registry_test_separates_missing_git_and_localdir() {
    let root = tempfile::tempdir().expect("fixture root");
    let user = UserScratch::new_in(root.path());
    let home = tempfile::tempdir().expect("separate HOME");
    let project = root.path().join("project");
    let reachable = root.path().join("localdir");
    for dir in [&project, &reachable] {
        fs::create_dir_all(dir).expect("fixture directory");
    }
    fs::write(home.path().join("empty-gitconfig"), "").expect("empty git config");
    let init = command(&user, home.path(), &project)
        .args(["init", "--path"])
        .arg(&project)
        .args(["--author", "Fixture"])
        .output()
        .expect("initialize isolated project");
    assert!(init.status.success(), "{init:?}");

    let missing_git = format!("file://{}", root.path().join("gitreg").display());
    let missing_dir = format!("file://{}", root.path().join("absent-dir").display());
    // Obtain the expected numeric status from real Git, never a fabricated 128.
    let mut git = Command::new("git");
    git.current_dir(root.path())
        .env("HOME", home.path())
        .env("USERPROFILE", home.path())
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", home.path().join("empty-gitconfig"))
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_ALLOW_PROTOCOL", "file")
        .env_remove("GIT_CONFIG")
        .env_remove("GIT_CONFIG_COUNT")
        .env_remove("GIT_CONFIG_PARAMETERS")
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE");
    let actual = git
        .args(["ls-remote", "--", &missing_git])
        .output()
        .expect("local Git must be available for this fixture");
    assert!(!actual.status.success(), "{actual:?}");
    let exit = actual.status.code().expect("numeric Git failure status");
    let diagnostic = format!("git ls-remote exit {exit}");
    let mut registries = String::new();
    for (name, kind, url) in [
        ("local", "localdir", format!("file://{}", reachable.display())),
        ("brokendir", "localdir", missing_dir),
        ("brokengit", "git", missing_git.clone()),
        ("brokenmono", "monorepo", missing_git),
    ] {
        registries.push_str(&format!(
            "[[registry]]\nname='{name}'\ntype='{kind}'\nurl={}\n",
            toml::Value::String(url)
        ));
    }
    fs::write(user.settings.join("registry.toml"), registries).expect("local registries");

    for json in [false, true] {
        let output = probe(&user, home.path(), &project, json);
        assert_eq!(output.status.code(), Some(1), "{output:?}");
        let stderr = String::from_utf8(output.stderr).expect("stderr UTF-8");
        assert!(stderr.contains("registry_test_failed"), "{stderr}");
        if json {
            let report: serde_json::Value = serde_json::from_slice(&output.stdout)
                .expect("exactly one complete JSON report on stdout");
            assert_eq!(report["ok"], false);
            let rows = report["registries"].as_array().expect("registry rows");
            assert_eq!(rows.len(), 4);
            for (name, status) in [
                ("local", "reachable"),
                ("brokendir", "missing"),
                ("brokengit", "unreachable"),
                ("brokenmono", "unreachable"),
            ] {
                let row = rows.iter().find(|row| row["name"] == name).expect(name);
                assert_eq!(row["status"], status, "{row}");
                if matches!(name, "brokengit" | "brokenmono") {
                    assert_eq!(row["note"], diagnostic);
                }
            }
        } else {
            let stdout = String::from_utf8(output.stdout).expect("stdout UTF-8");
            for (name, status) in [
                ("local", "reachable"),
                ("brokendir", "missing"),
                ("brokengit", "unreachable"),
                ("brokenmono", "unreachable"),
            ] {
                assert!(
                    stdout.lines().any(|line| line.contains(name) && line.contains(status)),
                    "{stdout}"
                );
            }
            assert_eq!(stdout.matches(&diagnostic).count(), 2, "{stdout}");
            assert!(stdout.contains("does not exist"), "{stdout}");
            assert!(stdout.contains("1 of 4 reachable"), "{stdout}");
            assert!(!stdout.contains("registry_test_failed"), "{stdout}");
        }
    }
}
