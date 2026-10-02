//! Baseline MCP characterization and the direct-library/MCP owner race.
use super::*;
// Characterization captured against the incumbent MCP actor before extraction.
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-OPERATION-SERVICE",
    r = 1
)]
#[test]
fn fixed_route_characterization_absent_and_unknown_reference() {
    let f = Fixture::new();
    let mut s = f.server(f.host(false));
    assert_eq!(
        inspect(&mut s, "unknown"),
        json!({
            "isError": false,
            "structuredContent": {"schema":1,"reply":{"kind":"absent","key":"unknown"}},
            "content":[{"type":"text","text":"Lifecycle inspect returned owner data; use lifecycle_operation inspect for a fresh submitted view."}]
        })
    );
    assert_eq!(
        submit(&mut s, "forged", "once"),
        json!({
            "isError":true,
            "structuredContent": {
                "schema":1,"code":"unknown_reference",
                "message":"accepted reference is not retained by this owner bridge",
                "message_truncated":false,"requested_action":"submit","requested_phase":"validate",
                "requested_scope":{"context":"mcp-native","target":"root","principal":"trusted-host"},
                "scope_unavailable_reason":null,"known_run_id":null,"view_json":null,
                "effect_observation":"unavailable"
            },
            "content":[{"type":"text","text":"structured preflight: Lifecycle action refused: accepted reference is not retained by this owner bridge (governed by spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-015#STRUCTURED-TOOL-PREFLIGHT; fix: inspect the typed report and pass arguments matching the tool's inputSchema)"}]
        })
    );
    f.no_provider();
}

#[test]
fn direct_library_route_and_registered_mcp_race_one_owner_and_effect() {
    use vibe_orchestrator::machine_operation::fixed_lifecycle::FixedLifecycleRoute;
    use vibe_wire::generated::machine::e1::lifecycle_action_reply::Reply;
    let f = Fixture::new();
    let h = f.host(true);
    let route = Arc::new(FixedLifecycleRoute::new(h.clone()));
    let direct_call = |args: Value| {
        FixedLifecycleRoute::dispatch(
            Some(route.as_ref()),
            args.to_string().as_bytes(),
            f.dir.path(),
        )
        .unwrap()
    };
    let request = json!({"schema":1,"action":{"kind":"prepare","phase":"validate","requested_options":{"force":false,"trace":false,"deploy_profile":null,"control":"detach"}}});
    let Reply::Question(q) = direct_call(request).reply else {
        panic!("question")
    };
    let mut q: d::InputDecisionMessage = serde_json::from_str(&q.payload_json).unwrap();
    q.body = d::Body::ApprovalAnswer(Box::new(d::BodyApprovalAnswer { approved: true }));
    let Reply::Validated(accepted) = direct_call(json!({"schema":1,"action":{"kind":"answer","message_json":serde_json::to_string(&q).unwrap()}})).reply else { panic!("validated") };
    let mut server = f.server(h);
    let mcp_ref = approve(&mut server, "detach");
    let barrier = Arc::new(std::sync::Barrier::new(3));
    let context = std::mem::replace(&mut server.context, ServerContext::new("unused"));
    let mcp_barrier = barrier.clone();
    let mcp = std::thread::spawn(move || {
        let mut server = Server::new(MemoryTransport::with_input(""), context);
        mcp_barrier.wait();
        assert_eq!(submit(&mut server, &mcp_ref, "once")["isError"], false);
        server.context
    });
    let library_barrier = barrier.clone();
    let library_route = route.clone();
    let root = f.dir.path().to_path_buf();
    let library = std::thread::spawn(move || {
        library_barrier.wait();
        FixedLifecycleRoute::dispatch(Some(library_route.as_ref()),json!({"schema":1,"action":{"kind":"submit","accepted_ref":accepted.accepted_ref,"key":"once"}}).to_string().as_bytes(),&root).unwrap()
    });
    barrier.wait();
    let _ = library.join().unwrap();
    let mut server = Server::new(MemoryTransport::with_input(""), mcp.join().unwrap());
    until(|| f.dir.path().join("entered").exists());
    let direct_view = direct_call(json!({"schema":1,"action":{"kind":"inspect","key":"once"}}));
    let direct_run = row(&json!({"structuredContent":serde_json::to_value(direct_view).unwrap()}))
        .run_id
        .unwrap();
    assert_eq!(
        row(&inspect(&mut server, "once")).run_id.as_deref(),
        Some(direct_run.as_str())
    );
    f.count();
    f.release();
    until(|| {
        matches!(
            row(&inspect(&mut server, "once")).status,
            v::Status::ResultAvailable(_)
        )
    });
    let direct_terminal = direct_call(json!({"schema":1,"action":{"kind":"inspect","key":"once"}}));
    assert_eq!(
        serde_json::to_value(direct_terminal).unwrap(),
        inspect(&mut server, "once")["structuredContent"]
    );
    f.count();
}
