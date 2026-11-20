use serde_json::json;
use vibe_wire::behaviour::read_doc::parse;
#[test]
fn read_doc_request_is_closed_nonnull_and_value_free_on_decode_errors() {
    let old = json!({"address":"spec://org.example/manual/guide"});
    assert_eq!(
        serde_json::to_value(parse(old.clone()).unwrap()).unwrap(),
        old
    );
    let valid = json!({"address":"spec://org.example/manual/guide","input":{"selections":{"mode":{"intent":"exact","members":["off"]}}}});
    assert_eq!(
        serde_json::to_value(parse(valid.clone()).unwrap()).unwrap(),
        valid
    );
    use sha2::{Digest, Sha256};
    let marker = "synthetic-doc-decode-private-value";
    let digest = format!("{:x}", Sha256::digest(marker.as_bytes()));
    for world in [
        "null",
        "unknown",
        "wrong",
        "intent",
        "format",
        "nested",
        "digest-key",
    ] {
        let mut value = valid.clone();
        match world {
            "null" => value["input"] = serde_json::Value::Null,
            "unknown" => value[marker] = true.into(),
            "wrong" => value["input"]["selections"]["mode"]["members"] = marker.into(),
            "intent" => value["input"]["selections"]["mode"]["intent"] = marker.into(),
            "format" => value["format"] = marker.into(),
            "nested" => value["input"][marker] = true.into(),
            "digest-key" => value[&digest] = true.into(),
            _ => unreachable!(),
        }
        let error = parse(value).unwrap_err();
        assert!(
            !error.to_string().contains(marker) && !error.to_string().contains(&digest),
            "Doc decode disclosure; value suppressed"
        );
        assert!(error.to_string().contains("invalid shape"));
        assert!(std::error::Error::source(&error).is_none());
    }
}
