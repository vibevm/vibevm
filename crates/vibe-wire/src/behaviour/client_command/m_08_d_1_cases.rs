use super::*;
const INSTALL: &[u8] = include_bytes!(
    "../../../../../formats/corpora/client-command-request/e1/valid/fixed-install.json"
);
#[test]
fn strict_corpus_and_roundtrip() {
    for bytes in [
        INSTALL,
        include_bytes!(
            "../../../../../formats/corpora/client-command-request/e1/valid/quoted-path.json"
        ),
    ] {
        let q = read_request(bytes).unwrap();
        assert_eq!(
            serde_json::to_value(&q).unwrap(),
            serde_json::to_value(read_request(&serde_json::to_vec(&q).unwrap()).unwrap()).unwrap()
        );
    }
    for bytes in [
        include_bytes!(
            "../../../../../formats/corpora/client-command-request/e1/invalid/arbitrary-shell.json"
        )
        .as_slice(),
        include_bytes!(
            "../../../../../formats/corpora/client-command-request/e1/invalid/unknown-operation.json"
        ),
    ] {
        assert!(read_request(bytes).is_err());
    }
    let p = read_plan(include_bytes!(
        "../../../../../formats/corpora/client-command-plan/e1/valid/bound-plan.json"
    ))
    .unwrap();
    assert!(read_plan(&serde_json::to_vec(&p).unwrap()).is_ok());
    assert!(
        read_plan(include_bytes!(
            "../../../../../formats/corpora/client-command-plan/e1/invalid/missing-executable.json"
        ))
        .is_err()
    );
}
#[test]
fn inert_grammars_reject_shell_native_and_unknown_epochs() {
    let q: serde_json::Value = serde_json::from_slice(INSTALL).unwrap();
    for (key, value) in [
        ("schema", serde_json::json!(2)),
        ("contract_version", serde_json::json!(2)),
        ("operation", serde_json::json!("native")),
        ("client", serde_json::json!("unknown")),
        ("argv", serde_json::json!(["sh", "-c", "evil"])),
        ("coordinate", serde_json::json!("--eval=evil")),
    ] {
        let mut bad = q.clone();
        bad[key] = value;
        assert!(read_request(&serde_json::to_vec(&bad).unwrap()).is_err());
    }
    assert!(read_request(br#"{"schema":1,"schema":1}"#).is_err());
    assert!(crate::behaviour::adapter_plan::read_request(INSTALL).is_err());
}
