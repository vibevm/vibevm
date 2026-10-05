use super::*;

fn options(trace: bool) -> Value {
    json!({"kind":"prepare","phase":"validate","requested_options":{
        "force":true,"trace":trace,"deploy_profile":null,"control":"none"}})
}

fn approve_local(client: &mut Client) -> String {
    let mut question = client.call(options(true)).question();
    assert!(question.requested_options.force && question.requested_options.trace);
    question.body = d::Body::SelectionAnswer(Box::new(d::BodySelectionAnswer {
        choice_ids: vec!["none".into()],
    }));
    let mut approval = client.call(answer(&question)).question();
    let reference = approval.question_id.clone().unwrap();
    approval.body = d::Body::ApprovalAnswer(Box::new(d::BodyApprovalAnswer { approved: true }));
    assert!(matches!(
        client.call(answer(&approval)).reply().reply,
        Reply::Validated(_)
    ));
    reference
}

fn approve_mcp(server: &mut Server<MemoryTransport>, trace: bool) -> String {
    let response = mcp_call(server, options(trace));
    assert_eq!(response["isError"], false, "{response}");
    let mut question: d::InputDecisionMessage = serde_json::from_str(
        response["structuredContent"]["reply"]["payload_json"]
            .as_str()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(question.requested_options.trace, trace);
    question.body = d::Body::SelectionAnswer(Box::new(d::BodySelectionAnswer {
        choice_ids: vec!["none".into()],
    }));
    let response = mcp_call(server, answer(&question));
    assert_eq!(response["isError"], false, "{response}");
    let mut approval: d::InputDecisionMessage = serde_json::from_str(
        response["structuredContent"]["reply"]["payload_json"]
            .as_str()
            .unwrap(),
    )
    .unwrap();
    approval.body = d::Body::ApprovalAnswer(Box::new(d::BodyApprovalAnswer { approved: true }));
    let response = mcp_call(server, answer(&approval));
    assert_eq!(response["isError"], false, "{response}");
    response["structuredContent"]["reply"]["accepted_ref"]
        .as_str()
        .unwrap()
        .into()
}

#[test]
fn framed_and_mcp_combined_option_race_share_c1_and_refuse_changed_options() {
    let fixture = Fixture::new();
    let host = fixture.host(true);
    let mut local = fixture.client(host.clone());
    let mut mcp = Server::new(MemoryTransport::with_input(""), fixture.context(host));
    let local_ref = approve_local(&mut local);
    let mcp_ref = approve_mcp(&mut mcp, true);
    let changed_ref = approve_mcp(&mut mcp, false);
    fixture.no_provider();

    let barrier = Arc::new(std::sync::Barrier::new(3));
    let local_barrier = barrier.clone();
    let local_worker = std::thread::spawn(move || {
        local_barrier.wait();
        let response = local.call(submit(&local_ref, "options-race"));
        assert!(!response.refused);
        (local, response)
    });
    let mcp_context = std::mem::replace(
        &mut mcp.context,
        ServerContext::with_store_root("unused", "unused"),
    );
    let mcp_barrier = barrier.clone();
    let mcp_worker = std::thread::spawn(move || {
        let mut mcp = Server::new(MemoryTransport::with_input(""), mcp_context);
        mcp_barrier.wait();
        let response = mcp_call(&mut mcp, submit(&mcp_ref, "options-race"));
        assert_eq!(response["isError"], false, "{response}");
        (mcp.context, response)
    });
    barrier.wait();
    let (mut local, local_submit) = local_worker.join().unwrap();
    let (mcp_context, mcp_submit) = mcp_worker.join().unwrap();
    let mut mcp = Server::new(MemoryTransport::with_input(""), mcp_context);
    assert_eq!(
        local_submit.outcome.completeness,
        f::OutcomeCompleteness::Partial
    );
    until(|| fixture.dir.path().join("entered").exists());
    fixture.count();

    let local_run = local.call(inspect("options-race")).row().run_id.unwrap();
    assert_eq!(mcp_submit["isError"], false);
    let mcp_row: v::LifecycleSubmissionView = serde_json::from_str(
        mcp_call(&mut mcp, inspect("options-race"))["structuredContent"]["reply"]["payload_json"]
            .as_str()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(mcp_row.run_id.as_deref(), Some(local_run.as_str()));
    let changed = mcp_call(&mut mcp, submit(&changed_ref, "options-race"));
    assert_eq!(changed["isError"], true, "{changed}");
    assert_eq!(changed["structuredContent"]["code"], "submission_refused");
    fixture.count();

    fixture.release();
    until(|| {
        matches!(
            local.call(inspect("options-race")).row().status,
            v::Status::ResultAvailable(_)
        )
    });
    let terminal = local.call(inspect("options-race")).row();
    assert_eq!(terminal.run_id.as_deref(), Some(local_run.as_str()));
    let peer: v::LifecycleSubmissionView = serde_json::from_str(
        mcp_call(&mut mcp, inspect("options-race"))["structuredContent"]["reply"]["payload_json"]
            .as_str()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(peer.run_id, terminal.run_id);
    assert_eq!(peer.status, terminal.status);
    assert_eq!(peer.work_outcomes, terminal.work_outcomes);
    let report: Value = serde_json::from_slice(
        &fs::read(
            fixture
                .dir
                .path()
                .join(format!(".vibe/lifecycle-outcomes/{local_run}.report.json")),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(report["trace"]["run_id"], local_run);
    assert_eq!(report["trace"]["finalised"], true);
    fixture.count();
    let lease = LifecycleLease::acquire(fixture.dir.path()).unwrap();
    let reservation = SubmissionReservations::inspect_existing_key(
        &lease,
        &fixture.ns,
        &fixture.scope,
        "options-race",
    )
    .unwrap()
    .unwrap();
    assert_eq!(reservation.domain_run_id, local_run);
}
