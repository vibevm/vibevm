//! `vibe app env` and the version-file precedence, end to end through the
//! real `vibe` binary and a local directory registry
//! (PROP-075 `#VERSION-FILE-FORMATS`, `#ACTIVATION-ENV-OUTPUT`).
//!
//! Two real application shapes, because the two halves of PROP-059's PATH
//! story resolve differently: a runtime that exposes a whole payload
//! directory (`path_dirs`, what Node needs) and a tool that claims one
//! public command (`public_commands`, what a single-binary tool needs).
//!
//! The campaign's acceptance names this file `cli_app_env`; that test
//! target is already taken by the PROP-065 env-step fixture, so the name
//! carries `_activation`. Recorded in `campaigns/app-versions-2026-11/LOG.md`.

#![cfg(all(test, target_os = "linux"))]

mod common;

use std::fs;
use std::path::{Path, PathBuf};

use common::app_registry::{AppRegistry, failed, json, ok};
use serde_json::Value;

/// A runtime package whose option exposes its payload `bin` whole and which
/// declares the two plain version files Node's slot would.
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
    // The package declares what it reads; the home records it beside the
    // index at install time.
    let manifest = root.join("vibe.toml");
    let text = fs::read_to_string(&manifest).unwrap();
    fs::write(
        &manifest,
        text.replace(
            "[application]\nid = \"runtime\"\n",
            "[application]\nid = \"runtime\"\n\n\
             [[application.version_file]]\nformat = \"plain\"\n\
             names = [\".runtime-version\", \".rtrc\"]\n\n\
             [[application.version_file]]\nformat = \"tool-versions\"\nkey = \"runtime\"\n",
        ),
    )
    .unwrap();
    r.script(&root, "rt", marker);
}

/// The second shape: one public command and no `path_dirs`, declaring one
/// plain version file.
fn tool(r: &AppRegistry, version: &str, marker: &str) {
    let root = r.package(
        "tool",
        version,
        "tool",
        r#"
[[installer_options.steps]]
id='copy'
title='Copy payload'
kind='copy'
from='payload'
[[installer_options.launchers]]
id='main'
name='Tool'
executable='$app_bin/tl'
kind='command'
default=true
[[installer_options.public_commands]]
name='tl'
launcher='main'
"#,
    );
    let manifest = root.join("vibe.toml");
    let text = fs::read_to_string(&manifest).unwrap();
    fs::write(
        &manifest,
        text.replace(
            "[application]\nid = \"tool\"\n",
            "[application]\nid = \"tool\"\n\n\
             [[application.version_file]]\nformat = \"plain\"\nnames = [\".tool-version\"]\n",
        ),
    )
    .unwrap();
    r.script(&root, "tl", marker);
}

fn install(r: &AppRegistry, spelling: &str) {
    ok(&r.app(
        &["install", "-g"],
        spelling,
        &["--offline", "--unattended", "--json"],
    ));
}

/// A project directory with a `vibe.toml`, so the upward search has a root
/// to stop at.
fn project(r: &AppRegistry, name: &str) -> PathBuf {
    let dir = r.root.path().join(name);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("vibe.toml"),
        format!("[project]\nname = \"{name}\"\nversion = \"1.0.0\"\n"),
    )
    .unwrap();
    dir
}

/// `vibe app env --project --json` in `dir`.
fn env_in(r: &AppRegistry, dir: &Path, extra: &[&str]) -> Value {
    let mut args = vec!["app", "env", "--project", "--json"];
    args.extend_from_slice(extra);
    json(&r.vibe_in(dir, &args))
}

fn selection<'a>(document: &'a Value, slot: &str) -> &'a Value {
    document["selections"]
        .as_array()
        .expect("selections")
        .iter()
        .find(|row| row["slot"] == slot)
        .unwrap_or_else(|| panic!("no selection for `{slot}` in {document:#}"))
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSION-FILE-FORMATS")]
fn a_version_file_selects_an_installed_version_and_the_index_keeps_the_declaration() {
    let r = AppRegistry::new();
    runtime(&r, "1.0.0", "one");
    runtime(&r, "2.0.0", "two");
    install(&r, "org.example/runtime@1.0.0");
    install(&r, "org.example/runtime@2.0.0");

    // The declaration is retained beside the index, and the index itself
    // stays at the protocol its payload directories need.
    let sidecar: Value = serde_json::from_slice(
        &fs::read(r.settings().join("applications/version-files.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(sidecar["protocol"], "vibe-app-version-files/1");
    let declared = &sidecar["installations"]["org.example/runtime@2.0.0"];
    assert_eq!(declared["slot"], "runtime");
    assert_eq!(declared["rows"][0]["format"], "plain");
    assert_eq!(
        declared["rows"][0]["names"],
        serde_json::json!([".runtime-version", ".rtrc"])
    );
    assert_eq!(declared["rows"][1]["format"], "tool-versions");
    assert_eq!(r.index()["protocol"], "vibe-user-applications/6");

    let dir = project(&r, "app");
    fs::write(dir.join(".runtime-version"), "# pinned\n1.0.0\n").unwrap();
    let document = env_in(&r, &dir, &[]);
    assert_eq!(document["protocol"], "vibe-app-env/1");
    let chosen = selection(&document, "runtime");
    assert_eq!(chosen["source"], "version_file");
    assert_eq!(chosen["state"], "selected");
    assert_eq!(chosen["key"], "org.example/runtime@1.0.0");
    assert_eq!(chosen["requested"], "1.0.0");
    assert!(
        chosen["file"]
            .as_str()
            .unwrap()
            .ends_with(".runtime-version")
    );
    assert_eq!(
        document["variables"]["VIBE_APP_USE"],
        "runtime=org.example/runtime@1.0.0"
    );
    // The chosen payload directory leads PATH.
    let first = document["path"][0].as_str().unwrap();
    assert!(
        fs::canonicalize(first)
            .unwrap()
            .to_string_lossy()
            .contains("runtime@1.0.0"),
        "{first}"
    );

    // A nearer file wins, and the declaration order breaks a tie.
    let deep = dir.join("sub");
    fs::create_dir_all(&deep).unwrap();
    fs::write(deep.join(".rtrc"), "2.0.0\n").unwrap();
    fs::write(deep.join(".runtime-version"), "1.0.0\n").unwrap();
    let nearer = env_in(&r, &deep, &[]);
    assert_eq!(
        selection(&nearer, "runtime")["key"],
        "org.example/runtime@1.0.0",
        "`.runtime-version` is declared before `.rtrc`"
    );
    fs::remove_file(deep.join(".runtime-version")).unwrap();
    let tie = env_in(&r, &deep, &[]);
    assert_eq!(
        selection(&tie, "runtime")["key"],
        "org.example/runtime@2.0.0"
    );

    // `.tool-versions` reads its key and reports the versions it did not use.
    fs::remove_file(deep.join(".rtrc")).unwrap();
    fs::write(
        deep.join(".tool-versions"),
        "# the team's tools\nruntime 1.0.0 2.0.0\n",
    )
    .unwrap();
    let asdf = env_in(&r, &deep, &[]);
    let chosen = selection(&asdf, "runtime");
    assert_eq!(chosen["key"], "org.example/runtime@1.0.0");
    assert_eq!(chosen["unused"], serde_json::json!(["2.0.0"]));
    assert!(
        asdf["notices"]
            .as_array()
            .unwrap()
            .iter()
            .any(|notice| notice.as_str().unwrap().contains("2.0.0")),
        "{asdf:#}"
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSION-FILE-FORMATS")]
fn the_project_requirement_decides_before_a_version_file() {
    let r = AppRegistry::new();
    runtime(&r, "1.0.0", "one");
    runtime(&r, "2.0.0", "two");
    install(&r, "org.example/runtime@1.0.0");
    install(&r, "org.example/runtime@2.0.0");

    let dir = project(&r, "app");
    fs::write(
        dir.join("vibe.toml"),
        "[project]\nname = \"app\"\nversion = \"1.0.0\"\n\n[requires.packages]\n\
         \"org.example/runtime\" = \"=1.0.0\"\n",
    )
    .unwrap();
    fs::write(dir.join(".runtime-version"), "2.0.0\n").unwrap();
    let chosen = env_in(&r, &dir, &[]);
    let row = selection(&chosen, "runtime");
    assert_eq!(row["source"], "project_requirement");
    assert_eq!(row["key"], "org.example/runtime@1.0.0");

    // And an explicit `--use` wins over the requirement.
    let flagged = env_in(&r, &dir, &["--use", "runtime=2.0.0"]);
    let row = selection(&flagged, "runtime");
    assert_eq!(row["source"], "flag");
    assert_eq!(row["key"], "org.example/runtime@2.0.0");
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#ACTIVATION-ENV-OUTPUT")]
fn a_second_application_shape_resolves_the_same_way() {
    let r = AppRegistry::new();
    tool(&r, "1.0.0", "tool-one");
    tool(&r, "3.0.0", "tool-three");
    install(&r, "org.example/tool@1.0.0");
    install(&r, "org.example/tool@3.0.0");

    let dir = project(&r, "app");
    fs::write(dir.join(".tool-version"), "1.0.0\n").unwrap();
    let document = env_in(&r, &dir, &[]);
    let row = selection(&document, "tool");
    assert_eq!(row["source"], "version_file");
    assert_eq!(row["key"], "org.example/tool@1.0.0");
    // A `public_commands`-only option exposes no payload directory, so the
    // selection carries the choice without inventing a PATH entry for it.
    assert_eq!(
        document["variables"]["VIBE_APP_USE"],
        "tool=org.example/tool@1.0.0"
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#ACTIVATION-ENV-OUTPUT")]
fn the_lines_are_the_shell_syntax_of_each_shell_and_nothing_else() {
    let r = AppRegistry::new();
    runtime(&r, "1.0.0", "one");
    install(&r, "org.example/runtime@1.0.0");
    let dir = project(&r, "app");
    fs::write(dir.join(".runtime-version"), "1.0.0\n").unwrap();

    for (shell, path_prefix, use_prefix) in [
        ("bash", "export PATH='", "export VIBE_APP_USE='"),
        ("zsh", "export PATH='", "export VIBE_APP_USE='"),
        ("posix", "export PATH='", "export VIBE_APP_USE='"),
        ("fish", "set -gx PATH '", "set -gx VIBE_APP_USE '"),
        ("pwsh", "$env:PATH = '", "$env:VIBE_APP_USE = '"),
        ("cmd", "PATH=", "VIBE_APP_USE="),
    ] {
        let out = ok(&r.vibe_in(&dir, &["app", "env", "--project", "--shell", shell]));
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines.len(), 2, "{shell}: {out}");
        assert!(lines[0].starts_with(path_prefix), "{shell}: {out}");
        assert!(lines[1].starts_with(use_prefix), "{shell}: {out}");
        assert!(
            lines[1].contains("runtime=org.example/runtime@1.0.0"),
            "{shell}: {out}"
        );
    }

    // `--unset` is nvm's `deactivate`: the variable is cleared.
    for (shell, cleared) in [
        ("bash", "unset VIBE_APP_USE;"),
        ("fish", "set -e VIBE_APP_USE;"),
        ("cmd", "VIBE_APP_USE="),
    ] {
        let out = ok(&r.vibe_in(&dir, &["app", "env", "--unset", "--shell", shell]));
        assert!(out.lines().any(|line| line == cleared), "{shell}: {out}");
    }
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#ACTIVATION-ENV-OUTPUT")]
fn app_env_and_shell_use_put_a_shell_in_the_same_state() {
    // The invariant PROP-075 asks for, asserted on the composed values:
    // `vibe app env --use X` prints exactly the PATH and VIBE_APP_USE a
    // `vibe shell --use X` child would carry. `vibe shell` needs a terminal,
    // so the comparison is against the library's own answer, which both
    // surfaces call: `vibe app env --json` reports it as data.
    let r = AppRegistry::new();
    runtime(&r, "1.0.0", "one");
    runtime(&r, "2.0.0", "two");
    install(&r, "org.example/runtime@1.0.0");
    install(&r, "org.example/runtime@2.0.0");
    let key = "org.example/runtime@1.0.0";

    let document = json(&r.vibe(&["app", "env", "--use", &format!("runtime={key}"), "--json"]));
    assert_eq!(
        document["variables"]["VIBE_APP_USE"],
        format!("runtime={key}")
    );
    let front = document["path"][0].as_str().unwrap();
    // That leading directory is the installation's own payload `bin`, which
    // is what `vibe shell --use` prepends through the same library call.
    let payload = fs::canonicalize(front).unwrap();
    assert!(payload.ends_with("bin"), "{front}");
    assert!(
        payload.to_string_lossy().contains("runtime@1.0.0"),
        "{front}"
    );
    // The slot's own link directory is untouched: a session choice
    // repoints nothing.
    let link = r.settings().join("opt/paths/runtime/0");
    let before = fs::read_link(&link).unwrap();
    ok(&r.vibe(&["app", "env", "--use", &format!("runtime={key}"), "--json"]));
    assert_eq!(
        fs::read_link(&link).unwrap(),
        before,
        "a session choice repoints nothing"
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#ACTIVATION-HOOK")]
fn a_version_that_is_not_installed_is_reported_with_its_install_command() {
    let r = AppRegistry::new();
    runtime(&r, "1.0.0", "one");
    install(&r, "org.example/runtime@1.0.0");
    let dir = project(&r, "app");
    fs::write(dir.join(".runtime-version"), "9.9.9\n").unwrap();

    // Not an error: the lines still come out, so an `eval` in a prompt hook
    // leaves the shell as it was.
    let out = r.vibe_in(&dir, &["app", "env", "--project", "--shell", "bash"]);
    assert!(out.status.success(), "{out:?}");
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    assert!(
        stderr.contains("vibe install -g org.example/runtime@9.9.9"),
        "{stderr}"
    );
    // The notice is on stderr, never on the stdout a shell evaluates.
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(!stdout.contains("vibe install"), "{stdout}");

    let document = env_in(&r, &dir, &[]);
    let row = selection(&document, "runtime");
    assert_eq!(row["state"], "not_installed");
    assert_eq!(row["install"], "vibe install -g org.example/runtime@9.9.9");

    // An unusable line is reported the same way and never fails.
    fs::write(dir.join(".runtime-version"), "not a version!\n").unwrap();
    let out = r.vibe_in(&dir, &["app", "env", "--project", "--shell", "bash"]);
    assert!(out.status.success(), "{out:?}");
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("not a version!"),
        "{out:?}"
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-059#slot-off")]
fn an_explicit_none_withdraws_the_slot_link_directory() {
    let r = AppRegistry::new();
    runtime(&r, "1.0.0", "one");
    install(&r, "org.example/runtime@1.0.0");
    let link = r.settings().join("opt/paths/runtime/0");
    let inherited = format!("{}:/usr/bin:/bin", link.display());

    let mut command = r.user.vibe();
    command
        .current_dir(r.root.path())
        .env("HOME", r.home())
        .env("PATH", &inherited)
        .env_remove("VIBE_APP_USE")
        .args(["app", "env", "--use", "runtime=none", "--json"]);
    let document: Value = serde_json::from_str(&ok(&command.output().unwrap())).unwrap();
    let path: Vec<String> = document["path"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry.as_str().unwrap().to_owned())
        .collect();
    assert!(
        !path
            .iter()
            .any(|entry| entry == &link.display().to_string()),
        "the withdrawn link directory is out: {path:?}"
    );
    assert_eq!(selection(&document, "runtime")["state"], "off");
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSION-FILE-FORMATS")]
fn the_search_stops_at_the_project_root() {
    let r = AppRegistry::new();
    runtime(&r, "1.0.0", "one");
    install(&r, "org.example/runtime@1.0.0");
    // A version file ABOVE the project root must never be read.
    fs::write(r.root.path().join(".runtime-version"), "1.0.0\n").unwrap();
    let dir = project(&r, "app");
    let deep = dir.join("a/b");
    fs::create_dir_all(&deep).unwrap();
    let document = env_in(&r, &deep, &[]);
    assert!(
        document["selections"].as_array().unwrap().is_empty(),
        "{document:#}"
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#ACTIVATION-HOOK")]
fn the_project_form_reads_no_registry_and_makes_no_network_call() {
    let r = AppRegistry::new();
    runtime(&r, "1.0.0", "one");
    install(&r, "org.example/runtime@1.0.0");
    let dir = project(&r, "app");
    fs::write(dir.join(".runtime-version"), "1.0.0\n").unwrap();
    // No `--registry`, no network: a denied transport would be the only way
    // this could pass if it reached out, and there is none configured here.
    let out = ok(&r.vibe_in(&dir, &["app", "env", "--project", "--shell", "bash"]));
    assert!(out.contains("VIBE_APP_USE"), "{out}");
    // The resolution cache is written beside the home, which is what makes
    // the second call free.
    assert!(
        r.settings().join("cache/version-files.json").is_file(),
        "the resolution cache is written"
    );
    let cache: Value =
        serde_json::from_slice(&fs::read(r.settings().join("cache/version-files.json")).unwrap())
            .unwrap();
    assert_eq!(cache["protocol"], "vibe-app-version-file-cache/1");
    assert!(!cache["entries"].as_array().unwrap().is_empty());
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#ACTIVATION-ENV-OUTPUT")]
fn an_unknown_shell_is_refused_by_name() {
    let r = AppRegistry::new();
    let error = failed(&r.vibe(&["app", "env", "--shell", "nushell"]));
    assert!(
        error.contains("nushell") && error.contains("cmd"),
        "{error}"
    );
}
