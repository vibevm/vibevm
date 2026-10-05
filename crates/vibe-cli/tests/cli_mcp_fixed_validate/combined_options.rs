//! Process-level proof that fixed Validate retains the combined approved options.

use super::*;

fn approved_force_options(server: &mut LiveServer, trace: bool) -> String {
    let mut selection = typed_question(&server.call(json!({
        "kind":"prepare", "phase":"validate",
        "requested_options":{"force":true,"trace":trace,"deploy_profile":null,"control":"none"}
    })));
    assert!(selection.requested_options.force);
    assert_eq!(selection.requested_options.trace, trace);
    selection.body = d::Body::SelectionAnswer(Box::new(d::BodySelectionAnswer {
        choice_ids: vec!["none".into()],
    }));
    let mut approval = typed_question(&server.call(json!({
        "kind":"answer", "message_json":serde_json::to_string(&selection).unwrap()
    })));
    assert!(approval.requested_options.force);
    assert_eq!(approval.requested_options.trace, trace);
    let accepted_ref = approval.question_id.clone().unwrap();
    approval.body = d::Body::ApprovalAnswer(Box::new(d::BodyApprovalAnswer { approved: true }));
    let accepted = server.call(json!({
        "kind":"answer", "message_json":serde_json::to_string(&approval).unwrap()
    }));
    assert_eq!(accepted["isError"], false, "{accepted}");
    assert_eq!(
        accepted["structuredContent"]["reply"]["accepted_ref"],
        accepted_ref
    );
    accepted_ref
}

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

#[test]
fn two_mcp_processes_racing_combined_options_replay_one_run_and_effect() {
    let user = UserScratch::new();
    let root = project();
    let mut issued = LiveServer::start(&user, root.path(), "issue");
    let issued_ref = approved_force_options(&mut issued, true);
    let mut reattached = LiveServer::start(&user, root.path(), "reattach");
    let reattached_ref = approved_force_options(&mut reattached, true);

    let barrier = std::sync::Arc::new(std::sync::Barrier::new(3));
    let key = "stdio-combined-process-race";
    let first_barrier = barrier.clone();
    let first = std::thread::spawn(move || {
        first_barrier.wait();
        let reply = issued.call(json!({
            "kind":"submit", "accepted_ref":issued_ref, "key":key
        }));
        (issued, reply)
    });
    let second_barrier = barrier.clone();
    let second = std::thread::spawn(move || {
        second_barrier.wait();
        let reply = reattached.call(json!({
            "kind":"submit", "accepted_ref":reattached_ref, "key":key
        }));
        (reattached, reattached_ref, reply)
    });
    barrier.wait();
    let (mut issued, first_reply) = first.join().unwrap();
    let (mut reattached, _reattached_ref, second_reply) = second.join().unwrap();
    let mut run = None;
    for reply in [&first_reply, &second_reply] {
        if reply["isError"] == false {
            let view = typed_view(reply);
            if let Some(observed) = view.run_id {
                if let Some(expected) = &run {
                    assert_eq!(&observed, expected);
                } else {
                    run = Some(observed);
                }
            }
        } else {
            assert_eq!(
                reply["structuredContent"]["code"], "inspection_unavailable",
                "{reply}"
            );
            assert!(
                matches!(
                    reply["structuredContent"]["effect_observation"].as_str(),
                    Some("unavailable" | "uncertain")
                ),
                "{reply}"
            );
        }
    }
    assert!(first_reply["isError"] == false || second_reply["isError"] == false);
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        let mut terminal = false;
        for server in [&mut issued, &mut reattached] {
            let reply = server.call(json!({"kind":"inspect", "key":key}));
            if reply["isError"] == true {
                assert_eq!(
                    reply["structuredContent"]["code"], "inspection_unavailable",
                    "{reply}"
                );
                continue;
            }
            let view = typed_view(&reply);
            if let Some(observed) = &view.run_id {
                if let Some(expected) = &run {
                    assert_eq!(observed, expected);
                } else {
                    run = Some(observed.clone());
                }
            }
            terminal |= matches!(view.status, v::Status::ResultAvailable(_));
        }
        if terminal {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "process race terminal deadline: first={first_reply}; second={second_reply}; counter={:?}",
            fs::read(root.path().join("counter"))
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    let run = run.expect("terminal owner identity");
    issued.finish();
    reattached.finish();

    let mut cold = LiveServer::start(&user, root.path(), "reattach");
    let peer = typed_view(&cold.call(json!({"kind":"inspect", "key":key})));
    assert_eq!(peer.run_id.as_deref(), Some(run.as_str()));
    assert!(matches!(peer.status, v::Status::ResultAvailable(_)));
    let replay_ref = approved_force_options(&mut cold, true);
    let replay = typed_view(&cold.call(json!({
        "kind":"submit", "accepted_ref":replay_ref, "key":key
    })));
    if let Some(observed) = replay.run_id.as_deref() {
        assert_eq!(observed, run);
    }
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        let replay = typed_view(&cold.call(json!({"kind":"inspect", "key":key})));
        if let Some(observed) = replay.run_id.as_deref() {
            assert_eq!(observed, run);
        }
        if matches!(replay.status, v::Status::ResultAvailable(_)) {
            assert_eq!(replay.run_id.as_deref(), Some(run.as_str()));
            break;
        }
        assert!(Instant::now() < deadline, "cold replay terminal deadline");
        std::thread::sleep(Duration::from_millis(10));
    }
    let changed_ref = approved_force_options(&mut cold, false);
    let changed = cold.call(json!({
        "kind":"submit", "accepted_ref":changed_ref, "key":key
    }));
    assert_eq!(changed["isError"], true, "{changed}");
    assert_eq!(changed["structuredContent"]["code"], "submission_refused");
    cold.finish();

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
    let report: Value = serde_json::from_slice(
        &fs::read(
            root.path()
                .join(format!(".vibe/lifecycle-outcomes/{run}.report.json")),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(report["trace"]["run_id"], run);
    assert_eq!(report["trace"]["status"], "ok", "report={report}");
    assert_eq!(report["trace"]["finalised"], true);
    assert_eq!(
        fs::read(root.path().join("counter")).ok().as_deref(),
        Some(b"effect\n".as_slice()),
        "report={report}"
    );
}
