//! The hermetic end-to-end: the BUILT server binary over real stdio on
//! a throwaway project — no node toolchain, no network, no vibe (the
//! common harness scrubs PATH of vibe). What CAN run nodeless runs for
//! real (init writes the surface); what NEEDS the consumer's
//! typescript hard-fails WITH THE RECIPE as an isError result — the
//! absent-toolchain posture, seen through MCP.

mod common;

use common::Session;

#[test]
fn the_server_end_to_end_on_a_bare_project() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path();
    std::fs::create_dir_all(root.join("src")).expect("src");
    std::fs::write(root.join("src").join("index.ts"), "export {};\n").expect("index");

    let mut s = Session::spawn(root);

    let init = s.call(serde_json::json!({
        "jsonrpc": "2.0", "id": 0, "method": "initialize", "params": {},
    }));
    assert_eq!(
        init["result"]["protocolVersion"],
        mcp_core::PROTOCOL_VERSION
    );
    assert_eq!(
        init["result"]["serverInfo"]["name"],
        "discipline-typescript"
    );

    let list = s.call(serde_json::json!({
        "jsonrpc": "2.0", "id": 1, "method": "tools/list",
    }));
    let names: Vec<&str> = list["result"]["tools"]
        .as_array()
        .expect("tools array")
        .iter()
        .map(|t| t["name"].as_str().expect("name"))
        .collect();
    assert_eq!(names, discipline_mcp_typescript::TOOL_NAMES);

    // init writes the discipline surface — nodeless, for real.
    let init_tool = s.tool(2, "init", serde_json::json!({"namespace": "replay"}));
    assert_eq!(init_tool["result"]["isError"], false, "{init_tool}");
    assert!(root.join("conform.toml").is_file());
    assert!(root.join("specmap.toml").is_file());

    // conform_check needs the consumer's own typescript install; on a
    // bare project it hard-fails WITH THE RECIPE — as a tool result.
    let check = s.tool(
        3,
        "conform_check",
        serde_json::json!({"language": "typescript"}),
    );
    assert_eq!(check["result"]["isError"], true, "{check}");
    let report = check["result"]["content"][0]["text"]
        .as_str()
        .expect("report");
    assert!(report.contains("typescript"), "{report}");

    // The language guard refuses with the recipe naming the rust server.
    let wrong = s.tool(4, "floor", serde_json::json!({"language": "rust"}));
    assert_eq!(wrong["result"]["isError"], true);
    assert!(
        wrong["result"]["content"][0]["text"]
            .as_str()
            .expect("text")
            .contains("rust-ai-native-mcp"),
    );

    // Protocol errors stay protocol errors.
    let bad = s.call(serde_json::json!({
        "jsonrpc": "2.0", "id": 5, "method": "tools/call", "params": {},
    }));
    assert_eq!(bad["error"]["code"], -32602);
    let ghost = s.tool(6, "ghost_tool", serde_json::json!({}));
    assert_eq!(ghost["error"]["code"], -32601);
}
