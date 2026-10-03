//! Actual CLI frames and MCP tools/call sharing one retained C1 host.
use super::*;
use std::sync::Arc;
use vibe_core::manifest::TargetOs;
use vibe_mcp::{ServerContext, dispatch_one};
use vibe_orchestrator::{
    machine_operation::fixed_lifecycle::FixedLifecycleRoute,
    machine_submission::{HostPolicyFacts, TrustedPolicySource, cli_grant::CliGrantService},
};
use vibe_safefs::Project;

fn route_for_cli_root(root: &Path) -> Arc<FixedLifecycleRoute> {
    let project = Arc::new(Project::open(root).unwrap());
    let target = format!(
        "physical-root:{}",
        project.root_correlation_token().unwrap()
    );
    let service = CliGrantService::from_trusted_host(
        project,
        target,
        "vibe-cli-project-local/e1".into(),
        TargetOs::Linux,
        TrustedPolicySource::from_host_identifier("vibe-cli/fixed-validate/e1".into()).unwrap(),
        HostPolicyFacts {
            approve_captured_script: true,
            allow_detach: true,
            allow_cancel_pre_dispatch: true,
            allow_cancel_deferred_boundary: true,
        },
    )
    .unwrap();
    Arc::new(FixedLifecycleRoute::new(Arc::new(
        service.reattach().unwrap(),
    )))
}

fn mcp_call(route: &Arc<FixedLifecycleRoute>, root: &Path, action: Value) -> Value {
    let context = ServerContext::with_store_root(root, root.join("unused-cache"))
        .with_fixed_lifecycle_route(route.clone())
        .unwrap();
    let request = json!({
        "jsonrpc":"2.0", "id":1, "method":"tools/call",
        "params":{"name":"lifecycle_operation", "arguments":{"schema":1,"action":action}}
    });
    let response: Value =
        serde_json::from_str(&dispatch_one(context, &request.to_string()).unwrap()).unwrap();
    assert!(response["error"].is_null(), "{response}");
    response["result"].clone()
}

fn mcp_question(result: &Value) -> d::InputDecisionMessage {
    serde_json::from_str(
        result["structuredContent"]["reply"]["payload_json"]
            .as_str()
            .unwrap(),
    )
    .unwrap()
}

fn mcp_approve(route: &Arc<FixedLifecycleRoute>, root: &Path, control: &str) -> String {
    let mut question = mcp_question(&mcp_call(route, root, prepare(control)));
    if matches!(question.body, d::Body::SelectionQuestion(_)) {
        question.body = d::Body::SelectionAnswer(Box::new(d::BodySelectionAnswer {
            choice_ids: vec!["none".into()],
        }));
        question = mcp_question(&mcp_call(route, root, answer(&question)));
    }
    assert!(matches!(question.body, d::Body::ApprovalQuestion(_)));
    question.body = d::Body::ApprovalAnswer(Box::new(d::BodyApprovalAnswer { approved: true }));
    let accepted = mcp_call(route, root, answer(&question));
    assert_eq!(accepted["isError"], false, "{accepted}");
    accepted["structuredContent"]["reply"]["accepted_ref"]
        .as_str()
        .unwrap()
        .into()
}

fn mcp_row(result: &Value) -> v::LifecycleSubmissionView {
    serde_json::from_str(
        result["structuredContent"]["reply"]["payload_json"]
            .as_str()
            .unwrap_or_else(|| panic!("MCP returned no view: {result}")),
    )
    .unwrap()
}

#[test]
fn cli_and_mcp_racing_one_scoped_key_share_the_c1_run_and_terminal_truth() {
    let fixture = Fixture::new(true);
    let root = fixture.root();
    let mut cli = fixture.client("issue");
    let cli_reference = approve(&mut cli);
    let route = route_for_cli_root(&root);
    let mcp_reference = mcp_approve(&route, &root, "none");
    let cli_result = cli.call(submit(&cli_reference, "shared"));
    assert!(matches!(cli_result.envelope.body, e::Body::Result(_)));
    until(|| root.join("entered").exists());
    // The CLI has already reserved C1 and its provider is still active when
    // MCP submits the same key from another host instance.
    let mcp_result = mcp_call(&route, &root, submit(&mcp_reference, "shared"));
    assert!(
        mcp_result["isError"] == false
            || mcp_result["structuredContent"]["code"] == "inspection_unavailable",
        "{mcp_result}"
    );
    fixture.count_one();
    let cli_active = cli.call(inspect("shared")).row();
    let mcp_active = mcp_call(&route, &root, inspect("shared"));
    if mcp_active["isError"] == false {
        assert_eq!(cli_active.run_id, mcp_row(&mcp_active).run_id);
    } else {
        // The incumbent script may still hold the lifecycle lease. Contention
        // is a truthful temporary refusal, never a claim of absent effects.
        assert_eq!(
            mcp_active["structuredContent"]["code"],
            "inspection_unavailable"
        );
        assert_eq!(
            mcp_active["structuredContent"]["effect_observation"],
            "unavailable"
        );
    }
    fixture.release();
    let cli_terminal = terminal(&mut cli, "shared");
    let cli_terminal_row = cli_terminal.row();
    cli.finish();
    let deadline = Instant::now() + Duration::from_secs(3);
    let mcp_terminal = loop {
        let result = mcp_call(&route, &root, inspect("shared"));
        if result["isError"] == true {
            assert_eq!(
                result["structuredContent"]["code"],
                "inspection_unavailable"
            );
            if Instant::now() >= deadline {
                panic!("MCP cannot inspect completed CLI run: {result}");
            }
        } else {
            let row = mcp_row(&result);
            if matches!(row.status, v::Status::ResultAvailable(_)) {
                break row;
            }
            assert!(
                Instant::now() < deadline,
                "MCP never reached terminal: {result}"
            );
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    assert_same_terminal_truth(&cli_terminal_row, &mcp_terminal);
    let run = cli_terminal_row.run_id.unwrap();
    let changed_reference = mcp_approve(&route, &root, "detach");
    let changed = mcp_call(&route, &root, submit(&changed_reference, "shared"));
    assert_eq!(changed["isError"], true, "{changed}");
    assert_eq!(changed["structuredContent"]["code"], "submission_refused");
    fixture.count_one();
    let index = fixture.index();
    let namespaces = index["namespaces"].as_object().unwrap();
    assert_eq!(namespaces.len(), 1);
    let keys = namespaces.values().next().unwrap()["keys"]
        .as_object()
        .unwrap();
    assert_eq!(keys.len(), 1);
    assert_eq!(keys["shared"]["domain_run_id"], run);
}
