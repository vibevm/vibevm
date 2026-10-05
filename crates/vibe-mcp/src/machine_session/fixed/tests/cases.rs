use super::*;
#[test]
fn framed_selection_approval_and_registered_mcp_race_share_one_c1_run_and_effect() {
    let f = Fixture::new();
    let host = f.host(true);
    let mut local = f.client(host.clone());
    let mut mcp = Server::new(MemoryTransport::with_input(""), f.context(host));
    let local_ref = approve(&mut local, "none");
    let mcp_ref = mcp_approve(&mut mcp, "none");
    let changed = mcp_approve(&mut mcp, "detach");
    let barrier = Arc::new(std::sync::Barrier::new(3));
    let a = barrier.clone();
    let local_worker = std::thread::spawn(move || {
        a.wait();
        let response = local.call(submit(&local_ref, "once"));
        assert!(!response.refused);
        (local, local_ref, response)
    });
    let b = barrier.clone();
    let mcp_context = std::mem::replace(
        &mut mcp.context,
        ServerContext::with_store_root("unused", "unused"),
    );
    let mcp_worker = std::thread::spawn(move || {
        let mut mcp = Server::new(MemoryTransport::with_input(""), mcp_context);
        b.wait();
        let response = mcp_call(&mut mcp, submit(&mcp_ref, "once"));
        assert_eq!(response["isError"], false, "{response}");
        (mcp.context, response)
    });
    barrier.wait();
    let (mut local, reference, initial) = local_worker.join().unwrap();
    let (mcp_context, _) = mcp_worker.join().unwrap();
    let mut mcp = Server::new(MemoryTransport::with_input(""), mcp_context);
    assert_eq!(
        initial.outcome.completeness,
        f::OutcomeCompleteness::Partial
    );
    assert_eq!(
        initial.outcome.effect_state,
        f::OutcomeEffectState::Uncertain
    );
    until(|| f.dir.path().join("entered").exists());
    f.count();
    let run = local.call(inspect("once")).row().run_id.unwrap();
    let forged = local.call(submit("forged", "once"));
    assert_eq!(forged.error().code, Code::UnknownReference);
    assert_eq!(forged.error().known_run_id.as_deref(), Some(run.as_str()));
    assert_eq!(
        forged.outcome.effect_state,
        f::OutcomeEffectState::Uncertain
    );

    let mcp_row: v::LifecycleSubmissionView = serde_json::from_str(
        mcp_call(&mut mcp, inspect("once"))["structuredContent"]["reply"]["payload_json"]
            .as_str()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(mcp_row.run_id.as_deref(), Some(run.as_str()));
    assert_eq!(
        local
            .call(submit(&reference, "once"))
            .row()
            .run_id
            .as_deref(),
        Some(run.as_str())
    );
    let changed = mcp_call(&mut mcp, submit(&changed, "once"));
    assert_eq!(changed["isError"], true);
    assert_eq!(changed["structuredContent"]["code"], "submission_refused");
    f.release();
    until(|| {
        matches!(
            local.call(inspect("once")).row().status,
            v::Status::ResultAvailable(_)
        )
    });
    let terminal = local.call(inspect("once"));
    assert_eq!(
        terminal.outcome.effect_state,
        f::OutcomeEffectState::Uncertain
    );
    assert_eq!(
        terminal.outcome.completeness,
        f::OutcomeCompleteness::Complete
    );
    assert_eq!(
        terminal.outcome.provenance_id.as_deref(),
        Some(run.as_str())
    );
    let mcp_final: v::LifecycleSubmissionView = serde_json::from_str(
        mcp_call(&mut mcp, inspect("once"))["structuredContent"]["reply"]["payload_json"]
            .as_str()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(terminal.row(), mcp_final);
    f.count();
    let lease = LifecycleLease::acquire(f.dir.path()).unwrap();
    let c1 = SubmissionReservations::inspect_existing_key(&lease, &f.ns, &f.scope, "once")
        .unwrap()
        .unwrap();
    assert_eq!(c1.domain_run_id, run);
}
#[test]
fn ungranted_context_and_initialization_cannot_construct_callable_mode() {
    let dir = tempfile::tempdir().unwrap();
    assert!(matches!(
        FixedMachineSessionRunner::new(ServerContext::with_store_root(
            dir.path(),
            dir.path().join("unused")
        )),
        Err(FixedSessionBuildError::UnavailableHost)
    ));
    assert!(!dir.path().join(".vibe").exists());
    let ungranted = FixedLifecycleRoute::exchange_local(
        None,
        envelope(prepare("none")).to_string().as_bytes(),
        dir.path(),
    )
    .unwrap();
    let e::Body::Error(error) = ungranted.body else {
        panic!("refusal");
    };
    let error: LifecycleActionError =
        serde_json::from_str(error.payload_json.as_deref().unwrap()).unwrap();
    assert_eq!(error.code, Code::UnavailableHost);
    assert!(!dir.path().join(".vibe").exists());
    let f = Fixture::new();
    let host = f.host(true);
    let mut runner = FixedMachineSessionRunner::new(f.context(host)).unwrap();
    let mut incomplete = offer();
    let f::Body::InitializeOffer(body) = &mut incomplete.body else {
        unreachable!();
    };
    body.schema_offers
        .retain(|s| s.format_id != "machine-lifecycle-action-request");
    let f::Body::InitializationRefused(r) = send(&mut runner, &incomplete).body else {
        panic!("refusal");
    };
    assert_eq!(r.reason, f::BodyInitializationRefusedReason::NoCommonSchema);
    let mut client = Client::new(runner);
    assert!(matches!(
        client.call(inspect("absent")).reply().reply,
        Reply::Absent(_)
    ));
    f.no_provider();
}
#[test]
fn wrong_outer_ids_stage_identity_format_and_generated_shapes_refuse_without_provider() {
    let f = Fixture::new();
    let host = f.host(true);
    let mut c = f.client(host);
    for (context, target) in [
        ("foreign", Some(c.target.clone())),
        (c.context.as_str(), Some("foreign".into())),
        (c.context.as_str(), None),
    ] {
        let mut frame = f::SessionFrame {
            schema: 1,
            session_id: Some(c.session.clone()),
            request_id: Some(format!(
                "wrong-{}-{}",
                context,
                target.as_deref().unwrap_or("absent")
            )),
            body: f::Body::Request(Box::new(f::BodyRequest {
                context_id: context.into(),
                target_id: target,
                envelope_json: envelope(prepare("none")).to_string(),
            })),
        };
        let f::Body::Refusal(r) = send(&mut c.runner, &frame).body else {
            panic!("refusal");
        };
        assert!(r.envelope_json.is_none());
        assert_eq!(r.outcome.effect_state, f::OutcomeEffectState::NoEffects);
        frame.session_id = Some("foreign-session".into());
        assert!(matches!(
            send(&mut c.runner, &frame).body,
            f::Body::Refusal(_)
        ));
    }
    let mut controls = Vec::new();
    for (field, value) in [
        ("schema", json!(2)),
        ("operation_id", json!("foreign")),
        ("domain_owner", json!("foreign")),
        ("source_id", json!("caller-hash")),
        ("evidence_id", json!("grant")),
        ("root", json!("/tmp")),
    ] {
        let mut v = envelope(prepare("none"));
        v[field] = value;
        controls.push(v);
    }
    for format in [
        "migration-plan-request",
        "machine-lifecycle-action-reply",
        "unknown",
    ] {
        let mut v = envelope(prepare("none"));
        v["body"]["format_id"] = json!(format);
        controls.push(v);
    }
    let mut v = envelope(prepare("none"));
    v["body"] = json!({"stage":"admission","decision":"admitted","reason":null,"authority_evidence_id":"yes"});
    controls.push(v);
    for action in [
        json!({"kind":"attach","key":"once"}),
        json!({"kind":"control","observer_ref":"forged","message_json":"{}"}),
        json!({"kind":"replay","observer_ref":"forged","message_json":"{}"}),
        json!({"kind":"detach","observer_ref":"forged"}),
        json!({"kind":"inspect","key":false}),
        json!({"kind":"inspect","key":"once","grant":true}),
        json!({"kind":"inspect"}),
        json!({"kind":"unknown"}),
    ] {
        controls.push(envelope(action));
    }
    for payload in [
        json!({"schema":2,"action":{"kind":"inspect","key":"once"}}),
        json!({"schema":1,"root":"/tmp","action":{"kind":"inspect","key":"once"}}),
    ] {
        let mut v = envelope(inspect("once"));
        v["body"]["payload_json"] = json!(payload.to_string());
        controls.push(v);
    }
    for v in controls {
        let r = c.raw(v.to_string());
        assert_eq!(r.error().code, Code::InvalidRequest);
        assert_eq!(r.outcome.effect_state, f::OutcomeEffectState::NoEffects);
    }
    for raw in [
        "{}".to_owned(),
        "yes".into(),
        json!({"schema":1,"action":prepare("none")}).to_string(),
        "x".repeat(256 * 1024 + 1),
    ] {
        assert_eq!(c.raw(raw).error().code, Code::InvalidRequest);
    }
    for (phase, force, trace, profile) in [
        ("build", false, false, Value::Null),
        ("validate", false, false, json!("posix")),
    ] {
        let r=c.call(json!({"kind":"prepare","phase":phase,"requested_options":{"force":force,"trace":trace,"deploy_profile":profile,"control":"none"}}));
        assert_eq!(r.error().code, Code::ImplementationGap);
        assert_eq!(r.outcome.effect_state, f::OutcomeEffectState::NoEffects);
    }
    f.no_provider();
}
#[test]
fn owner_questions_foreign_principal_stale_policy_prose_and_assume_yes_never_grant() {
    let f = Fixture::new();
    let host = f.host(true);
    let mut c = f.client(host.clone());
    let mut q = c.call(prepare("none")).question();
    q.body = d::Body::SelectionAnswer(Box::new(d::BodySelectionAnswer {
        choice_ids: vec!["none".into()],
    }));
    q = c.call(answer(&q)).question();
    q.body = d::Body::ApprovalAnswer(Box::new(d::BodyApprovalAnswer { approved: true }));
    let mut foreign = q.clone();
    foreign.principal_id = Some("foreign".into());
    let refusal = c.call(answer(&foreign));
    assert_eq!(refusal.error().code, Code::DecisionRefused);
    assert_eq!(
        refusal.outcome.effect_state,
        f::OutcomeEffectState::Uncertain
    );
    for raw in [
        "yes".into(),
        "assume-yes".into(),
        json!({"approved":true}).to_string(),
    ] {
        assert_eq!(
            c.call(json!({"kind":"answer","message_json":raw}))
                .error()
                .code,
            Code::InvalidRequest
        );
    }
    let mut raw = serde_json::to_value(&q).unwrap();
    raw["assume_yes"] = json!(true);
    assert_eq!(
        c.call(json!({"kind":"answer","message_json":raw.to_string()}))
            .error()
            .code,
        Code::InvalidRequest
    );
    let mut wrong = q.clone();
    wrong.plan_id = Some("foreign-plan".into());
    assert_eq!(c.call(answer(&wrong)).error().code, Code::DecisionRefused);
    let mut wrong = q.clone();
    wrong.context_id = Some(c.context.clone());
    assert_eq!(c.call(answer(&wrong)).error().code, Code::DecisionRefused);
    let other = f.client(f.host(true));
    let mut c2 = other;
    assert_eq!(c2.call(answer(&q)).error().code, Code::DecisionRefused);
    let accepted = c.call(answer(&q));
    let Reply::Validated(r) = accepted.reply().reply else {
        panic!("validated");
    };
    host.set_host_policy(facts(false)).unwrap();
    let stale = c.call(submit(&r.accepted_ref, "once"));
    assert_eq!(stale.error().code, Code::SubmissionRefused);
    assert!(stale.error().message.contains("StalePolicy"));
    assert_eq!(stale.outcome.effect_state, f::OutcomeEffectState::Uncertain);
    f.no_provider();
}
