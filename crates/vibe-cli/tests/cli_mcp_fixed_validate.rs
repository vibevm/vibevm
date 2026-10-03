//! Explicit fixed Validate MCP startup over the retained lifecycle host.
mod common;

use common::UserScratch;
use serde_json::{Value, json};
use std::fs;

fn project() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("vibe.toml"), "[project]\nname='demo'\nversion='1.1.0'\n[[extension]]\nid='once'\npoint='phase:validate'\nhandler={kind='script',base='run'}\n").unwrap();
    fs::write(
        root.path().join("run.sh"),
        b"#!/bin/sh\nset -eu\nprintf 'effect\\n' >> counter\n",
    )
    .unwrap();
    root
}
fn request(action: Value) -> String {
    json!({"jsonrpc":"2.0","id":71,"method":"tools/call",
        "params":{"name":"lifecycle_operation","arguments":{"schema":1,"action":action}}})
    .to_string()
        + "\n"
}
fn prepare() -> String {
    request(
        json!({"kind":"prepare","phase":"validate","requested_options":{
        "force":false,"trace":false,"deploy_profile":null,"control":"none"}}),
    )
}
fn serve(user: &UserScratch, root: &std::path::Path, mode: Option<&str>, input: String) -> Value {
    let mut command = user.vibe();
    command
        .env("VIBE_NO_DEFAULT_REGISTRY", "1")
        .args(["--offline", "mcp", "serve", "--path"])
        .arg(root);
    if let Some(mode) = mode {
        command.args(["--fixed-validate", mode, "--approve-captured-script"]);
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
    let text = String::from_utf8(output.stdout).unwrap();
    assert_eq!(text.lines().count(), 1, "{text}");
    serde_json::from_str(text.trim()).unwrap()
}
fn question(reply: &Value) -> Value {
    assert_eq!(reply["result"]["isError"], false, "{reply}");
    serde_json::from_str(
        reply["result"]["structuredContent"]["reply"]["payload_json"]
            .as_str()
            .unwrap(),
    )
    .unwrap()
}

#[test]
fn issue_and_reattach_use_one_host_but_never_restore_a_process_local_question() {
    let user = UserScratch::new();
    let root = project();
    let ungranted = serve(&user, root.path(), None, prepare());
    assert_eq!(ungranted["result"]["isError"], true);
    assert_eq!(
        ungranted["result"]["structuredContent"]["code"],
        "unavailable_host"
    );
    assert!(!root.path().join(".vibe").exists());

    let first = question(&serve(&user, root.path(), Some("issue"), prepare()));
    let old_id = first["question_id"].as_str().unwrap().to_owned();
    let mut answer = first.clone();
    answer["body"] = match first["body"]["kind"].as_str().unwrap() {
        "selection_question" => json!({"kind":"selection_answer","choice_ids":["none"]}),
        "approval_question" => json!({"kind":"approval_answer","approved":true}),
        other => panic!("unexpected question: {other}"),
    };
    let stale = serve(
        &user,
        root.path(),
        Some("reattach"),
        request(json!({
            "kind":"answer","message_json":answer.to_string()
        })),
    );
    assert_eq!(stale["result"]["isError"], true, "{stale}");
    assert_eq!(
        stale["result"]["structuredContent"]["code"],
        "decision_refused"
    );
    let fresh = question(&serve(&user, root.path(), Some("reattach"), prepare()));
    assert_ne!(fresh["question_id"], old_id);
    assert!(!root.path().join("counter").exists());
    let index: Value = serde_json::from_slice(
        &fs::read(root.path().join(".vibe/machine-submissions.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(index["namespaces"].as_object().unwrap().len(), 1);
}

#[test]
fn machine_frame_issued_validate_grant_is_reused_by_mcp() {
    let user = UserScratch::new();
    let root = project();
    let issued = user
        .vibe()
        .args([
            "machine",
            "session",
            "--fixed-validate",
            "issue",
            "--ttl-ms",
            "60000",
            "--approve-captured-script",
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
    let next = question(&serve(&user, root.path(), Some("reattach"), prepare()));
    assert!(next["question_id"].is_string());
    assert!(!root.path().join("counter").exists());
}

#[test]
fn validate_startup_refuses_relative_root_missing_lifetime_and_missing_pointer() {
    let user = UserScratch::new();
    let root = project();
    let relative = user
        .vibe()
        .args([
            "--offline",
            "mcp",
            "serve",
            "--fixed-validate",
            "issue",
            "--ttl-ms",
            "60000",
        ])
        .write_stdin(prepare())
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
            "--fixed-validate",
            "issue",
            "--path",
        ])
        .arg(root.path())
        .write_stdin(prepare())
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
            "--fixed-validate",
            "reattach",
            "--path",
        ])
        .arg(root.path())
        .write_stdin(prepare())
        .output()
        .unwrap();
    assert!(!missing_pointer.status.success());
    assert!(missing_pointer.stdout.is_empty());
    assert!(!root.path().join("counter").exists());
}
