//! Cold MCP truth when a retained final report is absent after an effect.

use super::*;

#[test]
fn real_stdio_missing_report_retains_run_and_unavailable_effect_without_redispatch() {
    let user = UserScratch::new();
    let root = project();
    let mut server = LiveServer::start(&user, root.path(), "issue");
    let mut selection = typed_question(&server.call(json!({
        "kind":"prepare", "phase":"validate",
        "requested_options":{"force":false,"trace":false,"deploy_profile":null,"control":"none"}
    })));
    selection.body = d::Body::SelectionAnswer(Box::new(d::BodySelectionAnswer {
        choice_ids: vec!["none".into()],
    }));
    let mut approval = typed_question(&server.call(json!({
        "kind":"answer", "message_json":serde_json::to_string(&selection).unwrap()
    })));
    let accepted_ref = approval.question_id.clone().unwrap();
    approval.body = d::Body::ApprovalAnswer(Box::new(d::BodyApprovalAnswer { approved: true }));
    let accepted = server.call(json!({
        "kind":"answer", "message_json":serde_json::to_string(&approval).unwrap()
    }));
    assert_eq!(accepted["isError"], false, "{accepted}");
    let key = "missing-final-report";
    let started = typed_view(&server.call(json!({
        "kind":"submit", "accepted_ref":accepted_ref, "key":key
    })));
    let mut run = started.run_id;
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
        assert!(Instant::now() < deadline, "Validate terminal deadline");
        std::thread::sleep(Duration::from_millis(10));
    }
    let run = run.expect("terminal run identity");
    server.finish();
    assert_eq!(fs::read(root.path().join("counter")).unwrap(), b"effect\n");

    fs::remove_file(
        root.path()
            .join(format!(".vibe/lifecycle-outcomes/{run}.report.json")),
    )
    .unwrap();
    let mut cold = LiveServer::start(&user, root.path(), "reattach");
    let unavailable = cold.call(json!({"kind":"inspect", "key":key}));
    assert_eq!(unavailable["isError"], true, "{unavailable}");
    let error = &unavailable["structuredContent"];
    assert_eq!(error["code"], "inspection_unavailable", "{unavailable}");
    assert_eq!(error["known_run_id"], run, "{unavailable}");
    assert_eq!(error["requested_phase"], "validate", "{unavailable}");
    assert_eq!(error["effect_observation"], "unavailable", "{unavailable}");
    assert!(error["requested_scope"].is_object(), "{unavailable}");
    let view: v::LifecycleSubmissionView =
        serde_json::from_str(error["view_json"].as_str().unwrap()).unwrap();
    assert_eq!(view.run_id.as_deref(), Some(run.as_str()));
    assert_eq!(view.requested_phase, "validate");
    assert!(view.partial);
    assert!(matches!(view.status, v::Status::RecoveryRequired(_)));
    let mut selection = typed_question(&cold.call(json!({
        "kind":"prepare", "phase":"validate",
        "requested_options":{"force":false,"trace":false,"deploy_profile":null,"control":"none"}
    })));
    selection.body = d::Body::SelectionAnswer(Box::new(d::BodySelectionAnswer {
        choice_ids: vec!["none".into()],
    }));
    let mut approval = typed_question(&cold.call(json!({
        "kind":"answer", "message_json":serde_json::to_string(&selection).unwrap()
    })));
    let replay_ref = approval.question_id.clone().unwrap();
    approval.body = d::Body::ApprovalAnswer(Box::new(d::BodyApprovalAnswer { approved: true }));
    let accepted = cold.call(json!({
        "kind":"answer", "message_json":serde_json::to_string(&approval).unwrap()
    }));
    assert_eq!(accepted["isError"], false, "{accepted}");
    let replay = cold.call(json!({
        "kind":"submit", "accepted_ref":replay_ref, "key":key
    }));
    if replay["isError"] == false {
        let pending = typed_view(&replay);
        assert!(pending.run_id.as_deref().is_none_or(|id| id == run));
    }
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        let replay = cold.call(json!({"kind":"inspect", "key":key}));
        if replay["isError"] == true {
            assert_eq!(replay["structuredContent"]["known_run_id"], run, "{replay}");
            assert_ne!(
                replay["structuredContent"]["effect_observation"], "none",
                "{replay}"
            );
            break;
        }
        let pending = typed_view(&replay);
        assert!(pending.run_id.as_deref().is_none_or(|id| id == run));
        assert!(
            Instant::now() < deadline,
            "damaged replay inspection deadline"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    cold.finish();
    assert_eq!(fs::read(root.path().join("counter")).unwrap(), b"effect\n");
}
