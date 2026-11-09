//! `vibe app exec SLOT[@VER] -- COMMAND …` through the real `vibe` binary
//! (PROP-075 `#VERSION-EXEC-SPELLING`): the command runs with the chosen
//! version first on PATH, its exit status passes through unchanged, and
//! nothing about the home changes.

#![cfg(all(test, target_os = "linux"))]

mod common;

use std::fs;
use std::path::PathBuf;

use common::app_registry::{AppRegistry, failed, ok};

/// Two versions of a runtime that exposes its payload `bin` whole and reads
/// `.runtime-version`.
fn runtime(r: &AppRegistry, version: &str, marker: &str) {
    let root = r.package(
        "runtime",
        version,
        "runtime",
        r#"
path_dirs = ['bin']
[[installer_options.steps]]
id='copy'
title='Copy payload'
kind='copy'
from='payload'
[[installer_options.launchers]]
id='main'
name='Runtime'
executable='$app_bin/rt'
kind='command'
default=true
"#,
    );
    let manifest = root.join("vibe.toml");
    let text = fs::read_to_string(&manifest).unwrap();
    fs::write(
        &manifest,
        text.replace(
            "[application]\nid = \"runtime\"\n",
            "[application]\nid = \"runtime\"\n\n\
             [[application.version_file]]\nformat = \"plain\"\n\
             names = [\".runtime-version\"]\n",
        ),
    )
    .unwrap();
    r.script(&root, "rt", marker);
}

fn install(r: &AppRegistry, spelling: &str) {
    ok(&r.app(
        &["install", "-g"],
        spelling,
        &["--offline", "--unattended", "--json"],
    ));
}

fn project(r: &AppRegistry) -> PathBuf {
    let dir = r.root.path().join("app");
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("vibe.toml"),
        "[project]\nname = \"app\"\nversion = \"1.0.0\"\n",
    )
    .unwrap();
    dir
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSION-EXEC-SPELLING")]
fn exec_runs_the_named_version_and_the_default_stays_where_it_was() {
    let r = AppRegistry::new();
    runtime(&r, "1.0.0", "one");
    runtime(&r, "2.0.0", "two");
    install(&r, "org.example/runtime@1.0.0");
    install(&r, "org.example/runtime@2.0.0");
    ok(&r.vibe(&["app", "use", "runtime", "org.example/runtime@2.0.0"]));
    let link = r.settings().join("opt/paths/runtime/0");
    let before = fs::read_link(&link).unwrap();

    // `sh -c 'rt'` finds `rt` through the PATH the exec composed.
    let out = ok(&r.vibe(&[
        "app",
        "exec",
        "runtime@1.0.0",
        "--",
        "sh",
        "-c",
        "rt; echo use=$VIBE_APP_USE",
    ]));
    assert!(out.contains("one"), "{out}");
    assert!(
        out.contains("use=runtime=org.example/runtime@1.0.0"),
        "{out}"
    );
    // A requirement picks the newest matching installation.
    let out = ok(&r.vibe(&["app", "exec", "runtime@^1", "--", "rt"]));
    assert!(out.starts_with("one"), "{out}");
    // Nothing changed: not the link, not the index.
    assert_eq!(fs::read_link(&link).unwrap(), before);
    assert_eq!(
        r.index()["slots"]["runtime"]["default"],
        "org.example/runtime@2.0.0"
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSION-EXEC-SPELLING")]
fn exec_passes_the_exit_status_through_unchanged() {
    let r = AppRegistry::new();
    runtime(&r, "1.0.0", "one");
    install(&r, "org.example/runtime@1.0.0");
    for code in [0, 1, 7, 42] {
        let out = r.vibe(&[
            "app",
            "exec",
            "runtime@1.0.0",
            "--",
            "sh",
            "-c",
            &format!("exit {code}"),
        ]);
        assert_eq!(out.status.code(), Some(code), "{out:?}");
    }
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSION-EXEC-SPELLING")]
fn exec_forwards_a_signal_because_the_command_replaces_the_process() {
    let r = AppRegistry::new();
    runtime(&r, "1.0.0", "one");
    install(&r, "org.example/runtime@1.0.0");
    // On a POSIX host `vibe app exec` execs the command, so there is no
    // parent left that could swallow a signal: killing the pid kills the
    // command, and the shell reports the signal, not an exit code.
    let out = r.vibe(&[
        "app",
        "exec",
        "runtime@1.0.0",
        "--",
        "sh",
        "-c",
        "kill -TERM $$",
    ]);
    use std::os::unix::process::ExitStatusExt;
    assert_eq!(out.status.signal(), Some(15), "{out:?}");
    assert_eq!(out.status.code(), None, "{out:?}");
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSION-EXEC-SPELLING")]
fn exec_without_a_version_resolves_the_version_file_of_the_directory() {
    let r = AppRegistry::new();
    runtime(&r, "1.0.0", "one");
    runtime(&r, "2.0.0", "two");
    install(&r, "org.example/runtime@1.0.0");
    install(&r, "org.example/runtime@2.0.0");
    ok(&r.vibe(&["app", "use", "runtime", "org.example/runtime@2.0.0"]));
    let dir = project(&r);
    fs::write(dir.join(".runtime-version"), "1.0.0\n").unwrap();
    let out = ok(&r.vibe_in(&dir, &["app", "exec", "runtime", "--", "rt"]));
    assert!(out.starts_with("one"), "{out}");
    // Outside the project the slot's global default answers.
    let out = ok(&r.vibe(&["app", "exec", "runtime", "--", "rt"]));
    assert!(out.starts_with("two"), "{out}");
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSION-EXEC-SPELLING")]
fn every_refusal_comes_before_any_effect_and_names_its_fix() {
    let r = AppRegistry::new();
    runtime(&r, "1.0.0", "one");
    install(&r, "org.example/runtime@1.0.0");

    // An unknown slot.
    let error = failed(&r.vibe(&["app", "exec", "nosuch", "--", "true"]));
    assert!(error.contains("unknown_slot"), "{error}");

    // A version that is not installed, with the exact install command.
    let error = failed(&r.vibe(&["app", "exec", "runtime@9.9.9", "--", "true"]));
    assert!(error.contains("not_installed"), "{error}");
    assert!(
        error.contains("vibe install -g org.example/runtime@9.9.9"),
        "{error}"
    );

    // No command after `--`.
    let error = failed(&r.vibe(&["app", "exec", "runtime@1.0.0"]));
    assert!(error.contains("`--`"), "{error}");

    // A slot that is off.
    ok(&r.vibe(&["app", "use", "runtime", "--none"]));
    let error = failed(&r.vibe(&["app", "exec", "runtime", "--", "true"]));
    assert!(error.contains("not_installed"), "{error}");
    assert!(error.contains("--default"), "{error}");
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSION-EXEC-SPELLING")]
fn exec_is_not_app_run_and_takes_any_command() {
    let r = AppRegistry::new();
    runtime(&r, "1.0.0", "one");
    install(&r, "org.example/runtime@1.0.0");
    // `vibe app run` runs one declared launcher; `exec` runs whatever the
    // caller names, with the installation's directories in front of PATH.
    let out = ok(&r.vibe(&[
        "app",
        "exec",
        "runtime@1.0.0",
        "--",
        "sh",
        "-c",
        "command -v rt",
    ]));
    assert!(
        fs::canonicalize(out.trim())
            .unwrap()
            .to_string_lossy()
            .contains("runtime@1.0.0"),
        "{out}"
    );
}
