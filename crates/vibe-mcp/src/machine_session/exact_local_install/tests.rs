use super::*;
use serde_json::{Value, json};
use std::{fs, io::Cursor};
use vibe_core::{PackageRef, manifest::TargetOs};
use vibe_lifecycle::process::capture_package_tree_witness;
use vibe_lifecycle::{LifecycleLease, SubmissionReservations, SubmissionScope};
use vibe_orchestrator::{
    machine_operation::input_decision as d,
    machine_submission::{
        HostPolicyFacts, TrustedPolicySource,
        exact_local_install_grant::ExactLocalInstallGrantService,
    },
};
use vibe_package_source::PinnedLocalFactory;
use vibe_safefs::Project;
use vibe_wire::generated::{
    format_id::FormatId,
    machine::e1::{
        lifecycle_action_error::{LifecycleActionError, LifecycleActionErrorCode},
        lifecycle_action_reply::{LifecycleActionReply, Reply},
    },
};

#[test]
fn ungranted_context_refuses_without_creating_root() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("absent");
    assert!(matches!(
        ExactLocalInstallMachineSessionRunner::new(ServerContext::with_store_root(
            &root,
            dir.path().join("cache")
        )),
        Err(ExactLocalInstallSessionBuildError::UnavailableHost)
    ));
    assert!(!root.exists());
}

fn fixture() -> (
    tempfile::TempDir,
    std::path::PathBuf,
    SubmissionScope,
    Arc<ExactLocalInstallRoute>,
    vibe_lifecycle::SubmissionNamespace,
) {
    let dir = tempfile::tempdir().unwrap();
    let project = dir.path().join("project");
    let registry = dir.path().join("registry");
    let package = registry.join("org.example/fixture/v1.1.0");
    fs::create_dir_all(&project).unwrap();
    fs::create_dir_all(&package).unwrap();
    fs::write(project.join("vibe.toml"), "[project]\nname = 'consumer'\nversion = '1.1.0'\n[requires.packages]\n'flow:org.example/fixture' = '=1.1.0'\n").unwrap();
    fs::write(
        package.join("vibe.toml"),
        "[package]\ngroup = 'org.example'\nname = 'fixture'\nkind = 'flow'\nversion = '1.1.0'\n",
    )
    .unwrap();
    fs::write(package.join("README.md"), b"# Fixture\n").unwrap();
    let pin = PackageRef::parse("flow:org.example/fixture@=1.1.0").unwrap();
    let witness =
        capture_package_tree_witness(&package, &pin, &["README.md", "vibe.toml"]).unwrap();
    let source = Arc::new(PinnedLocalFactory::new(&registry, &pin, &witness).unwrap());
    let scope = SubmissionScope {
        context: "mcp-exact-install".into(),
        target: "project".into(),
        principal: "trusted-host".into(),
    };
    let service = ExactLocalInstallGrantService::from_trusted_host(
        Arc::new(Project::open(&project).unwrap()),
        scope.clone(),
        TargetOs::Linux,
        TrustedPolicySource::from_host_identifier("fixture-install-policy".into()).unwrap(),
        HostPolicyFacts {
            approve_captured_script: false,
            approve_exact_local_install: true,
            allow_detach: false,
            allow_cancel_pre_dispatch: false,
            allow_cancel_deferred_boundary: false,
        },
        source,
    )
    .unwrap();
    let grant = service.issue(60_000).unwrap();
    let (host, namespace) = grant.into_parts();
    let route = Arc::new(ExactLocalInstallRoute::new(host, namespace.clone()));
    (dir, project, scope, route, namespace)
}
fn context(project: &std::path::Path, route: Arc<ExactLocalInstallRoute>) -> ServerContext {
    ServerContext::with_store_root(project, project.join("unused-store"))
        .with_exact_local_install_route(route)
        .unwrap()
}
fn mcp(project: &std::path::Path, route: Arc<ExactLocalInstallRoute>, action: Value) -> Value {
    let request = json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"lifecycle_operation","arguments":{"schema":1,"operation_id":vibe_orchestrator::machine_operation::exact_local_install::OPERATION_ID,"action":action}}});
    let raw = crate::dispatch_one(context(project, route), &request.to_string()).unwrap();
    serde_json::from_str::<Value>(&raw).unwrap()["result"].clone()
}
fn send(
    runner: &mut ExactLocalInstallMachineSessionRunner,
    frame: f::SessionFrame,
) -> f::SessionFrame {
    let mut bytes = Vec::new();
    write_frame(&mut bytes, &frame).unwrap();
    let mut output = Vec::new();
    assert!(matches!(
        runner.step(&mut Cursor::new(bytes), &mut output).unwrap(),
        RunnerStep::Replied
    ));
    read_frame(&mut Cursor::new(output)).unwrap().unwrap()
}
fn initialized(runner: &mut ExactLocalInstallMachineSessionRunner) -> (String, String, String) {
    let schemas = [
        FormatId::MachineSessionFrame,
        FormatId::MachineOperationEnvelope,
        FormatId::MachineLifecycleActionRequest,
        FormatId::MachineLifecycleActionReply,
        FormatId::MachineLifecycleActionError,
        FormatId::MachineInputDecisionMessage,
        FormatId::MachineLifecycleSubmissionView,
    ]
    .into_iter()
    .map(|id| json!({"format_id":id.id(),"epoch":id.epoch()}))
    .collect::<Vec<_>>();
    let offer: f::SessionFrame = serde_json::from_value(json!({"schema":1,"session_id":null,"request_id":"init","body":{"kind":"initialize_offer","api_versions":["1"],"position_encodings":["raw-byte-offsets"],"schema_offers":schemas,"requested_capabilities":{"streaming":true,"replay":true,"control":true}}})).unwrap();
    let answer = send(runner, offer);
    let f::Body::Initialized(body) = answer.body else {
        panic!("initialized");
    };
    (
        answer.session_id.unwrap(),
        body.contexts[0].context_id.clone(),
        body.contexts[0].target_ids[0].clone(),
    )
}
fn local(
    runner: &mut ExactLocalInstallMachineSessionRunner,
    ids: &(String, String, String),
    request_id: &str,
    action: Value,
) -> (bool, e::OperationEnvelope, f::Outcome) {
    let envelope = json!({"schema":1,"operation_id":vibe_orchestrator::machine_operation::exact_local_install::OPERATION_ID,
        "domain_owner":vibe_orchestrator::machine_operation::exact_local_install::DOMAIN_OWNER,
        "source_id":null,"evidence_id":null,"body":{"stage":"input","format_id":FormatId::MachineLifecycleActionRequest.id(),"payload_json":json!({"schema":1,"action":action}).to_string()}});
    let frame = f::SessionFrame {
        schema: 1,
        session_id: Some(ids.0.clone()),
        request_id: Some(request_id.into()),
        body: f::Body::Request(Box::new(f::BodyRequest {
            context_id: ids.1.clone(),
            target_id: Some(ids.2.clone()),
            envelope_json: envelope.to_string(),
        })),
    };
    let response = send(runner, frame);
    let (refused, json, outcome) = match response.body {
        f::Body::Complete(body) => (false, body.envelope_json.unwrap(), body.outcome),
        f::Body::Refusal(body) => (true, body.envelope_json.unwrap(), body.outcome),
        _ => panic!("published action"),
    };
    (refused, serde_json::from_str(&json).unwrap(), outcome)
}
fn action_reply(envelope: &e::OperationEnvelope) -> LifecycleActionReply {
    let e::Body::Result(result) = &envelope.body else {
        panic!("result");
    };
    serde_json::from_str(result.payload_json.as_deref().unwrap()).unwrap()
}

#[test]
fn mcp_answer_and_local_inspect_share_one_route_namespace_and_key() {
    assert!(vibe_test_support::isolated_home().is_some());
    let (_dir, project, scope, route, namespace) = fixture();
    let prepare = mcp(
        &project,
        route.clone(),
        json!({"kind":"prepare","phase":"install","requested_options":{"force":false,"trace":false,"deploy_profile":null,"control":"none"}}),
    );
    assert_eq!(prepare["isError"], false, "{prepare}");
    let reply: LifecycleActionReply =
        serde_json::from_value(prepare["structuredContent"].clone()).unwrap();
    let Reply::Question(question) = reply.reply else {
        panic!("question");
    };
    let mut question: d::InputDecisionMessage =
        serde_json::from_str(&question.payload_json).unwrap();
    let reference = question.question_id.clone().unwrap();
    question.body = d::Body::ApprovalAnswer(Box::new(d::BodyApprovalAnswer { approved: true }));
    let mut runner =
        ExactLocalInstallMachineSessionRunner::new(context(&project, route.clone())).unwrap();
    let ids = initialized(&mut runner);
    let (refused, answer, outcome) = local(
        &mut runner,
        &ids,
        "answer",
        json!({"kind":"answer","message_json":serde_json::to_string(&question).unwrap()}),
    );
    assert!(!refused, "{answer:?}");
    assert_eq!(outcome.effect_state, f::OutcomeEffectState::Uncertain);
    assert_eq!(outcome.completeness, f::OutcomeCompleteness::Partial);
    let Reply::Validated(validated) = action_reply(&answer).reply else {
        panic!("validated");
    };
    assert_eq!(validated.accepted_ref, reference);
    let submitted = mcp(
        &project,
        route.clone(),
        json!({"kind":"submit","accepted_ref":reference,"key":"once"}),
    );
    assert_eq!(submitted["isError"], false, "{submitted}");
    let lease = LifecycleLease::acquire(&project).unwrap();
    let reserved = SubmissionReservations::inspect_existing_key(&lease, &namespace, &scope, "once")
        .unwrap()
        .unwrap();
    drop(lease);
    let (refused, inspected, outcome) = local(
        &mut runner,
        &ids,
        "inspect",
        json!({"kind":"inspect","key":"once"}),
    );
    assert!(!refused, "{inspected:?}");
    assert!(matches!(action_reply(&inspected).reply, Reply::View(_)));
    assert_eq!(outcome.effect_state, f::OutcomeEffectState::Uncertain);
    assert_eq!(outcome.completeness, f::OutcomeCompleteness::Complete);
    assert_eq!(
        outcome.provenance_id.as_deref(),
        Some(reserved.domain_run_id.as_str())
    );
    assert!(project.join("vibe.lock").exists());
}

#[test]
fn unsupported_options_and_unknown_key_refuse_without_effects() {
    let (_dir, project, _scope, route, _namespace) = fixture();
    let result = mcp(
        &project,
        route.clone(),
        json!({"kind":"prepare","phase":"install","requested_options":{"force":true,"trace":false,"deploy_profile":null,"control":"none"}}),
    );
    assert_eq!(result["isError"], true);
    let error: LifecycleActionError =
        serde_json::from_value(result["structuredContent"].clone()).unwrap();
    assert_eq!(error.code, LifecycleActionErrorCode::DecisionRefused);
    let mut runner = ExactLocalInstallMachineSessionRunner::new(context(&project, route)).unwrap();
    let ids = initialized(&mut runner);
    let (refused, envelope, outcome) = local(
        &mut runner,
        &ids,
        "unknown",
        json!({"kind":"submit","accepted_ref":"unknown","key":"once"}),
    );
    assert!(refused);
    let e::Body::Error(body) = envelope.body else {
        panic!("error");
    };
    let error: LifecycleActionError =
        serde_json::from_str(body.payload_json.as_deref().unwrap()).unwrap();
    assert_eq!(error.code, LifecycleActionErrorCode::UnknownReference);
    assert_eq!(outcome.effect_state, f::OutcomeEffectState::Uncertain);
    assert!(!project.join("vibe.lock").exists());
}
