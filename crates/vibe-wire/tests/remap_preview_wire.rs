use serde_json::json;
use vibe_wire::behaviour::remap_preview::parse;
#[test]
fn local_offer_preview_input_is_closed_and_decode_diagnostics_are_value_free() {
    let valid = json!({"selections":{"mode":{"intent":"exact","members":["off"]}}});
    assert_eq!(
        serde_json::to_value(parse(&serde_json::to_vec(&valid).unwrap()).unwrap()).unwrap(),
        valid
    );
    use sha2::{Digest, Sha256};
    let marker = "synthetic-remap-private-value";
    let digest = format!("{:x}", Sha256::digest(marker.as_bytes()));
    for world in ["unknown", "null", "wrong", "intent", "digest-key"] {
        let mut value = valid.clone();
        match world {
            "unknown" => value[marker] = true.into(),
            "digest-key" => value[&digest] = true.into(),
            "null" => value["selections"] = serde_json::Value::Null,
            "wrong" => value["selections"]["mode"]["members"] = marker.into(),
            "intent" => value["selections"]["mode"]["intent"] = marker.into(),
            _ => unreachable!(),
        }
        let error = parse(&serde_json::to_vec(&value).unwrap()).unwrap_err();
        assert!(
            !error.to_string().contains(marker) && !error.to_string().contains(&digest),
            "remap decode disclosure; value suppressed"
        );
        assert!(error.to_string().contains("invalid shape"));
        assert!(std::error::Error::source(&error).is_none());
    }
}
