//! Behavioural oracles for the two configuration-preview `McpTool` cells
//! (PROP-069 `##SURFACE-PARITY`). The admitted happy path needs the whole
//! isolated host posture and is pinned end to end through the registered
//! stdio server by the CLI suite; what is characterised here is the part
//! that belongs to the cells themselves: registration, the request shape
//! each one advertises, the bounded-input and closed-shape refusals, and
//! the fact that a refused preview leaves the project exactly as it was.

use std::collections::BTreeMap;
use std::path::Path;

use serde_json::{Value, json};
use vibe_mcp::tools::{
    ConfigurationPreviewMcpTool, ConfigurationPreviewV2McpTool, McpTool, default_tools,
};
use vibe_mcp::{ServerContext, ToolError, dispatch_one};

const PACKAGE: &str = "org.example/driver";
const V2_PROTOCOL: &str = "vibe-ordinary-integer-preview/2";
/// A value no diagnostic may echo back: shape errors omit caller bytes.
const CALLER_VALUE: &str = "caller-supplied-value-4f1c";

/// A project that requires nothing, so no preview can be admitted for
/// `PACKAGE` whatever registry posture the test process inherited.
fn cold_project() -> (tempfile::TempDir, ServerContext) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("vibe.toml"),
        "[project]\nname=\"x\"\nversion=\"0.0.1\"\n",
    )
    .unwrap();
    let ctx = ServerContext::new(dir.path());
    (dir, ctx)
}

fn tree(root: &Path) -> BTreeMap<String, Vec<u8>> {
    std::fs::read_dir(root)
        .unwrap()
        .map(|entry| {
            let entry = entry.unwrap();
            let name = entry.file_name().to_string_lossy().into_owned();
            let bytes = if entry.file_type().unwrap().is_file() {
                std::fs::read(entry.path()).unwrap()
            } else {
                Vec::new()
            };
            (name, bytes)
        })
        .collect()
}

fn oversized(mut request: Value) -> Value {
    request["package"] = Value::String("p".repeat(64 * 1024 + 1));
    request
}

#[test]
fn both_preview_cells_are_registered_under_their_descriptor_names() {
    let names: Vec<String> = default_tools()
        .iter()
        .map(|tool| tool.descriptor().name)
        .collect();
    assert_eq!(
        ConfigurationPreviewMcpTool.descriptor().name,
        "configuration_preview"
    );
    assert_eq!(
        ConfigurationPreviewV2McpTool.descriptor().name,
        "configuration_preview_v2"
    );
    for name in ["configuration_preview", "configuration_preview_v2"] {
        assert_eq!(
            names.iter().filter(|known| known.as_str() == name).count(),
            1,
            "{name} must be registered exactly once: {names:?}"
        );
    }
}

#[test]
fn descriptors_close_the_request_shape_and_never_accept_a_project_root() {
    let v1 = ConfigurationPreviewMcpTool.descriptor().input_schema;
    assert_eq!(v1["additionalProperties"], false);
    assert_eq!(v1["required"], json!(["package"]));
    assert_eq!(
        v1["properties"]["input"]["properties"]["parameters"]["additionalProperties"]["type"],
        "boolean"
    );

    let v2 = ConfigurationPreviewV2McpTool.descriptor().input_schema;
    assert_eq!(v2["additionalProperties"], false);
    assert_eq!(v2["required"], json!(["protocol", "package"]));
    assert_eq!(v2["properties"]["protocol"]["const"], V2_PROTOCOL);
    assert_eq!(
        v2["properties"]["input"]["properties"]["parameters"]["additionalProperties"]["type"],
        "integer"
    );

    for schema in [&v1, &v2] {
        let properties = schema["properties"].as_object().unwrap();
        assert!(
            !properties.contains_key("path") && !properties.contains_key("root"),
            "the host supplies project scope: {schema}"
        );
    }
}

#[test]
fn an_oversized_request_is_refused_as_invalid_arguments_before_any_read() {
    let (dir, ctx) = cold_project();
    let before = tree(dir.path());

    let v1 = ConfigurationPreviewMcpTool
        .run(&oversized(json!({ "package": "" })), &ctx)
        .unwrap_err();
    assert!(matches!(v1, ToolError::InvalidArguments(_)), "{v1}");
    assert!(v1.to_string().contains("bounded input limit"), "{v1}");

    let v2 = ConfigurationPreviewV2McpTool
        .run(
            &oversized(json!({ "protocol": V2_PROTOCOL, "package": "" })),
            &ctx,
        )
        .unwrap_err();
    assert!(matches!(v2, ToolError::InvalidArguments(_)), "{v2}");
    assert!(v2.to_string().contains("too large"), "{v2}");

    assert_eq!(tree(dir.path()), before);
}

#[test]
fn a_request_outside_the_closed_shape_is_refused_without_echoing_caller_values() {
    let (_dir, ctx) = cold_project();

    for request in [
        json!({ "package": PACKAGE, "path": CALLER_VALUE }),
        json!({ "package": PACKAGE, "input": Value::Null }),
        json!({ "package": PACKAGE, "input": {
            "selections": {}, "parameters": { "verbose": CALLER_VALUE }
        }}),
    ] {
        let error = ConfigurationPreviewMcpTool.run(&request, &ctx).unwrap_err();
        assert!(matches!(error, ToolError::InvalidArguments(_)), "{error}");
        assert!(!error.to_string().contains(CALLER_VALUE), "{error}");
    }

    for request in [
        // The protocol tag is compulsory: a v1-shaped call is not a v2 call.
        json!({ "package": PACKAGE }),
        json!({ "protocol": V2_PROTOCOL, "package": PACKAGE, "path": CALLER_VALUE }),
        json!({ "protocol": V2_PROTOCOL, "package": PACKAGE, "input": {
            "selections": {}, "parameters": { "workers": CALLER_VALUE }
        }}),
    ] {
        let error = ConfigurationPreviewV2McpTool
            .run(&request, &ctx)
            .unwrap_err();
        assert!(matches!(error, ToolError::InvalidArguments(_)), "{error}");
        assert!(!error.to_string().contains(CALLER_VALUE), "{error}");
    }
}

#[test]
fn an_unadmitted_source_is_a_typed_refusal_that_leaves_the_project_untouched() {
    let (dir, ctx) = cold_project();
    let before = tree(dir.path());

    let v1 = ConfigurationPreviewMcpTool
        .run(&json!({ "package": PACKAGE }), &ctx)
        .unwrap_err();
    // The library's own typed refusal survives the adapter, so the MCP
    // surface says what the CLI says for the same request.
    assert!(matches!(v1, ToolError::ConfigurationPreview(_)), "{v1}");
    assert!(v1.to_string().contains(PACKAGE), "{v1}");
    assert!(v1.to_string().contains("PROP-069"), "{v1}");

    let v2 = ConfigurationPreviewV2McpTool
        .run(
            &json!({ "protocol": V2_PROTOCOL, "package": PACKAGE }),
            &ctx,
        )
        .unwrap_err();
    assert!(matches!(v2, ToolError::PreExecution(_)), "{v2}");
    assert!(
        v2.to_string().contains("integer preview v2 refused"),
        "{v2}"
    );

    // A foreign protocol tag is not a shape error: it passes the adapter's
    // parse and the request is refused as a pre-execution failure. The
    // refusal is opaque, so this does not tell a tag mismatch apart from
    // the unadmitted source above.
    let foreign = ConfigurationPreviewV2McpTool
        .run(
            &json!({ "protocol": "vibe-ordinary-integer-preview/3", "package": PACKAGE }),
            &ctx,
        )
        .unwrap_err();
    assert!(matches!(foreign, ToolError::PreExecution(_)), "{foreign}");

    assert_eq!(tree(dir.path()), before, "a preview creates no state");
}

#[test]
fn the_registered_server_reports_a_refused_preview_as_an_error_result() {
    let (dir, ctx) = cold_project();
    let before = tree(dir.path());

    let request = json!({
        "jsonrpc": "2.0", "id": 7, "method": "tools/call",
        "params": { "name": "configuration_preview", "arguments": { "package": PACKAGE } }
    });
    let response: Value =
        serde_json::from_str(&dispatch_one(ctx, &request.to_string()).unwrap()).unwrap();

    assert_eq!(response["id"], 7);
    assert_eq!(response["result"]["isError"], true, "{response}");
    assert!(response.to_string().contains(PACKAGE), "{response}");
    assert_eq!(tree(dir.path()), before);
}
