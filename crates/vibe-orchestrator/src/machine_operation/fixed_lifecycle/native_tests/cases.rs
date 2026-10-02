#![cfg(test)]
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-OPERATION-SERVICE");
use super::*;
use LifecycleActionErrorCode as Code;
use serde_json::json;

#[test]
fn explicit_force_route_typed_approval_retry_and_changed_force_refusal() {
    let f = Fixture::new();
    let root = f.dir.path();
    let route = FixedLifecycleRoute::new(f.host(true));
    let ordinary = approve(&route, root, RequestedOptionsControl::None);
    let mut action = prepare(RequestedOptionsControl::None);
    let Action::Prepare(p) = &mut action else {
        unreachable!()
    };
    p.requested_options.force = true;
    let mut q = question(call(&route, root, action).unwrap());
    assert!(q.requested_options.force);
    q.body = d::Body::SelectionAnswer(Box::new(d::BodySelectionAnswer {
        choice_ids: vec!["none".into()],
    }));
    q = question(call(&route, root, answer(&q)).unwrap());
    let mut wrong = q.clone();
    wrong.requested_options.force = false;
    wrong.body = d::Body::ApprovalAnswer(Box::new(d::BodyApprovalAnswer { approved: true }));
    assert_eq!(
        call(&route, root, answer(&wrong)).unwrap_err().error.code,
        Code::DecisionRefused
    );
    q.body = d::Body::ApprovalAnswer(Box::new(d::BodyApprovalAnswer { approved: true }));
    let Reply::Validated(approved) = call(&route, root, answer(&q)).unwrap().reply else {
        panic!("exact approved force")
    };
    let reference = approved.accepted_ref;
    call(&route, root, submit(&reference, "once")).unwrap();
    until(|| root.join("entered").exists());
    f.count();
    let run = row(call(&route, root, inspect("once")).unwrap())
        .run_id
        .unwrap();
    call(&route, root, submit(&reference, "once")).unwrap();
    let refusal = call(&route, root, submit(&ordinary, "once")).unwrap_err();
    assert_eq!(refusal.error.code, Code::SubmissionRefused);
    assert!(refusal.error.message.contains("AnswerConflict"));
    assert_eq!(refusal.error.known_run_id.as_deref(), Some(run.as_str()));
    f.release();
    until(|| {
        matches!(
            row(call(&route, root, inspect("once")).unwrap()).status,
            v::Status::ResultAvailable(_)
        )
    });
    let terminal = row(call(&route, root, inspect("once")).unwrap());
    // A lost submit reply can be retried through the same registered action shape.
    assert_eq!(
        row(call(&route, root, submit(&reference, "once")).unwrap()),
        terminal
    );
    let capsule: serde_json::Value = serde_json::from_slice(
        &fs::read(
            root.join(".vibe/lifecycle-outcomes")
                .join(format!("{run}.admitted-input.json")),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(capsule["request"]["requested_options"]["force"], true);
    assert_eq!(capsule["plan"]["requested_options"]["force"], true);
    assert_eq!(capsule["plan"]["base_v4"]["force"], false);
    f.count();
}

#[test]
fn explicit_trace_force_route_typed_approval_retry_and_changed_trace_refusal() {
    let f = Fixture::new();
    let root = f.dir.path();
    let route = FixedLifecycleRoute::new(f.host(true));
    let mut ordinary_action = prepare(RequestedOptionsControl::None);
    let Action::Prepare(p) = &mut ordinary_action else {
        unreachable!()
    };
    p.requested_options.force = true;
    let mut q = question(call(&route, root, ordinary_action).unwrap());
    q.body = d::Body::SelectionAnswer(Box::new(d::BodySelectionAnswer {
        choice_ids: vec!["none".into()],
    }));
    q = question(call(&route, root, answer(&q)).unwrap());
    q.body = d::Body::ApprovalAnswer(Box::new(d::BodyApprovalAnswer { approved: true }));
    let Reply::Validated(approved) = call(&route, root, answer(&q)).unwrap().reply else {
        panic!("ordinary force approval")
    };
    let ordinary = approved.accepted_ref;
    let mut action = prepare(RequestedOptionsControl::None);
    let Action::Prepare(p) = &mut action else {
        unreachable!()
    };
    p.requested_options.force = true;
    p.requested_options.trace = true;
    let mut q = question(call(&route, root, action).unwrap());
    assert!(q.requested_options.force && q.requested_options.trace);
    q.body = d::Body::SelectionAnswer(Box::new(d::BodySelectionAnswer {
        choice_ids: vec!["none".into()],
    }));
    q = question(call(&route, root, answer(&q)).unwrap());
    let mut wrong = q.clone();
    wrong.requested_options.trace = false;
    wrong.body = d::Body::ApprovalAnswer(Box::new(d::BodyApprovalAnswer { approved: true }));
    assert_eq!(
        call(&route, root, answer(&wrong)).unwrap_err().error.code,
        Code::DecisionRefused
    );
    q.body = d::Body::ApprovalAnswer(Box::new(d::BodyApprovalAnswer { approved: true }));
    let Reply::Validated(approved) = call(&route, root, answer(&q)).unwrap().reply else {
        panic!("exact approved force and trace")
    };
    let reference = approved.accepted_ref;
    call(&route, root, submit(&reference, "once")).unwrap();
    until(|| root.join("entered").exists());
    f.count();
    let run = row(call(&route, root, inspect("once")).unwrap())
        .run_id
        .unwrap();
    call(&route, root, submit(&reference, "once")).unwrap();
    let refusal = call(&route, root, submit(&ordinary, "once")).unwrap_err();
    assert_eq!(refusal.error.code, Code::SubmissionRefused);
    assert!(refusal.error.message.contains("AnswerConflict"));
    assert_eq!(refusal.error.known_run_id.as_deref(), Some(run.as_str()));
    f.release();
    until(|| {
        matches!(
            row(call(&route, root, inspect("once")).unwrap()).status,
            v::Status::ResultAvailable(_)
        )
    });
    let terminal = row(call(&route, root, inspect("once")).unwrap());
    // A lost submit reply can be retried through the same registered action shape.
    assert_eq!(
        row(call(&route, root, submit(&reference, "once")).unwrap()),
        terminal
    );
    let capsule: serde_json::Value = serde_json::from_slice(
        &fs::read(
            root.join(".vibe/lifecycle-outcomes")
                .join(format!("{run}.admitted-input.json")),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(capsule["request"]["requested_options"]["force"], true);
    assert_eq!(capsule["plan"]["requested_options"]["force"], true);
    assert_eq!(capsule["plan"]["base_v4"]["force"], false);
    assert_eq!(capsule["request"]["requested_options"]["trace"], true);
    assert_eq!(capsule["plan"]["requested_options"]["trace"], true);
    assert_eq!(capsule["plan"]["base_v4"]["trace_flag"], false);
    assert_eq!(capsule["plan"]["base_v4"]["effective_trace"], false);
    let report: serde_json::Value = serde_json::from_slice(
        &fs::read(
            root.join(".vibe/lifecycle-outcomes")
                .join(format!("{run}.report.json")),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(report["trace"]["status"], "ok");
    assert_eq!(report["trace"]["run_id"], run);
    assert_eq!(report["trace"]["snapshots"], "0");
    assert_eq!(report["trace"]["finalised"], true);
    f.count();
}

#[test]
fn direct_typed_decisions_same_key_owner_run_and_before_effect_refusals() {
    let f = Fixture::new();
    let host = f.host(true);
    let route = FixedLifecycleRoute::new(host.clone());
    let root = f.dir.path();
    let reference = approve(&route, root, RequestedOptionsControl::None);
    let changed = approve(&route, root, RequestedOptionsControl::Detach);
    assert_eq!(
        call(&route, root, submit("forged", "once"))
            .unwrap_err()
            .error
            .code,
        Code::UnknownReference
    );
    let foreign = tempfile::tempdir().unwrap();
    assert_eq!(
        call(&route, foreign.path(), submit(&reference, "once"))
            .unwrap_err()
            .error
            .code,
        Code::ContextRootMismatch
    );
    f.no_provider();
    assert!(call(&route, root, submit(&reference, "once")).is_ok());
    until(|| root.join("entered").exists());
    f.count();
    let run = row(call(&route, root, inspect("once")).unwrap())
        .run_id
        .unwrap();
    let repeated = row(call(&route, root, submit(&reference, "once")).unwrap());
    assert_eq!(repeated.run_id.as_deref(), Some(run.as_str()));
    let refused = call(&route, root, submit(&changed, "once")).unwrap_err();
    assert_eq!(refused.error.code, Code::SubmissionRefused);
    assert_eq!(refused.error.known_run_id.as_deref(), Some(run.as_str()));
    assert!(refused.effects_may_have_begun);
    f.count();
    f.release();
    until(|| {
        matches!(
            row(call(&route, root, inspect("once")).unwrap()).status,
            v::Status::ResultAvailable(_)
        )
    });
    let terminal = row(call(&route, root, inspect("once")).unwrap());
    assert_eq!(terminal.run_id.as_deref(), Some(run.as_str()));
    let lease = LifecycleLease::acquire(root).unwrap();
    let c1 = SubmissionReservations::inspect_existing_key(&lease, &f.ns, &f.scope, "once")
        .unwrap()
        .unwrap();
    assert_eq!(c1.domain_run_id, run);
    drop(lease);
    host.set_host_policy(facts(false)).unwrap();
    let stale = call(&route, root, submit(&reference, "other-key")).unwrap_err();
    assert_eq!(stale.error.code, Code::SubmissionRefused);
    assert!(stale.error.message.contains("StalePolicy"));
    f.count();
}

#[test]
fn cold_unknown_reference_preserves_retained_run_and_effect_uncertainty() {
    let f = Fixture::new();
    let root = f.dir.path();
    let route = FixedLifecycleRoute::new(f.host(true));
    let accepted = approve(&route, root, RequestedOptionsControl::None);
    call(&route, root, submit(&accepted, "once")).unwrap();
    until(|| root.join("entered").exists());
    f.release();
    until(|| {
        matches!(
            row(call(&route, root, inspect("once")).unwrap()).status,
            v::Status::ResultAvailable(_)
        )
    });
    let run = row(call(&route, root, inspect("once")).unwrap())
        .run_id
        .unwrap();
    drop(route);

    let cold = FixedLifecycleRoute::new(f.host(true));
    let refusal = call(&cold, root, submit(&accepted, "once")).unwrap_err();
    assert_eq!(refusal.error.code, Code::UnknownReference);
    assert_eq!(refusal.error.known_run_id.as_deref(), Some(run.as_str()));
    assert!(refusal.effects_may_have_begun);
    assert!(refusal.error.view_json.is_some());
    assert_eq!(
        row(call(&cold, root, inspect("once")).unwrap())
            .run_id
            .as_deref(),
        Some(run.as_str())
    );
    f.count();
}

#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-OPERATION-SERVICE",
    r = 1
)]
#[test]
fn old_mcp_characterization_and_raw_no_grant_options_bounds() {
    let f = Fixture::new();
    let root = f.dir.path();
    let route = FixedLifecycleRoute::new(f.host(false));
    let absent = call(&route, root, inspect("unknown")).unwrap();
    assert_eq!(
        serde_json::to_value(absent).unwrap(),
        json!({"schema":1,"reply":{"kind":"absent","key":"unknown"}})
    );
    let error = call(&route, root, submit("forged", "once")).unwrap_err();
    assert!(!error.effects_may_have_begun);
    assert_eq!(
        serde_json::to_value(error.error).unwrap(),
        json!({
            "schema":1,"code":"unknown_reference","message":"accepted reference is not retained by this owner bridge",
            "message_truncated":false,"requested_action":"submit","requested_phase":"validate",
            "requested_scope":{"context":"mcp-native","target":"root","principal":"trusted-host"},
            "scope_unavailable_reason":null,"known_run_id":null,"view_json":null,"effect_observation":"unavailable"
        })
    );
    for (phase, force, trace, profile) in [
        ("build", false, false, None),
        ("validate", false, false, Some("posix".into())),
    ] {
        let mut req = request(prepare(RequestedOptionsControl::None));
        let Action::Prepare(p) = &mut req.action else {
            unreachable!()
        };
        p.phase = phase.into();
        p.requested_options.force = force;
        p.requested_options.trace = trace;
        p.requested_options.deploy_profile = profile;
        let error = FixedLifecycleRoute::dispatch(Some(&route), &bytes(&req), root).unwrap_err();
        assert_eq!(error.error.code, Code::ImplementationGap);
        assert_eq!(
            error.error.message,
            "fixed owner implements Validate without deploy_profile"
        );
        assert!(!error.effects_may_have_begun);
    }
    let absent_root = root.join("absent");
    let req = bytes(&request(inspect("once")));
    let ungranted = FixedLifecycleRoute::dispatch(None, &req, &absent_root).unwrap_err();
    assert_eq!(ungranted.error.code, Code::UnavailableHost);
    assert!(!ungranted.effects_may_have_begun);
    assert!(!absent_root.exists());
    for raw in [
        b"{\"grant\":true}".to_vec(),
        vec![b' '; REQUEST_CAP + 1],
        b"{\"schema\":2,\"action\":{\"kind\":\"inspect\",\"key\":\"once\"}}".to_vec(),
    ] {
        assert_eq!(
            FixedLifecycleRoute::dispatch(None, &raw, &absent_root)
                .unwrap_err()
                .error
                .code,
            Code::InvalidRequest
        );
    }
    let inner = call(
        &route,
        root,
        Action::Answer(Box::new(ActionAnswer {
            message_json: " ".repeat(65537),
        })),
    )
    .unwrap_err();
    assert_eq!(inner.error.code, Code::InvalidRequest);
    assert!(!inner.effects_may_have_begun);
    f.no_provider();
}

#[test]
fn publication_refusal_after_counted_effect_keeps_known_owner_truth() {
    let f = Fixture::new();
    let root = f.dir.path();
    let route = FixedLifecycleRoute::new(f.host(true));
    let reference = approve(&route, root, RequestedOptionsControl::None);
    call(&route, root, submit(&reference, "once")).unwrap();
    until(|| root.join("entered").exists());
    f.release();
    until(|| {
        matches!(
            row(call(&route, root, inspect("once")).unwrap()).status,
            v::Status::ResultAvailable(_)
        )
    });
    let run = row(call(&route, root, inspect("once")).unwrap())
        .run_id
        .unwrap();
    let req = bytes(&request(inspect("once")));
    // The incumbent marks arbitrary script effects Uncertain even after natural exit.
    // Exercise the production publication-error branch by lowering only its final cap.
    let refusal = FixedLifecycleRoute::dispatch_bounded(Some(&route), &req, root, 1).unwrap_err();
    assert_eq!(refusal.error.code, Code::PublicationUnavailable);
    assert!(refusal.effects_may_have_begun);
    assert_eq!(refusal.error.known_run_id.as_deref(), Some(run.as_str()));
    assert_eq!(
        refusal.error.requested_scope.as_ref().unwrap().context,
        f.scope.context
    );
    assert_eq!(
        refusal.error.effect_observation,
        LifecycleActionErrorEffectObservation::Uncertain
    );
    assert!(refusal.error.view_json.is_some());
    let surface = FixedLifecycleRouteError::publication_unavailable(
        "surface serialization failed",
        Some(&route),
        &req,
    );
    assert!(surface.effects_may_have_begun);
    assert_eq!(surface.error.known_run_id.as_deref(), Some(run.as_str()));
    assert_eq!(
        surface.error.effect_observation,
        LifecycleActionErrorEffectObservation::Uncertain
    );
    fs::write(
        root.join(".vibe/lifecycle-outcomes")
            .join(format!("{run}.report.json")),
        "corrupt",
    )
    .unwrap();
    let damaged = call(&route, root, inspect("once")).unwrap_err();
    assert_eq!(damaged.error.code, Code::InspectionUnavailable);
    assert!(damaged.effects_may_have_begun);
    assert_eq!(damaged.error.known_run_id.as_deref(), Some(run.as_str()));
    assert_eq!(
        damaged.error.effect_observation,
        LifecycleActionErrorEffectObservation::Unavailable
    );
    assert!(damaged.error.view_json.is_some());
    let _ = call(&route, root, submit(&reference, "once"));
    f.count();
}
