//! The live chain (P3/P4): the BUILT server binary over real stdio
//! against the committed ts-demo consumer — initialize, the 17-tool
//! list, a clean tcg_validate, a seeded TS2322 through a pure overlay,
//! and the conform gate; disk byte-identical after. The common harness
//! scrubs vibe from the child's PATH, so a pass IS the PROP-027 §2.6
//! vibe-free acceptance. Ignored by default: it needs node ≥22.6 and
//! ts-demo's node_modules junction; `cargo test … -- --ignored` runs
//! it in the wave panel.

mod common;

use std::path::PathBuf;

use common::Session;

fn ts_demo() -> PathBuf {
    // packages/org.vibevm/discipline-typescript/v0.4.0/crates/… → six
    // ancestors up to the repo root (0 = this dir).
    let here = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let repo = here
        .ancestors()
        .nth(6)
        .expect("repo root above the package")
        .to_path_buf();
    repo.join("research").join("ts-demo")
}

#[test]
#[ignore = "needs node + ts-demo's node_modules junction; run in the wave panel"]
fn live_chain_on_ts_demo_without_vibe() {
    let root = ts_demo();
    assert!(root.join("vibe.toml").is_file(), "ts-demo present");
    let cell = "src/cells/greeting/index.ts";
    let disk_before = std::fs::read_to_string(root.join(cell)).expect("demo cell");

    let mut s = Session::spawn(&root);

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
        .expect("tools")
        .iter()
        .map(|t| t["name"].as_str().expect("name"))
        .collect();
    assert_eq!(names, discipline_mcp_typescript::TOOL_NAMES);

    // Clean validate: the committed demo cell, from disk.
    let clean = s.tool(2, "tcg_validate", serde_json::json!({"file": cell}));
    assert_eq!(clean["result"]["isError"], false, "{clean}");

    // A seeded type error through a PURE overlay — a TS2322 the oracle
    // must catch, the disk must never see.
    let seeded = format!("{disk_before}\nexport const seededTypeError: number = \"nope\";\n");
    let red = s.tool(
        3,
        "tcg_validate",
        serde_json::json!({"file": cell, "content": seeded}),
    );
    assert_eq!(red["result"]["isError"], true, "{red}");
    let report = red["result"]["content"][0]["text"].as_str().expect("text");
    assert!(report.contains("2322"), "{report}");

    // The discipline half over the same session: the demo's TS conform
    // gate is green through the server too (its one frozen brand-cast
    // finding is baselined, not new).
    let conform = s.tool(4, "conform_check", serde_json::json!({}));
    assert_eq!(conform["result"]["isError"], false, "{conform}");

    drop(s);
    let disk_after = std::fs::read_to_string(root.join(cell)).expect("demo cell after");
    assert_eq!(disk_before, disk_after, "the overlay never touched disk");
}
