use super::*;
#[test]
fn request_is_closed_and_epoch_bound_before_context() {
    let base = json!({"schema":1,"mode":"show","id":"chg-0123456789abcdef0123456789abcdef"});
    assert!(read_request(&base).is_ok());
    let corpus: Value = serde_json::from_slice(include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../formats/corpora/change-query-request/e1/valid.json"
    )))
    .unwrap();
    assert!(read_request(&corpus).is_ok());
    for (raw, code) in [
        (
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../formats/corpora/change-query-request/e1/unknown-field.json"
            ))
            .as_slice(),
            ResponseRefusedCode::InvalidRequest,
        ),
        (
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../formats/corpora/change-query-request/e1/unknown-epoch.json"
            ))
            .as_slice(),
            ResponseRefusedCode::UnknownEpoch,
        ),
    ] {
        let value: Value = serde_json::from_slice(raw).unwrap();
        let Response::Refused(error) = read_request(&value).unwrap_err() else {
            panic!("wrong channel")
        };
        assert_eq!(error.code, code);
    }
    for (name, value, code) in [
        (
            "root",
            json!("/elsewhere"),
            ResponseRefusedCode::InvalidRequest,
        ),
        ("schema", json!(99), ResponseRefusedCode::UnknownEpoch),
        (
            "roles",
            json!(["intent", "intent"]),
            ResponseRefusedCode::InvalidRequest,
        ),
        (
            "roles",
            json!(["foreign"]),
            ResponseRefusedCode::InvalidRequest,
        ),
    ] {
        let mut row = base.clone();
        row[name] = value;
        let Response::Refused(error) = read_request(&row).unwrap_err() else {
            panic!("wrong channel")
        };
        assert_eq!(error.code, code);
    }
}

#[test]
fn actual_response_reader_preserves_incomplete_and_refuses_false_complete() {
    let good: Value = serde_json::from_slice(include_bytes!(
        "../../../../../formats/corpora/change-query-response/e1/useful-incomplete.json"
    ))
    .unwrap();
    let response = read_response(&good).unwrap();
    assert!(matches!(response, Response::Success(_)));
    let refusal: Value = serde_json::from_slice(include_bytes!(
        "../../../../../formats/corpora/change-query-response/e1/refused.json"
    ))
    .unwrap();
    assert!(matches!(
        read_response(&refusal).unwrap(),
        Response::Refused(_)
    ));
    let wrong: Value = serde_json::from_slice(include_bytes!(
        "../../../../../formats/corpora/change-query-response/e1/false-complete.json"
    ))
    .unwrap();
    assert_eq!(
        read_response(&wrong).unwrap_err(),
        "incomplete change scope reported complete"
    );
}

#[test]
fn generated_corpus_preserves_coverage_axes_and_closed_response_shape() {
    for raw in [
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../formats/corpora/change-query-response/e1/complete-check.json"
        ))
        .as_slice(),
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../formats/corpora/change-query-response/e1/complete-show.json"
        ))
        .as_slice(),
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../formats/corpora/change-query-response/e1/complete-coverage.json"
        ))
        .as_slice(),
    ] {
        let value: Value = serde_json::from_slice(raw).unwrap();
        let response = read_response(&value).unwrap();
        assert_eq!(serde_json::to_value(response).unwrap(), value);
        assert_eq!(value["scope_complete"], true);
        assert_eq!(value["terminal"], "not-evaluated");
        assert_eq!(value["artifact_evidence"], "not-observed");
        for (key, wrong) in [
            ("schema", json!(99)),
            ("unknown", json!(true)),
            ("terminal", json!("satisfied")),
        ] {
            let mut malformed = value.clone();
            malformed[key] = wrong;
            let error = read_response(&malformed).unwrap_err();
            match key {
                "schema" => assert_eq!(error, "change result identity"),
                "unknown" => assert!(error.contains("unknown field `unknown`")),
                _ => assert!(error.contains("satisfied")),
            };
        }
        for missing in ["binding", "capture"] {
            let mut malformed = value.clone();
            if missing == "binding" {
                malformed["origins"][0]["declared_content"] = Value::Null;
                malformed["origins"][0]["observed_content"] = Value::Null;
            } else {
                malformed["origins"][0]["capture"] = json!("");
            }
            assert_eq!(
                read_response(&malformed).unwrap_err(),
                "matched origin inventory mismatch"
            );
        }
        let mut malformed = value.clone();
        malformed["origins"][0]["observed_content"] =
            json!("sha256:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff");
        assert_eq!(
            read_response(&malformed).unwrap_err(),
            "matched origin inventory mismatch"
        );
    }
}
