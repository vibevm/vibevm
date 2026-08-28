//! Production stdio wiring and the CLI↔MCP omnichannel oracle for the shared
//! default lifecycle command.

mod common;

use std::fs;

use common::UserScratch;
use serde_json::{Value, json};
use vibe_mcp::{ServerContext, dispatch_one};
use vibe_orchestrator::InstallPolicy;
use vibe_wire::generated::lifecycle_state::LifecycleState;

fn project() -> tempfile::TempDir {
    let project = tempfile::tempdir().unwrap();
    fs::write(
        project.path().join("vibe.toml"),
        "[package]\ngroup='org.parity'\nname='demo'\nkind='flow'\nversion='0.1.0'\n",
    )
    .unwrap();
    project
}

fn request(phase: &str) -> String {
    json!({
        "jsonrpc": "2.0",
        "id": 9,
        "method": "tools/call",
        "params": {
            "name": "lifecycle_run",
            "arguments": { "phase": phase }
        }
    })
    .to_string()
}

fn mcp_context(root: &std::path::Path) -> ServerContext {
    let policy = InstallPolicy {
        offline: true,
        ..InstallPolicy::default()
    };
    ServerContext::new(root).with_lifecycle_execution(policy, None, false)
}

#[test]
fn production_mcp_serve_executes_lifecycle_run_over_one_stdio_frame() {
    let user = UserScratch::new();
    let project = project();
    let output = user
        .vibe()
        .args(["--offline", "mcp", "serve", "--path"])
        .arg(project.path())
        .write_stdin(request("build") + "\n")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(
        stdout.lines().count(),
        1,
        "one request, one frame: {stdout}"
    );
    let response: Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(response["id"], 9);
    assert_eq!(response["result"]["isError"], false);
    assert_eq!(
        response["result"]["structuredContent"]["chain"],
        json!(["validate", "install", "generate", "build"])
    );
}

#[test]
fn cli_json_and_mcp_return_the_same_report_and_normalized_state() {
    let user = UserScratch::new();
    let cli_project = project();
    let mcp_project = project();

    let cli = user
        .vibe()
        .args([
            "--json",
            "--offline",
            "--agent-mode",
            "agent",
            "--invoked-by",
            "parity-test",
            "--unattended",
            "build",
            "--assume-yes",
            "--path",
        ])
        .arg(cli_project.path())
        .output()
        .unwrap();
    assert!(
        cli.status.success(),
        "{}",
        String::from_utf8_lossy(&cli.stderr)
    );
    let mut cli_documents = serde_json::Deserializer::from_slice(&cli.stdout)
        .into_iter::<Value>()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let mut cli_report = cli_documents.pop().expect("the lifecycle root is last");
    let object = cli_report.as_object_mut().unwrap();
    object.remove("invoked_by");
    object.remove("unattended");

    let mcp: Value = serde_json::from_str(
        &dispatch_one(mcp_context(mcp_project.path()), &request("build")).unwrap(),
    )
    .unwrap();
    assert_eq!(cli_report, mcp["result"]["structuredContent"]);

    let mut cli_state: LifecycleState = toml::from_str(
        &fs::read_to_string(cli_project.path().join(".vibe/lifecycle.toml")).unwrap(),
    )
    .unwrap();
    let mut mcp_state: LifecycleState = toml::from_str(
        &fs::read_to_string(mcp_project.path().join(".vibe/lifecycle.toml")).unwrap(),
    )
    .unwrap();
    // These are the only intentionally fresh values. Both surfaces ran in
    // Agent mode, so execution fingerprints (when present) are never erased
    // from the parity oracle.
    cli_state.run.run_id = Some("0".repeat(32));
    mcp_state.run.run_id = Some("0".repeat(32));
    cli_state.run.started = "NORMALIZED".into();
    mcp_state.run.started = "NORMALIZED".into();
    assert_eq!(
        serde_json::to_value(cli_state).unwrap(),
        serde_json::to_value(mcp_state).unwrap()
    );
}
