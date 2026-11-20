use serde_json::json;
use vibe_wire::behaviour::ordinary_install_answers::parse;
#[test]
fn ordinary_answers_strict_required_objects_false_and_protocol() {
    let value = json!({"protocol":"vibe-ordinary-install-answers/1","package":"org.example/flow","project_witness":"witness","source":{"registry":"local","content_hash":"sha256:example"},"input":{"selections":{"mode":"off"},"parameters":{"verbose":false}}});
    let parsed = parse(&serde_json::to_vec(&value).unwrap()).unwrap();
    assert!(!parsed.input.parameters["verbose"]);
    assert_eq!(serde_json::to_value(parsed).unwrap(), value);
    for field in ["input", "source", "package", "project_witness"] {
        let mut invalid = value.clone();
        invalid[field] = serde_json::Value::Null;
        assert!(parse(&serde_json::to_vec(&invalid).unwrap()).is_err());
        invalid.as_object_mut().unwrap().remove(field);
        assert!(parse(&serde_json::to_vec(&invalid).unwrap()).is_err());
    }
    for object in ["source", "input"] {
        let mut invalid = value.clone();
        invalid[object]["foreign"] = true.into();
        assert!(parse(&serde_json::to_vec(&invalid).unwrap()).is_err());
    }
    let mut invalid = value.clone();
    invalid["protocol"] = "app-input/1".into();
    assert!(parse(&serde_json::to_vec(&invalid).unwrap()).is_err());
}
