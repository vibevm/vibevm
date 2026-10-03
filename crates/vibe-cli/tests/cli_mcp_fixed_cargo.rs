//! Explicit fixed Cargo MCP startup over the existing host and build journal.
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
    fs::write(root.path().join("Cargo.toml"), b"[package]\nname = \"vibe-r8-fixture\"\nversion = \"1.1.0\"\nedition = \"2021\"\n\n[[bin]]\nname = \"r8probe\"\npath = \"src/main.rs\"\n\n[workspace]\n").unwrap();
    fs::create_dir_all(root.path().join("src")).unwrap();
    fs::write(
        root.path().join("src/main.rs"),
        b"fn main() {\n    println!(\"r8\");\n}\n",
    )
    .unwrap();
    root
}

fn request(kind: &str, key: &str) -> String {
    json!({"jsonrpc":"2.0","id":51,"method":"tools/call",
        "params":{"name":"fixed_cargo_operation","arguments":{
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
        command.args(["--fixed-cargo", mode]);
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
fn explicit_cargo_issue_and_reattach_retain_one_build_run() {
    let user = UserScratch::new();
    let root = project();
    let ungranted = serve(&user, root.path(), None, request("inspect", "once"));
    assert_eq!(ungranted["result"]["isError"], true);
    assert_eq!(
        ungranted["result"]["structuredContent"]["code"],
        "unavailable_host"
    );
    assert!(!root.path().join("target").exists());

    let first = serve(&user, root.path(), Some("issue"), request("submit", "once"));
    assert_eq!(first["result"]["isError"], false, "{first}");
    let reply = &first["result"]["structuredContent"]["reply"];
    assert_eq!(reply["kind"], "completed");
    assert!(reply["warning"].as_str().unwrap().contains("rustup"));
    let run = reply["run_id"].as_str().unwrap().to_owned();
    assert!(
        root.path()
            .join(format!(".vibe/build-operations/{run}.cargo-result.json"))
            .is_file()
    );

    let cold = serve(
        &user,
        root.path(),
        Some("reattach"),
        request("submit", "once"),
    );
    assert_eq!(
        cold["result"]["structuredContent"],
        first["result"]["structuredContent"]
    );
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
    let index: Value = serde_json::from_slice(
        &fs::read(root.path().join(".vibe/machine-submissions.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(index["namespaces"].as_object().unwrap().len(), 1);
}

#[test]
fn cargo_grant_from_machine_frame_can_be_reattached_by_mcp() {
    let user = UserScratch::new();
    let root = project();
    let issued = user
        .vibe()
        .args([
            "machine",
            "session",
            "--fixed-cargo",
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
    let inspect = serve(
        &user,
        root.path(),
        Some("reattach"),
        request("inspect", "other"),
    );
    assert_eq!(inspect["result"]["isError"], false, "{inspect}");
    assert_eq!(
        inspect["result"]["structuredContent"]["reply"]["kind"],
        "absent"
    );
    assert!(!root.path().join("target").exists());
}

#[test]
fn cargo_startup_rejects_relative_root_bad_lifetime_and_missing_pointer() {
    let user = UserScratch::new();
    let root = project();
    let relative = user
        .vibe()
        .args([
            "--offline",
            "mcp",
            "serve",
            "--fixed-cargo",
            "issue",
            "--ttl-ms",
            "60000",
        ])
        .write_stdin(request("submit", "once"))
        .output()
        .unwrap();
    assert!(!relative.status.success());
    assert!(relative.stdout.is_empty());
    assert!(String::from_utf8_lossy(&relative.stderr).contains("absolute --path"));
    let missing_lifetime = user
        .vibe()
        .args([
            "--offline",
            "mcp",
            "serve",
            "--fixed-cargo",
            "issue",
            "--path",
        ])
        .arg(root.path())
        .write_stdin(request("submit", "once"))
        .output()
        .unwrap();
    assert!(!missing_lifetime.status.success());
    assert!(missing_lifetime.stdout.is_empty());
    assert!(String::from_utf8_lossy(&missing_lifetime.stderr).contains("requires --ttl-ms"));
    let missing_pointer = user
        .vibe()
        .args([
            "--offline",
            "mcp",
            "serve",
            "--fixed-cargo",
            "reattach",
            "--path",
        ])
        .arg(root.path())
        .write_stdin(request("submit", "once"))
        .output()
        .unwrap();
    assert!(!missing_pointer.status.success());
    assert!(missing_pointer.stdout.is_empty());
    assert!(!root.path().join("Cargo.lock").exists());
    assert!(!root.path().join("target").exists());
}
