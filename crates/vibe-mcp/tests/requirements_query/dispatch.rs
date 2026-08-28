//! The real JSON-RPC dispatcher: one call, one frame, the generated root
//! under `structuredContent` and the library's bounded projection in the
//! single text row.

use serde_json::{Value, json};
use vibe_mcp::{ServerContext, dispatch_one};

use super::support::{ADDRESS, HOST, PROSE, REQUEST_ID, project_with_map, request};

fn call(root: &std::path::Path, arguments: Value) -> Value {
    let line = dispatch_one(ServerContext::new(root), &request(arguments)).unwrap();
    assert_eq!(
        line.lines().count(),
        1,
        "a tool call is exactly one JSON-RPC frame: {line}"
    );
    serde_json::from_str(&line).unwrap()
}

/// `tools/list` advertises the tool with the grammar §6.2 fixed: three
/// optional members, no `path`, and no second requirements/evidence tool.
#[test]
fn tools_list_advertises_three_optional_members_and_no_path() {
    let project = project_with_map();
    let line = dispatch_one(
        ServerContext::new(project.path()),
        &json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/list", "params": {} }).to_string(),
    )
    .unwrap();
    let response: Value = serde_json::from_str(&line).unwrap();
    let tools = response["result"]["tools"].as_array().unwrap();
    let descriptor = tools
        .iter()
        .find(|tool| tool["name"] == "requirements_query")
        .expect("the tool is advertised");
    let properties = descriptor["inputSchema"]["properties"].as_object().unwrap();
    let mut names: Vec<&str> = properties.keys().map(String::as_str).collect();
    names.sort_unstable();
    assert_eq!(names, ["address_prefix", "limit", "relations"]);
    assert_eq!(descriptor["inputSchema"]["additionalProperties"], false);
    assert!(descriptor["inputSchema"]["required"].is_null());
    assert_eq!(
        tools
            .iter()
            .filter(|tool| tool["name"] == "requirements_query")
            .count(),
        1
    );
}

/// The ordinary answer over the wire: `isError: false`, the generated root
/// as the single `structuredContent`, one text row, and no fact prose in
/// either channel.
#[test]
fn one_call_returns_the_generated_root_and_the_bounded_text() {
    let project = project_with_map();
    let response = call(project.path(), json!({}));

    assert!(response["error"].is_null(), "{response}");
    assert_eq!(response["id"], REQUEST_ID);
    assert_eq!(response["result"]["isError"], false);

    let structured = &response["result"]["structuredContent"];
    assert_eq!(structured["requirements"], 1);
    assert_eq!(structured["query"]["limit"], 100);
    assert_eq!(structured["query"]["relations"], false);
    assert_eq!(structured["truncated"], false);
    assert_eq!(structured["rows"][0]["address"], ADDRESS);
    assert_eq!(structured["sources"][0]["source"]["package"], HOST);
    assert_eq!(structured["sources"][0]["state"], "available");
    assert_eq!(structured["relation_sources"][0]["state"], "not-requested");
    assert!(structured["observation"]["observation_id"].is_string());
    assert!(structured["observation"]["lifecycle_run_id"].is_null());

    let content = response["result"]["content"].as_array().unwrap();
    assert_eq!(content.len(), 1);
    assert_eq!(content[0]["type"], "text");
    let text = content[0]["text"].as_str().unwrap();
    assert!(text.contains(ADDRESS), "{text}");
    assert!(text.contains("authoring=marked=impl/done"), "{text}");
    assert!(
        !text.contains(PROSE),
        "the text channel carried prose: {text}"
    );
    assert!(!structured.to_string().contains(PROSE));
}

/// `relations = true` over a project that really carries a map: the
/// injected provider ran a fresh project-map build and the enrichment layer
/// says so — `current` / `fresh-project-map`, the two words a host source
/// alone may wear.
#[test]
fn a_relations_call_runs_a_real_scan_and_reports_a_fresh_project_map() {
    let project = project_with_map();
    let response = call(project.path(), json!({ "relations": true }));

    assert_eq!(response["result"]["isError"], false);
    let structured = &response["result"]["structuredContent"];
    assert_eq!(structured["query"]["relations"], true);
    let relation = &structured["relation_sources"][0];
    assert_eq!(relation["package"], HOST);
    assert_eq!(relation["state"], "current", "{structured}");
    assert_eq!(relation["provenance"], "fresh-project-map");
    assert!(relation["reason_code"].is_null());
    // The base layer answered exactly as it does without enrichment.
    assert_eq!(structured["rows"][0]["address"], ADDRESS);
    // Nothing was written: a scan is a read.
    assert!(!project.path().join("specmap.json").exists());
    assert!(!project.path().join(".vibe").exists());
}

/// A preflight refusal over the wire: `isError: true`, the typed chain in
/// the text row, NO `structuredContent` — and not one byte created.
#[test]
fn an_unknown_member_refuses_over_the_wire_with_no_structured_content() {
    let project = project_with_map();
    for arguments in [json!({ "path": "." }), json!({ "limit": 0 })] {
        let response = call(project.path(), arguments.clone());
        assert!(response["error"].is_null(), "{response}");
        assert_eq!(response["result"]["isError"], true, "{arguments}");
        assert!(
            response["result"]["structuredContent"].is_null(),
            "a refusal that never ran carries no report: {response}"
        );
        assert!(
            response["result"]["content"][0]["text"]
                .as_str()
                .is_some_and(|text| text.contains("invalid arguments")),
            "{response}"
        );
        assert!(!project.path().join(".vibe").exists());
    }
}
