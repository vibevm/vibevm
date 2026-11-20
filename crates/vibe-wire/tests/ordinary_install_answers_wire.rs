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

#[test]
fn ordinary_decode_errors_are_closed_before_secret_classification() {
    use sha2::{Digest, Sha256};
    let marker = "synthetic-decode-boundary-value";
    let digest = format!("{:x}", Sha256::digest(marker.as_bytes()));
    let input = json!({"selections":{"mode":"off"},"parameters":{"verbose":false}});
    let envelope = json!({"protocol":"vibe-ordinary-install-answers/1","package":"org.example/flow","project_witness":"witness","source":{"registry":"local","content_hash":"sha256:example"},"input":input});
    for world in ["wrong-value", "unknown-key", "digest-key", "malformed"] {
        let mut bad_input = input.clone();
        match world {
            "wrong-value" => bad_input["parameters"]["verbose"] = marker.into(),
            "unknown-key" => {
                bad_input[marker] = true.into();
            }
            "digest-key" => {
                bad_input[&digest] = true.into();
            }
            _ => {}
        }
        let bytes = if world == "malformed" {
            format!("{{\"{marker}\"").into_bytes()
        } else {
            serde_json::to_vec(&bad_input).unwrap()
        };
        let mut bad_envelope = envelope.clone();
        bad_envelope["input"] = bad_input.clone();
        let envelope_bytes = if world == "malformed" {
            bytes.clone()
        } else {
            serde_json::to_vec(&bad_envelope).unwrap()
        };
        let errors = [
            vibe_wire::behaviour::ordinary_configuration_preview::parse_input(&bytes).unwrap_err(),
            vibe_wire::behaviour::ordinary_configuration_preview::parse_request(json!({"package":"org.example/flow","input":if world == "malformed" { json!(marker) } else { bad_input }})).unwrap_err(),
            parse(&envelope_bytes).unwrap_err(),
        ];
        for error in errors {
            for message in [error.to_string(), format!("{error:?}")] {
                assert!(
                    !message.contains(marker) && !message.contains(&digest),
                    "decode diagnostic disclosure; value suppressed"
                );
                assert!(
                    message.contains("invalid shape"),
                    "closed decode class missing"
                );
                assert!(
                    message.contains("parameters boolean map"),
                    "static schema missing"
                );
            }
            assert!(
                std::error::Error::source(&error).is_none(),
                "decode chain unexpectedly retained"
            );
        }
    }
}
