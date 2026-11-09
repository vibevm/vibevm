//! Whole payload directories on PATH and the slot off switch
//! (PROP-059 `#path-directories`, `#slot-off`), end to end through the real
//! `vibe` binary and a local directory registry.
//!
//! The runtime this feature exists for writes new commands into its own
//! payload after installation (`npm install -g`), so the fixture simulates
//! exactly that: a file dropped into the payload `bin` once the package is
//! installed must be reachable through the slot's link without any further
//! vibe command.

#![cfg(all(test, target_os = "linux"))]

mod common;

use std::fs;
use std::path::{Path, PathBuf};

use common::app_registry::{AppRegistry, ok};
use serde_json::Value;

/// A runtime package whose option exposes its payload `bin` whole, and
/// declares `node`-style launchers so `vibe app run` keeps working.
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
    r.script(&root, "rt", marker);
}

fn link(r: &AppRegistry, index: u32) -> PathBuf {
    r.settings()
        .join("opt/paths/runtime")
        .join(index.to_string())
}

/// Run a command found through the slot's link directory alone.
fn through_link(r: &AppRegistry, command: &str) -> std::process::Output {
    std::process::Command::new(link(r, 0).join(command))
        .output()
        .unwrap()
}

/// The `env.sh` line vibe wrote for the slot's link directories.
fn env_script(r: &AppRegistry) -> String {
    fs::read_to_string(r.settings().join("env.sh")).unwrap_or_default()
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-059#path-directories")]
fn a_payload_directory_goes_on_path_whole_follows_the_default_and_reaches_later_files() {
    let r = AppRegistry::new();
    runtime(&r, "1.0.0", "one");
    runtime(&r, "2.0.0", "two");

    ok(&r.app(
        &["install", "-g"],
        "org.example/runtime@1.0.0",
        &["--offline", "--unattended", "--json"],
    ));
    // The index records the declaration and the slot's applied PATH entry,
    // and the writer moved to the protocol that can carry them.
    let index = r.index();
    assert_eq!(index["protocol"], "vibe-user-applications/6");
    assert_eq!(
        index["installations"]["org.example/runtime@1.0.0"]["pathDirs"],
        serde_json::json!(["bin"])
    );
    let entry = &index["slots"]["runtime"]["pathEnv"][0];
    assert_eq!(entry["op"], "path_prepend");
    assert_eq!(entry["value"], link(&r, 0).display().to_string());

    // The link is there and runs the payload.
    let out = through_link(&r, "rt");
    assert!(out.status.success(), "{:?}", out);
    assert!(String::from_utf8_lossy(&out.stdout).starts_with("one"));

    // The durable environment carries the link directory.
    let script = env_script(&r);
    assert!(
        script.contains(&link(&r, 0).display().to_string()),
        "env.sh: {script}"
    );
    assert!(script.contains("vibe-app slot:runtime"), "env.sh: {script}");

    // A command written into the payload after installation — what a global
    // npm install does — is reachable through the link, with no vibe
    // command in between.
    let payload = fs::read_link(link(&r, 0)).unwrap();
    let later = payload.join("later");
    fs::write(&later, "#!/bin/sh\necho later-one\n").unwrap();
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&later, fs::Permissions::from_mode(0o755)).unwrap();
    }
    let out = through_link(&r, "later");
    assert!(out.status.success());
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "later-one");

    // A second provider, then a switch: the one link repoints.
    ok(&r.app(
        &["install", "-g"],
        "org.example/runtime@2.0.0",
        &["--offline", "--unattended", "--json"],
    ));
    ok(&r.vibe(&["app", "use", "runtime", "org.example/runtime@1.0.0"]));
    assert!(
        String::from_utf8_lossy(&through_link(&r, "rt").stdout).starts_with("one"),
        "the link follows the chosen default"
    );
    ok(&r.vibe(&["app", "use", "runtime", "org.example/runtime@2.0.0"]));
    assert!(String::from_utf8_lossy(&through_link(&r, "rt").stdout).starts_with("two"));
    // One link directory only: switching never multiplies PATH entries.
    assert_eq!(
        r.index()["slots"]["runtime"]["pathEnv"]
            .as_array()
            .unwrap()
            .len(),
        1
    );

    // A storage move repoints the link at the payload's new place.
    let before = fs::read_link(link(&r, 0)).unwrap();
    let library = r.root.path().join("library");
    ok(&r.vibe(&["storage", "add", library.to_str().unwrap()]));
    let listed: Value =
        serde_json::from_slice(&r.vibe(&["storage", "list", "--json"]).stdout).unwrap();
    let moved_to = listed["libraries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["id"].as_str().unwrap().to_owned())
        .find(|id| id != "default")
        .expect("the attached library is listed");
    ok(&r.vibe(&[
        "storage",
        "move",
        "org.example/runtime@2.0.0",
        "--to",
        &moved_to,
    ]));
    let after = fs::read_link(link(&r, 0)).unwrap();
    assert_ne!(before, after, "the move repointed the link");
    assert!(after.starts_with(&library), "{}", after.display());
    assert!(String::from_utf8_lossy(&through_link(&r, "rt").stdout).starts_with("two"));

    // The last provider going takes the link, its directory and its PATH
    // entry with it.
    ok(&r.vibe(&[
        "uninstall",
        "-g",
        "org.example/runtime@1.0.0",
        "--unattended",
        "--json",
    ]));
    assert!(link(&r, 0).exists(), "one provider is left");
    ok(&r.vibe(&[
        "uninstall",
        "-g",
        "org.example/runtime@2.0.0",
        "--unattended",
        "--json",
    ]));
    assert!(!link(&r, 0).exists());
    assert!(!r.settings().join("opt/paths/runtime").exists());
    assert!(
        !env_script(&r).contains("slot:runtime"),
        "env.sh: {}",
        env_script(&r)
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-059#slot-off")]
fn switching_a_slot_off_falls_through_to_the_system_program_and_on_restores_it() {
    let r = AppRegistry::new();
    // A "system" program of the same name, later on PATH than opt/bin.
    let system = r.root.path().join("system");
    fs::create_dir_all(&system).unwrap();
    let systemd_rt = system.join("rt");
    fs::write(&systemd_rt, "#!/bin/sh\necho system-rt\n").unwrap();
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&systemd_rt, fs::Permissions::from_mode(0o755)).unwrap();
    }
    // A package with both surfaces: a public command and a payload dir.
    let root = r.package(
        "runtime",
        "1.0.0",
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
[[installer_options.public_commands]]
name='rt'
launcher='main'
"#,
    );
    r.script(&root, "rt", "payload");
    ok(&r.app(
        &["install", "-g"],
        "org.example/runtime",
        &["--offline", "--unattended", "--json"],
    ));
    let proxy = r.settings().join("opt/bin/rt");
    assert!(proxy.is_file());
    assert!(link(&r, 0).exists());

    // Off: the proxy and the link go, the installation and the remembered
    // default stay, and `app list` / `app alternatives` say so.
    ok(&r.vibe(&["app", "use", "runtime", "--none"]));
    assert!(!proxy.exists(), "the slot's proxies are withdrawn");
    assert!(!link(&r, 0).exists(), "the slot's link is withdrawn");
    assert!(
        r.settings().join("opt/paths/runtime").is_dir(),
        "the PATH entry keeps working, so the lookup falls through"
    );
    let index = r.index();
    assert_eq!(index["slots"]["runtime"]["off"], true);
    assert_eq!(
        index["slots"]["runtime"]["default"],
        "org.example/runtime@1.0.0"
    );
    assert!(index["installations"]["org.example/runtime@1.0.0"].is_object());

    let listing: Value =
        serde_json::from_str(&ok(&r.vibe(&["app", "alternatives", "runtime", "--json"]))).unwrap();
    assert_eq!(listing["off"], true);
    assert_eq!(listing["default"], "org.example/runtime@1.0.0");
    assert_eq!(
        listing["providers"][0]["path_dirs"],
        serde_json::json!(["bin"])
    );
    let list: Value = serde_json::from_str(&ok(&r.vibe(&["app", "list", "-g", "--json"]))).unwrap();
    assert_eq!(list["installations"][0]["slot_off"], true);
    let text = ok(&r.vibe(&["app", "alternatives", "runtime"]));
    assert!(text.contains("[off:"), "{text}");

    // A PATH that holds the slot's link directory, then opt/bin, then the
    // system directory: with the slot off, the system program answers.
    let path = std::env::join_paths([
        r.settings().join("opt/paths/runtime/0"),
        r.settings().join("opt/bin"),
        system.clone(),
    ])
    .unwrap();
    let out = std::process::Command::new("/bin/sh")
        .args(["-c", "command -v rt && rt"])
        .env("PATH", &path)
        .output()
        .unwrap();
    assert!(out.status.success(), "{:?}", out);
    assert!(
        String::from_utf8_lossy(&out.stdout).contains("system-rt"),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );

    // On again: both surfaces come back at the remembered default.
    ok(&r.vibe(&["app", "use", "runtime", "--default"]));
    assert!(proxy.is_file());
    assert!(link(&r, 0).exists());
    assert!(
        r.index()["slots"]["runtime"]["off"].as_bool() != Some(true),
        "the switch is back on, so the default omits it"
    );
    let out = std::process::Command::new("/bin/sh")
        .args(["-c", "rt"])
        .env("PATH", &path)
        .env_remove("VIBE_APP_USE")
        .output()
        .unwrap();
    assert!(
        String::from_utf8_lossy(&out.stdout).starts_with("payload"),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );

    // A session choice of `none` makes the proxy itself fall through,
    // without changing anything durable. `vibe shell --use SLOT=none`
    // composes exactly this PATH: the slot's link directories withdrawn,
    // `opt/bin` and the system directory left.
    let session_path =
        std::env::join_paths([r.settings().join("opt/bin"), system.clone()]).unwrap();
    let out = std::process::Command::new(&proxy)
        .env("PATH", &session_path)
        .env("VIBE_APP_USE", "runtime=none")
        .output()
        .unwrap();
    assert!(
        String::from_utf8_lossy(&out.stdout).contains("system-rt"),
        "{} / {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        r.index()["slots"]["runtime"]["off"].as_bool() != Some(true),
        "a session choice changes nothing durable"
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-059#path-directories")]
fn a_foreign_directory_at_a_link_name_refuses_the_install_and_is_left_alone() {
    let r = AppRegistry::new();
    runtime(&r, "1.0.0", "one");
    let occupied = link(&r, 0);
    fs::create_dir_all(&occupied).unwrap();
    fs::write(occupied.join("mine"), b"the operator's own").unwrap();
    let out = r.app(
        &["install", "-g"],
        "org.example/runtime@1.0.0",
        &["--offline", "--unattended", "--json"],
    );
    let said = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(!out.status.success(), "{said}");
    assert!(
        said.contains("foreign_path_link") || said.contains("not a link vibe wrote"),
        "{said}"
    );
    assert_eq!(
        fs::read(occupied.join("mine")).unwrap(),
        b"the operator's own"
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-059#path-directories")]
fn a_declaration_outside_the_payload_is_refused_by_the_manifest() {
    let r = AppRegistry::new();
    let root = r.package(
        "runtime",
        "1.0.0",
        "runtime",
        r#"
path_dirs = ['../escape']
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
    r.script(&root, "rt", "one");
    let out = r.app(
        &["install", "-g"],
        "org.example/runtime@1.0.0",
        &["--offline", "--unattended", "--json"],
    );
    let said = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(!out.status.success(), "{said}");
    assert!(said.contains("path_dirs"), "{said}");
    assert!(!Path::new(&link(&r, 0)).exists());
}
