//! Retained phase and effect truth when a real MCP script fails after writing.

use super::*;

#[test]
fn real_stdio_failed_validate_retains_prior_effect_and_phase_cold() {
    let user = UserScratch::new();
    let root = project();
    let quoted = root.path().to_str().unwrap().replace('\'', "'\\''");
    fs::write(
        root.path().join("run.sh"),
        format!("#!/bin/sh\nset -eu\ncd '{quoted}'\nprintf 'effect\\n' >> counter\nexit 17\n"),
    )
    .unwrap();

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
    let key = "failed-after-effect";
    let started = typed_view(&server.call(json!({
        "kind":"submit", "accepted_ref":accepted_ref, "key":key
    })));
    let mut run = started.run_id;
    let deadline = Instant::now() + Duration::from_secs(20);
    let terminal = loop {
        let view = typed_view(&server.call(json!({"kind":"inspect", "key":key})));
        if let Some(observed) = &view.run_id {
            if let Some(expected) = &run {
                assert_eq!(observed, expected);
            } else {
                run = Some(observed.clone());
            }
        }
        if matches!(view.status, v::Status::ResultAvailable(_)) {
            break view;
        }
        assert!(
            Instant::now() < deadline,
            "failed Validate terminal deadline"
        );
        std::thread::sleep(Duration::from_millis(10));
    };
    let run = run.expect("failed result retains C1 run identity");
    assert_eq!(fs::read(root.path().join("counter")).unwrap(), b"effect\n");
    assert!(terminal.partial, "{terminal:?}");
    assert_eq!(
        terminal.last_phase.as_ref().map(|p| p.phase.as_str()),
        Some("validate")
    );
    assert!(terminal.work_outcomes.iter().any(|w| w.status == "fail"));
    assert!(matches!(
        terminal.status,
        v::Status::ResultAvailable(ref result)
            if matches!(result.outcome, v::StatusResultAvailableOutcome::Failed)
                && matches!(result.effect_state, v::StatusResultAvailableEffectState::Uncertain)
    ));
    server.finish();

    let mut cold = LiveServer::start(&user, root.path(), "reattach");
    let view = typed_view(&cold.call(json!({"kind":"inspect", "key":key})));
    assert_eq!(view.run_id.as_deref(), Some(run.as_str()));
    assert!(matches!(
        view.basis,
        v::LifecycleSubmissionViewBasis::RetainedReport
    ));
    assert!(view.partial);
    assert_eq!(
        view.last_phase.as_ref().map(|p| p.phase.as_str()),
        Some("validate")
    );
    assert_eq!(
        view.last_phase.as_ref().and_then(|p| p.status.as_deref()),
        Some("fail")
    );
    assert_eq!(view.work_outcomes, terminal.work_outcomes);
    assert_eq!(view.status, terminal.status);
    cold.finish();
    assert_eq!(fs::read(root.path().join("counter")).unwrap(), b"effect\n");
}
