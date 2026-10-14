use crate::generated::{
    boot_debugger::e1::{error, progress, request, result},
    format_id::FormatId,
};
#[test]
fn strict_registered_request_and_required_nullable_carriers() {
    let valid = r#"{"schema":1,"context_id":"host","command":{"action":"tree","snapshot_id":"capture","inspection_id":null,"cursor":null,"filter":"","limit":1}}"#;
    let parsed: request::Request = serde_json::from_str(valid).unwrap();
    assert_eq!(
        serde_json::from_slice::<request::Request>(&serde_json::to_vec(&parsed).unwrap()).unwrap(),
        parsed
    );
    assert!(
        serde_json::from_str::<request::Request>(&valid.replace("\"cursor\":null,", "")).is_err()
    );
    assert!(
        serde_json::from_str::<request::Request>(
            &valid.replace("\"limit\":1", "\"limit\":1,\"foreign\":true")
        )
        .is_err()
    );
    for id in [
        "boot-debugger-request",
        "boot-debugger-result",
        "boot-debugger-error",
        "boot-debugger-progress",
    ] {
        assert!(FormatId::ALL.iter().any(|f| f.id() == id && f.epoch() == 1));
    }
}
#[test]
fn canonical_result_error_progress_reject_unknown_fields() {
    let result: result::Result = serde_json::from_str(r#"{"schema":1,"context_id":"host","snapshot_id":"snapshot","result_id":"result","body":{"kind":"explain","chain":[0],"unavailable":null}}"#).unwrap();
    assert_eq!(
        serde_json::from_slice::<result::Result>(&serde_json::to_vec(&result).unwrap()).unwrap(),
        result
    );
    let error: error::Error = serde_json::from_str(
        r#"{"schema":1,"context_id":null,"code":"wrong_context","message":"foreign"}"#,
    )
    .unwrap();
    assert_eq!(
        serde_json::from_slice::<error::Error>(&serde_json::to_vec(&error).unwrap()).unwrap(),
        error
    );
    let progress: progress::Progress = serde_json::from_str(r#"{"schema":1,"context_id":"host","snapshot_id":"snapshot","inspection_id":"inspection","stream_format_id":"machine-operation-stream-message","stream_json":"{}"}"#).unwrap();
    let mut value = serde_json::to_value(progress).unwrap();
    value["foreign"] = true.into();
    assert!(serde_json::from_value::<progress::Progress>(value).is_err());
}

#[test]
fn captured_emitted_positions_and_source_identity_are_strict_and_nullable() {
    let node = serde_json::json!({"contribution":0,"occurrence":0,"source":1,"path":"a.md","origin":"pkg","requested":null,"parent":null,"root":true,"artifact_lines":null});
    let parsed: result::Node = serde_json::from_value(node.clone()).unwrap();
    assert!(parsed.artifact_lines.is_none());
    let mut missing = node.clone();
    missing.as_object_mut().unwrap().remove("artifact_lines");
    assert!(serde_json::from_value::<result::Node>(missing).is_err());
    let span = serde_json::json!({"basis":"captured_emitted_artifact_one_based_inclusive_lines","snapshot_id":"s","resource_id":"s:resource-1","digest":"sha256:captured","start_line":"2","end_line":"5"});
    let mut positioned = node;
    positioned["artifact_lines"] = span;
    let parsed: result::Node = serde_json::from_value(positioned.clone()).unwrap();
    assert_eq!(serde_json::to_value(parsed).unwrap(), positioned);
    positioned["artifact_lines"]["basis"] = "authored_xml_lines".into();
    assert!(serde_json::from_value::<result::Node>(positioned).is_err());
    let source = serde_json::json!({"source":1,"identity":{"kind":"spec","address":"spec://org.test/pkg/doc"},"snapshot_id":"s","artifact_resource_id":"s:resource-1","artifact_digest":"sha256:captured","authored_resource_id":null});
    let parsed: result::Source = serde_json::from_value(source.clone()).unwrap();
    assert_eq!(serde_json::to_value(parsed).unwrap(), source);
}
