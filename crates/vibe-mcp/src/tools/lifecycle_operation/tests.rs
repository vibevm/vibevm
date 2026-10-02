use super::*;
use serde_json::json;
fn call(ctx: ServerContext, args: Value) -> Value {
    let request = json!({"jsonrpc":"2.0","id":41,"method":"tools/call","params":{"name":"lifecycle_operation","arguments":args}});
    serde_json::from_str::<Value>(&crate::dispatch_one(ctx, &request.to_string()).unwrap()).unwrap()
        ["result"]
        .clone()
}
#[test]
fn ungranted_constructor_registration_list_and_call_create_no_project_state() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("absent");
    for ctx in [
        ServerContext::new(&root),
        ServerContext::with_store_root(&root, dir.path().join("cache")),
    ] {
        assert!(ctx.fixed_lifecycle_bridge().is_none());
        assert_eq!(
            LifecycleOperationMcpTool.descriptor().name,
            "lifecycle_operation"
        );
        let list = json!({"jsonrpc":"2.0","id":1,"method":"tools/list"});
        let listed: Value = serde_json::from_str(
            &crate::dispatch_one(ServerContext::new(&root), &list.to_string()).unwrap(),
        )
        .unwrap();
        assert!(
            listed["result"]["tools"]
                .as_array()
                .unwrap()
                .iter()
                .any(|v| v["name"] == "lifecycle_operation")
        );
        let result = call(
            ctx,
            json!({"schema":1,"action":{"kind":"inspect","key":"once"}}),
        );
        assert_eq!(result["isError"], true);
        let e: LifecycleActionError =
            serde_json::from_value(result["structuredContent"].clone()).unwrap();
        assert_eq!(e.code, Code::UnavailableHost);
        assert!(e.requested_scope.is_none());
        assert!(e.view_json.is_none());
        assert_eq!(
            e.effect_observation,
            LifecycleActionErrorEffectObservation::Unavailable
        );
        assert!(!root.exists());
        assert!(!dir.path().join("cache").exists());
    }
}
#[test]
fn malformed_and_bounded_requests_remain_generated_preflight() {
    let dir = tempfile::tempdir().unwrap();
    for args in [
        json!({"schema":2,"action":{"kind":"inspect","key":"once"}}),
        json!({"schema":1,"action":{"kind":"submit","key":"once","accepted_ref":"forged","root":"/tmp"}}),
        json!({"schema":1,"action":{"kind":"answer","message_json":"x".repeat(128*1024)}}),
        json!({"schema":1,"action":{"kind":"attach","key":"once","namespace":"forged"}}),
        json!({"schema":1,"action":{"kind":"control","observer_ref":"forged","message_json":"{}","permission":true}}),
        json!({"schema":1,"action":{"kind":"replay","observer_ref":"forged","message_json":"x".repeat(128*1024)}}),
    ] {
        let result = call(ServerContext::new(dir.path()), args);
        assert_eq!(result["isError"], true);
        let e: LifecycleActionError =
            serde_json::from_value(result["structuredContent"].clone()).unwrap();
        assert_eq!(e.code, Code::InvalidRequest);
        assert_eq!(
            e.effect_observation,
            LifecycleActionErrorEffectObservation::Unavailable
        );
        assert!(!dir.path().join(".vibe").exists());
    }
}
#[test]
fn descriptor_is_authoritative_closed_projection() {
    let schema = descriptor::input_schema();
    assert_eq!(schema["additionalProperties"], false);
    let arms = schema["properties"]["action"]["oneOf"].as_array().unwrap();
    assert_eq!(arms.len(), 8);
    for arm in arms {
        assert_eq!(arm["additionalProperties"], false);
    }
    assert!(contract::size(&json!({"s":"\n".repeat(65536)}), REQUEST_CAP).is_err());
}

#[test]
fn bounded_rejected_phase_preserves_actual_utf8_request_without_silent_prefix() {
    let dir = tempfile::tempdir().unwrap();
    let actual_phase = "未".repeat(1500);
    let result = call(
        ServerContext::new(dir.path()),
        json!({
            "schema":1,"action":{"kind":"prepare","phase":actual_phase,
            "requested_options":{"force":false,"trace":false,"deploy_profile":null,"control":"none"}}
        }),
    );
    assert_eq!(result["isError"], true);
    let error: LifecycleActionError =
        serde_json::from_value(result["structuredContent"].clone()).unwrap();
    assert_eq!(
        error.requested_phase.as_deref(),
        Some(actual_phase.as_str())
    );
    assert_eq!(
        error.effect_observation,
        LifecycleActionErrorEffectObservation::Unavailable
    );
    assert!(!dir.path().join(".vibe").exists());
}
