//! Cross-surface parity for machine-readable package paths.

mod common;

use std::fs;

use common::UserScratch;
use serde_json::{Value, json};
use vibe_mcp::ServerContext;
use vibe_mcp::tools::{McpTool, QueryPackageMcpTool};

const WINDOWS_STYLE_LOCKFILE: &str = r#"
[meta]
generated_by = "vibe-test"
generated_at = "2026-08-20T00:00:00Z"
schema_version = 6

[[package]]
kind = "flow"
group = "org.vibevm"
name = "wal"
version = "0.2.0"
source_url = "https://example.invalid/org.vibevm.wal.git"
content_hash = "sha256:deadbeef"
files_written = [
    'spec\flows\wal\PROTOCOL.md',
    'spec\boot\10-flow-wal.md',
]
"#;

#[test]
fn list_json_and_query_package_return_identical_files_written_paths() {
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    user.init_project(project.path());
    fs::write(project.path().join("vibe.lock"), WINDOWS_STYLE_LOCKFILE).unwrap();

    let output = user
        .vibe()
        .arg("--json")
        .arg("list")
        .arg("--path")
        .arg(project.path())
        .output()
        .expect("spawn vibe list --json");
    assert!(
        output.status.success(),
        "vibe list --json failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let cli: Value = serde_json::from_slice(&output.stdout).expect("CLI JSON document");
    let cli_files = &cli["packages"][0]["files_written"];

    let ctx = ServerContext::with_store_root(
        project.path().to_path_buf(),
        project.path().join("store-root"),
    );
    let mcp = QueryPackageMcpTool
        .run(&json!({ "name": "org.vibevm/wal" }), &ctx)
        .expect("query_package result");
    let mcp_files = &mcp["files_written"];

    assert_eq!(cli_files, mcp_files, "CLI and MCP path forms diverged");
    assert_eq!(
        cli_files,
        &json!(["spec/flows/wal/PROTOCOL.md", "spec/boot/10-flow-wal.md"])
    );
}
