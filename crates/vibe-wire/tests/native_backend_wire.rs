use serde_json::json;
use vibe_wire::behaviour::native_backend::{
    AdmittedBackendReply, BackendReplyError, DECODED_CAP_BYTES, DIAGNOSTIC_CAP_BYTES,
    ENVELOPE_EPOCH, REPLY_CAP_BYTES, decode_reply, fail_reply, ok_reply, validate_reply,
};
use vibe_wire::generated::native::e1::backend_reply::{
    BackendReply, BackendReplyFail, BackendReplyOk,
};

#[test]
fn ok_and_fail_are_strict_bytes_only_shapes() {
    let ok = json!({"status":"ok","envelope":1,"bytes_b64":"AP8=","message":"done"});
    let decoded = decode_reply(&serde_json::to_vec(&ok).unwrap()).unwrap();
    assert_eq!(
        decoded,
        AdmittedBackendReply::Ok {
            bytes: vec![0, 255],
            message: Some("done".into())
        }
    );
    let fail = fail_reply("bounded refusal".into()).unwrap();
    assert_eq!(
        validate_reply(&fail).unwrap(),
        AdmittedBackendReply::Fail {
            message: "bounded refusal".into()
        }
    );

    for hostile in [
        json!({"status":"ok","envelope":1,"bytes_b64":"","provenance":{}}),
        json!({"status":"fail","envelope":1,"message":"no","digest":"00"}),
        json!({"status":"skip","envelope":1}),
    ] {
        assert!(decode_reply(&serde_json::to_vec(&hostile).unwrap()).is_err());
    }
}

#[test]
fn canonical_base64_round_trips_binary_and_rejects_aliases() {
    for bytes in [vec![], vec![0], vec![0, 255], vec![0, 1, 2, 253, 254, 255]] {
        let reply = ok_reply(&bytes, None).unwrap();
        let AdmittedBackendReply::Ok { bytes: actual, .. } = validate_reply(&reply).unwrap() else {
            panic!("ok constructor returns ok")
        };
        assert_eq!(actual, bytes);
    }
    for value in ["Zg", "Zg=", "Zg==\n", "Zh==", "Zm9="] {
        let reply = BackendReply::Ok(Box::new(BackendReplyOk {
            bytes_b64: value.into(),
            envelope: ENVELOPE_EPOCH,
            message: None,
        }));
        assert_eq!(validate_reply(&reply), Err(BackendReplyError::Base64));
    }
}

#[test]
fn caps_and_messages_refuse_before_decoded_bytes_are_allocated() {
    let oversized_raw = vec![b' '; REPLY_CAP_BYTES + 1];
    assert_eq!(
        decode_reply(&oversized_raw),
        Err(BackendReplyError::ReplyCap)
    );
    let oversized_bytes = vec![0; DECODED_CAP_BYTES + 1];
    assert_eq!(
        ok_reply(&oversized_bytes, None),
        Err(BackendReplyError::DecodedCap)
    );
    for message in [
        "   ".to_owned(),
        "line\nbreak".to_owned(),
        "x".repeat(DIAGNOSTIC_CAP_BYTES + 1),
    ] {
        assert_eq!(fail_reply(message), Err(BackendReplyError::Message));
    }
    assert!(ok_reply(b"", Some(String::new())).is_ok());
    assert_eq!(
        ok_reply(b"", Some("line\nbreak".into())),
        Err(BackendReplyError::Message)
    );
}

#[test]
fn envelope_and_schema_metadata_pin_the_admission_laws() {
    let wrong = BackendReply::Fail(Box::new(BackendReplyFail {
        envelope: 2,
        message: "no".into(),
    }));
    assert_eq!(validate_reply(&wrong), Err(BackendReplyError::Envelope));
    let schema: serde_json::Value = serde_json::from_str(include_str!(
        "../../../schemas/native/e1/backend_reply.jtd.json"
    ))
    .unwrap();
    assert_eq!(
        schema["metadata"]["x-diagnostic-cap-bytes"],
        DIAGNOSTIC_CAP_BYTES
    );
    assert_eq!(schema["metadata"]["x-decoded-cap-bytes"], DECODED_CAP_BYTES);
    assert_eq!(schema["metadata"]["x-reply-cap-bytes"], REPLY_CAP_BYTES);
    let text = serde_json::to_string(&schema).unwrap();
    for forbidden in ["payload", "context", "provenance", "digest", "witness"] {
        assert!(!text.contains(&format!("\"{forbidden}\"")), "{text}");
    }
}
