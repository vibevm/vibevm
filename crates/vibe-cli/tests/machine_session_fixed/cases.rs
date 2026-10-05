use super::*;

#[test]
fn foreign_physical_root_cannot_use_another_projects_pointer() {
    let first = Fixture::new(false);
    let second = Fixture::new(false);
    first.client("issue").finish();
    second.client("issue").finish();
    let retained = fs::read(second.root().join(".vibe/machine-submissions.json")).unwrap();
    fs::write(second.pointer(), fs::read(first.pointer()).unwrap()).unwrap();
    refusal(
        &mut second.fixed_command("reattach", None, &FLAGS),
        "ForeignBinding",
        1,
    );
    refusal(
        &mut second.fixed_command("issue", Some("60000"), &FLAGS),
        "ForeignBinding",
        1,
    );
    assert_eq!(
        fs::read(second.root().join(".vibe/machine-submissions.json")).unwrap(),
        retained
    );
    first.no_provider();
    second.no_provider();
}

#[test]
fn independent_issue_and_reattach_preserve_one_namespace_run_effect_and_terminal_truth() {
    let fixture = Fixture::new(false);
    let mut first = fixture.client("issue");
    let old_question = approval(&mut first);
    let old_reference = approve(&mut first);
    first.call(submit(&old_reference, "once"));
    let original = terminal(&mut first, "once");
    let run = original.row().run_id.clone().unwrap();
    fixture.count_one();
    let pointer_before = fs::read(fixture.pointer()).unwrap();
    first.finish();
    let mut second = fixture.client("reattach");
    assert_eq!(
        second.call(answer(&old_question)).error().code,
        Code::DecisionRefused
    );
    let stale = second.call(submit(&old_reference, "once"));
    assert_eq!(stale.error().code, Code::UnknownReference);
    assert_same_terminal_truth(&second.call(inspect("once")).row(), &original.row());
    let reference = approve(&mut second);
    second.call(submit(&reference, "once"));
    let retained = terminal(&mut second, "once");
    assert_same_terminal_truth(&retained.row(), &original.row());
    assert_eq!(retained.outcome, original.outcome);
    second.finish();
    assert_eq!(fs::read(fixture.pointer()).unwrap(), pointer_before);
    fixture.count_one();
    let index = fixture.index();
    let namespaces = index["namespaces"].as_object().unwrap();
    assert_eq!(namespaces.len(), 1);
    let keys = namespaces.values().next().unwrap()["keys"]
        .as_object()
        .unwrap();
    assert_eq!(keys.len(), 1);
    assert_eq!(keys["once"]["domain_run_id"], run);
    assert!(
        fixture
            .root()
            .join(format!(".vibe/lifecycle-outcomes/{run}.marker.json"))
            .is_file()
    );
    assert!(
        fixture
            .root()
            .join(format!(".vibe/lifecycle-outcomes/{run}.report.json"))
            .is_file()
    );
}

#[test]
fn cold_stale_submit_preserves_known_run_and_uncertain_truth() {
    let fixture = Fixture::new(false);
    let mut first = fixture.client("issue");
    let reference = approve(&mut first);
    first.call(submit(&reference, "once"));
    let original = terminal(&mut first, "once");
    let run = original.row().run_id.unwrap();
    first.finish();
    let mut cold = fixture.client("reattach");
    let refusal = cold.call(submit(&reference, "once"));
    assert_eq!(refusal.error().code, Code::UnknownReference);
    assert_eq!(
        refusal.error().known_run_id.as_deref(),
        Some(run.as_str()),
        "cold refusal truth: {:?}",
        refusal.outcome
    );
    assert_eq!(
        refusal.outcome.effect_state,
        f::OutcomeEffectState::Uncertain
    );
    assert_eq!(refusal.outcome.provenance_id.as_deref(), Some(run.as_str()));
    cold.finish();
    fixture.count_one();
}

#[test]
fn independent_cli_connections_assign_fresh_outer_session_context_and_target_ids() {
    let fixture = Fixture::new(false);
    let first = fixture.client("issue");
    let old_ids = (
        first.session.clone(),
        first.context.clone(),
        first.target.clone(),
    );
    first.finish();
    let second = fixture.client("reattach");
    assert_ne!(second.session, old_ids.0);
    assert_ne!(second.context, old_ids.1);
    assert_ne!(second.target, old_ids.2);
    second.finish();
}

#[test]
fn invalid_mode_ttl_permissions_and_roots_refuse_before_project_mutation() {
    let fixture = Fixture::new(false);
    let before = snapshot(fixture.dir.path());
    for suffix in [
        vec!["--fixed-validate", "issue"],
        vec!["--fixed-validate", "other"],
        vec!["--fixed-validate", "reattach", "--ttl-ms", "60000"],
        vec!["--ttl-ms", "60000"],
        vec!["--fixed-validate", "issue", "--ttl-ms", "0"],
        vec!["--fixed-validate", "issue", "--ttl-ms", "86400001"],
        vec!["--fixed-validate", "issue", "--ttl-ms", "bad"],
        vec![
            "--fixed-validate",
            "issue",
            "--ttl-ms",
            "60000",
            "--fixed-validate",
            "reattach",
        ],
        vec!["--approve-captured-script"],
        vec!["--allow-detach"],
        vec!["--allow-cancel-pre-dispatch"],
        vec!["--allow-cancel-deferred-boundary"],
        vec!["--fixed-validate", "reattach", "--force"],
        vec!["--fixed-validate", "reattach", "--trace"],
        vec!["--fixed-validate", "reattach", "--deploy-profile", "posix"],
        vec!["--fixed-validate", "reattach", "--principal", "caller"],
        vec!["--fixed-validate", "reattach", "--namespace", "caller"],
        vec!["--fixed-validate", "reattach", "--target", "caller"],
    ] {
        let mut command = fixture.command();
        command
            .args(["machine", "session", "--root"])
            .arg(fixture.root())
            .args(suffix);
        refusal(&mut command, "", 2);
        assert_eq!(snapshot(fixture.dir.path()), before);
    }
    for root in [
        fixture.dir.path().join("missing"),
        fixture.dir.path().join("home"),
    ] {
        let mut command = fixture.command();
        command
            .args(["machine", "session", "--root"])
            .arg(root)
            .args(["--fixed-validate", "issue", "--ttl-ms", "60000"]);
        refusal(&mut command, "", 1);
        assert_eq!(snapshot(fixture.dir.path()), before);
    }
    refusal(
        fixture.command().args([
            "machine",
            "session",
            "--fixed-validate",
            "issue",
            "--ttl-ms",
            "60000",
        ]),
        "--root",
        2,
    );
    assert_eq!(snapshot(fixture.dir.path()), before);
}

#[test]
fn missing_corrupt_foreign_pointer_and_policy_disagreement_never_reissue() {
    let fixture = Fixture::new(false);
    refusal(
        &mut fixture.fixed_command("reattach", None, &FLAGS),
        "MissingPointer",
        1,
    );
    assert!(
        !fixture
            .root()
            .join(".vibe/machine-submissions.json")
            .exists()
    );
    fixture.client("issue").finish();
    let index = fs::read(fixture.root().join(".vibe/machine-submissions.json")).unwrap();
    let path = fixture.pointer();
    let bytes = fs::read(&path).unwrap();
    refusal(
        &mut fixture.fixed_command("issue", Some("60000"), &FLAGS),
        "AlreadyIssued",
        1,
    );
    for flags in [&FLAGS[..0], &FLAGS[..1], &FLAGS[..2], &FLAGS[..3]] {
        refusal(
            &mut fixture.fixed_command("reattach", None, flags),
            "StartupDisagreement",
            1,
        );
    }
    for contents in [
        b"{corrupt".to_vec(),
        {
            let mut pointer: Value = serde_json::from_slice(&bytes).unwrap();
            pointer["policy_source"] = json!("foreign-source");
            serde_json::to_vec(&pointer).unwrap()
        },
        {
            let mut pointer: Value = serde_json::from_slice(&bytes).unwrap();
            pointer["root_token"] = json!("foreign-root");
            serde_json::to_vec(&pointer).unwrap()
        },
    ] {
        fs::write(&path, contents).unwrap();
        refusal(
            &mut fixture.fixed_command("reattach", None, &FLAGS),
            "MACHINE-CLI-HOST-REATTACH",
            1,
        );
        refusal(
            &mut fixture.fixed_command("issue", Some("60000"), &FLAGS),
            "MACHINE-CLI-HOST-REATTACH",
            1,
        );
    }
    fs::remove_file(&path).unwrap();
    refusal(
        &mut fixture.fixed_command("reattach", None, &FLAGS),
        "MissingPointer",
        1,
    );
    refusal(
        &mut fixture.fixed_command("issue", Some("60000"), &FLAGS),
        "MissingPointer",
        1,
    );
    assert_eq!(
        fs::read(fixture.root().join(".vibe/machine-submissions.json")).unwrap(),
        index
    );
    assert!(!path.exists());
    fixture.no_provider();
}

#[test]
fn frame_supplied_authority_outer_ids_and_unsupported_options_never_dispatch() {
    let fixture = Fixture::new(false);
    let mut client = fixture.client("issue");
    for (context, target) in [
        ("caller".to_owned(), Some(client.target.clone())),
        (client.context.clone(), Some("caller".into())),
        (client.context.clone(), None),
    ] {
        let mut frame = client.frame(envelope(prepare("none")).to_string());
        let f::Body::Request(body) = &mut frame.body else {
            panic!()
        };
        body.context_id = context;
        body.target_id = target;
        let f::Body::Refusal(body) = client.send(&frame).body else {
            panic!("outer refusal")
        };
        assert!(body.envelope_json.is_none());
        assert_eq!(body.outcome.effect_state, f::OutcomeEffectState::NoEffects);
    }
    for field in ["root", "namespace", "policy", "principal", "target"] {
        let mut forged = envelope(inspect("absent"));
        forged[field] = json!("caller");
        assert_eq!(client.raw(forged).error().code, Code::InvalidRequest);
        let mut forged = envelope(inspect("absent"));
        forged["body"]["payload_json"] =
            json!(json!({"schema":1,field:"caller","action":inspect("absent")}).to_string());
        assert_eq!(client.raw(forged).error().code, Code::InvalidRequest);
    }
    let publication = client.call(json!({"kind":"prepare","phase":"validate",
        "requested_options":{"force":false,"trace":false,"deploy_profile":"posix","control":"none"}}));
    assert_eq!(publication.error().code, Code::ImplementationGap);
    assert_eq!(
        publication.outcome.effect_state,
        f::OutcomeEffectState::NoEffects
    );
    fixture.no_provider();
    client.finish();
}

#[test]
fn explicit_force_uses_a_typed_approval_and_replays_one_run_across_cli_exit() {
    fn approved(client: &mut Client, force: bool) -> String {
        let mut question = client
            .call(
                json!({"kind":"prepare","phase":"validate","requested_options":{
                    "force":force,"trace":false,"deploy_profile":null,"control":"none"
                }}),
            )
            .question();
        assert!(matches!(question.body, d::Body::SelectionQuestion(_)));
        question.body = d::Body::SelectionAnswer(Box::new(d::BodySelectionAnswer {
            choice_ids: vec!["none".into()],
        }));
        question = client.call(answer(&question)).question();
        question.body = d::Body::ApprovalAnswer(Box::new(d::BodyApprovalAnswer { approved: true }));
        let Reply::Validated(reply) = client.call(answer(&question)).reply().reply else {
            panic!("exact typed force approval")
        };
        reply.accepted_ref
    }

    let fixture = Fixture::new(false);
    let mut first = fixture.client("issue");
    let ordinary = approved(&mut first, false);
    first.call(submit(&ordinary, "ordinary"));
    let ordinary_run = terminal(&mut first, "ordinary").row().run_id.unwrap();
    fixture.count_one();
    let forced = approved(&mut first, true);
    first.call(submit(&forced, "forced"));
    let forced_run = terminal(&mut first, "forced").row().run_id.unwrap();
    assert_ne!(forced_run, ordinary_run);
    assert_eq!(
        fs::read(fixture.root().join("counter")).unwrap(),
        b"effect\neffect\n"
    );
    first.finish();

    let mut second = fixture.client("reattach");
    let forced_again = approved(&mut second, true);
    second.call(submit(&forced_again, "forced"));
    assert_eq!(
        terminal(&mut second, "forced").row().run_id.as_deref(),
        Some(forced_run.as_str())
    );
    let changed = approved(&mut second, false);
    assert_eq!(
        second.call(submit(&changed, "forced")).error().code,
        Code::SubmissionRefused
    );
    second.finish();
    assert_eq!(
        fs::read(fixture.root().join("counter")).unwrap(),
        b"effect\neffect\n"
    );
}

#[test]
fn absent_script_permission_denies_exact_approval_and_controls() {
    let fixture = Fixture::new(false);
    let mut client = Client::new(
        fixture
            .fixed_command("issue", Some("60000"), &[])
            .spawn()
            .unwrap(),
    );
    for control in ["detach", "cancel_pre_dispatch", "cancel_deferred_boundary"] {
        let refusal = client.call(prepare(control));
        assert!(matches!(refusal.envelope.body, e::Body::Error(_)));
    }
    // With no granted controls the owner may ask approval directly.
    let mut question = client.call(prepare("none")).question();
    if matches!(question.body, d::Body::SelectionQuestion(_)) {
        question.body = d::Body::SelectionAnswer(Box::new(d::BodySelectionAnswer {
            choice_ids: vec!["none".into()],
        }));
        question = client.call(answer(&question)).question();
    }
    question.body = d::Body::ApprovalAnswer(Box::new(d::BodyApprovalAnswer { approved: true }));
    assert_eq!(
        client.call(answer(&question)).error().code,
        Code::DecisionRefused
    );
    fixture.no_provider();
    client.finish();
}

#[test]
fn retired_namespace_preserves_reserved_terminal_key_but_refuses_fresh_effect() {
    let fixture = Fixture::new(false);
    let mut first = fixture.client("issue");
    let reference = approve(&mut first);
    first.call(submit(&reference, "once"));
    let original = terminal(&mut first, "once").row();
    first.finish();
    let pointer: Value = serde_json::from_slice(&fs::read(fixture.pointer()).unwrap()).unwrap();
    let scope = vibe_lifecycle::SubmissionScope {
        context: pointer["scope"]["context"].as_str().unwrap().into(),
        target: pointer["scope"]["target"].as_str().unwrap().into(),
        principal: pointer["scope"]["principal"].as_str().unwrap().into(),
    };
    let namespace = vibe_lifecycle::SubmissionNamespace::from_host_token(
        pointer["grant"]["namespace"].as_str().unwrap().into(),
    )
    .unwrap();
    let lease = vibe_lifecycle::LifecycleLease::acquire(&fixture.root()).unwrap();
    vibe_lifecycle::SubmissionReservations::retire_namespace(&lease, &namespace, &scope).unwrap();
    drop(lease);
    let mut cold = fixture.client("reattach");
    assert_same_terminal_truth(&cold.call(inspect("once")).row(), &original);
    let reference = approve(&mut cold);
    cold.call(submit(&reference, "fresh"));
    until(|| {
        matches!(
            cold.call(inspect("fresh")).row().status,
            v::Status::RecoveryRequired(_)
        )
    });
    fixture.count_one();
    cold.finish();
}

#[test]
fn expired_namespace_reattaches_for_observation_but_never_dispatches() {
    let fixture = Fixture::new(false);
    let mut first = Client::new(
        fixture
            .fixed_command("issue", Some("1"), &FLAGS)
            .spawn()
            .unwrap(),
    );
    let pointer: Value = serde_json::from_slice(&fs::read(fixture.pointer()).unwrap()).unwrap();
    let expiry = pointer["grant"]["expires_at_ms"].as_u64().unwrap();
    until(|| {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
            >= u128::from(expiry)
    });
    let reference = approve(&mut first);
    first.call(submit(&reference, "expired"));
    until(|| {
        matches!(
            first.call(inspect("expired")).row().status,
            v::Status::RecoveryRequired(_)
        )
    });
    first.finish();
    let mut cold = fixture.client("reattach");
    assert!(matches!(
        cold.call(inspect("unreserved")).reply().reply,
        Reply::Absent(_)
    ));
    cold.finish();
    fixture.no_provider();
    assert_eq!(fixture.index()["namespaces"].as_object().unwrap().len(), 1);
}

#[test]
fn lost_submission_response_and_eof_preserve_one_terminal_run() {
    let fixture = Fixture::new(false);
    let mut first = fixture.client("issue");
    let reference = approve(&mut first);
    let frame = first.frame(envelope(submit(&reference, "once")).to_string());
    first.write(&frame);
    // Caller loses the submit reply, then observes owner completion before EOF.
    let _lost_reply = first
        .replies
        .recv_timeout(Duration::from_secs(20))
        .unwrap()
        .unwrap();
    let original = terminal(&mut first, "once");
    let run = original.row().run_id.clone().unwrap();
    first.finish();
    let mut cold = fixture.client("reattach");
    assert_eq!(
        cold.call(inspect("once")).row().run_id.as_deref(),
        Some(run.as_str())
    );
    let reference = approve(&mut cold);
    cold.call(submit(&reference, "once"));
    assert_same_terminal_truth(&terminal(&mut cold, "once").row(), &original.row());
    cold.finish();
    fixture.count_one();
}

#[test]
fn active_owner_death_retains_known_run_uncertain_recovery_and_no_second_effect() {
    let fixture = Fixture::new(true);
    let mut first = fixture.client("issue");
    let reference = approve(&mut first);
    first.call(submit(&reference, "once"));
    until(|| fixture.root().join("entered").exists());
    let run = first.call(inspect("once")).row().run_id.unwrap();
    fixture.count_one();
    first.kill();
    fixture.release();
    let mut cold = fixture.client("reattach");
    let retained = cold.call(inspect("once"));
    assert_eq!(retained.row().run_id.as_deref(), Some(run.as_str()));
    assert_eq!(
        retained.outcome.effect_state,
        f::OutcomeEffectState::Uncertain
    );
    assert!(matches!(
        retained.row().status,
        v::Status::RecoveryRequired(_)
    ));
    let reference = approve(&mut cold);
    cold.call(submit(&reference, "once"));
    let mut final_recovery = None;
    until(|| {
        let publication = cold.call(inspect("once"));
        if matches!(publication.row().status, v::Status::RecoveryRequired(_)) {
            final_recovery = Some(publication);
            true
        } else {
            false
        }
    });
    let retained = final_recovery.unwrap();
    assert_eq!(retained.row().run_id.as_deref(), Some(run.as_str()));
    assert_eq!(
        retained.outcome.effect_state,
        f::OutcomeEffectState::Uncertain
    );
    assert!(matches!(
        retained.row().status,
        v::Status::RecoveryRequired(_)
    ));
    cold.finish();
    fixture.count_one();
}

#[test]
fn malformed_and_oversized_fixed_frames_refuse_without_provider_effect() {
    let fixture = Fixture::new(false);
    fixture.client("issue").finish();
    for (input, reason) in [
        (vec![0, 0, 0, 0], "ZeroLength"),
        (vec![0, 0], "TruncatedPrefix"),
        (vec![0, 0, 0, 1, b'{'], "InvalidJson"),
        (
            (8 * 1024 * 1024 + 1_u32).to_be_bytes().to_vec(),
            "OverLimit",
        ),
    ] {
        let mut child = fixture
            .fixed_command("reattach", None, &FLAGS)
            .spawn()
            .unwrap();
        child.stdin.take().unwrap().write_all(&input).unwrap();
        let output = child.wait_with_output().unwrap();
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains(reason));
        fixture.no_provider();
        assert_eq!(fixture.index()["namespaces"].as_object().unwrap().len(), 1);
    }
}
