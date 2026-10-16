use super::*;
#[test]
fn strict_action_reply_and_retained_prefix_corpus() {
    for action in [
        "plan", "apply", "verify", "update", "recover", "remove", "inspect", "cancel",
    ] {
        let q = serde_json::json!({"schema":1,"action":action,"plan_id":"plan","key":"key"});
        assert!(read_request(&serde_json::to_vec(&q).unwrap()).is_ok());
    }
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../formats/corpora/client-deploy/e1/valid");
    for name in ["plan", "apply", "recover"] {
        assert!(read_request(&std::fs::read(root.join(format!("{name}.json"))).unwrap()).is_ok())
    }
    for name in ["partial", "authority-required"] {
        assert!(read_reply(&std::fs::read(root.join(format!("{name}.json"))).unwrap()).is_ok())
    }
    let state = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../formats/corpora/deploy-client-state/e1/valid/retained-prefix.json");
    let bytes = std::fs::read(state).unwrap();
    read_state(&bytes).unwrap();
    let mut basis: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    basis["published"] = false.into();
    basis["finalization_digest"] = "a".repeat(64).into();
    basis["finalization_generation"] = 1.into();
    read_state(&serde_json::to_vec(&basis).unwrap()).unwrap();
    for field in ["finalization_digest", "finalization_generation"] {
        let mut broken = basis.clone();
        broken.as_object_mut().unwrap().remove(field);
        assert!(read_state(&serde_json::to_vec(&broken).unwrap()).is_err());
    }
    basis["published"] = true.into();
    assert!(read_state(&serde_json::to_vec(&basis).unwrap()).is_err());
}
#[test]
fn unknown_identity_and_forged_completed_partial_refuse() {
    let q = br#"{"schema":1,"action":"shell","plan_id":"p","key":"k","grant":true}"#;
    assert!(read_request(q).is_err());
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../formats/corpora/client-deploy/e1");
    assert!(
        read_request(&std::fs::read(root.join("invalid/foreign-operation.json")).unwrap()).is_err()
    );
    let mut r: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("valid/partial.json")).unwrap()).unwrap();
    r["status"] = "completed".into();
    assert!(read_reply(&serde_json::to_vec(&r).unwrap()).is_err());
    let state = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../formats/corpora/deploy-client-state/e1/invalid/changed-subject.json");
    assert!(read_state(&std::fs::read(state).unwrap()).is_err());
}
