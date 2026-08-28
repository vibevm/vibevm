use serde_json::{Value, json};
use vibe_mcp::ToolError;
use vibe_mcp::tools::{LifecycleRunMcpTool, McpTool, default_tools};

use super::support::{context, project, run, tree};

#[test]
fn descriptor_is_the_exact_closed_default_phase_grammar_and_is_registered() {
    let descriptor = LifecycleRunMcpTool.descriptor();
    assert_eq!(descriptor.name, "lifecycle_run");
    assert_eq!(descriptor.input_schema["required"], json!(["phase"]));
    assert_eq!(descriptor.input_schema["additionalProperties"], false);
    assert_eq!(
        descriptor.input_schema["properties"]["phase"]["enum"],
        json!([
            "validate", "install", "generate", "build", "test", "create", "verify", "package",
            "deploy"
        ])
    );
    let names = default_tools()
        .into_iter()
        .map(|tool| tool.descriptor().name)
        .collect::<Vec<_>>();
    assert!(names.contains(&"lifecycle_run".to_string()));
    assert!(names.contains(&"lifecycle_tasks".to_string()));
}

#[test]
fn every_invalid_argument_shape_refuses_before_the_lease_or_any_vibe_state() {
    let project = project("");
    let ctx = context(project.path());
    let before = tree(project.path());
    let invalid = [
        Value::Null,
        json!([]),
        json!("build"),
        json!({}),
        json!({ "phase": 7 }),
        json!({ "phase": "BUILD" }),
        json!({ "phase": "clean" }),
        json!({ "phase": "build", "path": "elsewhere" }),
        json!({ "phase": "build", "force": true }),
    ];
    for arguments in invalid {
        let error = LifecycleRunMcpTool.run(&arguments, &ctx).unwrap_err();
        assert!(
            matches!(error, ToolError::InvalidArguments(_)),
            "{arguments}: {error}"
        );
        assert_eq!(tree(project.path()), before, "{arguments} mutated the tree");
        assert!(!project.path().join(".vibe/lifecycle.lock").exists());
    }
}

#[test]
fn invalid_arguments_are_in_band_text_only_with_no_structured_root() {
    let project = project("");
    let response = super::support::dispatch(
        context(project.path()),
        json!({ "phase": "build", "path": "elsewhere" }),
    );
    assert_eq!(response["result"]["isError"], true);
    assert!(response["result"]["structuredContent"].is_null());
    assert!(
        response["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("invalid arguments")
    );
}

#[test]
fn a_busy_workspace_is_pre_execution_and_writes_no_run_state_or_outbox() {
    let project = project("");
    let lease = vibe_lifecycle::LifecycleLease::acquire(project.path()).unwrap();
    let before = tree(project.path());
    let error = run(&context(project.path()), "build").unwrap_err();
    assert!(matches!(error, ToolError::PreExecution(_)));
    assert!(error.to_string().contains("lifecycle.lock"));
    assert_eq!(tree(project.path()), before);
    assert!(!project.path().join(".vibe/lifecycle.toml").exists());
    assert!(!project.path().join(".vibe/agentic/outbox").exists());
    drop(lease);
}
