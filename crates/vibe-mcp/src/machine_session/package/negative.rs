use super::*;

#[test]
fn explicit_grant_and_matching_root_are_required_and_package_offers_are_exact() {
    let unbound = tempfile::tempdir().unwrap();
    assert!(matches!(
        PackageMachineSessionRunner::new(ServerContext::with_store_root(unbound.path(), "unused")),
        Err(PackageSessionBuildError::UnavailableHost)
    ));
    assert!(!unbound.path().join(".vibe").exists());
    let fixture = Fixture::new();
    let route = fixture.route();
    assert!(
        ServerContext::with_store_root(unbound.path(), "unused")
            .with_fixed_package_route(route.clone())
            .is_err()
    );
    let mut altered = fixture.context(route.clone());
    altered.project_root = unbound.path().into();
    assert!(matches!(
        PackageMachineSessionRunner::new(altered),
        Err(PackageSessionBuildError::Root(_))
    ));
    let mut runner = PackageMachineSessionRunner::new(fixture.context(route)).unwrap();
    for missing in FORMATS {
        let mut frame = offer();
        let f::Body::InitializeOffer(body) = &mut frame.body else {
            unreachable!()
        };
        body.schema_offers
            .retain(|row| row.format_id != missing.id());
        let f::Body::InitializationRefused(refusal) = send(&mut runner, &frame).body else {
            panic!("refusal")
        };
        assert_eq!(
            refusal.reason,
            f::BodyInitializationRefusedReason::NoCommonSchema
        );
        assert_eq!(refusal.supported_schemas.len(), 5);
        let mut frame = offer();
        let f::Body::InitializeOffer(body) = &mut frame.body else {
            unreachable!()
        };
        body.schema_offers
            .iter_mut()
            .find(|row| row.format_id == missing.id())
            .unwrap()
            .epoch = 2;
        assert!(matches!(
            send(&mut runner, &frame).body,
            f::Body::InitializationRefused(_)
        ));
    }
    let mut offered = offer();
    let f::Body::InitializeOffer(body) = &mut offered.body else {
        unreachable!()
    };
    body.schema_offers.push(f::SchemaOffer {
        format_id: FormatId::MachineLifecycleActionRequest.id().into(),
        epoch: 1,
    });
    let f::Body::Initialized(body) = send(&mut runner, &offered).body else {
        panic!("initialized")
    };
    assert_eq!(body.selected_schemas.len(), 5);
    assert!(
        !body
            .selected_schemas
            .iter()
            .any(|row| row.format_id == FormatId::MachineLifecycleActionRequest.id())
    );
    fixture.no_copy();
}

#[test]
fn foreign_stale_session_context_target_and_missing_target_refuse_before_copy() {
    let fixture = Fixture::new();
    let route = fixture.route();
    let mut a = fixture.client(route.clone());
    let b = fixture.client(route.clone());
    let mut cold = fixture.client(route);
    assert_ne!(a.session, b.session);
    assert_ne!(a.context, b.context);
    assert_ne!(a.target, b.target);
    let ids = (a.session.clone(), a.context.clone(), a.target.clone());
    for which in 0..4 {
        let mut frame = a.frame(envelope("submit", "copy").to_string());
        let f::Body::Request(body) = &mut frame.body else {
            unreachable!()
        };
        match which {
            0 => frame.session_id = Some(b.session.clone()),
            1 => body.context_id = b.context.clone(),
            2 => body.target_id = Some(b.target.clone()),
            _ => body.target_id = None,
        }
        let f::Body::Refusal(refusal) = send(&mut a.runner, &frame).body else {
            panic!("refusal")
        };
        assert!(refusal.envelope_json.is_none());
        assert_eq!(
            refusal.outcome.effect_state,
            f::OutcomeEffectState::NoEffects
        );
    }
    drop(a);
    drop(b);
    for which in 0..3 {
        let mut frame = cold.frame(envelope("submit", "copy").to_string());
        let f::Body::Request(body) = &mut frame.body else {
            unreachable!()
        };
        match which {
            0 => frame.session_id = Some(ids.0.clone()),
            1 => body.context_id = ids.1.clone(),
            _ => body.target_id = Some(ids.2.clone()),
        }
        assert!(matches!(
            send(&mut cold.runner, &frame).body,
            f::Body::Refusal(_)
        ));
    }
    fixture.no_copy();
}

#[test]
fn wrong_envelope_identity_format_nested_values_epoch_and_authority_refuse_before_copy() {
    let fixture = Fixture::new();
    let mut client = fixture.client(fixture.route());
    let good = envelope("submit", "copy");
    let mut cases = vec![json!({}), json!(null)];
    for (field, value) in [
        ("schema", json!(2)),
        ("operation_id", json!("foreign")),
        ("domain_owner", json!("foreign")),
        ("source_id", json!("caller")),
        ("evidence_id", json!("grant")),
        ("root", json!("/tmp")),
    ] {
        let mut value_case = good.clone();
        value_case[field] = value;
        cases.push(value_case);
    }
    for field in ["operation_id", "domain_owner", "source_id", "evidence_id"] {
        let mut value = good.clone();
        value.as_object_mut().unwrap().remove(field);
        cases.push(value);
    }
    for format in [
        "unknown",
        "machine-lifecycle-action-request",
        "machine-fixed-package-action-reply",
        "machine-fixed-package-action-error",
    ] {
        let mut value = good.clone();
        value["body"]["format_id"] = json!(format);
        cases.push(value);
    }
    let mut value = good.clone();
    value["body"] = json!({"stage":"admission","decision":"admitted","reason":null,"authority_evidence_id":"yes"});
    cases.push(value);
    for action in [
        json!({"kind":"execute","key":"copy"}),
        json!({"kind":"submit"}),
        json!({"kind":"submit","key":false}),
        json!({"kind":"submit","key":null}),
        json!({"kind":"control","key":"copy"}),
        json!({"kind":"replay","key":"copy"}),
    ] {
        let mut value = good.clone();
        value["body"]["payload_json"] = json!(json!({"schema":1,"action":action}).to_string());
        cases.push(value);
    }
    for key in [
        "".to_string(),
        " ".into(),
        " copy".into(),
        "copy\n".into(),
        "x".repeat(257),
        "é".repeat(129),
    ] {
        cases.push(envelope("submit", &key));
    }
    for field in [
        "root",
        "namespace",
        "scope",
        "provider",
        "target",
        "destination",
        "HOME",
        "created_at",
        "assume_yes",
    ] {
        for nested in [false, true] {
            let mut payload = json!({"schema":1,"action":{"kind":"submit","key":"copy"}});
            if nested {
                payload["action"][field] = json!("caller");
            } else {
                payload[field] = json!("caller");
            }
            let mut value = good.clone();
            value["body"]["payload_json"] = json!(payload.to_string());
            cases.push(value);
        }
    }
    for payload in [
        json!({"schema":2,"action":{"kind":"submit","key":"copy"}}).to_string(),
        "{}".into(),
        "yes".into(),
        " ".repeat(128 * 1024 + 1),
    ] {
        let mut value = good.clone();
        value["body"]["payload_json"] = json!(payload);
        cases.push(value);
    }
    assert_eq!(cases.len(), 51);
    for value in cases {
        let publication = client.raw(value.to_string());
        assert_eq!(
            publication.error().code,
            err::FixedPackageActionErrorCode::InvalidRequest
        );
        assert_eq!(
            publication.outcome.effect_state,
            f::OutcomeEffectState::NoEffects
        );
        assert!(publication.outcome.provenance_id.is_none());
    }
    assert_eq!(
        client.raw("x".repeat(256 * 1024 + 1)).error().code,
        err::FixedPackageActionErrorCode::InvalidRequest
    );
    fixture.no_copy();
}

#[test]
fn malformed_over_limit_frames_close_before_dispatch_or_further_read() {
    let fixture = Fixture::new();
    let mut raw_frames = vec![
        vec![0, 0],
        vec![0, 0, 0, 0],
        vec![0, 0, 0, 8, b'{'],
        ((crate::machine_session::framing::MAX_FRAME_BYTES + 1) as u32)
            .to_be_bytes()
            .to_vec(),
    ];
    for body in [vec![0xff], b"not-json".to_vec(), b"{}".to_vec(), serde_json::to_vec(&json!({"schema":2,"session_id":null,"request_id":"init","body":{"kind":"initialize_offer","api_versions":["1"],"position_encodings":[],"schema_offers":[],"requested_capabilities":{"streaming":false,"replay":false,"control":false}}})).unwrap()] {
        let mut bytes = (body.len() as u32).to_be_bytes().to_vec(); bytes.extend(body); raw_frames.push(bytes);
    }
    let mut shape = serde_json::to_value(offer()).unwrap();
    shape["body"]["root"] = json!("/tmp");
    let body = serde_json::to_vec(&shape).unwrap();
    let mut bytes = (body.len() as u32).to_be_bytes().to_vec();
    bytes.extend(body);
    raw_frames.push(bytes);
    assert_eq!(raw_frames.len(), 9);
    for bytes in raw_frames {
        let mut runner =
            PackageMachineSessionRunner::new(fixture.context(fixture.route())).unwrap();
        let mut output = Vec::new();
        assert!(matches!(
            runner.step(&mut Cursor::new(bytes), &mut output),
            Err(PackageRunnerError::Frame(_))
        ));
        assert!(output.is_empty());
        let mut next = Vec::new();
        write_frame(&mut next, &offer()).unwrap();
        let mut next = Cursor::new(next);
        assert_eq!(
            runner.step(&mut next, &mut output).unwrap(),
            RunnerStep::Closed
        );
        assert_eq!(next.position(), 0);
    }
    fixture.no_copy();
}

#[test]
fn physical_root_replacement_refuses_at_dispatch_grant_and_construction() {
    let fixture = Fixture::new();
    let route = fixture.route();
    let granted = fixture.context(route.clone());
    let mut client = fixture.client(route.clone());
    let moved = fixture.dir.path().with_extension("moved");
    // Keep moved root inside another owned temporary directory for cleanup.
    let parent = tempfile::tempdir().unwrap();
    let moved = parent.path().join(moved.file_name().unwrap());
    fs::rename(fixture.dir.path(), &moved).unwrap();
    fs::create_dir(fixture.dir.path()).unwrap();
    fs::write(fixture.dir.path().join("source.bin"), b"replacement").unwrap();
    assert!(fixture.context_for_foreign_root(&route).is_err());
    assert!(matches!(
        PackageMachineSessionRunner::new(granted),
        Err(PackageSessionBuildError::Root(_))
    ));
    let refusal = client.call("submit", "copy");
    assert_eq!(
        refusal.error().code,
        err::FixedPackageActionErrorCode::ContextRootMismatch
    );
    assert_eq!(
        refusal.outcome.effect_state,
        f::OutcomeEffectState::NoEffects
    );
    assert!(!fixture.dir.path().join("target").exists() && !moved.join("target").exists());
}
impl Fixture {
    fn context_for_foreign_root(
        &self,
        route: &Arc<FixedPackageRoute>,
    ) -> Result<ServerContext, FixedPackageHostError> {
        ServerContext::with_store_root(self.dir.path(), "unused")
            .with_fixed_package_route(route.clone())
    }
}
