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
             [[installer_options.launchers]]\nid = \"main\"\nname = \"Tool\"\nexecutable = \"$app_bin/tool\"\nkind = \"command\"\ndefault = true\n\n\
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
            .env("HOME", root.path())
            .env("USERPROFILE", root.path())
            .env("VIBE_SETTINGS", &user.settings)
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
    let source_text = PTY_RUNNER_SOURCE
        .replace(".ws_row = 24, .ws_col = 200", ".ws_row = 40, .ws_col = 100")
        .replace("int master = -1;", "alarm(30); int master = -1;")
        .replace("close(master);", "alarm(25); close(master);");
    fs::write(&source, source_text).unwrap();
    let output = Command::new("cc")
        .env("HOME", scope)
        .env("USERPROFILE", scope)
        .env("VIBE_SETTINGS", scope.join("compiler-settings"))
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
        .env("USERPROFILE", home)
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
        text(&json).contains("not_a_terminal"),
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

#[test]
fn shell_missing_selection_starts_anyway() {
    use vibe_app::{ActivationRequest, ActivationState, Index, SettingsRoot};
    let (root, user) = installed();
    let runner = pty_runner(root.path());
    let home = tempfile::tempdir().unwrap();
    let missing = in_pty(
        &runner, &user, home.path(),
        &[vibe(), "shell", "-g", "--use", "node=22", "--use", "node=22"],
        "printf '%s%s\\n' F30_ MISSING_CHILD\nexit 0\n",
    ).current_dir(root.path()).env_remove("VIBE_APP_USE").output().unwrap();
    assert!(missing.status.success(), "{}", text(&missing));
    let stdout = String::from_utf8_lossy(&missing.stdout).replace('\r', "");
    assert_eq!(stdout.matches("node wants 22, which is not installed").count(), 1, "{stdout}");
    let warning = stdout.find("node wants 22, which is not installed").unwrap();
    let remedy = stdout.find("  vibe install -g node@22").unwrap();
    let continuation = stdout.find("  Continuing without this version.").unwrap();
    let marker = stdout.find("F30_MISSING_CHILD").unwrap();
    assert!(warning < remedy && remedy < continuation && continuation < marker, "{stdout}");

    for flags in [vec![], vec!["--unattended"], vec!["--json"]] {
        let refused = user.vibe().args(["shell", "--use", "node=22"])
            .args(&flags).env("HOME", home.path()).env("USERPROFILE", home.path())
            .env("VIBE_SETTINGS", &user.settings).env_remove("VIBE_UNATTENDED")
            .current_dir(root.path()).output().unwrap();
        assert!(!refused.status.success(), "{}", text(&refused));
        assert!(refused.stdout.is_empty(), "{}", text(&refused));
        let stderr = String::from_utf8_lossy(&refused.stderr);
        assert!(stderr.contains("not_a_terminal"), "{stderr}");
        assert!(stderr.contains("attached to a terminal"), "{stderr}");
        if flags == ["--json"] {
            let document: serde_json::Value = serde_json::from_slice(&refused.stderr).unwrap();
            assert_eq!(document["ok"], false);
            assert!(document["error"].as_str().unwrap().contains("not_a_terminal"));
        } else {
            assert!(stderr.contains("fix   eval \"$(vibe app env --use node=22)\""), "{stderr}");
            assert!(stderr.contains("spec://org.vibevm.core/vibevm/common/PROP-059#shell-command"), "{stderr}");
        }
    }

    let settings = SettingsRoot::new(&user.settings);
    let index = Index::load(&settings).unwrap();
    let uses = vec!["tool=org.example/beta@1.0.0".to_owned()];
    let request = ActivationRequest {
        uses: &uses, directory: root.path(), project: None, requirements: &[],
        consult_project: false, channels: None,
    };
    assert_eq!(vibe_app::resolve_activation(&settings, &index, &request).unwrap()
        .of_slot("tool").unwrap().state, ActivationState::Selected);
    assert!(vibe_app::slot_session_dirs(&settings, &index,
        &vibe_app::Storage::load(&settings).unwrap(), "org.example/beta@1.0.0")
        .unwrap().is_empty(), "the selected-provider control must have zero bins");
    let project = root.path().join("project");
    fs::create_dir_all(&project).unwrap();
    fs::write(project.join("vibe.toml"), "[project]\nname = \"shell-control\"\nversion = \"0.0.1\"\n").unwrap();
    let project_home = SettingsRoot::project(&project, &settings);
    fs::create_dir_all(project_home.path()).unwrap();
    assert!(Index::load(&project_home).unwrap().installations.is_empty());
    for (directory, global, marker) in [
        (root.path(), true, "F30_ZERO_BINS_CHILD"),
        (project.as_path(), false, "F30_SECOND_HOME_CHILD"),
    ] {
        let mut args = vec![vibe(), "shell", "--use", uses[0].as_str()];
        if global { args.push("-g"); }
        let input = format!("printf '%s%s\\n' F30_ {}\nexit 0\n", marker.strip_prefix("F30_").unwrap());
        let output = in_pty(&runner, &user, home.path(), &args, &input)
            .current_dir(directory).env_remove("VIBE_APP_USE").output().unwrap();
        assert!(output.status.success(), "{}", text(&output));
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains(marker), "{stdout}");
        assert!(!stdout.contains("not installed"), "{stdout}");
        assert!(!stdout.contains("vibe install"), "{stdout}");
    }
}

/// PROP-059 `#hidden-installations`, `#session-selection`: `--with` puts a
/// hidden installation on one shell's PATH through the same `VIBE_APP_USE`
/// choice `--use` makes; outside the shell nothing changed.
#[test]
fn with_exposes_a_hidden_installation_for_one_session() {
    let root = tempfile::tempdir().unwrap();
    let user = UserScratch::new();
    let registry = root.path().join("registry");
    for (name, extra) in [("alpha", None), ("beta", Some("--hidden"))] {
        package(&registry, name);
        let mut command = user.vibe();
        command
            .args([
                "install",
                "-g",
                &format!("org.example/{name}"),
                "--registry",
            ])
            .arg(&registry);
        if let Some(extra) = extra {
            command.arg(extra);
        }
        let output = command.output().unwrap();
        assert!(output.status.success(), "{}", text(&output));
    }
    let alternatives = user
        .vibe()
        .args(["app", "alternatives", "tool", "--json"])
        .output()
        .unwrap();
    let listing: serde_json::Value = serde_json::from_slice(&alternatives.stdout).unwrap();
    assert_eq!(
        listing["providers"].as_array().unwrap().len(),
        1,
        "a hidden installation has no place among the alternatives"
    );
    let runner = pty_runner(root.path());
    let home = tempfile::tempdir().unwrap();
    let output = in_pty(
        &runner,
        &user,
        home.path(),
        &[vibe(), "shell", "-g", "--with", "org.example/beta"],
        "printf 'PATH_VALUE=<%s>\\n' \"$PATH\"\nprintf 'USE_VALUE=<%s>\\n' \"$VIBE_APP_USE\"\ntool session\nexit 3\n",
    )
    .env_remove("VIBE_APP_USE")
    .output()
    .unwrap();
    assert_eq!(output.status.code(), Some(3), "{}", text(&output));
    let stdout = text(&output);
    let session = user
        .settings
        .join("cache/app-proxies/org.example.beta@1.0.0/bin");
    assert!(
        stdout.contains(&format!("PATH_VALUE=<{}:", session.display())),
        "{stdout}"
    );
    assert!(
        stdout.contains("USE_VALUE=<tool=org.example/beta@1.0.0>"),
        "{stdout}"
    );
    assert!(stdout.contains("BETA_RAN session"), "{stdout}");
    let global = Command::new(user.settings.join("opt/bin/tool"))
        .env_remove("VIBE_APP_USE")
        .output()
        .unwrap();
    assert!(
        text(&global).contains("ALPHA_RAN"),
        "the default is untouched"
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
