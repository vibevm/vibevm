use super::*;
use std::{fs, sync::Arc};
use vibe_core::manifest::TargetOs;
use vibe_orchestrator::machine_submission::cargo_grant::CargoGrantService;

fn fixture() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("Cargo.toml"), b"[package]\nname = \"vibe-r8-fixture\"\nversion = \"1.1.0\"\nedition = \"2021\"\n\n[[bin]]\nname = \"r8probe\"\npath = \"src/main.rs\"\n\n[workspace]\n").unwrap();
    fs::create_dir_all(root.path().join("src")).unwrap();
    fs::write(
        root.path().join("src/main.rs"),
        b"fn main() {\n    println!(\"r8\");\n}\n",
    )
    .unwrap();
    root
}
fn call(ctx: ServerContext, args: Value) -> Value {
    let request = json!({"jsonrpc":"2.0","id":41,"method":"tools/call",
        "params":{"name":"fixed_cargo_operation","arguments":args}});
    let response = crate::dispatch_one(ctx, &request.to_string()).unwrap();
    serde_json::from_str::<Value>(&response).unwrap()["result"].clone()
}
fn action(kind: &str, key: &str) -> Value {
    json!({"schema":1,"action":{"kind":kind,"key":key}})
}
fn bound(root: &std::path::Path) -> Arc<FixedCargoRoute> {
    let project = Arc::new(vibe_safefs::Project::open(root).unwrap());
    let service = CargoGrantService::from_trusted_builtin_host(
        project,
        "shared-principal".into(),
        TargetOs::current().unwrap(),
    )
    .unwrap();
    Arc::new(FixedCargoRoute::new(Arc::new(
        service.issue(60_000).unwrap(),
    )))
}
fn context(root: &std::path::Path, route: Arc<FixedCargoRoute>) -> ServerContext {
    ServerContext::with_store_root(root, root.join("unused"))
        .with_fixed_cargo_route(route)
        .unwrap()
}

#[test]
fn registered_closed_descriptor_and_ungranted_call_create_no_owner_state() {
    let root = tempfile::tempdir().unwrap();
    let descriptor = FixedCargoOperationMcpTool.descriptor();
    assert_eq!(descriptor.name, "fixed_cargo_operation");
    assert_eq!(descriptor.input_schema["additionalProperties"], false);
    for arm in descriptor.input_schema["properties"]["action"]["oneOf"]
        .as_array()
        .unwrap()
    {
        assert_eq!(arm["additionalProperties"], false);
    }
    let listing = json!({"jsonrpc":"2.0","id":1,"method":"tools/list"});
    let listed: Value = serde_json::from_str(
        &crate::dispatch_one(
            ServerContext::with_store_root(root.path(), root.path().join("unused")),
            &listing.to_string(),
        )
        .unwrap(),
    )
    .unwrap();
    assert!(
        listed["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|tool| tool["name"] == "fixed_cargo_operation")
    );
    let result = call(
        ServerContext::with_store_root(root.path(), root.path().join("unused")),
        action("inspect", "once"),
    );
    assert_eq!(result["isError"], true);
    assert_eq!(result["structuredContent"]["code"], "unavailable_host");
    assert_eq!(result["structuredContent"]["effect_observation"], "none");
    assert!(!root.path().join(".vibe").exists());
}

#[test]
fn forged_and_oversized_arguments_refuse_with_generated_preflight_before_cargo() {
    let root = fixture();
    let route = bound(root.path());
    for args in [
        json!({"schema":2,"action":{"kind":"submit","key":"once"}}),
        json!({"schema":1,"action":{"kind":"submit","key":"once","namespace":"caller"}}),
        json!({"schema":1,"action":{"kind":"submit","key":"once","argv":["cargo"]}}),
        json!({"schema":1,"action":{"kind":"submit","key":7}}),
        json!({"schema":1,"action":{"kind":"submit","key":"x".repeat(130_000)}}),
    ] {
        let result = call(context(root.path(), route.clone()), args);
        assert_eq!(result["isError"], true, "{result}");
        assert_eq!(
            result["structuredContent"]["code"], "invalid_request",
            "{result}"
        );
        assert_eq!(
            result["structuredContent"]["requested_scope"]["target"],
            "fixture-tool"
        );
        assert_eq!(result["structuredContent"]["effect_observation"], "none");
    }
    assert!(!root.path().join("target").exists());
    assert!(!root.path().join("Cargo.lock").exists());
}

#[test]
fn granted_submit_replay_and_damaged_result_keep_one_run_and_owner_truth() {
    let root = fixture();
    let route = bound(root.path());
    let first = call(
        context(root.path(), route.clone()),
        action("submit", "once"),
    );
    assert_eq!(first["isError"], false, "{first}");
    let reply = &first["structuredContent"]["reply"];
    assert_eq!(reply["kind"], "completed");
    assert!(reply["warning"].as_str().unwrap().contains("rustup"));
    let run = reply["run_id"].as_str().unwrap().to_owned();
    let result = root
        .path()
        .join(format!(".vibe/build-operations/{run}.cargo-result.json"));
    assert!(result.is_file());
    let replay = call(
        context(root.path(), route.clone()),
        action("submit", "once"),
    );
    assert_eq!(replay["structuredContent"], first["structuredContent"]);
    let index: Value = serde_json::from_slice(
        &fs::read(root.path().join(".vibe/machine-submissions.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(index["namespaces"].as_object().unwrap().len(), 1);
    fs::remove_file(result).unwrap();
    let uncertain = call(
        context(root.path(), route.clone()),
        action("inspect", "once"),
    );
    assert_eq!(uncertain["structuredContent"]["reply"]["kind"], "uncertain");
    assert_eq!(uncertain["structuredContent"]["reply"]["run_id"], run);
    let retry = call(context(root.path(), route), action("submit", "once"));
    assert_eq!(retry["structuredContent"], uncertain["structuredContent"]);
}

#[test]
fn after_effect_error_remains_executed_structured_with_known_run() {
    let error = json!({"schema":1,"code":"submission_refused",
        "message":"owner result unavailable","message_truncated":false,
        "requested_action":"submit","requested_scope":{
            "context":"host-context","target":"fixture-tool","principal":"host-principal"},
        "scope_unavailable_reason":null,"known_run_id":"run-1","view":null,
        "effect_observation":"unavailable"});
    let candidate = json!({"schema":1,"operation_id":OPERATION_ID,"domain_owner":DOMAIN_OWNER,
        "source_id":null,"evidence_id":null,"body":{"stage":"error",
        "format_id":FormatId::MachineFixedCargoActionError.id(),
        "payload_json":error.to_string(),"effect_state":"uncertain",
        "completeness":"unavailable","provenance_id":"run-1"}});
    let envelope: e::OperationEnvelope = serde_json::from_value(candidate).unwrap();
    let output = published(envelope).unwrap();
    assert!(output.is_error());
    assert!(output.has_structured());
    assert_eq!(output.structured()["known_run_id"], "run-1");
    assert_eq!(output.structured()["effect_observation"], "unavailable");
    assert_eq!(
        output.structured()["requested_scope"]["target"],
        "fixture-tool"
    );
}
