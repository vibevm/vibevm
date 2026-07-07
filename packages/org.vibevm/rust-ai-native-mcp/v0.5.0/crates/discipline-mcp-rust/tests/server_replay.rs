//! The hermetic end-to-end: the BUILT server binary over real stdio on
//! a throwaway single-crate project — no rust-analyzer, no network, no
//! vibe (the common harness scrubs PATH). The discipline half runs for
//! real: init writes the surface, conform_check reports the gate's own
//! words, the language guard refuses with the recipe, and protocol
//! errors stay in-protocol.

mod common;

use common::Session;

#[test]
fn the_server_end_to_end_on_a_bare_project() {
    // A minimal law-abiding single-crate project (the mini-fix
    // campaign's bare shape): the discipline tools run on it for real.
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path();
    std::fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"replay-demo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .expect("manifest");
    std::fs::create_dir_all(root.join("src")).expect("src");
    // No pub surface yet: an untagged pub item on a fresh init'd
    // project is an ORPHAN the specmap ratchet rightly blocks (the
    // same refusal the CLI gives — parity includes the refusals).
    std::fs::write(root.join("src").join("lib.rs"), "//! the replay fixture\n").expect("lib");

    let mut s = Session::spawn(root);

    let init = s.call(serde_json::json!({
        "jsonrpc": "2.0", "id": 0, "method": "initialize", "params": {},
    }));
    assert_eq!(
        init["result"]["protocolVersion"],
        mcp_core::PROTOCOL_VERSION
    );
    assert_eq!(init["result"]["serverInfo"]["name"], "discipline-rust");

    let list = s.call(serde_json::json!({
        "jsonrpc": "2.0", "id": 1, "method": "tools/list",
    }));
    let names: Vec<&str> = list["result"]["tools"]
        .as_array()
        .expect("tools array")
        .iter()
        .map(|t| t["name"].as_str().expect("name"))
        .collect();
    assert_eq!(names, discipline_mcp_rust::TOOL_NAMES);
    // Heavy tools announce their budgets to the agent.
    let floor = list["result"]["tools"]
        .as_array()
        .expect("tools")
        .iter()
        .find(|t| t["name"] == "floor")
        .expect("floor listed");
    assert!(
        floor["description"]
            .as_str()
            .expect("description")
            .contains("minutes"),
    );

    // init writes the discipline surface into the temp project; its
    // report is the runner's own words, captured process-wide.
    let init_tool = s.tool(2, "init", serde_json::json!({"namespace": "replay"}));
    assert_eq!(init_tool["result"]["isError"], false, "{init_tool}");
    assert!(root.join("conform.toml").is_file());
    assert!(root.join("specmap.toml").is_file());
    let report = init_tool["result"]["content"][0]["text"]
        .as_str()
        .expect("report");
    assert!(report.contains("init: wrote conform.toml"), "{report}");

    // conform_check runs green on the generated policy (everything
    // discovered starts exempt), language guard satisfied explicitly.
    let check = s.tool(3, "conform_check", serde_json::json!({"language": "rust"}));
    assert_eq!(check["result"]["isError"], false, "{check}");
    let report = check["result"]["content"][0]["text"]
        .as_str()
        .expect("report");
    assert!(report.contains("conform check:"), "{report}");
    assert!(report.contains("0 crate(s) gated, 1 exempt"), "{report}");

    // specmap_write mints the index; specmap_check then byte-agrees —
    // and the scan-vacuity warning (nothing tagged yet) rides the
    // report, non-blocking: the mini-fix feature, seen through MCP.
    let write = s.tool(4, "specmap_write", serde_json::json!({}));
    assert_eq!(write["result"]["isError"], false, "{write}");
    let check = s.tool(5, "specmap_check", serde_json::json!({}));
    assert_eq!(check["result"]["isError"], false, "{check}");
    let report = check["result"]["content"][0]["text"]
        .as_str()
        .expect("report");
    assert!(report.contains("green by vacuity"), "{report}");

    // A red gate is an isError RESULT carrying the findings, never a
    // protocol error: gate the crate, then check a file with an unwrap.
    std::fs::write(
        root.join("conform.toml"),
        format!(
            "roots = [\".\"]\ngated_crates = [\"{}\"]\n",
            root.file_name().expect("dir name").to_string_lossy()
        ),
    )
    .expect("gate the crate");
    std::fs::write(
        root.join("src").join("lib.rs"),
        "pub fn risky() -> String { std::env::var(\"X\").unwrap() }\n",
    )
    .expect("seed violations");
    let red = s.tool(6, "conform_check", serde_json::json!({}));
    assert_eq!(red["result"]["isError"], true, "{red}");
    let report = red["result"]["content"][0]["text"]
        .as_str()
        .expect("report");
    assert!(report.contains("no-unwrap-in-domain"), "{report}");

    // The language guard refuses with the recipe, as a TOOL failure.
    let wrong = s.tool(7, "floor", serde_json::json!({"language": "typescript"}));
    assert_eq!(wrong["result"]["isError"], true);
    assert!(
        wrong["result"]["content"][0]["text"]
            .as_str()
            .expect("text")
            .contains("typescript-ai-native-mcp"),
    );

    // Protocol errors stay protocol errors.
    let bad = s.call(serde_json::json!({
        "jsonrpc": "2.0", "id": 8, "method": "tools/call", "params": {},
    }));
    assert_eq!(bad["error"]["code"], -32602);
    let ghost = s.tool(9, "ghost_tool", serde_json::json!({}));
    assert_eq!(ghost["error"]["code"], -32601);
}
