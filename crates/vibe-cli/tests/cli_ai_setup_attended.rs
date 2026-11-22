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
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice(&output.stdout).unwrap()
    }
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
        ("Install AI agent packages now?", "y"),
        ("Select agent packages to install", " \u{1b}[B \r"),
        ("Platform", "\r"),
        ("Architecture", "\r"),
        ("Create a default AI connection now?", "n"),
    ]);
    let transcript = report["transcript"].as_str().unwrap();
    assert_eq!(report["status"], 0, "{transcript}");
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

#[test]
fn attended_setup_without_default_creates_and_selects_connection() {
    let w = World::new();
    let key_line = format!("{KEY}\r");
    let report = w.setup(&[
        ("Install AI agent packages now?", "n"),
        ("Create a default AI connection now?", "y"),
        ("Connection label", "\r"),
        ("Endpoint URL", "http://127.0.0.1:32123/v1\r"),
        ("API kind", "\r"),
        ("Default model", "synthetic\r"),
        ("API key (hidden)", key_line.as_str()),
        ("native settings", "n"),
        ("Configure existing connection", "\u{1b}"),
    ]);
    let transcript = report["transcript"].as_str().unwrap();
    assert_eq!(report["status"], 0, "{transcript}");
    assert_eq!(report["sent"], 9, "{transcript}");
    assert!(
        transcript.contains("Default AI connection created: Default"),
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
    let secrets = w.json(&["secrets", "list"], None);
    assert_eq!(secrets["items"].as_array().unwrap().len(), 1);
    assert_eq!(secrets["items"][0]["kind"], "api-key");
}

#[test]
fn attended_setup_without_default_declined_writes_nothing() {
    let w = World::new();
    let report = w.setup(&[
        ("Install AI agent packages now?", "n"),
        ("Create a default AI connection now?", "n"),
    ]);
    let transcript = report["transcript"].as_str().unwrap();
    assert_eq!(report["status"], 0, "{transcript}");
    assert!(
        transcript.contains("AI setup skipped; connections and bindings unchanged."),
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
