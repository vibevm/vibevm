//! Public Linux fixture for slot alternatives, PATH proxies and session
//! choices (PROP-059 `#alternatives`, `#proxy`, `#session-selection`): two
//! providers of one slot, a global switch, per-process `VIBE_APP_USE`
//! choices, a refused command claim from another slot, default handover on
//! uninstall and an unavailable library seen through the proxy.

#![cfg(target_os = "linux")]

mod common;

use std::ffi::OsString;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use common::UserScratch;
use serde_json::Value;
use sha2::{Digest, Sha256};
use vibe_wire::generated::application_alternatives::ApplicationAlternatives;

const ALPHA: &str = "org.example/alpha@1.0.0";
const BETA: &str = "org.example/beta@1.0.0";

/// One local registry holding every package a test installs.
struct Registry {
    root: tempfile::TempDir,
}

impl Registry {
    fn new() -> Self {
        Self {
            root: tempfile::tempdir().unwrap(),
        }
    }

    fn path(&self) -> PathBuf {
        self.root.path().join("registry")
    }

    /// `org.example/<name>@1.0.0` in `slot`, a zip whose `bin/tool` prints
    /// `<MARKER> <args>`, exposing public command `command`.
    fn package(&self, name: &str, slot: &str, command: &str, extra: &str) {
        let dir = self.path().join(format!("org.example/{name}/v1.0.0"));
        fs::create_dir_all(&dir).unwrap();
        let marker = name.to_uppercase();
        let mut zip = zip::ZipWriter::new(fs::File::create(dir.join("tool.zip")).unwrap());
        let options = zip::write::SimpleFileOptions::default().unix_permissions(0o755);
        zip.start_file("bin/tool", options).unwrap();
        zip.write_all(format!("#!/bin/sh\necho \"{marker} $*\"\n").as_bytes())
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
                 [application]\nid = \"{slot}\"\n\n\
                 [[installer_options]]\nid = \"zip\"\nname = \"Zip\"\nplatform = \"linux\"\nplatform_name = \"Linux\"\narch = \"{arch}\"\n{extra}\n\
                 [installer_options.installation]\ntype = \"zip\"\nsource = \"tool.zip\"\nsha256 = \"{sha}\"\n\n\
                 [[installer_options.launchers]]\nid = \"main\"\nname = \"Tool\"\nexecutable = \"$app_bin/tool\"\nkind = \"command\"\nargs = [\"fixed\"]\ndefault = true\n\n\
                 [[installer_options.public_commands]]\nname = \"{command}\"\nlauncher = \"main\"\n",
                arch = std::env::consts::ARCH,
            ),
        )
        .unwrap();
    }

    fn vibe(&self, user: &UserScratch, args: &[&str]) -> Output {
        user.vibe()
            .current_dir(self.root.path())
            .args(args)
            .output()
            .unwrap()
    }

    fn install(&self, user: &UserScratch, package: &str, extra: &[&str]) -> Output {
        let registry = self.path();
        let mut args = vec!["install", "-g", package, "--registry"];
        args.push(registry.to_str().unwrap());
        args.extend_from_slice(extra);
        self.vibe(user, &args)
    }
}

fn ok(output: &Output) -> String {
    assert!(
        output.status.success(),
        "status={} stdout={} stderr={}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn failed(output: &Output) -> String {
    assert!(
        !output.status.success(),
        "unexpected success: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn json(output: &Output) -> Value {
    serde_json::from_str(&ok(output)).unwrap()
}

/// `tool` resolved through PATH with `opt/bin` first, as a shell would.
fn tool(user: &UserScratch, session: Option<&str>) -> Command {
    let mut path = OsString::from(user.settings.join("opt/bin"));
    path.push(":");
    path.push(std::env::var_os("PATH").unwrap_or_default());
    let mut command = Command::new("tool");
    command
        .arg("arg")
        .env("PATH", path)
        .env_remove("VIBE_APP_USE");
    if let Some(value) = session {
        command.env("VIBE_APP_USE", value);
    }
    command
}

fn run_tool(user: &UserScratch, session: Option<&str>) -> String {
    ok(&tool(user, session).output().unwrap())
}

fn proxy_image() -> Vec<u8> {
    fs::read(env!("CARGO_BIN_EXE_vibe-app-proxy")).unwrap()
}

#[test]
fn two_providers_switch_globally_and_per_process_and_hand_over_on_uninstall() {
    let registry = Registry::new();
    let user = UserScratch::new();
    let hook_marker = registry.root.path().join("gamma-hook-ran");
    registry.package("alpha", "tool", "tool", "");
    registry.package("beta", "tool", "tool", "");
    registry.package(
        "gamma",
        "other",
        "tool",
        &format!("pre_install = \"touch '{}'\"\n", hook_marker.display()),
    );

    let report = json(&registry.install(&user, "org.example/alpha", &["--json"]));
    assert_eq!(
        report["slotDefault"], ALPHA,
        "the first provider is the default"
    );
    let report = json(&registry.install(&user, "org.example/beta", &["--json"]));
    assert_eq!(report["slotDefault"], ALPHA);
    let proxy = user.settings.join("opt/bin/tool");
    assert_eq!(fs::read(&proxy).unwrap(), proxy_image());
    let unrelated = user.settings.join("opt/bin/unrelated");
    fs::write(&unrelated, "a version-manager shim").unwrap();

    assert_eq!(run_tool(&user, None), "ALPHA fixed arg\n");
    ok(&registry.vibe(&user, &["app", "use", "tool", BETA]));
    assert_eq!(run_tool(&user, None), "BETA fixed arg\n");
    let refused = failed(&registry.vibe(&user, &["app", "use", "tool", "org.example/none@1.0.0"]));
    assert!(refused.contains("not an installed provider"), "{refused}");

    // Two concurrent processes with different session choices; the global
    // default stays BETA for everyone else.
    let first = tool(&user, Some(&format!("tool={ALPHA}")))
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let second = tool(&user, Some(&format!(" ; tool = {BETA} ;")))
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    assert_eq!(ok(&first.wait_with_output().unwrap()), "ALPHA fixed arg\n");
    assert_eq!(ok(&second.wait_with_output().unwrap()), "BETA fixed arg\n");
    assert_eq!(run_tool(&user, None), "BETA fixed arg\n");
    let malformed = tool(&user, Some("tool")).output().unwrap();
    assert_eq!(malformed.status.code(), Some(127));
    assert!(String::from_utf8_lossy(&malformed.stderr).starts_with("vibe-app-proxy: malformed"));

    let listing = json(&registry.vibe(&user, &["app", "alternatives", "tool", "--json"]));
    let typed: ApplicationAlternatives = serde_json::from_value(listing.clone()).unwrap();
    assert_eq!(typed.default.as_deref(), Some(BETA));
    let keys: Vec<&str> = typed.providers.iter().map(|row| row.key.as_str()).collect();
    assert_eq!(keys, [ALPHA, BETA]);
    assert!(typed.providers.iter().all(|row| row.commands == ["tool"]));
    assert_eq!(typed.commands.len(), 1);
    assert_eq!(typed.commands[0].providers, [ALPHA, BETA]);
    let text = ok(&registry.vibe(&user, &["app", "alternatives", "tool"]));
    assert!(text.contains(&format!("{BETA} [default]")), "{text}");

    let refused = failed(&registry.install(&user, "org.example/gamma", &[]));
    assert!(
        refused.contains("`tool` is reserved by slot `tool`"),
        "{refused}"
    );
    assert!(!hook_marker.exists(), "refused before any child runs");
    assert!(!user.settings.join("opt/apps/other").exists());
    let index: Value =
        serde_json::from_slice(&fs::read(user.settings.join("applications/index.json")).unwrap())
            .unwrap();
    assert!(index["installations"]["org.example/gamma@1.0.0"].is_null());

    let report = json(&registry.vibe(&user, &["uninstall", "-g", BETA, "--json"]));
    assert_eq!(report["slotDefault"], ALPHA, "the only remaining provider");
    assert!(report["message"].as_str().unwrap().contains(ALPHA));
    assert_eq!(run_tool(&user, None), "ALPHA fixed arg\n");
    ok(&registry.vibe(&user, &["uninstall", "-g", ALPHA]));
    assert!(!proxy.exists(), "the last provider takes its proxy along");
    assert!(unrelated.is_file(), "nothing else in opt/bin is touched");
}

#[test]
fn an_unavailable_library_makes_the_proxy_exit_127() {
    let registry = Registry::new();
    let user = UserScratch::new();
    registry.package("alpha", "tool", "tool", "");
    let library = registry.root.path().join("usb");
    fs::create_dir_all(library.join("apps")).unwrap();
    fs::write(
        library.join("vibe-library.toml"),
        "id = \"usb\"\ncreated = \"2026-10-30T00:00:00Z\"\n",
    )
    .unwrap();
    fs::create_dir_all(&user.settings).unwrap();
    fs::write(
        user.settings.join("storage.toml"),
        format!(
            "[[library]]\nid = \"usb\"\npath = {}\n",
            toml::Value::String(library.display().to_string())
        ),
    )
    .unwrap();
    ok(&registry.install(&user, "org.example/alpha", &["--library", "usb"]));
    assert!(
        library
            .join("apps/tool/org.example.alpha@1.0.0/bin/tool")
            .is_file()
    );
    assert_eq!(run_tool(&user, None), "ALPHA fixed arg\n");

    let moved: &Path = &registry.root.path().join("usb-moved");
    fs::rename(&library, moved).unwrap();
    let output = tool(&user, None).output().unwrap();
    assert_eq!(output.status.code(), Some(127));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.starts_with("vibe-app-proxy: unavailable: library directory"),
        "{stderr}"
    );

    fs::rename(moved, &library).unwrap();
    assert_eq!(run_tool(&user, None), "ALPHA fixed arg\n");
}
