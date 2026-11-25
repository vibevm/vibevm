//! Synthetic callback tests and isolated startup fixtures: no services or px.
#![allow(dead_code)]

// Use the actual bridge with a memory-backed output boundary. No product
// injection flags or public domain APIs are introduced for these fixtures.
mod output {
    #[derive(Default)]
    pub struct Context(std::sync::Mutex<Vec<String>>);
    impl Context {
        pub fn is_json(&self) -> bool { false }
        pub fn is_quiet(&self) -> bool { false }
        pub fn is_machine_progress(&self) -> bool { false }
        pub fn diagnostic(&self, text: &str) { self.0.lock().unwrap().push(text.into()); }
        pub fn block_err(&self, text: &str) { self.diagnostic(text); }
        pub fn error_with_suffix(&self, error: &anyhow::Error, _: Option<&str>) {
            self.diagnostic(&error.to_string());
        }
    }
    pub mod language {
        pub enum Role { Accent }
        pub fn short_path(path: &str) -> String { path.into() }
        pub fn warning(text: &str) -> String { format!("! {text}") }
        pub fn paint(_: Role, text: &str) -> String { text.into() }
    }
}
#[path = "../src/composition/startup_notes.rs"]
mod startup_notes;

use std::process::{Command, Output};
use tracing_subscriber::prelude::*;

#[test]
fn classified_callbacks_are_bounded_once_and_debug_safe() {
    let diagnostics = startup_notes::Diagnostics::default();
    let subscriber = tracing_subscriber::registry().with(diagnostics.clone());
    let secret = "SYNTHETIC_CREDENTIAL_CANARY";
    tracing::subscriber::with_default(subscriber, || {
        for _ in 0..2 {
            tracing::warn!(target: "vibe_install::apply", diagnostic_code = "install_integrity",
                occurrence = 1u64, message = secret, url = secret);
            tracing::warn!(target: "vibe_install::apply", diagnostic_code = "install_rollback",
                occurrence = 2u64, message = secret, reason = secret);
            tracing::warn!(target: "vibe_net::route", diagnostic_code = "network_route",
                occurrence = 3u64, cause = "exited", log = secret);
            tracing::warn!(target: "vibe_net::cleanup", diagnostic_code = "network_cleanup",
                occurrence = 4u64, message = secret);
        }
        tracing::debug!(target: "vibe_net::route", diagnostic_code = "network_route",
            occurrence = 3u64, cause = secret, message = secret);
    });
    diagnostics.px_failure(3, &vibe_net::PxError::Exited {
        status: secret.into(), log: secret.into(),
    });
    let lines = diagnostics.take();
    assert!(lines.len() == 4, "each classified occurrence must be reported once");
    assert!(lines.iter().all(|line| !line.contains(secret)), "classified payload leaked");
    for code in ["install_integrity", "install_rollback", "network_route", "network_cleanup"] {
        assert!(lines.iter().filter(|line| line.starts_with(code)).count() == 1,
            "classified diagnostic is missing or duplicated");
    }
    assert!(diagnostics.take().is_empty(), "draining repeated a diagnostic");
    diagnostics.px_failure(5, &vibe_net::PxError::Exited {
        status: secret.into(), log: secret.into(),
    });
    let direct = diagnostics.take();
    assert!(direct.len() == 1, "direct px failure must be reported once");
    assert!(!direct[0].contains(secret), "direct px failure leaked a payload");
    assert!(direct[0].contains("px exited"), "typed px cause was lost");
}

#[test]
fn unrelated_records_are_not_classified() {
    let diagnostics = startup_notes::Diagnostics::default();
    tracing::subscriber::with_default(tracing_subscriber::registry().with(diagnostics.clone()), || {
        tracing::warn!(target: "unrelated", diagnostic_code = "network_route", occurrence = 1u64);
        tracing::warn!(target: "vibe_registry", url = "file:///synthetic", error = "fixture");
    });
    assert!(diagnostics.take().is_empty(), "unrelated tracing was forwarded");
}

struct Fixture {
    root: tempfile::TempDir,
}
impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        for name in ["home", "settings", "cache"] {
            std::fs::create_dir(root.path().join(name)).unwrap();
        }
        std::fs::write(root.path().join("settings/config.toml"), "[proxy]\nurl = \"x\"\n").unwrap();
        Self { root }
    }
    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_vibe"))
            .env_clear()
            .env("HOME", self.root.path().join("home"))
            .env("USERPROFILE", self.root.path().join("home"))
            .env("VIBE_SETTINGS", self.root.path().join("settings"))
            .env("VIBE_REGISTRY_CACHE", self.root.path().join("cache"))
            .env("VIBE_NO_DEFAULT_REGISTRY", "1")
            .env("TERM", "dumb")
            .current_dir(self.root.path())
            .args(args).output().unwrap()
    }
}

#[test]
fn malformed_config_reports_once() {
    let fixture = Fixture::new();
    let out = fixture.run(&["user", "status"]);
    assert!(out.status.success(), "tolerant user status failed");
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.matches("config.toml").count() == 1, "configuration notice duplicated");
    assert!(stderr.contains("is not valid TOML for vibe"), "bounded notice missing");
    assert!(stderr.contains("vibe show config"), "repair command missing");
    assert!(!stderr.contains("remap layer skipped"), "remap repeated configuration notice");
}

#[test]
fn malformed_config_refuses_before_network_once() {
    let fixture = Fixture::new();
    // chosen_route reads the malformed configuration before the first probe.
    let out = fixture.run(&["--no-progress", "dl", "--show", "http://127.0.0.1:1/a"]);
    assert!(out.status.code() == Some(1), "fatal config must retain exit one");
    assert!(out.stdout.is_empty(), "fatal config wrote stdout");
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.matches("config_malformed").count() == 1, "coded refusal missing or duplicated");
    assert!(!stderr.contains("ignores it"), "fatal config also emitted tolerant notice");
    assert!(!stderr.contains("remap layer skipped"), "fatal config emitted remap duplicate");
}

#[test]
fn tolerant_json_is_one_document() {
    let fixture = Fixture::new();
    let out = fixture.run(&["--json", "user", "status"]);
    assert!(out.status.success(), "tolerant JSON status failed");
    let _: serde_json::Value = serde_json::from_slice(&out.stdout).expect("one JSON document");
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.matches("config.toml").count() == 1, "JSON diagnostic duplicated");
}
