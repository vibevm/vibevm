use super::*;

#[test]
fn framed_combined_options_require_typed_approval_and_share_mcp_owner_result() {
    let fixture = Fixture::new();
    let host = fixture.host(true);
    let mut local = fixture.client(host.clone());
    let mut mcp = Server::new(MemoryTransport::with_input(""), fixture.context(host));
    let requested = json!({"kind":"prepare","phase":"validate","requested_options":{
        "force":true,"trace":true,"deploy_profile":null,"control":"none"}});
    let mut question = local.call(requested).question();
    assert!(question.requested_options.force);
    assert!(question.requested_options.trace);
    assert!(matches!(question.body, d::Body::SelectionQuestion(_)));

    let mut premature = question.clone();
    premature.body = d::Body::ApprovalAnswer(Box::new(d::BodyApprovalAnswer { approved: true }));
    assert_eq!(
        local.call(answer(&premature)).error().code,
        Code::DecisionRefused
    );
    fixture.no_provider();

    question.body = d::Body::SelectionAnswer(Box::new(d::BodySelectionAnswer {
        choice_ids: vec!["none".into()],
    }));
    let mut approval = local.call(answer(&question)).question();
    assert!(approval.requested_options.force);
    assert!(approval.requested_options.trace);
    assert_ne!(approval.plan_id, question.plan_id);
    let reference = approval.question_id.clone().unwrap();
    let mut changed = approval.clone();
    changed.requested_options.trace = false;
    changed.body = d::Body::ApprovalAnswer(Box::new(d::BodyApprovalAnswer { approved: true }));
    assert_eq!(
        local.call(answer(&changed)).error().code,
        Code::DecisionRefused
    );
    fixture.no_provider();

    approval.body = d::Body::ApprovalAnswer(Box::new(d::BodyApprovalAnswer { approved: true }));
    let Reply::Validated(validated) = local.call(answer(&approval)).reply().reply else {
        panic!("validated owner answer");
    };
    assert_eq!(validated.accepted_ref, reference);
    let submitted = local.call(submit(&reference, "combined-once"));
    assert!(!submitted.refused);
    until(|| fixture.dir.path().join("entered").exists());
    fixture.release();
    let mut terminal = None;
    until(|| {
        let observed = local.call(inspect("combined-once"));
        if matches!(observed.row().status, v::Status::ResultAvailable(_)) {
            terminal = Some(observed);
            true
        } else {
            false
        }
    });
    let terminal = terminal.unwrap();
    let run = terminal.row().run_id.unwrap();
    let admitted: Value = serde_json::from_slice(
        &fs::read(fixture.dir.path().join(format!(
            ".vibe/lifecycle-outcomes/{run}.admitted-input.json"
        )))
        .unwrap(),
    )
    .unwrap();
    for stage in ["request", "plan"] {
        assert_eq!(admitted[stage]["requested_options"]["force"], true);
        assert_eq!(admitted[stage]["requested_options"]["trace"], true);
    }
    let report: Value = serde_json::from_slice(
        &fs::read(
            fixture
                .dir
                .path()
                .join(format!(".vibe/lifecycle-outcomes/{run}.report.json")),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(report["trace"]["run_id"], run);
    assert_eq!(report["trace"]["status"], "ok");
    assert_eq!(report["trace"]["finalised"], true);

    let peer = mcp_call(&mut mcp, inspect("combined-once"));
    assert_eq!(peer["isError"], false, "{peer}");
    let peer_row: v::LifecycleSubmissionView = serde_json::from_str(
        peer["structuredContent"]["reply"]["payload_json"]
            .as_str()
            .unwrap(),
    )
    .unwrap();
    let local_row = terminal.row();
    assert_eq!(peer_row.run_id.as_deref(), Some(run.as_str()));
    assert_eq!(peer_row.status, local_row.status);
    assert_eq!(peer_row.work_outcomes, local_row.work_outcomes);
    assert_eq!(peer_row.requested_scope, local_row.requested_scope);
    assert_eq!(
        local
            .call(submit(&reference, "combined-once"))
            .row()
            .run_id
            .as_deref(),
        Some(run.as_str())
    );
    fixture.count();
    let lease = LifecycleLease::acquire(fixture.dir.path()).unwrap();
    let reservation = SubmissionReservations::inspect_existing_key(
        &lease,
        &fixture.ns,
        &fixture.scope,
        "combined-once",
    )
    .unwrap()
    .unwrap();
    assert_eq!(reservation.domain_run_id, run);
}
