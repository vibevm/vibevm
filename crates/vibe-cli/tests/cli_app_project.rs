//! Public Linux fixture for project-local applications (PROP-059
//! `#project-scope`): `vibe install` without `-g` records an app package in
//! `vibe.toml` / `vibe.lock` and installs it into the project home
//! `vibevm/vibeopt`, `vibe app` and `vibe shell` see that home first, a second
//! checkout reinstalls from the lockfile only with consent, and `vibe
//! uninstall` removes payload, proxy and dependency row together.

#![cfg(target_os = "linux")]

mod common;

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use common::UserScratch;
use serde_json::Value;
use sha2::{Digest, Sha256};

const INPUT_ENV: &str = "VIBE_TEST_PTY_INPUT";
const PTY_RUNNER_SOURCE: &str = include_str!("common/pty-runner.c");

/// A local registry with the zip app package `org.example/alpha@1.0.0`.
fn registry(root: &Path) -> PathBuf {
    let registry = root.join("registry");
    let dir = registry.join("org.example/alpha/v1.0.0");
    fs::create_dir_all(&dir).unwrap();
    let mut zip = zip::ZipWriter::new(fs::File::create(dir.join("tool.zip")).unwrap());
    let options = zip::write::SimpleFileOptions::default().unix_permissions(0o755);
    zip.start_file("bin/tool", options).unwrap();
    zip.write_all(b"#!/bin/sh\necho \"ALPHA_RAN $*\"\n")
        .unwrap();
    zip.finish().unwrap();
    let sha = format!(
        "{:x}",
        Sha256::digest(fs::read(dir.join("tool.zip")).unwrap())
    );
    fs::write(
        dir.join("vibe.toml"),
        format!(
            "[package]\nname = \"alpha\"\ngroup = \"org.example\"\nkind = \"app\"\nversion = \"1.0.0\"\n\n\
             [application]\nid = \"tool\"\n\n\
             [[installer_options]]\nid = \"zip\"\nname = \"Zip\"\nplatform = \"linux\"\nplatform_name = \"Linux\"\narch = \"{arch}\"\n\n\
             [installer_options.installation]\ntype = \"zip\"\nsource = \"tool.zip\"\nsha256 = \"{sha}\"\n\n\
             [[installer_options.launchers]]\nid = \"main\"\nname = \"Tool\"\nexecutable = \"$app_bin/tool\"\ndefault = true\n\n\
             [[installer_options.public_commands]]\nname = \"tool\"\nlauncher = \"main\"\n",
            arch = std::env::consts::ARCH,
        ),
    )
    .unwrap();
    registry
}

fn vibe_in(user: &UserScratch, dir: &Path, args: &[&str]) -> Output {
    let mut command = user.vibe();
    command
        .current_dir(dir)
        .args(args)
        .env("VIBE_NO_DEFAULT_REGISTRY", "1")
        .env_remove("VIBE_UNATTENDED");
    command.output().unwrap()
}

fn text(output: &Output) -> String {
    format!(
        "status={}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn ok(output: &Output) -> String {
    assert!(output.status.success(), "{}", text(output));
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn failed(output: &Output) -> String {
    assert!(
        !output.status.success(),
        "unexpected success: {}",
        text(output)
    );
    text(output)
}

fn listed_keys(output: &Output) -> Vec<String> {
    let listing: Value = serde_json::from_str(&ok(output)).unwrap();
    listing["installations"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["key"].as_str().unwrap().to_owned())
        .collect()
}

fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_default()
}

fn assert_user_home_untouched(user: &UserScratch) {
    for entry in ["applications", "opt", "storage.toml"] {
        assert!(
            !user.settings.join(entry).exists(),
            "the user home gained `{entry}`"
        );
    }
}

/// Compile the PTY runner shared with `cli_app_shell_linux`.
fn pty_runner(scope: &Path) -> PathBuf {
    let source = scope.join("pty-runner.c");
    let binary = scope.join("pty-runner");
    fs::write(&source, PTY_RUNNER_SOURCE).unwrap();
    let output = Command::new("cc")
        .args(["-O0", "-o"])
        .arg(&binary)
        .arg(&source)
        .arg("-lutil")
        .output()
        .unwrap();
    assert!(output.status.success(), "cc failed: {}", text(&output));
    binary
}

fn shell_in_pty(runner: &Path, user: &UserScratch, project: &Path, extra: &[&str]) -> String {
    let home = tempfile::tempdir().unwrap();
    let output = Command::new(runner)
        .arg(env!("CARGO_BIN_EXE_vibe"))
        .arg("shell")
        .args(extra)
        .current_dir(project)
        .env("VIBE_SETTINGS", &user.settings)
        .env("HOME", home.path())
        .env("TERM", "dumb")
        .env(
            INPUT_ENV,
            "printf 'PATH_VALUE=<%s>\\n' \"$PATH\"\ntool through-path\nexit 5\n",
        )
        .env_remove("VIBE_UNATTENDED")
        .env_remove("VIBE_APP_USE")
        .env_remove("ENV")
        .env_remove("BASH_ENV")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(5), "{}", text(&output));
    String::from_utf8_lossy(&output.stdout).into_owned()
}

#[test]
fn an_app_dependency_installs_into_the_project_home_and_follows_the_lockfile() {
    let root = tempfile::tempdir().unwrap();
    let user = UserScratch::new();
    let registry = registry(root.path());
    let registry_arg = registry.to_str().unwrap();
    let project = root.path().join("project");
    user.init_project(&project);
    let project = project.canonicalize().unwrap();
    let home = project.join("vibevm/vibeopt");
    let payload = home.join("opt/apps/tool/org.example.alpha@1.0.0/bin/tool");
    let proxy = home.join("opt/bin/tool");

    // A project home has only its default library.
    let refused = failed(&vibe_in(
        &user,
        &project,
        &[
            "install",
            "--library",
            "usb",
            "org.example/alpha",
            "--registry",
            registry_arg,
            "--assume-yes",
        ],
    ));
    assert!(refused.contains("PROP-059#project-scope"), "{refused}");
    assert!(!read(&project.join("vibe.toml")).contains("org.example/alpha"));

    let installed = ok(&vibe_in(
        &user,
        &project,
        &[
            "install",
            "org.example/alpha",
            "--registry",
            registry_arg,
            "--assume-yes",
        ],
    ));
    assert!(
        installed.contains("installed org.example/alpha@1.0.0 via installer `zip`"),
        "{installed}"
    );
    assert!(read(&project.join("vibe.toml")).contains("\"org.example/alpha\" = \"^1.0.0\""));
    let lock = read(&project.join("vibe.lock"));
    assert!(
        lock.contains(
            "kind = \"app\"\nname = \"alpha\"\ngroup = \"org.example\"\nversion = \"1.0.0\""
        ),
        "{lock}"
    );
    assert!(payload.is_file(), "the payload lands in the project home");
    assert_eq!(
        fs::read(&proxy).unwrap(),
        fs::read(env!("CARGO_BIN_EXE_vibe-app-proxy")).unwrap(),
        "the reserved command has its proxy in the project opt/bin"
    );
    assert!(
        read(&project.join(".gitignore"))
            .lines()
            .any(|line| line == "vibevm/vibeopt/"),
        "the project home is ignored by git"
    );
    assert_user_home_untouched(&user);

    // `vibe app` acts on the project home inside the project; `-g` on the user home.
    assert_eq!(
        listed_keys(&vibe_in(&user, &project, &["app", "list", "--json"])),
        ["org.example/alpha@1.0.0"]
    );
    assert!(listed_keys(&vibe_in(&user, &project, &["app", "list", "-g", "--json"])).is_empty());
    let ran = ok(&vibe_in(
        &user,
        &project,
        &["app", "run", "org.example/alpha", "main", "--", "direct"],
    ));
    assert!(ran.contains("ALPHA_RAN direct"), "{ran}");

    // `vibe shell`: the project opt/bin first, then the user opt/bin; `-g`
    // drops the project home.
    let runner = pty_runner(root.path());
    let user_bin = user.settings.join("opt/bin");
    let shell = shell_in_pty(&runner, &user, &project, &[]);
    assert!(
        shell.contains(&format!(
            "PATH_VALUE=<{}:{}:",
            home.join("opt/bin").display(),
            user_bin.display()
        )),
        "{shell}"
    );
    assert!(shell.contains("ALPHA_RAN through-path"), "{shell}");
    let global = shell_in_pty(&runner, &user, &project, &["-g"]);
    assert!(
        global.contains(&format!("PATH_VALUE=<{}:", user_bin.display())),
        "{global}"
    );
    assert!(!global.contains("ALPHA_RAN"), "{global}");

    // A second checkout reinstalls from the lockfile, and only with consent.
    let second = root.path().join("second");
    fs::create_dir_all(&second).unwrap();
    for file in ["vibe.toml", "vibe.lock"] {
        fs::copy(project.join(file), second.join(file)).unwrap();
    }
    let second_payload = second.join("vibevm/vibeopt/opt/apps/tool/org.example.alpha@1.0.0");
    let no_consent = failed(&vibe_in(
        &user,
        &second,
        &["install", "--registry", registry_arg, "--json"],
    ));
    assert!(
        no_consent.contains("PROP-059#project-scope") && no_consent.contains("--assume-yes"),
        "{no_consent}"
    );
    assert!(
        second
            .join("vibevm/vibedeps/org.example.alpha/1.0.0/vibe.toml")
            .is_file(),
        "the dependency itself is materialised"
    );
    assert!(!second_payload.exists(), "no installer ran without consent");
    let reinstalled = ok(&vibe_in(
        &user,
        &second,
        &["install", "--registry", registry_arg, "--assume-yes"],
    ));
    assert!(
        reinstalled.contains("installed org.example/alpha@1.0.0"),
        "{reinstalled}"
    );
    assert!(second_payload.join("bin/tool").is_file());
    let again = ok(&vibe_in(
        &user,
        &second,
        &["install", "--registry", registry_arg, "--assume-yes"],
    ));
    assert!(
        !again.contains("installed org.example/alpha"),
        "an installed key is skipped: {again}"
    );

    // Uninstall removes payload, proxy and dependency row.
    let removed = ok(&vibe_in(
        &user,
        &project,
        &["uninstall", "org.example/alpha", "--assume-yes"],
    ));
    assert!(
        removed.contains("removed org.example/alpha@1.0.0"),
        "{removed}"
    );
    assert!(!payload.exists() && !proxy.exists());
    assert!(!read(&project.join("vibe.toml")).contains("org.example/alpha"));
    assert!(!read(&project.join("vibe.lock")).contains("name = \"alpha\""));
    assert!(listed_keys(&vibe_in(&user, &project, &["app", "list", "--json"])).is_empty());
    assert_user_home_untouched(&user);
}
