//! Actual generated common contracts: null is omission; empty is an explicit set.
use vibe_wire::generated::{
    configuration::e1::{description::Description, input::ConfigurationSubmission},
    shared as w,
};

#[test]
fn required_nullable_member_sets_have_three_distinct_states() {
    for members in [
        serde_json::Value::Null,
        serde_json::json!([]),
        serde_json::json!(["a"]),
    ] {
        let source = serde_json::json!({"intent":"exact","members":members});
        let value: w::ConfigurationSelection = serde_json::from_value(source.clone()).unwrap();
        assert_eq!(serde_json::to_value(value).unwrap(), source);
    }
    assert!(serde_json::from_str::<w::ConfigurationSelection>(r#"{"intent":"exact"}"#).is_err());
    for (mode, key) in [("choice", "defaults"), ("closure", "fallback")] {
        for members in [
            serde_json::Value::Null,
            serde_json::json!([]),
            serde_json::json!(["a"]),
        ] {
            let mut source = serde_json::json!({"mode":mode,"id":"g","name":"G","cardinality":"zero-or-more","members":["a"]});
            source[key] = members;
            if mode == "choice" {
                source["guards"] = serde_json::json!([]);
            }
            let value: w::ConfigurationGroup = serde_json::from_value(source.clone()).unwrap();
            assert_eq!(serde_json::to_value(value).unwrap(), source);
            source.as_object_mut().unwrap().remove(key);
            assert!(serde_json::from_value::<w::ConfigurationGroup>(source).is_err());
        }
    }
    let mut control = serde_json::json!({"id":"g","active":false,"cardinality":"zero-or-more","members":[],"projections":[],"blockedBy":[],"selection":null});
    let parsed: w::ConfigurationGroupControl = serde_json::from_value(control.clone()).unwrap();
    assert_eq!(parsed.selection, None);
    control["selection"] = serde_json::json!([]);
    let parsed: w::ConfigurationGroupControl = serde_json::from_value(control.clone()).unwrap();
    assert_eq!(parsed.selection, Some(vec![]));
    control.as_object_mut().unwrap().remove("selection");
    assert!(serde_json::from_value::<w::ConfigurationGroupControl>(control).is_err());
}
#[test]
fn submitted_input_remains_closed_through_nested_intent() {
    let source = serde_json::json!({"protocol":"vibe-configuration-input/1","input":{"intent":{"policy":"require-selection","features":[],"defaultFeatures":false,"selections":{"g":{"intent":"exact","members":[]}},"parameters":{}},"disabledFeatures":[]}});
    assert!(serde_json::from_value::<ConfigurationSubmission>(source.clone()).is_ok());
    for pointer in ["", "/input", "/input/intent", "/input/intent/selections/g"] {
        let mut invalid = source.clone();
        invalid
            .pointer_mut(pointer)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert("future".into(), serde_json::json!(true));
        assert!(
            serde_json::from_value::<ConfigurationSubmission>(invalid).is_err(),
            "{pointer}"
        );
    }
}
#[test]
fn optional_projected_definition_is_absent_or_an_object_but_never_null() {
    let mut source = serde_json::json!({"protocol":"vibe-configuration-description/1","package":{"group":"example","name":"a","version":"1.0.0"},"application":{"id":"a","portable":false},"options":[]});
    let absent: Description = serde_json::from_value(source.clone()).unwrap();
    assert!(absent.configuration.is_none());
    source["configuration"] = serde_json::json!({"features":[],"groups":[],"parameters":[],"profiles":[],"configValues":{},"future":true});
    let present: Description = serde_json::from_value(source.clone()).unwrap();
    assert!(present.configuration.is_some());
    assert!(!serde_json::to_string(&present).unwrap().contains("future"));
    source["configuration"] = serde_json::Value::Null;
    assert!(serde_json::from_value::<Description>(source).is_err());
}
