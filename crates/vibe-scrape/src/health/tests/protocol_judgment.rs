#[test]
fn protocol_rejects_duplicate_keys_ids_and_contradictory_status() {
    let duplicate_key =
        br#"{"protocol":1,"protocol":1,"status":"pass","summary":"ok","findings":[],"metrics":{}}"#;
    assert!(parse_health_result(duplicate_key, 1024).is_err());
    let duplicate_id = br#"{"protocol":1,"status":"warn","summary":"x","findings":[{"id":"x","severity":"warning","message":"a"},{"id":"x","severity":"warning","message":"b"}],"metrics":{}}"#;
    assert!(parse_health_result(duplicate_id, 1024).is_err());
    let contradiction = br#"{"protocol":1,"status":"pass","summary":"x","findings":[{"id":"x","severity":"warning","message":"a"}],"metrics":{}}"#;
    assert!(parse_health_result(contradiction, 1024).is_err());
    let valid = br#"{"protocol":1,"status":"fail","summary":"x","findings":[{"id":"x","severity":"error","message":"a"}],"metrics":{"tests":2}}"#;
    assert_eq!(
        parse_health_result(valid, 1024).unwrap().status,
        HealthStatus::Fail
    );
}

#[test]
fn bounded_stream_keeps_full_digest_and_split_utf8_state() {
    let mut accumulator = StreamAccumulator::new(5);
    accumulator.push(b"ab\xf0\x9f").unwrap();
    accumulator.push(b"\x98\x80cdef").unwrap();
    let evidence = accumulator.finish();
    assert_eq!(evidence.total_bytes, 10);
    assert!(evidence.truncated);
    assert!(evidence.redacted);
    assert!(evidence.head.is_empty());
    assert!(evidence.tail.is_empty());
    assert!(evidence.redacted);
    assert_eq!(evidence.utf8, Utf8State::Valid);
    let (out, err) = drain_concurrently(
        Cursor::new(vec![b'x'; 64]),
        Cursor::new(vec![0xff; 64]),
        4,
        4,
    )
    .unwrap();
    assert_eq!(out.total_bytes, 64);
    assert_eq!(err.utf8, Utf8State::Invalid);
}

#[test]
fn stream_evidence_never_persists_secret_excerpts() {
    let secret = b"token=super-secret-value";
    let mut accumulator = StreamAccumulator::new(1024);
    accumulator.push(secret).unwrap();
    let evidence = accumulator.finish();
    assert_eq!(evidence.total_bytes, secret.len() as u64);
    assert_eq!(
        evidence.sha256,
        format!("sha256:{:x}", Sha256::digest(secret))
    );
    assert!(evidence.head.is_empty());
    assert!(evidence.tail.is_empty());
}

fn finding(id: &str, severity: Severity) -> Finding {
    Finding {
        id: id.to_owned(),
        severity,
        message: String::new(),
        evidence: None,
    }
}

fn structured(status: HealthStatus, findings: Vec<Finding>) -> CheckState {
    CheckState::Completed(HealthVerdict::Structured(StructuredVerdict {
        status,
        summary: String::new(),
        findings,
        metrics: BTreeMap::new(),
    }))
}

fn phase(phase: HealthPhase, state: CheckState) -> PhaseHealthResult {
    PhaseHealthResult {
        phase,
        plan_id: "sha256:plan".to_owned(),
        checks: vec![CheckResult {
            id: "domain".to_owned(),
            state,
            commands: Vec::new(),
        }],
        assurance_reduced: false,
    }
}

#[test]
fn no_regression_accepts_subset_and_rejects_new_or_worse_finding() {
    let before = phase(
        HealthPhase::Before,
        structured(
            HealthStatus::Fail,
            vec![
                finding("a", Severity::Error),
                finding("b", Severity::Warning),
            ],
        ),
    );
    let improved = phase(
        HealthPhase::After,
        structured(HealthStatus::Warn, vec![finding("b", Severity::Warning)]),
    );
    assert_eq!(
        judge(BaselinePolicy::NoRegression, &before, &improved),
        BaselineDecision::AcceptReduced
    );
    let worse = phase(
        HealthPhase::After,
        structured(
            HealthStatus::Fail,
            vec![
                finding("b", Severity::Error),
                finding("new", Severity::Info),
            ],
        ),
    );
    assert_eq!(
        judge(BaselinePolicy::NoRegression, &before, &worse),
        BaselineDecision::RollbackAfter
    );
}

#[test]
fn strict_refuses_a_red_before_and_rolls_back_a_red_after() {
    let red_before = phase(
        HealthPhase::Before,
        structured(
            HealthStatus::Warn,
            vec![finding("existing", Severity::Warning)],
        ),
    );
    let green_after = phase(
        HealthPhase::After,
        structured(HealthStatus::Pass, Vec::new()),
    );
    assert_eq!(
        judge(BaselinePolicy::Strict, &red_before, &green_after),
        BaselineDecision::RefuseBefore
    );

    let green_before = phase(
        HealthPhase::Before,
        structured(HealthStatus::Pass, Vec::new()),
    );
    let red_after = phase(
        HealthPhase::After,
        structured(HealthStatus::Fail, vec![finding("new", Severity::Error)]),
    );
    assert_eq!(
        judge(BaselinePolicy::Strict, &green_before, &red_after),
        BaselineDecision::RollbackAfter
    );
}
