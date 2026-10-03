//! Actual CLI MCP startup grant and cross-process fixed package recovery.
mod common;

use common::UserScratch;
use serde_json::{Value, json};
use std::fs;

fn project() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("vibe.toml"),
        "[package]\ngroup='org.machine'\nname='fixture'\nkind='flow'\nversion='0.1.0'\n",
    )
    .unwrap();
    fs::write(root.path().join("source.bin"), b"copy once").unwrap();
    root
}

fn request(kind: &str, key: &str) -> String {
    json!({"jsonrpc":"2.0","id":41,"method":"tools/call",
        "params":{"name":"fixed_package_operation","arguments":{
            "schema":1,"action":{"kind":kind,"key":key}}}})
    .to_string()
        + "\n"
}

fn serve(user: &UserScratch, root: &std::path::Path, mode: Option<&str>, input: String) -> Value {
    let mut command = user.vibe();
    command
        .env("VIBE_NO_DEFAULT_REGISTRY", "1")
        .args(["--offline", "mcp", "serve", "--path"])
        .arg(root);
    if let Some(mode) = mode {
        command.args(["--fixed-package", mode]);
        if mode == "issue" {
            command.args(["--ttl-ms", "60000"]);
        }
    }
    let output = command.write_stdin(input).output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(stdout.lines().count(), 1, "{stdout}");
    serde_json::from_str(stdout.trim()).unwrap()
}

#[test]
fn explicit_issue_and_reattach_share_one_package_run_across_mcp_processes() {
    let user = UserScratch::new();
    let root = project();
    let output = root
        .path()
        .join("target/vibe-package/launcher/launcher.bin");
    let ungranted = serve(&user, root.path(), None, request("submit", "once"));
    assert_eq!(ungranted["result"]["isError"], true);
    assert_eq!(
        ungranted["result"]["structuredContent"]["code"],
        "unavailable_host"
    );
    assert!(!output.exists());

    let first = serve(&user, root.path(), Some("issue"), request("submit", "once"));
    assert_eq!(first["result"]["isError"], false, "{first}");
    let first_reply = &first["result"]["structuredContent"];
    assert_eq!(first_reply["reply"]["kind"], "completed");
    let run = first_reply["reply"]["run_id"].as_str().unwrap().to_owned();
    assert_eq!(fs::read(&output).unwrap(), b"copy once");

    let cold = serve(
        &user,
        root.path(),
        Some("reattach"),
        request("submit", "once"),
    );
    assert_eq!(cold["result"]["structuredContent"], *first_reply);
    let inspect = serve(
        &user,
        root.path(),
        Some("reattach"),
        request("inspect", "once"),
    );
    assert_eq!(
        inspect["result"]["structuredContent"]["reply"]["run_id"],
        run
    );
    assert_eq!(
        fs::read_dir(root.path().join(".vibe/package-operations"))
            .unwrap()
            .count(),
        3,
        "one intent, capsule and result"
    );
}

#[test]
fn machine_frame_issued_package_grant_is_reused_by_mcp_reattach() {
    let user = UserScratch::new();
    let root = project();
    let issued = user
        .vibe()
        .args([
            "machine",
            "session",
            "--fixed-package",
            "issue",
            "--ttl-ms",
            "60000",
            "--root",
        ])
        .arg(root.path())
        .write_stdin("")
        .output()
        .unwrap();
    assert!(
        issued.status.success(),
        "{}",
        String::from_utf8_lossy(&issued.stderr)
    );
    assert!(issued.stdout.is_empty());
    let mcp = serve(
        &user,
        root.path(),
        Some("reattach"),
        request("submit", "same-host"),
    );
    assert_eq!(mcp["result"]["isError"], false, "{mcp}");
    assert_eq!(
        mcp["result"]["structuredContent"]["reply"]["kind"],
        "completed"
    );
    assert_eq!(
        fs::read(
            root.path()
                .join("target/vibe-package/launcher/launcher.bin")
        )
        .unwrap(),
        b"copy once"
    );
    let index: Value = serde_json::from_slice(
        &fs::read(root.path().join(".vibe/machine-submissions.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(index["namespaces"].as_object().unwrap().len(), 1);
}

#[test]
fn fixed_package_startup_rejects_relative_root_bad_lifetime_and_missing_pointer() {
    let user = UserScratch::new();
    let root = project();
    let mut relative = user.vibe();
    let response = relative
        .args([
            "--offline",
            "mcp",
            "serve",
            "--fixed-package",
            "issue",
            "--ttl-ms",
            "60000",
        ])
        .write_stdin(request("submit", "once"))
        .output()
        .unwrap();
    assert!(!response.status.success());
    assert!(response.stdout.is_empty());
    assert!(String::from_utf8_lossy(&response.stderr).contains("absolute --path"));

    let mut missing_lifetime = user.vibe();
    let response = missing_lifetime
        .args([
            "--offline",
            "mcp",
            "serve",
            "--fixed-package",
            "issue",
            "--path",
        ])
        .arg(root.path())
        .write_stdin(request("submit", "once"))
        .output()
        .unwrap();
    assert!(!response.status.success());
    assert!(response.stdout.is_empty());
    assert!(String::from_utf8_lossy(&response.stderr).contains("requires --ttl-ms"));

    let mut reattach_lifetime = user.vibe();
    let response = reattach_lifetime
        .args([
            "--offline",
            "mcp",
            "serve",
            "--fixed-package",
            "reattach",
            "--ttl-ms",
            "60000",
            "--path",
        ])
        .arg(root.path())
        .write_stdin(request("submit", "once"))
        .output()
        .unwrap();
    assert!(!response.status.success());
    assert!(response.stdout.is_empty());
    assert!(String::from_utf8_lossy(&response.stderr).contains("reattach forbids it"));

    let mut missing_pointer = user.vibe();
    let response = missing_pointer
        .args([
            "--offline",
            "mcp",
            "serve",
            "--fixed-package",
            "reattach",
            "--path",
        ])
        .arg(root.path())
        .write_stdin(request("submit", "once"))
        .output()
        .unwrap();
    assert!(!response.status.success());
    assert!(response.stdout.is_empty());
    assert!(
        !root
            .path()
            .join("target/vibe-package/launcher/launcher.bin")
            .exists()
    );
}
