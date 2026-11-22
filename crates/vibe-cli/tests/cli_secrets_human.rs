//! Human keyring metadata contract with isolated synthetic settings.
#![cfg(unix)]

use std::{
    io::Write,
    os::unix::fs::PermissionsExt,
    path::Path,
    process::{Command, Stdio},
};

const SENTINEL: &str = "SYNTHETIC_HUMAN_SECRET_DO_NOT_PRINT";

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
        let settings: &Path = self.0.path();
        let mut child = Command::new(env!("CARGO_BIN_EXE_vibe"))
            .env_clear()
            .env("HOME", settings)
            .env("VIBE_SETTINGS", settings)
            .env("PATH", "/usr/bin:/bin")
            .args(["secrets"])
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
        assert!(!String::from_utf8_lossy(&out.stdout).contains(SENTINEL));
        assert!(!String::from_utf8_lossy(&out.stderr).contains(SENTINEL));
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        out
    }
}

#[test]
fn human_list_details_and_references_render_masked_tables() {
    let fixture = Fixture::new();
    let empty = fixture.run(&["list"], None);
    assert!(String::from_utf8_lossy(&empty.stdout).contains("Local keyring is empty."));

    let created = fixture.run(
        &[
            "create",
            "--kind",
            "api-key",
            "--label",
            "Synthetic",
            "--value-stdin",
            "--json",
        ],
        Some(SENTINEL.as_bytes()),
    );
    let created: serde_json::Value = serde_json::from_slice(&created.stdout).unwrap();
    let id = created["items"][0]["id"].as_str().unwrap();

    let details = fixture.run(&["details", id], None);
    let details = String::from_utf8_lossy(&details.stdout);
    for expected in [
        "store revision 1",
        id,
        "Synthetic",
        "api-key",
        "stored (hidden)",
    ] {
        assert!(details.contains(expected), "missing {expected}: {details}");
    }
    assert!(!details.contains("\"protocol\""));

    let list = fixture.run(&["list"], None);
    let list = String::from_utf8_lossy(&list.stdout);
    assert!(list.contains(id));
    assert!(list.contains("stored (hidden)"));

    let references = fixture.run(&["references", id], None);
    let references = String::from_utf8_lossy(&references.stdout);
    assert!(references.contains(&format!("No registered consumers reference {id}.")));
    assert!(!references.contains("\"protocol\""));

    let json = fixture.run(&["details", id, "--json"], None);
    let json: serde_json::Value = serde_json::from_slice(&json.stdout).unwrap();
    assert_eq!(json["protocol"], "vibe-secrets/1");
    assert_eq!(json, created);
}
