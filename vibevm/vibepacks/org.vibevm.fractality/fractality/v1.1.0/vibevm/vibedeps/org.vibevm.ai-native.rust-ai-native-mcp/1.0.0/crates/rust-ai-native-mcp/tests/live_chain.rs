//! The live chain (P3/P4): the BUILT server binary over real stdio
//! against the committed rust-demo consumer — initialize, the 18-tool
//! list, a clean tcg_validate, a seeded type error through a pure
//! overlay, and the conform gate; disk byte-identical after. The
//! common harness scrubs vibe from the child's PATH, so a pass IS the
//! PROP-027 §2.6 vibe-free acceptance. Ignored by default: it needs
//! rust-analyzer (the stack prerequisite) and the in-repo demo;
//! `cargo test … -- --ignored` runs it in the wave panel.

mod common;

use std::path::PathBuf;

use common::Session;

fn rust_demo() -> PathBuf {
    // packages/org.vibevm.ai-native/rust-ai-native-mcp/v0.5.0/crates/rust-ai-native-mcp
    // → six ancestors up to the repo root (0 = this dir).
    let here = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let repo = here
        .ancestors()
        .nth(6)
        .expect("repo root above the package")
        .to_path_buf();
    repo.join("research").join("rust-demo")
}

#[test]
#[ignore = "needs rust-analyzer + the in-repo rust-demo; run in the wave panel"]
fn live_chain_on_rust_demo_without_vibe() {
    let root = rust_demo();
    assert!(root.join("vibe.toml").is_file(), "rust-demo present");
    let disk_before = std::fs::read_to_string(root.join("crates/rust-demo/src/cells/greeting.rs"))
        .expect("demo cell");

    let mut s = Session::spawn(&root);

    let init = s.call(serde_json::json!({
        "jsonrpc": "2.0", "id": 0, "method": "initialize", "params": {},
    }));
    assert_eq!(
        init["result"]["protocolVersion"],
        mcp_core::PROTOCOL_VERSION
    );
    assert_eq!(init["result"]["serverInfo"]["name"], "rust-ai-native");

    let list = s.call(serde_json::json!({
        "jsonrpc": "2.0", "id": 1, "method": "tools/list",
    }));
    let names: Vec<&str> = list["result"]["tools"]
        .as_array()
        .expect("tools")
        .iter()
        .map(|t| t["name"].as_str().expect("name"))
        .collect();
    assert_eq!(names, rust_ai_native_mcp::TOOL_NAMES);

    // Clean validate: the committed demo cell, from disk.
    let clean = s.tool(
        2,
        "tcg_validate",
        serde_json::json!({"file": "crates/rust-demo/src/cells/greeting.rs"}),
    );
    assert_eq!(clean["result"]["isError"], false, "{clean}");

    // A seeded type error through a PURE overlay — an E0308 the oracle
    // must catch, the disk must never see.
    let seeded = format!(
        "{disk_before}\npub fn seeded_type_error() {{ let _x: u32 = \"not a number\"; }}\n"
    );
    let red = s.tool(
        3,
        "tcg_validate",
        serde_json::json!({
            "file": "crates/rust-demo/src/cells/greeting.rs",
            "content": seeded,
        }),
    );
    assert_eq!(red["result"]["isError"], true, "{red}");
    let report = red["result"]["content"][0]["text"].as_str().expect("text");
    assert!(
        report.contains("E0308") || report.contains("expected"),
        "{report}"
    );

    // The discipline half over the same session: the demo's conform
    // gate is green through the server too.
    let conform = s.tool(4, "conform_check", serde_json::json!({}));
    assert_eq!(conform["result"]["isError"], false, "{conform}");

    drop(s);
    let disk_after = std::fs::read_to_string(root.join("crates/rust-demo/src/cells/greeting.rs"))
        .expect("demo cell after");
    assert_eq!(disk_before, disk_after, "the overlay never touched disk");
}
