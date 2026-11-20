//! Strict ordinary-only preview shapes preserve false, absence and empties.
use serde_json::json;
use vibe_wire::behaviour::ordinary_configuration_preview::parse_request;
use vibe_wire::generated::ordinary_configuration_preview_result::OrdinaryConfigurationPreviewResult as Report;

#[test]
fn request_preserves_false_and_refuses_foreign_shapes() {
    let cold = json!({"package":"org.example/demo"});
    let parsed = parse_request(cold.clone()).unwrap();
    assert!(parsed.input.is_none());
    assert_eq!(serde_json::to_value(parsed).unwrap(), cold);
    let answered = json!({"package":"org.example/demo","input":{"selections":{"mode":"small"},"parameters":{"verbose":false}}});
    let parsed = parse_request(answered.clone()).unwrap();
    assert!(!parsed.input.as_ref().unwrap().parameters["verbose"]);
    assert_eq!(serde_json::to_value(parsed).unwrap(), answered);
    for invalid in [
        json!({"package":"org.example/demo","source":"/outside"}),
        json!({"package":"org.example/demo","input":null}),
        json!({"package":"org.example/demo","input":{"selections":{},"parameters":{"verbose":"false"}}}),
        json!({"package":"org.example/demo","input":{"selections":{},"parameters":{},"extra":true}}),
        json!({"protocol":"vibe-configuration-input/1","input":{}}),
    ] {
        assert!(parse_request(invalid).is_err());
    }
}
#[test]
fn report_preserves_empty_questions_and_closed_vocabularies() {
    let value = json!({"protocol":"vibe-ordinary-configuration-preview/1","package":"org.example/demo","version":"1.0.0","source":{"registry":"local","content_hash":"sha256:synthetic"},"description":{"choices":[],"parameters":[]},"questions":[],"original":{"selections":{},"parameters":{"verbose":false}},"resolved":{"features":[],"parameters":{"verbose":false}}});
    let report: Report = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(serde_json::to_value(report).unwrap(), value);
    for invalid in [
        json!({"id":"verbose","kind":"secret","members":[]}),
        json!({"id":"verbose","kind":"parameter","members":[],"extra":true}),
    ] {
        let mut report = value.clone();
        report["questions"] = json!([invalid]);
        assert!(serde_json::from_value::<Report>(report).is_err());
    }
    let mut invalid = value;
    invalid["description"]["parameters"] =
        json!([{"id":"x","name":"X","required":true,"type":"string"}]);
    assert!(serde_json::from_value::<Report>(invalid).is_err());
}
