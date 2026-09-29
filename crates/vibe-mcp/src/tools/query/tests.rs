use super::*;

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-OPERATION-VALIDATION",
    r = 1
)]
fn malformed_advertised_arguments_refuse_before_trace_build() {
    let absent_project = tempfile::tempdir().unwrap().path().join("no-project");
    let ctx = ServerContext::with_store_root(&absent_project, &absent_project);
    for args in [
        Value::Null,
        json!([]),
        json!({"uri": null}),
        json!({"uri": 7}),
        json!({"symbol": false}),
        json!({"kind": []}),
        json!({"limit": null}),
        json!({"limit": "50"}),
        json!({"limit": 3.0}),
        json!({"limit": -1}),
        json!({"limit": 0}),
        json!({"limit": 201}),
        json!({"limit": u64::MAX}),
        json!({"extra": true}),
    ] {
        assert!(
            matches!(
                QueryMcpTool.run(&args, &ctx),
                Err(ToolError::InvalidArguments(_))
            ),
            "malformed query reached trace owner: {args}"
        );
    }
    assert!(!absent_project.exists());
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-OPERATION-VALIDATION",
    r = 1
)]
fn valid_omission_and_filters_keep_exact_defaults_without_io() {
    let default = parse_query_args(&json!({})).unwrap();
    assert_eq!(default.limit, vibe_trace::search::DEFAULT_LIMIT);
    assert!(default.uri.is_none() && default.symbol.is_none() && default.kind.is_none());

    let selected = parse_query_args(&json!({
        "uri": "spec://org.example/core/PROP-001#root",
        "symbol": "package::resolve",
        "kind": "fn",
        "limit": 200
    }))
    .unwrap();
    assert_eq!(
        selected.uri.as_deref(),
        Some("spec://org.example/core/PROP-001#root")
    );
    assert_eq!(selected.symbol.as_deref(), Some("package::resolve"));
    assert_eq!(selected.kind.as_deref(), Some("fn"));
    assert_eq!(selected.limit, vibe_trace::search::MAX_LIMIT);
}
