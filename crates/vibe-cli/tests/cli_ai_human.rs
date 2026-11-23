//! Human mode of `vibe ai connections`, `default` and `bindings` (PROP-067
//! `#coverage`): rows and blocks, never the JSON document. Synthetic only.
#![cfg(unix)]

use std::{
    io::Write,
    os::unix::fs::PermissionsExt,
    process::{Command, Stdio},
};

const VALUE: &[u8] = b"SYNTHETIC_AI_HUMAN_KEY_NEVER_OUTPUT";

struct Fixture(tempfile::TempDir);

impl Fixture {
    fn new() -> Self {
        Self(
            tempfile::Builder::new()
                .permissions(std::fs::Permissions::from_mode(0o700))
                .tempdir()
                .unwrap(),
        )
    }

    fn run(&self, args: &[&str], input: Option<&[u8]>) -> std::process::Output {
        let mut child = Command::new(env!("CARGO_BIN_EXE_vibe"))
            .env_clear()
            .env("HOME", self.0.path())
            .env("VIBE_SETTINGS", self.0.path())
            .env("PATH", "/usr/bin:/bin")
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        if let Some(bytes) = input {
            child.stdin.take().unwrap().write_all(bytes).unwrap();
        } else {
            drop(child.stdin.take());
        }
        let out = child.wait_with_output().unwrap();
        let value = String::from_utf8_lossy(VALUE).into_owned();
        assert!(!String::from_utf8_lossy(&out.stdout).contains(&value));
        assert!(!String::from_utf8_lossy(&out.stderr).contains(&value));
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        out
    }

    fn json(&self, args: &[&str]) -> serde_json::Value {
        serde_json::from_slice(&self.run(args, None).stdout).unwrap()
    }

    /// A synthetic keyring item to point connections at.
    fn secret(&self) -> String {
        let created: serde_json::Value = serde_json::from_slice(
            &self
                .run(
                    &[
                        "secrets",
                        "create",
                        "--kind",
                        "api-key",
                        "--label",
                        "Synthetic",
                        "--value-stdin",
                        "--json",
                    ],
                    Some(VALUE),
                )
                .stdout,
        )
        .unwrap();
        created["items"][0]["id"].as_str().unwrap().to_owned()
    }
}

fn stdout(output: std::process::Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

#[test]
fn ai_connections_human_mode_prints_no_json() {
    let f = Fixture::new();
    let empty = stdout(f.run(&["ai", "connections", "list"], None));
    assert!(empty.contains("No AI connections. Add one with"), "{empty}");
    assert!(!empty.contains('{'), "{empty}");

    let key = f.secret();
    f.run(
        &[
            "ai",
            "connections",
            "add",
            "--label",
            "x",
            "--endpoint",
            "https://api.example.invalid",
            "--api-kind",
            "openai-chat-completions",
            "--default-model",
            "m",
            "--secret-ref",
            &key,
        ],
        None,
    );
    let report = f.json(&["ai", "connections", "list", "--json"]);
    assert_eq!(report["protocol"], "vibe-ai-connections/1");
    assert_eq!(report["ok"], true);
    let items = report["items"].as_object().unwrap();
    assert_eq!(items.len(), 1);
    let (id, connection) = items.iter().next().unwrap();
    let revision = report["store_revision"].as_u64().unwrap();
    assert!(revision > 0);

    let listed = stdout(f.run(&["ai", "connections", "list"], None));
    assert!(
        listed.contains(&format!("── ai connections · store revision {revision}")),
        "{listed}"
    );
    assert!(listed.contains(&format!("  {id}  x")), "{listed}");
    assert!(listed.contains("(openai-chat-completions)"), "{listed}");
    assert!(listed.contains("model m"), "{listed}");
    assert!(
        listed.contains("  └ endpoint https://api.example.invalid"),
        "{listed}"
    );
    assert!(listed.contains(&format!("  └ key {key}")), "{listed}");
    assert!(
        listed.contains(&format!(
            "  └ revision {}",
            connection["current_revision"].as_str().unwrap()
        )),
        "{listed}"
    );
    assert!(!listed.contains('{'), "{listed}");
    let quiet_list = stdout(f.run(&["ai", "connections", "list", "--quiet"], None));
    assert_eq!(
        quiet_list,
        format!("1 AI connection · store revision {revision}\n")
    );
    let default = stdout(f.run(&["ai", "connections", "default", "show"], None));
    assert_eq!(
        default,
        format!("○ no default connection · store revision {revision}\n")
    );
    let quiet = stdout(f.run(&["ai", "connections", "default", "show", "--quiet"], None));
    assert_eq!(
        quiet,
        format!("no default connection · store revision {revision}\n")
    );

    let binding = stdout(f.run(&["ai", "bindings", "show", "pi"], None));
    assert!(
        binding.starts_with("✓ pi  native · unmanaged · store revision "),
        "{binding}"
    );
    assert!(binding.contains("[compatible]"), "{binding}");
    assert!(!binding.contains('{'), "{binding}");
}
