//! The server replay: the whole JSON-RPC loop over the scripted
//! transport — initialize, tools/list carrying the declared inventory,
//! a malformed line answered without killing the loop, and a language
//! mismatch refused as a RESULT (never a protocol error). No agent
//! host, no gopls, no go anywhere near this suite.

use mcp_core::testing::Scripted;

fn frame(v: serde_json::Value) -> String {
    v.to_string()
}

#[test]
fn initialize_list_and_refusal_replay() {
    let mut transport = Scripted::new(vec![
        frame(serde_json::json!({
            "jsonrpc": "2.0", "id": 1, "method": "initialize",
            "params": { "protocolVersion": "2024-11-05" },
        })),
        "not json at all".to_string(),
        frame(serde_json::json!({
            "jsonrpc": "2.0", "id": 2, "method": "tools/list",
        })),
        frame(serde_json::json!({
            "jsonrpc": "2.0", "id": 3, "method": "tools/call",
            "params": {
                "name": "conform_check",
                "arguments": { "language": "typescript" },
            },
        })),
        frame(serde_json::json!({
            "jsonrpc": "2.0", "id": 4, "method": "ping",
        })),
    ]);
    let tools = go_ai_native_mcp::tool_set(std::path::Path::new("."));
    let mut server = mcp_core::Server::new(go_ai_native_mcp::SERVER_NAME, "test", tools);
    server.run(&mut transport).expect("the loop survives");

    let out: Vec<serde_json::Value> = transport
        .outbound
        .iter()
        .map(|l| serde_json::from_str(l).expect("outbound json"))
        .collect();

    // initialize
    assert_eq!(out[0]["id"], 1);
    assert_eq!(
        out[0]["result"]["serverInfo"]["name"],
        go_ai_native_mcp::SERVER_NAME
    );

    // the garbage line answered with parse error, loop alive
    assert_eq!(out[1]["error"]["code"], -32700);

    // tools/list is the declared inventory in stable order
    assert_eq!(out[2]["id"], 2);
    let listed: Vec<&str> = out[2]["result"]["tools"]
        .as_array()
        .expect("tools array")
        .iter()
        .filter_map(|t| t["name"].as_str())
        .collect();
    assert_eq!(listed, go_ai_native_mcp::TOOL_NAMES);

    // the language mismatch is an isError RESULT with the recipe
    assert_eq!(out[3]["id"], 3);
    assert_eq!(out[3]["result"]["isError"], true);
    let text = out[3]["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or_default();
    assert!(text.contains("typescript-ai-native-mcp"), "{text}");

    // ping
    assert_eq!(out[4]["id"], 4);
}
