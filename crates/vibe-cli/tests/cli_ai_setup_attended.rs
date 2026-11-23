//! Attended `vibe ai setup` under a pseudo-terminal, synthetic inputs only.
#![cfg(target_os = "linux")]

use std::{
    fs,
    io::Write,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    process::{Command, Stdio},
};

struct World {
    root: tempfile::TempDir,
}

impl World {
    fn new() -> Self {
        let root = tempfile::Builder::new()
            .permissions(fs::Permissions::from_mode(0o700))
            .tempdir()
            .unwrap();
        for dir in ["settings", "home"] {
            fs::create_dir(root.path().join(dir)).unwrap();
            fs::set_permissions(root.path().join(dir), fs::Permissions::from_mode(0o700)).unwrap();
        }
        Self { root }
    }
    fn path(&self, tail: &str) -> PathBuf {
        self.root.path().join(tail)
    }
    fn command(&self, program: &str) -> Command {
        let mut command = Command::new(program);
        command
            .env_clear()
            .env("HOME", self.path("home"))
            .env("VIBE_SETTINGS", self.path("settings"))
            .env("VIBE_NO_DEFAULT_REGISTRY", "1")
            .env("PATH", "/usr/bin:/bin")
            .env("TERM", "xterm");
        command
    }
    fn ok(&self, args: &[&str]) {
        let output = self
            .command(env!("CARGO_BIN_EXE_vibe"))
            .args(args)
            .stdin(Stdio::null())
            .output()
            .unwrap();
        assert_no_keys(&String::from_utf8_lossy(&output.stdout));
        assert_no_keys(&String::from_utf8_lossy(&output.stderr));
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    fn json(&self, args: &[&str], stdin: Option<&[u8]>) -> serde_json::Value {
        let mut child = self
            .command(env!("CARGO_BIN_EXE_vibe"))
            .args(args)
            .arg("--json")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let mut input = child.stdin.take().unwrap();
        if let Some(bytes) = stdin {
            input.write_all(bytes).unwrap();
        }
        drop(input);
        let output = child.wait_with_output().unwrap();
        assert_no_keys(&String::from_utf8_lossy(&output.stdout));
        assert_no_keys(&String::from_utf8_lossy(&output.stderr));
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice(&output.stdout).unwrap()
    }
    fn setup(&self, steps: &[(&str, &str)]) -> serde_json::Value {
        let driver = self.path("ai-setup-pty.py");
        fs::write(&driver, include_str!("common/ai-setup-pty.py")).unwrap();
        let output = self
            .command("python3")
            .arg("-I")
            .arg(&driver)
            .arg(env!("CARGO_BIN_EXE_vibe"))
            .arg(serde_json::to_string(steps).unwrap())
            .output()
            .unwrap();
        assert_no_keys(&String::from_utf8_lossy(&output.stdout));
        assert_no_keys(&String::from_utf8_lossy(&output.stderr));
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_no_keys(report["transcript"].as_str().unwrap());
        report
    }
}

/// Snapshot only synthetic settings, comparing private bytes without printing them.
fn settings_bytes(root: &std::path::Path) -> Vec<(PathBuf, Vec<u8>)> {
    fn visit(root: &std::path::Path, dir: &std::path::Path, files: &mut Vec<(PathBuf, Vec<u8>)>) {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                visit(root, &path, files);
            } else {
                files.push((
                    path.strip_prefix(root).unwrap().to_owned(),
                    fs::read(path).unwrap(),
                ));
            }
        }
    }
    let mut files = Vec::new();
    visit(root, root, &mut files);
    files.sort_by(|a, b| a.0.cmp(&b.0));
    files
}

fn agent_registry(w: &World) {
    let package = w.path("registry/org.vibevm.agents/pi/v1.0.0");
    fs::create_dir_all(package.join("payload/bin")).unwrap();
    fs::write(
        package.join("vibe.toml"),
        format!(
            "[package]\nname = \"pi\"\ngroup = \"org.vibevm.agents\"\nkind = \"app\"\nversion = \"1.0.0\"\n\n\
             [application]\nid = \"pi\"\n\n\
             [[installer_options]]\nid = \"main\"\nname = \"Main\"\nplatform = \"linux\"\nplatform_name = \"Linux\"\narch = \"{}\"\n\n\
             [[installer_options.steps]]\nid = \"place\"\ntitle = \"Place pi\"\nkind = \"copy\"\nfrom = \"payload\"\n\n\
             [[installer_options.launchers]]\nid = \"main\"\nname = \"pi\"\nexecutable = \"$app_bin/pi\"\nkind = \"command\"\n\n\
             [[installer_options.public_commands]]\nname = \"pi\"\nlauncher = \"main\"\n",
            std::env::consts::ARCH
        ),
    )
    .unwrap();
    let pi = package.join("payload/bin/pi");
    fs::write(&pi, "#!/bin/sh\necho PI_RAN \"$@\"\n").unwrap();
    fs::set_permissions(&pi, fs::Permissions::from_mode(0o755)).unwrap();
    let url = format!("file://{}", w.path("registry").display());
    w.ok(&[
        "registry", "add", "agents", &url, "--type", "localdir", "--scope", "user",
    ]);
}

#[test]
fn attended_setup_installs_selected_agent_packages_with_independent_outcomes() {
    let w = World::new();
    agent_registry(&w);
    let report = w.setup(&[
        ("Install AI agent packages now?", "k\r"),
        ("Select agent packages to install", " \u{1b}[B \r"),
        ("Platform", "\r"),
        ("Architecture", "\r"),
        ("Create a default AI connection now?", "\r"),
    ]);
    let transcript = report["transcript"].as_str().unwrap();
    assert_eq!(report["status"], 0, "{transcript}");
    assert_eq!(report["terminal_restored"], true, "{transcript}");
    assert_eq!(report["sent"], 5, "{transcript}");
    assert!(
        transcript.contains("Agent package pi: installed"),
        "{transcript}"
    );
    assert!(
        transcript.contains("Agent package qwen: failed"),
        "{transcript}"
    );
    let list = w.json(&["app", "list", "-g"], None);
    assert!(
        list["installations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["key"] == "org.vibevm.agents/pi@1.0.0")
    );
}

const KEY: &str = "SYNTHETIC_ONBOARDING_KEY_NEVER_OUTPUT";

fn assert_no_keys(text: &str) {
    for key in [
        KEY,
        "SYNTHETIC_PROFILE_KEY_NEVER_OUTPUT",
        "SYNTHETIC_OLD_KEY_NEVER_OUTPUT",
        "SYNTHETIC_REPLACEMENT_KEY_NEVER_OUTPUT",
    ] {
        assert!(!text.contains(key), "synthetic protected value leaked");
    }
}

#[test]
fn attended_setup_without_default_creates_and_selects_connection() {
    let w = World::new();
    let key_line = format!("{KEY}\r");
    // Enter accepts the displayed text default; typed text replaces that default.
    // Default-no lists need k + Enter for yes, Enter alone for no.
    let report = w.setup(&[
        ("Install AI agent packages now?", "\r"),
        ("Create a default AI connection now?", "k\r"),
        ("Label", "\r"),
        ("Endpoint", "http://127.0.0.1:32123/v1\r"),
        ("API kind", "\r"),
        ("Model", "synthetic\r"),
        ("API key (hidden)", key_line.as_str()),
        ("native settings", "\r"),
        ("Configure existing connection", "\u{1b}"),
    ]);
    let transcript = report["transcript"].as_str().unwrap();
    assert_eq!(report["status"], 0, "{transcript}");
    assert_eq!(report["terminal_restored"], true, "{transcript}");
    assert_eq!(report["sent"], 9, "{transcript}");
    assert!(
        transcript.contains("created default connection Default"),
        "{transcript}"
    );
    assert!(!transcript.contains(KEY));
    let connections = w.json(&["ai", "connections", "list"], None);
    let items = connections["items"].as_object().unwrap();
    assert_eq!(items.len(), 1);
    let (id, connection) = items.iter().next().unwrap();
    assert_eq!(connection["metadata"]["label"], "Default");
    assert_eq!(
        connection["metadata"]["endpoint"],
        "http://127.0.0.1:32123/v1"
    );
    assert_eq!(
        connection["metadata"]["api_kind"],
        "openai-chat-completions"
    );
    assert_eq!(connection["metadata"]["default_model"], "synthetic");
    assert_eq!(
        w.json(&["ai", "connections", "default", "show"], None)["connection"],
        id.as_str()
    );
    let index: toml::Value =
        toml::from_str(&fs::read_to_string(w.path("settings/secrets/ai/index.toml")).unwrap())
            .unwrap();
    assert_eq!(index["selection"]["connection"].as_str(), Some(id.as_str()));
    let secrets = w.json(&["secrets", "list"], None);
    assert_eq!(secrets["items"].as_array().unwrap().len(), 1);
    assert_eq!(secrets["items"][0]["kind"], "api-key");
}

#[test]
fn attended_setup_shows_profile_key_hint_and_proposes_profile_endpoint() {
    let w = World::new();
    fs::write(
        w.path("settings/site-profile.toml"),
        "schema = 1\n\n[ai]\nkey_hint_label = \"Example AI Hub\"\nkey_hint_url = \"https://ai-hub.example.com/keys\"\ndefault_endpoint = \"http://127.0.0.1:32125/v1\"\n",
    )
    .unwrap();
    let report = w.setup(&[
        ("Install AI agent packages now?", "\r"),
        ("Create a default AI connection now?", "k\r"),
        ("Label", "\r"),
        ("Endpoint", "\r"),
        ("API kind", "\r"),
        ("Model", "synthetic\r"),
        ("API key (hidden)", "SYNTHETIC_PROFILE_KEY_NEVER_OUTPUT\r"),
        ("native settings", "\r"),
        ("Configure existing connection", "\u{1b}"),
    ]);
    let transcript = report["transcript"].as_str().unwrap();
    assert_eq!(report["status"], 0, "{transcript}");
    assert_eq!(report["terminal_restored"], true, "{transcript}");
    assert_eq!(report["sent"], 9, "{transcript}");
    assert!(
        transcript.contains("get an API key: Example AI Hub · https://ai-hub.example.com/keys"),
        "{transcript}"
    );
    assert!(!transcript.contains("SYNTHETIC_PROFILE_KEY_NEVER_OUTPUT"));
    let connections = w.json(&["ai", "connections", "list"], None);
    let (_, connection) = connections["items"]
        .as_object()
        .unwrap()
        .iter()
        .next()
        .unwrap();
    assert_eq!(
        connection["metadata"]["endpoint"],
        "http://127.0.0.1:32125/v1"
    );
}

#[test]
fn attended_setup_without_default_declined_writes_nothing() {
    let w = World::new();
    let before = settings_bytes(&w.path("settings"));
    let report = w.setup(&[
        ("Install AI agent packages now?", "\r"),
        ("Create a default AI connection now?", "\r"),
    ]);
    let transcript = report["transcript"].as_str().unwrap();
    assert_eq!(report["status"], 0, "{transcript}");
    assert_eq!(report["terminal_restored"], true, "{transcript}");
    assert_eq!(report["sent"], 2, "{transcript}");
    assert!(
        settings_bytes(&w.path("settings")) == before,
        "decline changed settings bytes"
    );
    assert!(
        transcript.contains("ai setup skipped; connections and bindings unchanged"),
        "{transcript}"
    );
    assert_eq!(
        w.json(&["ai", "connections", "list"], None)["items"],
        serde_json::json!({})
    );
    assert_eq!(
        w.json(&["secrets", "list"], None)["items"],
        serde_json::json!([])
    );
}

fn seed_default(w: &World) -> String {
    let secret = w.json(
        &[
            "secrets",
            "create",
            "--kind",
            "api-key",
            "--label",
            "Old",
            "--value-stdin",
        ],
        Some(b"SYNTHETIC_OLD_KEY_NEVER_OUTPUT"),
    );
    let sid = secret["items"][0]["id"].as_str().unwrap().to_owned();
    let added = w.json(
        &[
            "ai",
            "connections",
            "add",
            "--label",
            "Old",
            "--endpoint",
            "http://127.0.0.1:32123/v1",
            "--api-kind",
            "openai-chat-completions",
            "--default-model",
            "synthetic",
            "--secret-ref",
            &sid,
        ],
        None,
    );
    let id = added["items"]
        .as_object()
        .unwrap()
        .keys()
        .next()
        .unwrap()
        .clone();
    let revision =
        |w: &World| w.json(&["ai", "connections", "list"], None)["store_revision"].to_string();
    w.json(
        &[
            "ai",
            "connections",
            "default",
            "set",
            &id,
            "--expect-store",
            &revision(w),
        ],
        None,
    );
    w.json(
        &[
            "ai",
            "bindings",
            "set",
            "qwen",
            "--mode",
            "connection",
            "--connection",
            &id,
            "--expect-store",
            &revision(w),
        ],
        None,
    );
    w.json(
        &[
            "ai",
            "bindings",
            "set",
            "pi",
            "--mode",
            "inherit-default",
            "--expect-store",
            &revision(w),
        ],
        None,
    );
    id
}

#[test]
fn attended_setup_with_default_reuses_it_without_writes() {
    let w = World::new();
    let old = seed_default(&w);
    let before = w.json(&["ai", "connections", "list"], None)["store_revision"].clone();
    let report = w.setup(&[
        ("Install AI agent packages now?", "\r"),
        ("Reuse the default connection?", "\r"),
        ("native settings", "\r"),
        ("Configure existing connection", "\u{1b}"),
    ]);
    let transcript = report["transcript"].as_str().unwrap();
    assert_eq!(report["status"], 0, "{transcript}");
    assert_eq!(report["terminal_restored"], true, "{transcript}");
    assert_eq!(report["sent"], 4, "{transcript}");
    assert!(
        transcript.contains("default connection Old · "),
        "{transcript}"
    );
    assert_eq!(
        w.json(&["ai", "connections", "list"], None)["store_revision"],
        before
    );
    assert_eq!(
        w.json(&["ai", "connections", "default", "show"], None)["connection"],
        old.as_str()
    );
    assert_eq!(
        w.json(&["secrets", "list"], None)["items"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn attended_setup_with_default_replaced_default_only_keeps_explicit_qwen() {
    let w = World::new();
    let old = seed_default(&w);
    let report = w.setup(&[
        ("Install AI agent packages now?", "\r"),
        ("Reuse the default connection?", "j\r"),
        ("Replace the default connection?", "k\r"),
        ("Label", "New\r"),
        ("Endpoint", "http://127.0.0.1:32124/v1\r"),
        ("API kind", "\r"),
        ("Model", "synthetic\r"),
        (
            "API key (hidden)",
            "SYNTHETIC_REPLACEMENT_KEY_NEVER_OUTPUT\r",
        ),
        ("Default update scope", "\r"),
        ("Review default and managed overrides", "\u{1b}[B\r"),
        ("native settings", "\r"),
        ("Configure existing connection", "\u{1b}"),
    ]);
    let transcript = report["transcript"].as_str().unwrap();
    assert_eq!(report["status"], 0, "{transcript}");
    assert_eq!(report["terminal_restored"], true, "{transcript}");
    assert_eq!(report["sent"], 12, "{transcript}");
    assert!(!transcript.contains("SYNTHETIC_REPLACEMENT_KEY_NEVER_OUTPUT"));
    let new = w.json(&["ai", "connections", "default", "show"], None)["connection"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_ne!(new, old);
    assert_eq!(
        w.json(&["ai", "connections", "list"], None)["items"][&new]["metadata"]["label"],
        "New"
    );
    let qwen = w.json(&["ai", "bindings", "show", "qwen"], None);
    assert_eq!(qwen["binding"]["mode"], "connection");
    assert_eq!(qwen["binding"]["connection"], old.as_str());
    assert_eq!(
        w.json(&["ai", "bindings", "show", "pi"], None)["binding"]["mode"],
        "inherit-default"
    );
}
