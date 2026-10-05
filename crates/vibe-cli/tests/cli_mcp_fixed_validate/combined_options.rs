//! Process-level proof that fixed Validate retains the combined approved options.

use super::*;

#[test]
fn real_stdio_combined_force_trace_retains_both_options_across_cold_inspect() {
    let user = UserScratch::new();
    let root = project();
    let mut server = LiveServer::start(&user, root.path(), "issue");
    let mut selection = typed_question(&server.call(json!({
        "kind":"prepare", "phase":"validate",
        "requested_options":{"force":true,"trace":true,"deploy_profile":null,"control":"none"}
    })));
    assert!(selection.requested_options.force && selection.requested_options.trace);
    selection.body = d::Body::SelectionAnswer(Box::new(d::BodySelectionAnswer {
        choice_ids: vec!["none".into()],
    }));
    let mut approval = typed_question(&server.call(json!({
        "kind":"answer", "message_json":serde_json::to_string(&selection).unwrap()
    })));
    assert!(approval.requested_options.force && approval.requested_options.trace);
    let accepted_ref = approval.question_id.clone().unwrap();
    let mut changed = approval.clone();
    changed.requested_options.trace = false;
    changed.body = d::Body::ApprovalAnswer(Box::new(d::BodyApprovalAnswer { approved: true }));
    let refused = server.call(json!({
        "kind":"answer", "message_json":serde_json::to_string(&changed).unwrap()
    }));
    assert_eq!(refused["isError"], true, "{refused}");
    assert!(!root.path().join("counter").exists());

    approval.body = d::Body::ApprovalAnswer(Box::new(d::BodyApprovalAnswer { approved: true }));
    let accepted = server.call(json!({
        "kind":"answer", "message_json":serde_json::to_string(&approval).unwrap()
    }));
    assert_eq!(
        accepted["structuredContent"]["reply"]["accepted_ref"],
        accepted_ref
    );
    let key = "stdio-force-trace-once";
    let first = typed_view(&server.call(json!({
        "kind":"submit", "accepted_ref":accepted_ref, "key":key
    })));
    let mut run = first.run_id;
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        let view = typed_view(&server.call(json!({"kind":"inspect", "key":key})));
        if let Some(observed) = &view.run_id {
            if let Some(expected) = &run {
                assert_eq!(observed, expected);
            } else {
                run = Some(observed.clone());
            }
        }
        if matches!(view.status, v::Status::ResultAvailable(_)) {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "combined options terminal deadline"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    let run = run.expect("terminal result retains its run identity");
    server.finish();

    let admitted: Value = serde_json::from_slice(
        &fs::read(root.path().join(format!(
            ".vibe/lifecycle-outcomes/{run}.admitted-input.json"
        )))
        .unwrap(),
    )
    .unwrap();
    for stage in ["request", "plan"] {
        assert_eq!(admitted[stage]["requested_options"]["force"], true);
        assert_eq!(admitted[stage]["requested_options"]["trace"], true);
    }
    assert_eq!(admitted["plan"]["base_v4"]["force"], false);
    assert_eq!(admitted["plan"]["base_v4"]["effective_trace"], false);
    let report: Value = serde_json::from_slice(
        &fs::read(
            root.path()
                .join(format!(".vibe/lifecycle-outcomes/{run}.report.json")),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(report["trace"]["run_id"], run);
    assert_eq!(report["trace"]["status"], "ok");
    assert_eq!(report["trace"]["finalised"], true);

    let mut cold = LiveServer::start(&user, root.path(), "reattach");
    let view = typed_view(&cold.call(json!({"kind":"inspect", "key":key})));
    assert_eq!(view.run_id.as_deref(), Some(run.as_str()));
    assert!(matches!(view.status, v::Status::ResultAvailable(_)));
    cold.finish();
    assert_eq!(fs::read(root.path().join("counter")).unwrap(), b"effect\n");
}
