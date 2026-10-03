use super::*;
use std::{
    fs,
    sync::{Arc, Barrier},
};
use vibe_lifecycle::{LifecycleLease, SubmissionReservations, SubmissionScope};
use vibe_orchestrator::machine_operation::fixed_package::FixedPackageHost;

fn fixture() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("source.bin"), b"copy once").unwrap();
    root
}
fn route(root: &std::path::Path) -> Arc<FixedPackageRoute> {
    let project = Arc::new(vibe_safefs::Project::open(root).unwrap());
    let scope = SubmissionScope {
        context: "fixed-package-mcp-test".into(),
        target: "launcher".into(),
        principal: "host-principal".into(),
    };
    let lease = LifecycleLease::acquire(root).unwrap();
    let namespace = SubmissionReservations::issue_namespace(&lease, &scope, 60_000).unwrap();
    drop(lease);
    Arc::new(FixedPackageRoute::new(Arc::new(
        FixedPackageHost::from_trusted_host(project, namespace, scope).unwrap(),
    )))
}
fn context(root: &std::path::Path, route: Arc<FixedPackageRoute>) -> ServerContext {
    ServerContext::with_store_root(root, root.join("unused"))
        .with_fixed_package_route(route)
        .unwrap()
}
fn action(kind: &str, key: &str) -> Value {
    json!({"schema":1,"action":{"kind":kind,"key":key}})
}
fn call(ctx: ServerContext, args: Value) -> Value {
    let request = json!({"jsonrpc":"2.0","id":41,"method":"tools/call",
        "params":{"name":"fixed_package_operation","arguments":args}});
    let response = crate::dispatch_one(ctx, &request.to_string()).unwrap();
    serde_json::from_str::<Value>(&response).unwrap()["result"].clone()
}

#[test]
fn registered_descriptor_without_grant_refuses_without_owner_state() {
    let root = tempfile::tempdir().unwrap();
    let descriptor = FixedPackageOperationMcpTool.descriptor();
    assert_eq!(descriptor.name, "fixed_package_operation");
    assert_eq!(descriptor.input_schema["additionalProperties"], false);
    for arm in descriptor.input_schema["properties"]["action"]["oneOf"]
        .as_array()
        .unwrap()
    {
        assert_eq!(arm["additionalProperties"], false);
    }
    let listed: Value = serde_json::from_str(
        &crate::dispatch_one(
            ServerContext::with_store_root(root.path(), root.path().join("unused")),
            &json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}).to_string(),
        )
        .unwrap(),
    )
    .unwrap();
    assert!(
        listed["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|tool| tool["name"] == "fixed_package_operation")
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
fn forged_arguments_and_foreign_root_refuse_before_copy() {
    let root = fixture();
    let granted = route(root.path());
    for args in [
        json!({"schema":2,"action":{"kind":"submit","key":"once"}}),
        json!({"schema":1,"action":{"kind":"submit","key":"once","namespace":"caller"}}),
        json!({"schema":1,"action":{"kind":"submit","key":"once","source":"other.bin"}}),
        json!({"schema":1,"action":{"kind":"submit","key":7}}),
        json!({"schema":1,"action":{"kind":"submit","key":"x".repeat(130_000)}}),
    ] {
        let result = call(context(root.path(), granted.clone()), args);
        assert_eq!(result["isError"], true, "{result}");
        assert_eq!(
            result["structuredContent"]["code"], "invalid_request",
            "{result}"
        );
        assert_eq!(result["structuredContent"]["effect_observation"], "none");
    }
    let foreign = tempfile::tempdir().unwrap();
    let result = call(
        ServerContext::with_store_root(foreign.path(), foreign.path().join("unused")),
        action("submit", "once"),
    );
    assert_eq!(result["isError"], true);
    assert!(
        !root
            .path()
            .join("target/vibe-package/launcher/launcher.bin")
            .exists()
    );
}

#[test]
fn granted_submit_replay_and_damaged_output_retain_one_run() {
    let root = fixture();
    let granted = route(root.path());
    let first = call(
        context(root.path(), granted.clone()),
        action("submit", "once"),
    );
    assert_eq!(first["isError"], false, "{first}");
    assert_eq!(first["structuredContent"]["reply"]["kind"], "completed");
    let run = first["structuredContent"]["reply"]["run_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let output = root
        .path()
        .join("target/vibe-package/launcher/launcher.bin");
    assert_eq!(fs::read(&output).unwrap(), b"copy once");
    let replay = call(
        context(root.path(), granted.clone()),
        action("submit", "once"),
    );
    assert_eq!(replay["structuredContent"], first["structuredContent"]);
    let index: Value = serde_json::from_slice(
        &fs::read(root.path().join(".vibe/machine-submissions.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(index["namespaces"].as_object().unwrap().len(), 1);
    fs::remove_file(&output).unwrap();
    let damaged = call(
        context(root.path(), granted.clone()),
        action("inspect", "once"),
    );
    assert_ne!(damaged["structuredContent"]["reply"]["kind"], "completed");
    assert_eq!(damaged["structuredContent"]["reply"]["run_id"], run);
    let retry = call(context(root.path(), granted), action("submit", "once"));
    assert_ne!(retry["structuredContent"]["reply"]["kind"], "completed");
    assert!(!output.exists());
}

#[test]
fn local_api_and_mcp_share_one_owner_run_for_the_same_key() {
    let root = fixture();
    let granted = route(root.path());
    let direct = FixedPackageRoute::dispatch(
        Some(&granted),
        action("submit", "shared").to_string().as_bytes(),
        root.path(),
    )
    .unwrap();
    let direct_run = match direct.reply {
        Reply::Completed(row) => row.run_id,
        other => panic!("expected retained completion: {other:?}"),
    };
    let mcp = call(context(root.path(), granted), action("submit", "shared"));
    assert_eq!(mcp["structuredContent"]["reply"]["run_id"], direct_run);
    assert_eq!(mcp["structuredContent"]["reply"]["kind"], "completed");
    assert_eq!(
        fs::read_dir(root.path().join(".vibe/package-operations"))
            .unwrap()
            .count(),
        3
    );
}

#[test]
fn concurrent_local_api_and_mcp_retry_attach_to_one_package_run() {
    let root = fixture();
    let granted = route(root.path());
    let start = Arc::new(Barrier::new(3));
    let direct_route = granted.clone();
    let direct_root = root.path().to_path_buf();
    let direct_start = start.clone();
    let direct = std::thread::spawn(move || {
        direct_start.wait();
        FixedPackageRoute::dispatch(
            Some(&direct_route),
            action("submit", "shared-race").to_string().as_bytes(),
            &direct_root,
        )
    });
    let mcp_route = granted.clone();
    let mcp_root = root.path().to_path_buf();
    let mcp_start = start.clone();
    let mcp = std::thread::spawn(move || {
        mcp_start.wait();
        call(
            context(&mcp_root, mcp_route),
            action("submit", "shared-race"),
        )
    });
    start.wait();
    let direct_initial = direct.join().unwrap();
    let mcp_initial = mcp.join().unwrap();
    assert!(
        direct_initial.is_ok() || mcp_initial["isError"] == false,
        "at least one contender must own the run"
    );
    let direct_retry = FixedPackageRoute::dispatch(
        Some(&granted),
        action("submit", "shared-race").to_string().as_bytes(),
        root.path(),
    )
    .unwrap();
    let run = match direct_retry.reply {
        Reply::Completed(row) => row.run_id,
        other => panic!("expected retained completion: {other:?}"),
    };
    let mcp_retry = call(
        context(root.path(), granted),
        action("submit", "shared-race"),
    );
    assert_eq!(mcp_retry["structuredContent"]["reply"]["run_id"], run);
    assert_eq!(mcp_retry["structuredContent"]["reply"]["kind"], "completed");
    assert_eq!(
        fs::read_dir(root.path().join(".vibe/package-operations"))
            .unwrap()
            .count(),
        3,
        "one intent, capsule and result after both contenders"
    );
}

#[test]
fn after_effect_error_is_structured_and_never_no_effects() {
    let error = json!({"schema":1,"code":"submission_refused",
        "message":"owner result unavailable","message_truncated":false,
        "requested_action":"submit","requested_scope":{
            "context":"host-context","target":"launcher","principal":"host-principal"},
        "scope_unavailable_reason":null,"known_run_id":"run-1","view":null,
        "effect_observation":"unavailable"});
    let candidate = json!({"schema":1,"operation_id":OPERATION_ID,"domain_owner":DOMAIN_OWNER,
        "source_id":null,"evidence_id":null,"body":{"stage":"error",
        "format_id":FormatId::MachineFixedPackageActionError.id(),
        "payload_json":error.to_string(),"effect_state":"uncertain",
        "completeness":"unavailable","provenance_id":"run-1"}});
    let envelope: e::OperationEnvelope = serde_json::from_value(candidate).unwrap();
    let output = published(envelope).unwrap();
    assert!(output.is_error());
    assert!(output.has_structured());
    assert_eq!(output.structured()["known_run_id"], "run-1");
    assert_eq!(output.structured()["effect_observation"], "unavailable");
}
