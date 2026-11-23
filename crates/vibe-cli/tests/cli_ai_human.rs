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

    fn raw(&self, args: &[&str], input: Option<&[u8]>) -> std::process::Output {
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
        out
    }

    fn run(&self, args: &[&str], input: Option<&[u8]>) -> std::process::Output {
        let out = self.raw(args, input);
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

impl Fixture {
    fn add(&self, label: &str, key: &str) -> std::process::Output {
        self.run(
            &[
                "ai",
                "connections",
                "add",
                "--label",
                label,
                "--endpoint",
                "https://api.example.invalid",
                "--api-kind",
                "openai-chat-completions",
                "--default-model",
                "m",
                "--secret-ref",
                key,
            ],
            None,
        )
    }

    fn snapshot(&self) -> serde_json::Value {
        self.json(&["ai", "connections", "list", "--json"])
    }

    fn mutation(&self, args: &[&str], what: &str, previous: &mut u64) {
        let output = self.run(args, None);
        let revision = self.snapshot()["store_revision"].as_u64().unwrap();
        assert_eq!(revision, *previous + 1);
        assert_trailer(output, what, revision);
        *previous = revision;
    }
}

fn assert_trailer(output: std::process::Output, what: &str, revision: u64) {
    let text = stdout(output);
    let lines: Vec<_> = text.lines().collect();
    assert_eq!(lines.len(), 1, "{text}");
    let (prefix, elapsed) = lines[0].rsplit_once(" · ").expect("elapsed field");
    assert_eq!(prefix, format!("✻ {what} · store revision {revision}"));
    assert!(!elapsed.is_empty(), "{text}");
    assert!(elapsed.chars().any(|c| c.is_ascii_digit()), "{text}");
    assert!(!text.contains('{'), "{text}");
    assert!(!text.contains("──"), "{text}");
}

#[test]
fn ai_mutation_trailers() {
    let f = Fixture::new();
    let key = f.secret();
    let added = f.add("x", &key);
    let snapshot = f.snapshot();
    let (id, row) = snapshot["items"]
        .as_object()
        .unwrap()
        .iter()
        .next()
        .unwrap();
    let current = row["current_revision"].as_str().unwrap();
    let mut revision = snapshot["store_revision"].as_u64().unwrap();
    assert_trailer(added, &format!("added connection x · {id}"), revision);
    f.mutation(
        &[
            "ai",
            "connections",
            "default",
            "set",
            id,
            "--expect-store",
            &revision.to_string(),
        ],
        &format!("default connection set to x ({id})"),
        &mut revision,
    );
    f.mutation(
        &[
            "ai",
            "bindings",
            "set",
            "qwen",
            "--mode",
            "connection",
            "--connection",
            id,
            "--expect-store",
            &revision.to_string(),
        ],
        &format!("bound qwen to connection {id}"),
        &mut revision,
    );
    f.mutation(
        &[
            "ai",
            "bindings",
            "set",
            "qwen",
            "--mode",
            "inherit-default",
            "--expect-store",
            &revision.to_string(),
            "--quiet",
        ],
        "bound qwen to inherit-default",
        &mut revision,
    );
    f.mutation(
        &[
            "ai",
            "bindings",
            "set",
            "qwen",
            "--mode",
            "native",
            "--expect-store",
            &revision.to_string(),
        ],
        "restored native binding for qwen",
        &mut revision,
    );
    f.mutation(
        &[
            "ai",
            "connections",
            "default",
            "clear",
            "--expect-store",
            &revision.to_string(),
            "--yes",
        ],
        "cleared default connection",
        &mut revision,
    );
    f.mutation(
        &[
            "ai",
            "connections",
            "remove",
            id,
            "--expect-store",
            &revision.to_string(),
            "--expect-current",
            current,
            "--yes",
        ],
        &format!("removed connection {id}"),
        &mut revision,
    );
    assert!(f.snapshot()["items"].as_object().unwrap().is_empty());
}

#[test]
fn ai_mutation_preserves_selected_details() {
    let f = Fixture::new();
    let key = f.secret();
    f.add("First synthetic", &key);
    let first = f.snapshot();
    let id = first["items"].as_object().unwrap().keys().next().unwrap();
    f.add("Second synthetic", &key);
    let snapshot = f.snapshot();
    let items = snapshot["items"].as_object().unwrap();
    assert_eq!(items.len(), 2);
    let other = items.keys().find(|other| *other != id).unwrap();
    assert_ne!(id, other);
    let listed = stdout(f.run(&["ai", "connections", "list"], None));
    for value in [
        id.as_str(),
        other.as_str(),
        "First synthetic",
        "Second synthetic",
    ] {
        assert!(listed.contains(value), "{listed}");
    }
    let details = stdout(f.run(&["ai", "connections", "details", id], None));
    assert!(details.contains(id), "{details}");
    assert!(details.contains("First synthetic"), "{details}");
    assert!(!details.contains(other), "{details}");
    assert!(!details.contains("Second synthetic"), "{details}");
    assert!(!details.contains('{'), "{details}");
    let json = f.json(&["ai", "connections", "details", id, "--json"]);
    assert_eq!(json["protocol"], "vibe-ai-connections/1");
    assert_eq!(json["ok"], true);
    assert_eq!(json["store_revision"], snapshot["store_revision"]);
    let selected = json["items"].as_object().unwrap();
    assert_eq!(selected.len(), 1);
    assert_eq!(selected.get(id), items.get(id));
}

#[test]
fn ai_mutation_refusals() {
    let f = Fixture::new();
    let key = f.secret();
    f.add("x", &key);
    let added = f.snapshot();
    let (id, row) = added["items"].as_object().unwrap().iter().next().unwrap();
    let current = row["current_revision"].as_str().unwrap();
    f.run(
        &[
            "ai",
            "connections",
            "default",
            "set",
            id,
            "--expect-store",
            &added["store_revision"].as_u64().unwrap().to_string(),
        ],
        None,
    );
    let snapshot = f.snapshot();
    let revision = snapshot["store_revision"].as_u64().unwrap().to_string();
    let index = f.0.path().join("secrets/ai/index.toml");
    let before = std::fs::read(&index).unwrap();
    let remove = [
        "ai",
        "connections",
        "remove",
        id,
        "--expect-store",
        &revision,
        "--expect-current",
        current,
    ];
    let clear = [
        "ai",
        "connections",
        "default",
        "clear",
        "--expect-store",
        &revision,
    ];
    let binding = [
        "ai",
        "bindings",
        "set",
        "qwen",
        "--mode",
        "connection",
        "--expect-store",
        &revision,
    ];
    for (args, code, headline, fix) in [
        (
            remove.as_slice(), "consent_required",
            format!("removing connection {id} needs --yes"),
            format!("vibe ai connections remove {id} --expect-store {revision} --expect-current {current} --yes"),
        ),
        (
            clear.as_slice(), "consent_required",
            "clearing the default connection needs --yes".into(),
            format!("vibe ai connections default clear --expect-store {revision} --yes"),
        ),
        (
            binding.as_slice(), "binding_arguments_invalid",
            "binding mode and connection argument do not match".into(),
            "pass --connection ID only with --mode connection; omit --connection for native and inherit-default".into(),
        ),
    ] {
        let output = f.raw(args, None);
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        let stderr = String::from_utf8_lossy(&output.stderr);
        let normalized = stderr.split_whitespace().collect::<Vec<_>>().join(" ");
        for expected in [code, headline.as_str(), fix.as_str(),
            "spec://org.vibevm.core/vibevm/common/PROP-000#TS-KEYRING-CONSUMER"] {
            assert!(normalized.contains(expected), "missing {expected:?}: {stderr}");
        }
        assert_eq!(f.snapshot(), snapshot);
        assert_eq!(std::fs::read(&index).unwrap(), before);
    }
}
