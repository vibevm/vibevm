//! Strict MCP adapter oracles for the generated lifecycle-task report.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use vibe_mcp::tools::{LifecycleTasksMcpTool, McpTool, default_tools};
use vibe_mcp::{ServerContext, ToolError, dispatch_one};
use vibe_wire::generated::lifecycle_state::{
    ExecutionRecord, ExecutionRecordScope, ExecutionRecordStatus, LifecycleState, StateArtifact,
    StateRun,
};
use vibe_wire::generated::lifecycle_tasks::{LifecycleTasks, LifecycleTasksStatus};

const RUN_ID: &str = "00112233445566778899aabbccddeeff";
const KEY: &str = "org.demo/provider#draft";

fn project() -> (tempfile::TempDir, ServerContext) {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("vibe.toml"),
        "[project]\nname = 'demo'\nversion = '0.1.0'\n",
    )
    .unwrap();
    let ctx = ServerContext::new(dir.path());
    (dir, ctx)
}

fn idle_state(started: &str) -> LifecycleState {
    LifecycleState {
        schema: 1,
        run: StateRun {
            chain: vec!["validate".into(), "install".into(), "create".into()],
            requested: "create".into(),
            started: started.into(),
            compile_trace: false,
            run_id: Some(RUN_ID.into()),
            selected: Some(".".into()),
            slot_continuation: None,
        },
        execution: BTreeMap::new(),
    }
}

fn parked_state() -> LifecycleState {
    let task = vibe_lifecycle::outbox_task_path(RUN_ID, KEY).unwrap();
    LifecycleState {
        schema: 1,
        run: StateRun {
            chain: vec!["validate".into(), "install".into(), "create".into()],
            requested: "create".into(),
            started: "parked".into(),
            compile_trace: false,
            run_id: Some(RUN_ID.into()),
            selected: Some(".".into()),
            slot_continuation: None,
        },
        execution: [(
            KEY.into(),
            ExecutionRecord {
                artifacts: vec![StateArtifact {
                    id: "guide".into(),
                    kind: "file".into(),
                    path: "docs/guide.md".into(),
                }],
                duration_ms: 0,
                fingerprint: "sha256:parked".into(),
                phase: "create".into(),
                status: ExecutionRecordStatus::Delegated,
                scope: Some(ExecutionRecordScope::Phase),
                tasks: vec![task],
            },
        )]
        .into_iter()
        .collect(),
    }
}

fn state_path(root: &Path) -> PathBuf {
    root.join(vibe_lifecycle::LifecycleStateStore::FILE)
}

fn write_state(root: &Path, state: &LifecycleState) {
    let path = state_path(root);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, toml::to_string_pretty(state).unwrap()).unwrap();
}

fn write_task(root: &Path, body: &str) -> String {
    let task = vibe_lifecycle::outbox_task_path(RUN_ID, KEY).unwrap();
    let path = root.join(&task);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, body).unwrap();
    task
}

fn decode_output(value: &Value) -> LifecycleTasks {
    serde_json::from_value(value.clone()).unwrap()
}

fn dispatch(ctx: ServerContext, arguments: Option<Value>) -> Value {
    let mut params = serde_json::Map::new();
    params.insert("name".into(), Value::String("lifecycle_tasks".into()));
    if let Some(arguments) = arguments {
        params.insert("arguments".into(), arguments);
    }
    let request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": params,
    })
    .to_string();
    serde_json::from_str(&dispatch_one(ctx, &request).unwrap()).unwrap()
}

#[test]
fn descriptor_registration_and_empty_argument_grammar_are_strict() {
    let descriptor = LifecycleTasksMcpTool.descriptor();
    assert_eq!(descriptor.name, "lifecycle_tasks");
    assert_eq!(descriptor.input_schema["type"], "object");
    assert_eq!(descriptor.input_schema["properties"], json!({}));
    assert_eq!(descriptor.input_schema["additionalProperties"], false);
    assert!(
        default_tools()
            .iter()
            .any(|tool| tool.descriptor().name == "lifecycle_tasks")
    );

    let (dir, ctx) = project();
    for args in [Value::Null, json!({})] {
        let output = LifecycleTasksMcpTool.run(&args, &ctx).unwrap();
        assert_eq!(
            decode_output(output.structured()).status,
            LifecycleTasksStatus::Absent
        );
    }
    assert!(!dir.path().join(".vibe").exists());

    let bad_ctx = ServerContext::new(dir.path().join("does-not-exist"));
    for args in [
        json!({"path": "."}),
        json!([]),
        json!(""),
        json!(1),
        json!(true),
    ] {
        let error = LifecycleTasksMcpTool.run(&args, &bad_ctx).unwrap_err();
        assert!(
            matches!(error, ToolError::InvalidArguments(_)),
            "{args}: {error}"
        );
    }
}

#[test]
fn dispatcher_accepts_omitted_arguments_and_lists_the_tool() {
    let (dir, ctx) = project();
    let response = dispatch(ctx, None);
    assert_eq!(response["result"]["isError"], false);
    assert_eq!(response["result"]["structuredContent"]["status"], "absent");
    assert!(!dir.path().join(".vibe").exists());

    let ctx = ServerContext::new(dir.path());
    let request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list",
        "params": {}
    })
    .to_string();
    let response: Value = serde_json::from_str(&dispatch_one(ctx, &request).unwrap()).unwrap();
    assert!(
        response["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|tool| tool["name"] == "lifecycle_tasks")
    );
}

#[test]
fn absent_idle_and_parked_are_generated_structured_content_with_same_text_projection() {
    let (dir, ctx) = project();
    let absent = dispatch(ctx, Some(json!({})));
    assert_report_projection(&absent, LifecycleTasksStatus::Absent);

    write_state(dir.path(), &idle_state("idle"));
    let idle = dispatch(ServerContext::new(dir.path()), Some(json!({})));
    assert_report_projection(&idle, LifecycleTasksStatus::Idle);
    assert_eq!(
        idle["result"]["structuredContent"]["run"]["started"],
        "idle"
    );

    write_state(dir.path(), &parked_state());
    let task = write_task(dir.path(), "exact task body\n");
    let parked = dispatch(ServerContext::new(dir.path()), Some(json!({})));
    assert_report_projection(&parked, LifecycleTasksStatus::Parked);
    assert_eq!(
        parked["result"]["structuredContent"]["tasks"][0]["path"],
        task
    );
    assert_eq!(
        parked["result"]["structuredContent"]["tasks"][0]["document"],
        "exact task body\n"
    );
}

fn assert_report_projection(response: &Value, status: LifecycleTasksStatus) {
    assert_eq!(response["result"]["isError"], false);
    let structured = response["result"]["structuredContent"].clone();
    let decoded: LifecycleTasks = serde_json::from_value(structured.clone()).unwrap();
    assert_eq!(decoded.status, status);
    let text = response["result"]["content"][0]["text"].as_str().unwrap();
    assert_eq!(text, serde_json::to_string_pretty(&structured).unwrap());
}

#[test]
fn lower_refusal_is_text_only_is_error_and_argument_refusal_precedes_filesystem() {
    let (dir, ctx) = project();
    write_state(dir.path(), &parked_state());
    // The state owns a task that is deliberately missing.
    let response = dispatch(ctx, Some(json!({})));
    assert_eq!(response["result"]["isError"], true);
    assert!(response["result"].get("structuredContent").is_none());
    assert!(
        response["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("absent")
    );

    let response = dispatch(
        ServerContext::new(dir.path().join("missing-root")),
        Some(json!({"path": "."})),
    );
    assert_eq!(response["result"]["isError"], true);
    assert!(response["result"].get("structuredContent").is_none());
    assert!(
        response["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("invalid arguments")
    );
}

#[test]
fn repeated_calls_on_one_context_observe_completion_without_cache() {
    let (dir, ctx) = project();
    write_state(dir.path(), &parked_state());
    write_task(dir.path(), "work\n");
    let first = LifecycleTasksMcpTool.run(&json!({}), &ctx).unwrap();
    assert_eq!(
        decode_output(first.structured()).status,
        LifecycleTasksStatus::Parked
    );

    write_state(dir.path(), &idle_state("completed"));
    let second = LifecycleTasksMcpTool.run(&json!({}), &ctx).unwrap();
    assert_eq!(
        decode_output(second.structured()).status,
        LifecycleTasksStatus::Idle
    );
}
