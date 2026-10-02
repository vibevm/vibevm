use super::*;

fn trusted(root: &str) -> TrustedContextSource {
    TrustedContextSource {
        kind: TrustedContextKind::Workspace,
        context: ServerContext::with_store_root(root, format!("{root}-store")),
        target_roots: vec![PathBuf::from(format!("{root}/member"))],
    }
}
fn offer(request_id: &str) -> SessionFrame {
    SessionFrame {
        schema: 1,
        session_id: None,
        request_id: Some(request_id.to_owned()),
        body: Body::InitializeOffer(Box::new(
            vibe_wire::generated::machine::e1::session_frame::BodyInitializeOffer {
                api_versions: vec!["future".to_owned(), API_VERSION.to_owned()],
                schema_offers: supported_schemas(),
                position_encodings: vec![POSITION_ENCODING.to_owned()],
                requested_capabilities: Capabilities {
                    streaming: true,
                    replay: true,
                    control: true,
                },
            },
        )),
    }
}
fn reply(decision: SessionDecision) -> SessionFrame {
    let SessionDecision::Reply(frame) = decision else {
        panic!("expected a typed reply")
    };
    frame
}
fn initialized(session: &mut MachineSession, request_id: &str) -> SessionFrame {
    let frame = reply(session.receive(offer(request_id)));
    assert!(matches!(frame.body, Body::Initialized(_)));
    frame
}
fn request(
    session_id: Option<String>,
    request_id: &str,
    context_id: &str,
    target_id: Option<String>,
) -> SessionFrame {
    SessionFrame {
        schema: 1,
        session_id,
        request_id: Some(request_id.to_owned()),
        body: Body::Request(Box::new(BodyRequest {
            context_id: context_id.to_owned(),
            target_id,
            envelope_json: "not-json-and-not-dispatched".to_owned(),
        })),
    }
}
fn refusal_reason(decision: SessionDecision) -> BodyRefusalReason {
    let frame = reply(decision);
    let Body::Refusal(body) = frame.body else {
        panic!("expected a typed refusal")
    };
    assert_eq!(body.outcome.effect_state, OutcomeEffectState::NoEffects);
    assert_eq!(body.outcome.completeness, OutcomeCompleteness::Unavailable);
    body.reason
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-SESSION-NEGOTIATION",
    r = 1
)]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-TRUSTED-CONTEXT",
    r = 1
)]
fn two_clients_get_distinct_private_contexts_and_no_false_capabilities() {
    let mut a = MachineSession::new(vec![trusted("/owned-a")]).unwrap();
    let mut b = MachineSession::new(vec![trusted("/owned-b")]).unwrap();
    let a_frame = initialized(&mut a, "init-a");
    let b_frame = initialized(&mut b, "init-b");
    let a_id = a_frame.session_id.clone().unwrap();
    let b_id = b_frame.session_id.clone().unwrap();
    assert_ne!(a_id, b_id);
    let Body::Initialized(a_body) = a_frame.body else {
        unreachable!()
    };
    let Body::Initialized(b_body) = b_frame.body else {
        unreachable!()
    };
    assert_eq!(a_body.api_version, "1");
    assert_eq!(a_body.selected_schemas, supported_schemas());
    assert_eq!(
        a_body
            .selected_schemas
            .iter()
            .map(|offer| offer.format_id.as_str())
            .collect::<Vec<_>>(),
        [
            FormatId::MachineSessionFrame.id(),
            FormatId::MachineOperationEnvelope.id(),
            FormatId::MigrationPlanRequest.id(),
            FormatId::MigrationPlanReport.id(),
            FormatId::MigrationPlanError.id(),
        ]
    );
    assert_eq!(a_body.position_encoding, POSITION_ENCODING);
    assert_eq!(a_body.legacy_mcp_observed_version, PROTOCOL_VERSION);
    assert!(!a_body.actual_capabilities.streaming);
    assert!(!a_body.actual_capabilities.replay);
    assert!(!a_body.actual_capabilities.control);
    let a_context = a_body
        .contexts
        .iter()
        .find(|c| c.kind == ContextKind::Workspace)
        .unwrap();
    let b_context = b_body
        .contexts
        .iter()
        .find(|c| c.kind == ContextKind::Workspace)
        .unwrap();
    assert_ne!(a_context.context_id, b_context.context_id);
    assert_eq!(a_context.availability, ContextAvailability::Available);
    assert_eq!(a_context.target_ids.len(), 1);
    for kind in [ContextKind::Home, ContextKind::Container] {
        let descriptor = a_body.contexts.iter().find(|c| c.kind == kind).unwrap();
        assert_eq!(descriptor.availability, ContextAvailability::Unavailable);
        assert!(descriptor.target_ids.is_empty());
    }
    let published = serde_json::to_string(&a_body).unwrap();
    assert!(!published.contains("/owned-a"));
    let SessionDecision::UnvalidatedRequest(bound) = a.receive(request(
        Some(a_id.clone()),
        "work-a",
        &a_context.context_id,
        Some(a_context.target_ids[0].clone()),
    )) else {
        panic!("assigned request did not reach the unvalidated boundary")
    };
    assert_eq!(bound.envelope_json, "not-json-and-not-dispatched");
    assert_eq!(bound.session_id, a_id);
    assert_eq!(
        bound.target_id.as_deref(),
        Some(a_context.target_ids[0].as_str())
    );
    assert_eq!(
        refusal_reason(b.receive(request(
            Some(b_id),
            "foreign-context",
            &a_context.context_id,
            None,
        ))),
        BodyRefusalReason::UnknownContext
    );
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-SESSION-NEGOTIATION",
    r = 1
)]
fn unsupported_offers_name_actual_supported_sets_without_initializing() {
    let mut session = MachineSession::new(vec![]).unwrap();
    for (change, expected) in [
        (0, BodyInitializationRefusedReason::NoCommonApiVersion),
        (1, BodyInitializationRefusedReason::NoCommonSchema),
        (2, BodyInitializationRefusedReason::NoCommonPositionEncoding),
    ] {
        let mut frame = offer(&format!("init-{change}"));
        let Body::InitializeOffer(body) = &mut frame.body else {
            unreachable!()
        };
        match change {
            0 => body.api_versions = vec!["future-only".to_owned()],
            1 => body.schema_offers = vec![supported_schemas()[0].clone()],
            _ => body.position_encodings = vec!["projected-utf16".to_owned()],
        }
        let response = reply(session.receive(frame));
        assert!(response.session_id.is_none());
        let Body::InitializationRefused(body) = response.body else {
            panic!("expected init refusal")
        };
        assert_eq!(body.reason, expected);
        assert_eq!(body.supported_api_versions, ["1"]);
        assert_eq!(body.supported_schemas, supported_schemas());
        assert_eq!(body.supported_position_encodings, [POSITION_ENCODING]);
        assert_eq!(session.phase(), SessionPhase::Uninitialized);
    }
    let mut wrong_epoch = offer("wrong-epoch");
    let Body::InitializeOffer(body) = &mut wrong_epoch.body else {
        unreachable!()
    };
    body.schema_offers[0].epoch += 1;
    let Body::InitializationRefused(body) = reply(session.receive(wrong_epoch)).body else {
        panic!("unregistered schema epoch was admitted")
    };
    assert_eq!(body.reason, BodyInitializationRefusedReason::NoCommonSchema);
    for missing in [FormatId::MigrationPlanReport, FormatId::MigrationPlanError] {
        let mut without = offer(&format!("missing-{}", missing.id()));
        let Body::InitializeOffer(body) = &mut without.body else {
            unreachable!()
        };
        body.schema_offers
            .retain(|row| row.format_id != missing.id());
        let Body::InitializationRefused(body) = reply(session.receive(without)).body else {
            panic!("missing domain result/error format was admitted")
        };
        assert_eq!(body.reason, BodyInitializationRefusedReason::NoCommonSchema);
        assert_eq!(body.supported_schemas, supported_schemas());

        let mut wrong_epoch = offer(&format!("wrong-{}-epoch", missing.id()));
        let Body::InitializeOffer(body) = &mut wrong_epoch.body else {
            unreachable!()
        };
        body.schema_offers
            .iter_mut()
            .find(|row| row.format_id == missing.id())
            .unwrap()
            .epoch += 1;
        let Body::InitializationRefused(body) = reply(session.receive(wrong_epoch)).body else {
            panic!("wrong domain result/error epoch was admitted")
        };
        assert_eq!(body.reason, BodyInitializationRefusedReason::NoCommonSchema);
        assert_eq!(body.supported_schemas, supported_schemas());
        assert_eq!(session.phase(), SessionPhase::Uninitialized);
    }
    let mut path_offer = serde_json::to_value(offer("path-in-body")).unwrap();
    path_offer["body"]["project_root"] = serde_json::json!("/not-authorized");
    assert!(serde_json::from_value::<SessionFrame>(path_offer).is_err());
    let mut bad_session = offer("bad-session");
    bad_session.session_id = Some("client-invented".to_owned());
    let Body::InitializationRefused(body) = reply(session.receive(bad_session)).body else {
        panic!("client-supplied session ID was admitted")
    };
    assert_eq!(body.reason, BodyInitializationRefusedReason::InvalidOffer);
    assert_eq!(session.phase(), SessionPhase::Uninitialized);
    let mut blank_request = offer(" ");
    blank_request.request_id = Some(" \n".to_owned());
    let Body::InitializationRefused(body) = reply(session.receive(blank_request)).body else {
        panic!("blank request ID was admitted")
    };
    assert_eq!(body.reason, BodyInitializationRefusedReason::InvalidOffer);
    assert_eq!(session.phase(), SessionPhase::Uninitialized);
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-TRUSTED-CONTEXT",
    r = 1
)]
fn preinit_foreign_unavailable_duplicate_and_closed_requests_refuse() {
    let mut session = MachineSession::new(vec![trusted("/owned")]).unwrap();
    assert_eq!(
        refusal_reason(session.receive(request(None, "preinit", "any", None))),
        BodyRefusalReason::NotInitialized
    );
    let initialized = initialized(&mut session, "init");
    let own_id = initialized.session_id.unwrap();
    let Body::Initialized(body) = initialized.body else {
        unreachable!()
    };
    let available = body
        .contexts
        .iter()
        .find(|c| c.kind == ContextKind::Workspace)
        .unwrap();
    let unavailable = body
        .contexts
        .iter()
        .find(|c| c.kind == ContextKind::Home)
        .unwrap();
    assert_eq!(
        refusal_reason(session.receive(request(
            Some("foreign".to_owned()),
            "foreign",
            &available.context_id,
            None
        ))),
        BodyRefusalReason::InvalidRequest
    );
    assert_eq!(
        refusal_reason(session.receive(request(
            Some(own_id.clone()),
            "unknown-context",
            "foreign-context",
            None
        ))),
        BodyRefusalReason::UnknownContext
    );
    assert_eq!(
        refusal_reason(session.receive(request(
            Some(own_id.clone()),
            "unavailable",
            &unavailable.context_id,
            None
        ))),
        BodyRefusalReason::Unavailable
    );
    assert_eq!(
        refusal_reason(session.receive(request(
            Some(own_id.clone()),
            "unknown-target",
            &available.context_id,
            Some("foreign-target".to_owned())
        ))),
        BodyRefusalReason::UnknownTarget
    );
    assert_eq!(
        refusal_reason(session.receive(request(
            Some(own_id.clone()),
            "unknown-target",
            &available.context_id,
            None
        ))),
        BodyRefusalReason::InvalidRequest
    );
    let Body::InitializationRefused(body) = reply(session.receive(offer("second-init"))).body
    else {
        panic!("second init was admitted")
    };
    assert_eq!(body.reason, BodyInitializationRefusedReason::InvalidOffer);
    session.close();
    assert_eq!(session.phase(), SessionPhase::Closed);
    assert_eq!(
        refusal_reason(session.receive(request(
            Some(own_id),
            "closed",
            &available.context_id,
            None
        ))),
        BodyRefusalReason::Unavailable
    );
}

#[test]
fn correlation_is_finite_and_never_reuses_duplicates() {
    let mut session = MachineSession::new(vec![trusted("/owned")]).unwrap();
    let initial = initialized(&mut session, "init");
    let Body::Initialized(body) = initial.body else {
        unreachable!()
    };
    let context = body
        .contexts
        .iter()
        .find(|c| c.availability == ContextAvailability::Available)
        .unwrap();
    for n in 1..MAX_REQUEST_IDS {
        assert!(matches!(
            session.receive(request(
                initial.session_id.clone(),
                &format!("request-{n}"),
                &context.context_id,
                None
            )),
            SessionDecision::UnvalidatedRequest(_)
        ));
    }
    assert_eq!(session.request_ids.len(), MAX_REQUEST_IDS);
    assert_eq!(
        refusal_reason(session.receive(request(
            initial.session_id.clone(),
            "request-1",
            &context.context_id,
            None
        ))),
        BodyRefusalReason::InvalidRequest
    );
    assert!(!session.correlation_exhausted());
    assert_eq!(
        refusal_reason(session.receive(request(
            initial.session_id.clone(),
            "over-limit",
            &context.context_id,
            None
        ))),
        BodyRefusalReason::InvalidRequest
    );
    assert!(session.correlation_exhausted());
    assert_eq!(session.request_ids.len(), MAX_REQUEST_IDS);
    assert!(session.request_ids.contains("request-1"));
    assert!(session.request_id_bytes <= MAX_CORRELATION_BYTES);
}

#[test]
fn oversized_request_id_cannot_initialize_or_enter_correlation_storage() {
    let mut session = MachineSession::new(vec![]).unwrap();
    let id = "x".repeat(MAX_REQUEST_ID_BYTES + 1);
    let Body::InitializationRefused(refusal) = reply(session.receive(offer(&id))).body else {
        panic!("oversized correlation admitted")
    };
    assert_eq!(
        refusal.reason,
        BodyInitializationRefusedReason::InvalidOffer
    );
    assert!(session.request_ids.is_empty());
    assert_eq!(session.request_id_bytes, 0);
    assert_eq!(session.phase(), SessionPhase::Uninitialized);
}

#[test]
fn cross_process_identity_probe() {
    if std::env::var_os("VIBE_MCP_CROSS_PROCESS_ID_PROBE").is_none() {
        return;
    }
    let mut session = MachineSession::new(vec![trusted("/owned")]).unwrap();
    let frame = initialized(&mut session, "init");
    let Body::Initialized(body) = frame.body else {
        unreachable!()
    };
    let context = body
        .contexts
        .iter()
        .find(|context| context.kind == ContextKind::Workspace)
        .unwrap();
    println!(
        "MACHINE_ID_PROBE:{}:{}:{}",
        frame.session_id.unwrap(),
        context.context_id,
        context.target_ids[0]
    );
}

#[test]
fn another_process_session_context_and_target_ids_cannot_match_current_grants() {
    let mut current = MachineSession::new(vec![trusted("/owned")]).unwrap();
    let frame = initialized(&mut current, "init");
    let current_id = frame.session_id.unwrap();
    let Body::Initialized(body) = frame.body else {
        unreachable!()
    };
    let context = body
        .contexts
        .iter()
        .find(|context| context.kind == ContextKind::Workspace)
        .unwrap();
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "machine_session::state::tests::cross_process_identity_probe",
            "--nocapture",
        ])
        .env("VIBE_MCP_CROSS_PROCESS_ID_PROBE", "1")
        .output()
        .unwrap();
    assert!(output.status.success());
    let output = String::from_utf8(output.stdout).unwrap();
    let marker = output
        .split_whitespace()
        .find_map(|word| word.strip_prefix("MACHINE_ID_PROBE:"))
        .expect("child ran the exact identity probe");
    let identities = marker.split(':').collect::<Vec<_>>();
    let [old_session, old_context, old_target] = identities.as_slice() else {
        panic!("three child identities")
    };
    assert_ne!(*old_session, current_id);
    assert_ne!(*old_context, context.context_id);
    assert_ne!(*old_target, context.target_ids[0]);
    assert_eq!(
        refusal_reason(current.receive(request(
            Some((*old_session).to_owned()),
            "old-session",
            old_context,
            Some((*old_target).to_owned()),
        ))),
        BodyRefusalReason::InvalidRequest
    );
    assert_eq!(
        refusal_reason(current.receive(request(
            Some(current_id.clone()),
            "old-context",
            old_context,
            None,
        ))),
        BodyRefusalReason::UnknownContext
    );
    assert_eq!(
        refusal_reason(current.receive(request(
            Some(current_id),
            "old-target",
            &context.context_id,
            Some((*old_target).to_owned()),
        ))),
        BodyRefusalReason::UnknownTarget
    );
}
