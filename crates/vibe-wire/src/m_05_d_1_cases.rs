use crate::behaviour::lock_configuration::{decode, encode};
fn record() -> vibe_core::manifest::LockedConfiguration {
    let requested = serde_json::from_str(include_str!(
        "../../../formats/corpora/lock-configuration/e1/requested.json"
    ))
    .unwrap();
    let resolved = serde_json::from_str(include_str!(
        "../../../formats/corpora/lock-configuration/e1/resolved.json"
    ))
    .unwrap();
    encode(requested, resolved).unwrap()
}
#[test]
fn generated_configuration_record_roundtrips_canonically() {
    let r = record();
    let (q, s) = decode(&r).unwrap();
    assert_eq!(encode(q, s).unwrap(), r);
}
#[test]
fn unknown_epoch_or_missing_exact_identity_refuses() {
    let (q, s) = decode(&record()).unwrap();
    let mut value = serde_json::to_value(s).unwrap();
    value["edges"] = serde_json::json!([{"parent":"org.example/a@=1.0.0","provider":"org.example/b@=1.0.0","alternative":0,"declaration":"requires.packages[0]","definition":"","request_digest":"pending","owner_identity":{"source":"","content":"sha256:a","definition":"sha256:a"},"provider_identity":{"source":"fixture","content":"sha256:b","definition":"sha256:b"}}]);
    assert!(encode(q, serde_json::from_value(value).unwrap()).is_err());

    let mut r = record();
    r.epoch = 99;
    assert!(decode(&r).is_err());
    let mut r = record();
    r.request_digest.clear();
    assert!(decode(&r).is_err());
    let mut r = record();
    r.requested = r.requested.replace("\"epoch\":\"1\"", "\"epoch\":\"99\"");
    r.request_digest = vibe_core::manifest::LockedConfiguration::digest(&r.requested);
    assert!(decode(&r).is_err());
}
