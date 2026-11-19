//! Public keyring contract, synthetic settings only; run inside outer isolation.
use std::{
    fs,
    io::Write,
    process::{Command, Stdio},
};
const SENTINEL: &str = "SYNTHETIC_CLI_SECRET_DO_NOT_PRINT";
struct Fixture(tempfile::TempDir);
impl Fixture {
    fn new() -> Self {
        #[cfg(unix)]
        use std::os::unix::fs::PermissionsExt;
        let mut builder = tempfile::Builder::new();
        #[cfg(unix)]
        builder.permissions(fs::Permissions::from_mode(0o700));
        Self(builder.tempdir().unwrap())
    }
    fn run(&self, args: &[&str], input: Option<&[u8]>) -> std::process::Output {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_vibe"));
        cmd.env_clear()
            .env("HOME", self.0.path())
            .env("VIBE_SETTINGS", self.0.path())
            .env("PATH", "/usr/bin:/bin")
            .args(["secrets"])
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = cmd.spawn().unwrap();
        if let Some(bytes) = input {
            let _ = child.stdin.take().unwrap().write_all(bytes);
        } else {
            drop(child.stdin.take());
        }
        let out = child.wait_with_output().unwrap();
        assert!(!String::from_utf8_lossy(&out.stdout).contains(SENTINEL));
        assert!(!String::from_utf8_lossy(&out.stderr).contains(SENTINEL));
        out
    }
    fn json(&self, args: &[&str], input: Option<&[u8]>) -> serde_json::Value {
        let out = self.run(args, input);
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        serde_json::from_slice(&out.stdout).unwrap()
    }
}
#[cfg(unix)]
#[test]
fn public_metadata_lifecycle_preserves_ids_tokens_and_refuses_unknown_removal() {
    let f = Fixture::new();
    assert_eq!(
        f.json(&["list", "--json"], None)["items"],
        serde_json::json!([])
    );
    assert!(!f.0.path().join("secrets").exists());
    fs::write(f.0.path().join("foreign"), b"KEEP").unwrap();
    let created = f.json(
        &[
            "create",
            "--kind",
            "api-key",
            "--label",
            "Synthetic",
            "--value-stdin",
            "--json",
        ],
        Some(format!("{SENTINEL}\n").as_bytes()),
    );
    let id = created["items"][0]["id"].as_str().unwrap();
    let rev = created["items"][0]["current_revision"].as_str().unwrap();
    let store = created["store_revision"].as_u64().unwrap().to_string();
    let value_path =
        f.0.path()
            .join("secrets/items")
            .join(id)
            .join(rev)
            .join("value");
    assert_eq!(
        fs::read(value_path).unwrap(),
        format!("{SENTINEL}\n").as_bytes()
    );
    assert_eq!(f.json(&["details", id, "--json"], None), created);
    let renamed = f.json(
        &[
            "edit",
            id,
            "--label",
            "Renamed",
            "--expect-store",
            &store,
            "--expect-current",
            rev,
            "--json",
        ],
        None,
    );
    assert_eq!(renamed["items"][0]["id"], id);
    assert_eq!(renamed["items"][0]["current_revision"], rev);
    assert!(
        !f.run(
            &[
                "edit",
                id,
                "--label",
                "Stale",
                "--expect-store",
                &store,
                "--expect-current",
                rev,
                "--json"
            ],
            None
        )
        .status
        .success()
    );
    let rotated = f.json(
        &[
            "rotate",
            id,
            "--expect-current",
            rev,
            "--value-stdin",
            "--json",
        ],
        Some(SENTINEL.as_bytes()),
    );
    assert_eq!(rotated["items"][0]["id"], id);
    assert_ne!(rotated["items"][0]["current_revision"], rev);
    let current = rotated["items"][0]["current_revision"].as_str().unwrap();
    let version = rotated["store_revision"].as_u64().unwrap().to_string();
    let path = f.0.path().join("secrets/index.toml");
    let before = fs::read(&path).unwrap();
    for yes in [false, true] {
        let mut args = vec![
            "remove",
            id,
            "--expect-store",
            &version,
            "--expect-current",
            current,
            "--json",
        ];
        if yes {
            args.push("--yes");
        }
        assert!(!f.run(&args, None).status.success());
        assert_eq!(fs::read(&path).unwrap(), before);
    }
    assert_eq!(fs::read(f.0.path().join("foreign")).unwrap(), b"KEEP");
}
#[test]
fn malformed_missing_empty_oversize_and_unattended_input_never_echo_values() {
    let f = Fixture::new();
    for args in [
        vec![
            "create",
            "--kind",
            "api-key",
            "--label",
            "Synthetic",
            "--json",
        ],
        vec![
            "create",
            "--kind",
            "api-key",
            "--label",
            "Synthetic",
            "--unattended",
        ],
        vec!["create", "--value", SENTINEL],
        vec!["details", SENTINEL],
        vec!["edit", SENTINEL, "--expect-store", SENTINEL],
    ] {
        assert!(!f.run(&args, None).status.success());
    }
    for bytes in [vec![], vec![b'X'; 1024 * 1024 + 1]] {
        assert!(
            !f.run(
                &[
                    "create",
                    "--kind",
                    "api-key",
                    "--label",
                    "Synthetic",
                    "--value-stdin",
                    "--json"
                ],
                Some(&bytes)
            )
            .status
            .success()
        );
    }
    assert!(!f.0.path().join("secrets").exists());
}

#[test]
fn parser_redaction_follows_root_command_not_global_operand_or_other_position() {
    let f = Fixture::new();
    let output = Command::new(env!("CARGO_BIN_EXE_vibe"))
        .env_clear()
        .env("HOME", f.0.path())
        .env("VIBE_SETTINGS", f.0.path())
        .args([
            "--invoked-by",
            "secrets",
            "list",
            "--definitely-invalid-option",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(!String::from_utf8_lossy(&output.stderr).contains("invalid secrets invocation"));
    assert!(String::from_utf8_lossy(&output.stderr).contains("--definitely-invalid-option"));
    let output = Command::new(env!("CARGO_BIN_EXE_vibe"))
        .env_clear()
        .env("HOME", f.0.path())
        .env("VIBE_SETTINGS", f.0.path())
        .args([
            "--invoked-by",
            "test",
            "secrets",
            "create",
            "--value",
            SENTINEL,
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(!String::from_utf8_lossy(&output.stderr).contains(SENTINEL));
    assert!(String::from_utf8_lossy(&output.stderr).contains("invalid secrets invocation"));
}
