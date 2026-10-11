use super::*;
use serde_json::{Value, json};
use std::{
    fs,
    io::Cursor,
    sync::{
        Barrier,
        atomic::{AtomicUsize, Ordering},
    },
    thread,
};
use vibe_core::{PackageRef, manifest::TargetOs};
use vibe_lifecycle::process::{PackageTreeWitness, capture_package_tree_witness};
use vibe_lifecycle::{LifecycleLease, SubmissionReservations, SubmissionScope};
use vibe_orchestrator::{
    machine_operation::input_decision as d,
    machine_submission::{
        HostPolicyFacts, TrustedPolicySource,
        exact_local_install_grant::ExactLocalInstallGrantService,
    },
    ports::{BoundOnePackageSource, PackageSource, PackageSourceBuild, PackageSourceFactory},
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

struct CountingLocalFactory {
    inner: PinnedLocalFactory,
    builds: Arc<AtomicUsize>,
}

impl BoundOnePackageSource for CountingLocalFactory {
    fn source_witness(&self) -> &PackageTreeWitness {
        self.inner.source_witness()
    }

    fn recheck_source_binding(&self, witness: &PackageTreeWitness) -> anyhow::Result<()> {
        self.inner.recheck_source_binding(witness)
    }
}

impl PackageSourceFactory for CountingLocalFactory {
    fn build(&self, input: PackageSourceBuild<'_>) -> anyhow::Result<Box<dyn PackageSource>> {
        self.builds.fetch_add(1, Ordering::SeqCst);
        self.inner.build(input)
    }
}

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
    Arc<AtomicUsize>,
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
    let builds = Arc::new(AtomicUsize::new(0));
    let source = Arc::new(CountingLocalFactory {
        inner: PinnedLocalFactory::new(&registry, &pin, &witness).unwrap(),
        builds: builds.clone(),
    });
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
    (dir, project, scope, route, namespace, builds)
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

fn reply_run_id(reply: &LifecycleActionReply) -> String {
    let payload = match &reply.reply {
        Reply::Submitted(row) => &row.payload_json,
        Reply::View(row) => &row.payload_json,
        other => panic!("expected retained submission view, got {other:?}"),
    };
    serde_json::from_str::<Value>(payload).unwrap()["run_id"]
        .as_str()
        .unwrap()
        .to_owned()
}

#[test]
fn mcp_answer_and_local_inspect_share_one_route_namespace_and_key() {
    assert!(vibe_test_support::isolated_home().is_some());
    let (_dir, project, scope, route, namespace, _builds) = fixture();
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
fn concurrent_api_mcp_and_frame_submit_invoke_one_exact_local_install() {
    assert!(vibe_test_support::isolated_home().is_some());
    let (_dir, project, scope, route, namespace, builds) = fixture();
    let prepare = mcp(
        &project,
        route.clone(),
        json!({"kind":"prepare","phase":"install","requested_options":{"force":false,"trace":false,"deploy_profile":null,"control":"none"}}),
    );
    assert_eq!(prepare["isError"], false, "{prepare}");
    let prepared: LifecycleActionReply =
        serde_json::from_value(prepare["structuredContent"].clone()).unwrap();
    let Reply::Question(question) = prepared.reply else {
        panic!("question");
    };
    let mut answer: d::InputDecisionMessage = serde_json::from_str(&question.payload_json).unwrap();
    let reference = answer.question_id.clone().unwrap();
    answer.body = d::Body::ApprovalAnswer(Box::new(d::BodyApprovalAnswer { approved: true }));
    let mut runner =
        ExactLocalInstallMachineSessionRunner::new(context(&project, route.clone())).unwrap();
    let ids = initialized(&mut runner);
    let (refused, envelope, _) = local(
        &mut runner,
        &ids,
        "race-answer",
        json!({"kind":"answer","message_json":serde_json::to_string(&answer).unwrap()}),
    );
    assert!(!refused, "{envelope:?}");
    assert!(matches!(action_reply(&envelope).reply, Reply::Validated(_)));
    assert_eq!(builds.load(Ordering::SeqCst), 0);

    let gate = Arc::new(Barrier::new(4));
    let (api, mcp_result, frame, mut runner) = thread::scope(|scope| {
        let api_gate = gate.clone();
        let api_route = route.clone();
        let api_ref = reference.clone();
        let api_project = &project;
        let api = scope.spawn(move || {
            api_gate.wait();
            ExactLocalInstallRoute::dispatch(
                Some(&api_route),
                json!({"schema":1,"action":{"kind":"submit","accepted_ref":api_ref,"key":"race"}})
                    .to_string()
                    .as_bytes(),
                api_project,
            )
        });
        let mcp_gate = gate.clone();
        let mcp_route = route.clone();
        let mcp_ref = reference.clone();
        let mcp_project = &project;
        let mcp_request = scope.spawn(move || {
            mcp_gate.wait();
            mcp(
                mcp_project,
                mcp_route,
                json!({"kind":"submit","accepted_ref":mcp_ref,"key":"race"}),
            )
        });
        let frame_gate = gate.clone();
        let frame_ref = reference.clone();
        let frame_ids = ids.clone();
        let framed = scope.spawn(move || {
            frame_gate.wait();
            let reply = local(
                &mut runner,
                &frame_ids,
                "race-submit",
                json!({"kind":"submit","accepted_ref":frame_ref,"key":"race"}),
            );
            (reply, runner)
        });
        gate.wait();
        let api = api.join().unwrap();
        let mcp_result = mcp_request.join().unwrap();
        let (frame, runner) = framed.join().unwrap();
        (api, mcp_result, frame, runner)
    });

    assert!(
        api.is_ok() || mcp_result["isError"] == false || !frame.0,
        "all competing submissions refused: api={api:?}, mcp={mcp_result}, frame={frame:?}"
    );
    let lease = LifecycleLease::acquire(&project).unwrap();
    let reserved = SubmissionReservations::inspect_existing_key(&lease, &namespace, &scope, "race")
        .unwrap()
        .unwrap();
    drop(lease);
    assert_eq!(
        builds.load(Ordering::SeqCst),
        1,
        "domain source built more than once"
    );
    assert!(project.join("vibe.lock").is_file());

    // A concurrent lease loser must attach to the retained run on retry.
    let api_replay = ExactLocalInstallRoute::dispatch(
        Some(&route),
        json!({"schema":1,"action":{"kind":"submit","accepted_ref":reference,"key":"race"}})
            .to_string()
            .as_bytes(),
        &project,
    )
    .unwrap();
    assert_eq!(reply_run_id(&api_replay), reserved.domain_run_id);
    let mcp_replay = mcp(
        &project,
        route.clone(),
        json!({"kind":"submit","accepted_ref":reference,"key":"race"}),
    );
    assert_eq!(mcp_replay["isError"], false, "{mcp_replay}");
    let mcp_replay: LifecycleActionReply =
        serde_json::from_value(mcp_replay["structuredContent"].clone()).unwrap();
    assert_eq!(reply_run_id(&mcp_replay), reserved.domain_run_id);
    let (refused, frame_replay, replay_outcome) = local(
        &mut runner,
        &ids,
        "race-replay",
        json!({"kind":"submit","accepted_ref":reference,"key":"race"}),
    );
    assert!(!refused, "{frame_replay:?}");
    assert_eq!(
        reply_run_id(&action_reply(&frame_replay)),
        reserved.domain_run_id
    );
    assert_eq!(
        replay_outcome.provenance_id.as_deref(),
        Some(reserved.domain_run_id.as_str())
    );

    let inspected = ExactLocalInstallRoute::dispatch(
        Some(&route),
        json!({"schema":1,"action":{"kind":"inspect","key":"race"}})
            .to_string()
            .as_bytes(),
        &project,
    )
    .unwrap();
    assert!(matches!(inspected.reply, Reply::View(_)));
    assert_eq!(reply_run_id(&inspected), reserved.domain_run_id);
    let mcp_inspected = mcp(
        &project,
        route.clone(),
        json!({"kind":"inspect","key":"race"}),
    );
    assert_eq!(mcp_inspected["isError"], false, "{mcp_inspected}");
    let mcp_reply: LifecycleActionReply =
        serde_json::from_value(mcp_inspected["structuredContent"].clone()).unwrap();
    assert!(matches!(mcp_reply.reply, Reply::View(_)));
    assert_eq!(reply_run_id(&mcp_reply), reserved.domain_run_id);
    let (refused, framed_inspected, outcome) = local(
        &mut runner,
        &ids,
        "race-inspect",
        json!({"kind":"inspect","key":"race"}),
    );
    assert!(!refused, "{framed_inspected:?}");
    assert!(matches!(
        action_reply(&framed_inspected).reply,
        Reply::View(_)
    ));
    assert_eq!(
        reply_run_id(&action_reply(&framed_inspected)),
        reserved.domain_run_id
    );
    assert_eq!(
        outcome.provenance_id.as_deref(),
        Some(reserved.domain_run_id.as_str())
    );

    let conflict = ExactLocalInstallRoute::dispatch(
        Some(&route),
        json!({"schema":1,"action":{"kind":"submit","accepted_ref":"foreign-reference","key":"race"}})
            .to_string()
            .as_bytes(),
        &project,
    )
    .unwrap_err();
    assert_eq!(
        conflict.error.code,
        LifecycleActionErrorCode::UnknownReference
    );
    assert_eq!(builds.load(Ordering::SeqCst), 1);
}

#[test]
fn missing_final_report_preserves_same_partial_install_truth_across_three_surfaces() {
    assert!(vibe_test_support::isolated_home().is_some());
    let (_dir, project, _scope, route, _namespace, builds) = fixture();
    let prepared = mcp(
        &project,
        route.clone(),
        json!({"kind":"prepare","phase":"install",
        "requested_options":{"force":false,"trace":false,"deploy_profile":null,"control":"none"}}),
    );
    let Reply::Question(question) =
        serde_json::from_value::<LifecycleActionReply>(prepared["structuredContent"].clone())
            .unwrap()
            .reply
    else {
        panic!("question")
    };
    let mut answer: d::InputDecisionMessage = serde_json::from_str(&question.payload_json).unwrap();
    let reference = answer.question_id.clone().unwrap();
    answer.body = d::Body::ApprovalAnswer(Box::new(d::BodyApprovalAnswer { approved: true }));
    let accepted = mcp(
        &project,
        route.clone(),
        json!({"kind":"answer",
        "message_json":serde_json::to_string(&answer).unwrap()}),
    );
    assert_eq!(accepted["isError"], false, "{accepted}");
    let submitted = mcp(
        &project,
        route.clone(),
        json!({"kind":"submit",
        "accepted_ref":reference,"key":"lost-report"}),
    );
    assert_eq!(submitted["isError"], false, "{submitted}");
    let reply: LifecycleActionReply =
        serde_json::from_value(submitted["structuredContent"].clone()).unwrap();
    let run = reply_run_id(&reply);
    assert_eq!(builds.load(Ordering::SeqCst), 1);
    let report = project.join(format!(".vibe/lifecycle-outcomes/{run}.report.json"));
    fs::remove_file(&report).unwrap();

    let action = json!({"kind":"inspect","key":"lost-report"});
    let input = json!({"schema":1,
        "operation_id":vibe_orchestrator::machine_operation::exact_local_install::OPERATION_ID,
        "domain_owner":vibe_orchestrator::machine_operation::exact_local_install::DOMAIN_OWNER,
        "source_id":null,"evidence_id":null,
        "body":{"stage":"input","format_id":FormatId::MachineLifecycleActionRequest.id(),
            "payload_json":json!({"schema":1,"action":action}).to_string()}});
    let api = ExactLocalInstallRoute::exchange_local(
        Some(&route),
        input.to_string().as_bytes(),
        &project,
    )
    .unwrap();
    let e::Body::Error(api_error) = api.body else {
        panic!("API error")
    };
    let owner_error: LifecycleActionError =
        serde_json::from_str(api_error.payload_json.as_deref().unwrap()).unwrap();
    assert_eq!(owner_error.requested_phase.as_deref(), Some("install"));
    assert_eq!(owner_error.known_run_id.as_deref(), Some(run.as_str()));
    let view: Value = serde_json::from_str(owner_error.view_json.as_deref().unwrap()).unwrap();
    assert_eq!(view["partial"], true);
    assert_eq!(view["run_id"], run);
    assert_eq!(api_error.effect_state, e::BodyErrorEffectState::Uncertain);
    assert_eq!(
        api_error.completeness,
        e::BodyErrorCompleteness::Unavailable
    );

    let mcp_result = mcp(&project, route.clone(), action.clone());
    assert_eq!(mcp_result["isError"], true, "{mcp_result}");
    assert_eq!(
        mcp_result["structuredContent"],
        serde_json::to_value(&owner_error).unwrap()
    );
    let mut runner = ExactLocalInstallMachineSessionRunner::new(context(&project, route)).unwrap();
    let ids = initialized(&mut runner);
    let (refused, frame, outcome) = local(&mut runner, &ids, "lost-report-inspect", action);
    assert!(refused);
    let e::Body::Error(frame_error) = frame.body else {
        panic!("frame error")
    };
    assert_eq!(frame_error.payload_json, api_error.payload_json);
    assert_eq!(outcome.effect_state, f::OutcomeEffectState::Uncertain);
    assert_eq!(outcome.completeness, f::OutcomeCompleteness::Unavailable);
    assert_eq!(outcome.provenance_id.as_deref(), Some(run.as_str()));
    assert_eq!(builds.load(Ordering::SeqCst), 1);
    assert!(!report.exists());
}

#[test]
fn unsupported_options_and_unknown_key_refuse_without_effects() {
    let (_dir, project, _scope, route, _namespace, _builds) = fixture();
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
