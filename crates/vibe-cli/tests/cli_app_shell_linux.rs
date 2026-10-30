//! Linux PTY coverage for `vibe shell` (PROP-059 `#shell-command`,
//! `#session-selection`): the shell's PATH starts with `opt/bin`, its
//! `VIBE_APP_USE` composes the inherited value with `--use`, the proxy honours
//! that per-shell choice over the global default, a parent bash is reused,
//! and JSON or non-terminal use is refused.

#![cfg(target_os = "linux")]

mod common;

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use common::UserScratch;
use sha2::{Digest, Sha256};

const INPUT_ENV: &str = "VIBE_TEST_PTY_INPUT";

fn package(registry: &Path, name: &str) {
    let dir = registry.join(format!("org.example/{name}/v1.0.0"));
    fs::create_dir_all(&dir).unwrap();
    let mut zip = zip::ZipWriter::new(fs::File::create(dir.join("tool.zip")).unwrap());
    let options = zip::write::SimpleFileOptions::default().unix_permissions(0o755);
    zip.start_file("bin/tool", options).unwrap();
    let marker = name.to_uppercase();
    zip.write_all(format!("#!/bin/sh\necho \"{marker}_RAN $*\"\n").as_bytes())
        .unwrap();
    zip.finish().unwrap();
    let sha = format!(
        "{:x}",
        Sha256::digest(fs::read(dir.join("tool.zip")).unwrap())
    );
    fs::write(
        dir.join("vibe.toml"),
        format!(
            "[package]\nname = \"{name}\"\ngroup = \"org.example\"\nkind = \"app\"\nversion = \"1.0.0\"\n\n\
             [application]\nid = \"tool\"\n\n\
             [[installer_options]]\nid = \"zip\"\nname = \"Zip\"\nplatform = \"linux\"\nplatform_name = \"Linux\"\narch = \"{arch}\"\n\n\
             [installer_options.installation]\ntype = \"zip\"\nsource = \"tool.zip\"\nsha256 = \"{sha}\"\n\n\
             [[installer_options.launchers]]\nid = \"main\"\nname = \"Tool\"\nexecutable = \"$app_bin/tool\"\ndefault = true\n\n\
             [[installer_options.public_commands]]\nname = \"tool\"\nlauncher = \"main\"\n",
            arch = std::env::consts::ARCH,
        ),
    )
    .unwrap();
}

/// A settings root with `alpha` (the global default) and `beta` installed.
fn installed() -> (tempfile::TempDir, UserScratch) {
    let root = tempfile::tempdir().unwrap();
    let user = UserScratch::new();
    let registry = root.path().join("registry");
    for name in ["alpha", "beta"] {
        package(&registry, name);
        let output = user
            .vibe()
            .args([
                "install",
                "-g",
                &format!("org.example/{name}"),
                "--registry",
            ])
            .arg(&registry)
            .output()
            .unwrap();
        assert!(output.status.success(), "{}", text(&output));
    }
    (root, user)
}

/// Compile a tiny runner that starts argv[1..] on a fresh pseudo-terminal,
/// types `$VIBE_TEST_PTY_INPUT`, relays the terminal output and returns the
/// child's exit status.
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

fn in_pty(
    runner: &Path,
    user: &UserScratch,
    home: &Path,
    program: &[&str],
    input: &str,
) -> Command {
    let mut command = Command::new(runner);
    command
        .args(program)
        .env("VIBE_SETTINGS", &user.settings)
        .env("HOME", home)
        .env("TERM", "dumb")
        .env(INPUT_ENV, input)
        .env_remove("VIBE_UNATTENDED")
        .env_remove("ENV")
        .env_remove("BASH_ENV");
    command
}

fn vibe() -> &'static str {
    env!("CARGO_BIN_EXE_vibe")
}

#[test]
fn the_shell_puts_opt_bin_first_and_carries_the_session_choice() {
    let (root, user) = installed();
    let runner = pty_runner(root.path());
    let home = tempfile::tempdir().unwrap();
    let output = in_pty(
        &runner,
        &user,
        home.path(),
        &[vibe(), "shell", "--use", "tool=org.example/beta@1.0.0"],
        "printf 'PATH_VALUE=<%s>\\n' \"$PATH\"\nprintf 'USE_VALUE=<%s>\\n' \"$VIBE_APP_USE\"\ntool session\nexit 7\n",
    )
    .env("VIBE_APP_USE", "editor=org.example/vim@9.1.0")
    .output()
    .unwrap();
    assert_eq!(output.status.code(), Some(7), "{}", text(&output));
    let stdout = text(&output);
    let opt_bin = user.settings.join("opt/bin");
    assert!(
        stdout.contains(&format!("PATH_VALUE=<{}:", opt_bin.display())),
        "{stdout}"
    );
    assert!(
        stdout.contains("USE_VALUE=<editor=org.example/vim@9.1.0;tool=org.example/beta@1.0.0>"),
        "{stdout}"
    );
    assert!(
        stdout.contains("BETA_RAN session"),
        "the session choice wins: {stdout}"
    );
    let global = Command::new(opt_bin.join("tool"))
        .env_remove("VIBE_APP_USE")
        .output()
        .unwrap();
    assert!(
        text(&global).contains("ALPHA_RAN"),
        "the global default is untouched"
    );
}

#[test]
fn a_parent_bash_is_reused_as_the_shell() {
    let Some(bash) = ["/bin/bash", "/usr/bin/bash"]
        .into_iter()
        .map(PathBuf::from)
        .find(|path| path.is_file())
    else {
        eprintln!("skipping: bash is unavailable");
        return;
    };
    let (root, user) = installed();
    let runner = pty_runner(root.path());
    let home = tempfile::tempdir().unwrap();
    let output = in_pty(
        &runner,
        &user,
        home.path(),
        &[
            bash.to_str().unwrap(),
            "--noprofile",
            "--norc",
            "-c",
            "\"$1\" shell -g; printf 'PARENT_GOT=<%s>\\n' \"$?\"",
            "parent",
            vibe(),
        ],
        "printf 'SHELL_EXE=<%s>\\n' \"$(readlink /proc/$$/exe)\"\nexit 5\n",
    )
    .output()
    .unwrap();
    let stdout = text(&output);
    let expected = fs::canonicalize(&bash).unwrap();
    assert!(
        stdout.contains(&format!("SHELL_EXE=<{}>", expected.display())),
        "{stdout}"
    );
    assert!(
        stdout.contains("PARENT_GOT=<5>"),
        "the shell's status returns: {stdout}"
    );
}

#[test]
fn json_and_non_terminal_use_are_refused() {
    let user = UserScratch::new();
    let json = user.vibe().args(["shell", "--json"]).output().unwrap();
    assert!(!json.status.success());
    assert!(
        text(&json).contains("--json and --unattended are refused"),
        "{}",
        text(&json)
    );
    let piped = user
        .vibe()
        .args(["shell", "--use", "tool=org.example/beta@1.0.0"])
        .env_remove("VIBE_UNATTENDED")
        .write_stdin("exit 0\n")
        .output()
        .unwrap();
    assert!(!piped.status.success());
    assert!(
        text(&piped).contains("attached to a terminal"),
        "{}",
        text(&piped)
    );
}

fn text(output: &Output) -> String {
    format!(
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

const PTY_RUNNER_SOURCE: &str = include_str!("common/pty-runner.c");
