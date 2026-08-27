//! Byte-exact dispatcher-compatibility oracles for the three result
//! arms around the `ToolOutput` seam: ordinary success with an object
//! value, ordinary success with a string value, and the preflight
//! `Err(ToolError)` arm. Out of line for the 600-line file budget, by
//! the crate's own idiom (`tools_oracle.rs` → this submodule); the
//! shared fixtures stay in the parent.

use serde_json::{Value, json};
use vibe_mcp::ToolError;
use vibe_mcp::dispatch_one;

use super::{LOCKFILE_FIXTURE, project_with_locked, project_with_specmap};

/// A representative OBJECT tool renders exactly as before the seam:
/// `isError: false`, `structuredContent` IS the value, and the text
/// channel is the pretty-JSON projection of that same value.
#[test]
fn dispatch_object_tool_renders_exactly_as_before_the_seam() {
    let (_dir, ctx) = project_with_locked(LOCKFILE_FIXTURE);
    let req = json!({
        "jsonrpc": "2.0",
        "id": 41,
        "method": "tools/call",
        "params": { "name": "query_package", "arguments": { "name": "org.vibevm/wal" } }
    })
    .to_string();
    let reply = dispatch_one(ctx, &req).unwrap();
    let v: Value = serde_json::from_str(&reply).unwrap();
    assert!(v["error"].is_null(), "{v}");
    assert_eq!(v["result"]["isError"], false);
    let structured = v["result"]["structuredContent"].clone();
    assert_eq!(structured["name"], "wal");
    assert_eq!(v["result"]["content"].as_array().map(|a| a.len()), Some(1));
    assert_eq!(v["result"]["content"][0]["type"], "text");
    assert_eq!(
        v["result"]["content"][0]["text"],
        serde_json::to_string_pretty(&structured).unwrap()
    );
}

/// A representative STRING tool (`explain`'s text view returns
/// `Value::String`) renders the RAW string in the text channel —
/// unquoted, not JSON — and the same string as `structuredContent`.
#[test]
fn dispatch_string_tool_renders_raw_text_as_before_the_seam() {
    let (_dir, ctx) = project_with_specmap();
    let req = json!({
        "jsonrpc": "2.0",
        "id": 42,
        "method": "tools/call",
        "params": { "name": "explain", "arguments": { "target": "spec://demo/D#req-r" } }
    })
    .to_string();
    let reply = dispatch_one(ctx, &req).unwrap();
    let v: Value = serde_json::from_str(&reply).unwrap();
    assert!(v["error"].is_null(), "{v}");
    assert_eq!(v["result"]["isError"], false);
    let text = v["result"]["content"][0]["text"].as_str().unwrap();
    assert!(!text.starts_with('"'), "raw text, not a JSON-quoted string");
    assert_eq!(v["result"]["structuredContent"].as_str(), Some(text));
}

/// The preflight arm renders exactly as before the seam: `isError:
/// true`, the error's own exact text, and NO `structuredContent` key at
/// all — there is no structured report to carry.
#[test]
fn dispatch_preflight_error_renders_text_only_without_structured_content() {
    let (_dir, ctx) = project_with_locked(LOCKFILE_FIXTURE);
    let req = json!({
        "jsonrpc": "2.0",
        "id": 43,
        "method": "tools/call",
        "params": { "name": "query_package", "arguments": {} }
    })
    .to_string();
    let reply = dispatch_one(ctx, &req).unwrap();
    let v: Value = serde_json::from_str(&reply).unwrap();
    assert!(v["error"].is_null(), "{v}");
    assert_eq!(v["result"]["isError"], true);
    assert_eq!(
        v["result"]["content"][0]["text"],
        ToolError::InvalidArguments("`name` must be a string".into()).to_string()
    );
    assert!(
        v["result"].get("structuredContent").is_none(),
        "the preflight arm carries no structuredContent: {v}"
    );
}
