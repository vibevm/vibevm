use super::*;
const OMITTED: &[u8] = include_bytes!(
    "../../../../../formats/corpora/consumer-configuration-request/e1/valid/omitted.json"
);
#[test]
fn result_shared_instance_rejects_inconsistent_configuration_or_package() {
    let request = decode_request(OMITTED).unwrap();
    let edge = serde_json::to_value(&request.edges[0]).unwrap();
    let outcome = serde_json::json!({
        "representative": 0, "status": "resolved", "active_features": [],
        "selected": {}, "needs": [], "conflicting_edges": [], "defaults_used": []
    });
    let mut result = serde_json::json!({
        "epoch": "1", "edges": [edge.clone(), edge],
        "outcomes": [outcome.clone(), outcome]
    });
    result["edges"][1]["consumer"] = serde_json::json!("consumer-b");
    result["outcomes"][1]["representative"] = serde_json::json!(1);
    for field in ["configuration", "package"] {
        let mut conflicting = result.clone();
        if field == "configuration" {
            conflicting["edges"][1][field] = serde_json::json!("config-b");
        } else {
            conflicting["edges"][1][field]["package"] = serde_json::json!("other-package");
        }
        // Different domains may independently reuse an instance name.
        let mut independent = conflicting.clone();
        independent["edges"][1]["domain"]["identity"] = serde_json::json!("other-domain");
        decode_result(&serde_json::to_vec(&independent).unwrap()).unwrap();

        let error = decode_result(&serde_json::to_vec(&conflicting).unwrap()).unwrap_err();
        assert!(error.contains("configuration/package"), "{field}: {error}");
        assert!(error.contains("edges 0 and 1"), "{field}: {error}");
    }
}
#[test]
fn strict_request_corpus_and_opaque_identity() {
    for bytes in [
        OMITTED,
        include_bytes!(
            "../../../../../formats/corpora/consumer-configuration-request/e1/valid/explicit-empty.json"
        ),
        include_bytes!(
            "../../../../../formats/corpora/consumer-configuration-request/e1/valid/dormant-native-version.json"
        ),
    ] {
        let request = decode_request(bytes).unwrap();
        let core = admit_request(&request).unwrap();
        assert_eq!(
            decode_request(&serde_json::to_vec(&request).unwrap()).unwrap(),
            request
        );
        assert!(core[0].instance.domain.lock.is_none());
        assert_eq!(
            core[0].instance.domain.definition.producer().as_str(),
            "definition-owner"
        );
        assert!(
            matches!(&core[0].instance.package,core::PackageIdentity::Native{version,..} if version=="2:1.0~rc1-4+vendor")
        );
    }
    for bytes in [include_bytes!("../../../../../formats/corpora/consumer-configuration-request/e1/invalid/unknown-epoch.json").as_slice(), include_bytes!("../../../../../formats/corpora/consumer-configuration-request/e1/invalid/foreign-guard.json")] { assert!(decode_request(bytes).is_err()); }
}
#[test]
fn omission_empty_and_null_are_not_conflated() {
    let mut value: serde_json::Value = serde_json::from_slice(OMITTED).unwrap();
    let omitted = admit_request(&decode_request(OMITTED).unwrap()).unwrap();
    assert!(matches!(
        omitted[0].selections.values().next().unwrap().selection,
        core::Selection::Omitted
    ));
    value["edges"][0]["selections"]["choice"]["members"] = serde_json::json!([]);
    let request = decode_request(&serde_json::to_vec(&value).unwrap()).unwrap();
    assert!(
        matches!(&admit_request(&request).unwrap()[0].selections.values().next().unwrap().selection,core::Selection::Explicit(m) if m.is_empty())
    );
    value["edges"][0]["selections"]["choice"]["members"] = serde_json::Value::Null;
    assert!(decode_request(&serde_json::to_vec(&value).unwrap()).is_err());
}
#[test]
fn result_corpus_and_invalid_provenance_refuse() {
    for bytes in [include_bytes!("../../../../../formats/corpora/consumer-configuration-result/e1/valid/needs-selection.json").as_slice(),include_bytes!("../../../../../formats/corpora/consumer-configuration-result/e1/valid/resolved-provenance.json"),include_bytes!("../../../../../formats/corpora/consumer-configuration-result/e1/valid/exact-conflict.json")] {
        let result=decode_result(bytes).unwrap(); assert_eq!(decode_result(&serde_json::to_vec(&result).unwrap()).unwrap(),result);
    }
    let mut result = decode_result(include_bytes!(
        "../../../../../formats/corpora/consumer-configuration-result/e1/valid/exact-conflict.json"
    ))
    .unwrap();
    result.outcomes[0].conflicting_edges = vec![0, 99];
    assert!(admit_result(&result).is_err());
    result.outcomes[0].conflicting_edges = vec![0, 0];
    assert!(admit_result(&result).is_err());
}
