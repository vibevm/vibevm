//! First durable consumer through separate public processes, synthetic only.
use std::{
    fs,
    io::Write,
    process::{Command, Stdio},
};
const VALUE: &str = "SYNTHETIC_AI_REFERENCE_KEY_NEVER_OUTPUT";
struct F(tempfile::TempDir);
impl F {
    fn new() -> Self {
        use std::os::unix::fs::PermissionsExt;
        Self(
            tempfile::Builder::new()
                .permissions(fs::Permissions::from_mode(0o700))
                .tempdir()
                .unwrap(),
        )
    }
    fn run(&self, args: &[&str], input: Option<&[u8]>) -> std::process::Output {
        let mut c = Command::new(env!("CARGO_BIN_EXE_vibe"));
        c.env_clear()
            .env("HOME", self.0.path())
            .env("VIBE_SETTINGS", self.0.path())
            .env("PATH", "/usr/bin:/bin")
            .args(args)
            .arg("--json")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut c = c.spawn().unwrap();
        if let Some(v) = input {
            c.stdin.take().unwrap().write_all(v).unwrap();
        } else {
            drop(c.stdin.take());
        }
        let o = c.wait_with_output().unwrap();
        assert!(!String::from_utf8_lossy(&o.stdout).contains(VALUE));
        assert!(!String::from_utf8_lossy(&o.stderr).contains(VALUE));
        o
    }
    fn json(&self, args: &[&str], input: Option<&[u8]>) -> serde_json::Value {
        let o = self.run(args, input);
        assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
        serde_json::from_slice(&o.stdout).unwrap()
    }
}
#[cfg(unix)]
#[test]
fn connection_reference_follows_rotation_blocks_delete_and_explicit_unbinding_releases_it() {
    let f = F::new();
    let empty = f.json(&["ai", "connections", "list"], None);
    assert_eq!(empty["items"], serde_json::json!({}));
    assert!(!f.0.path().join("secrets").exists());
    let secret = f.json(
        &[
            "secrets",
            "create",
            "--kind",
            "api-key",
            "--label",
            "Synthetic",
            "--value-stdin",
        ],
        Some(VALUE.as_bytes()),
    );
    let id = secret["items"][0]["id"].as_str().unwrap();
    let rev = secret["items"][0]["current_revision"].as_str().unwrap();
    fs::write(f.0.path().join("foreign"), b"KEEP").unwrap();
    let added = f.json(
        &[
            "ai",
            "connections",
            "add",
            "--label",
            "Synthetic connection",
            "--endpoint",
            "http://127.0.0.1:32123/v1",
            "--api-kind",
            "openai-chat-completions",
            "--default-model",
            "synthetic-model",
            "--secret-ref",
            id,
        ],
        None,
    );
    let rows = added["items"].as_object().unwrap();
    let (connection, row) = rows.iter().next().unwrap();
    assert_eq!(row["secret_ref"], id);
    let connrev = row["current_revision"].as_str().unwrap();
    let version = added["store_revision"].as_u64().unwrap().to_string();
    assert_eq!(
        f.json(&["ai", "connections", "details", connection], None),
        added
    );
    let rotated = f.json(
        &[
            "secrets",
            "rotate",
            id,
            "--expect-current",
            rev,
            "--value-stdin",
        ],
        Some(VALUE.as_bytes()),
    );
    let newrev = rotated["items"][0]["current_revision"].as_str().unwrap();
    assert_ne!(newrev, rev);
    let details = f.json(&["ai", "connections", "details", connection], None);
    assert_eq!(details["items"][connection]["secret_ref"], id);
    let before = fs::read(f.0.path().join("secrets/index.toml")).unwrap();
    let store = rotated["store_revision"].as_u64().unwrap().to_string();
    assert!(
        !f.run(
            &[
                "secrets",
                "remove",
                id,
                "--expect-store",
                &store,
                "--expect-current",
                newrev,
                "--yes"
            ],
            None
        )
        .status
        .success()
    );
    assert_eq!(
        fs::read(f.0.path().join("secrets/index.toml")).unwrap(),
        before
    );
    assert!(
        !f.run(
            &[
                "ai",
                "connections",
                "remove",
                connection,
                "--expect-store",
                "0",
                "--expect-current",
                connrev,
                "--yes"
            ],
            None
        )
        .status
        .success()
    );
    f.json(
        &[
            "ai",
            "connections",
            "remove",
            connection,
            "--expect-store",
            &version,
            "--expect-current",
            connrev,
            "--yes",
        ],
        None,
    );
    f.json(
        &[
            "secrets",
            "remove",
            id,
            "--expect-store",
            &store,
            "--expect-current",
            newrev,
            "--yes",
        ],
        None,
    );
    assert_eq!(fs::read(f.0.path().join("foreign")).unwrap(), b"KEEP");
    assert!(
        f.0.path()
            .join("secrets/items")
            .join(id)
            .join(newrev)
            .join("value")
            .exists()
    );
}
#[cfg(unix)]
#[test]
fn unknown_secret_and_credential_url_refuse_without_binding() {
    let f = F::new();
    let unknown = "11111111-1111-4111-8111-111111111111";
    for endpoint in [
        "https://user:pass@example.invalid/v1",
        "https://example.invalid/v1?token=bad",
        "http://example.invalid/v1",
        "http://127.0.0.1:32123/v1",
    ] {
        let out = f.run(
            &[
                "ai",
                "connections",
                "add",
                "--label",
                "Synthetic",
                "--endpoint",
                endpoint,
                "--api-kind",
                "openai-chat-completions",
                "--default-model",
                "synthetic",
                "--secret-ref",
                unknown,
            ],
            None,
        );
        assert!(!out.status.success());
        assert!(!f.0.path().join("secrets/ai").exists());
    }
}

#[cfg(unix)]
#[test]
fn corrupted_metadata_and_missing_registered_index_fail_closed() {
    for corruption in [0, 1, 2] {
        let f = F::new();
        let secret = f.json(
            &[
                "secrets",
                "create",
                "--kind",
                "api-key",
                "--label",
                "Synthetic",
                "--value-stdin",
            ],
            Some(VALUE.as_bytes()),
        );
        let id = secret["items"][0]["id"].as_str().unwrap();
        let added = f.json(
            &[
                "ai",
                "connections",
                "add",
                "--label",
                "Synthetic",
                "--endpoint",
                "http://127.0.0.1:32123/v1",
                "--api-kind",
                "openai-chat-completions",
                "--default-model",
                "synthetic",
                "--secret-ref",
                id,
            ],
            None,
        );
        let (connection, row) = added["items"].as_object().unwrap().iter().next().unwrap();
        let selected = f.json(&["secrets", "details", id], None);
        let version = selected["store_revision"].as_u64().unwrap().to_string();
        let revision = selected["items"][0]["current_revision"].as_str().unwrap();
        if corruption == 2 {
            fs::write(
                f.0.path()
                    .join("secrets/ai/connections")
                    .join(connection)
                    .join(row["current_revision"].as_str().unwrap())
                    .join("config.toml"),
                b"CORRUPTED",
            )
            .unwrap();
        } else if corruption == 1 {
            fs::write(
                f.0.path().join("secrets/ai/index.toml"),
                b"schema = 1\nstore_revision = 1\n",
            )
            .unwrap();
        } else {
            fs::rename(
                f.0.path().join("secrets/ai/index.toml"),
                f.0.path().join("saved-index"),
            )
            .unwrap();
        }
        let before = fs::read(f.0.path().join("secrets/index.toml")).unwrap();
        let refused = f.run(
            &[
                "secrets",
                "remove",
                id,
                "--expect-store",
                &version,
                "--expect-current",
                revision,
                "--yes",
            ],
            None,
        );
        assert!(!refused.status.success());
        assert_eq!(
            fs::read(f.0.path().join("secrets/index.toml")).unwrap(),
            before
        );
        assert!(!f.run(&["ai", "connections", "list"], None).status.success());
        assert!(
            f.0.path()
                .join("secrets/items")
                .join(id)
                .join(revision)
                .join("value")
                .exists()
        );
    }
}
