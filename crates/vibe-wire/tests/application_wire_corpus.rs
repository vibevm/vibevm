use std::path::PathBuf;

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;
use vibe_wire::generated::application::e1::{
    bundle_manifest::BundleManifest, context::ApplicationContext,
    distribution_index::DistributionIndex, index::ApplicationIndex, management::BinaryManagement,
    reply::ApplicationReply,
};
use vibe_wire::generated::format_id::FormatId;

type StrictReader = fn(&[u8]) -> bool;

fn corpus(name: &str) -> Vec<u8> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("formats/corpora/application/e1");
    std::fs::read(root.join(name)).unwrap()
}

fn round_trip<T>(name: &str)
where
    T: DeserializeOwned + Serialize,
{
    let bytes = corpus(name);
    let authored: Value = serde_json::from_slice(&bytes).unwrap();
    let decoded: T = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(serde_json::to_value(decoded).unwrap(), authored, "{name}");
}

#[test]
fn every_application_contract_reads_and_round_trips_its_corpus() {
    round_trip::<ApplicationIndex>("index.json");
    round_trip::<BinaryManagement>("management.json");
    round_trip::<ApplicationContext>("context.json");
    round_trip::<ApplicationReply>("reply.json");
    round_trip::<DistributionIndex>("distribution-index.json");
    round_trip::<BundleManifest>("bundle-manifest.json");
}

#[test]
fn every_application_contract_keeps_the_preexisting_strict_reader() {
    let cases: &[(&str, StrictReader)] = &[
        ("index.json", |bytes| {
            serde_json::from_slice::<ApplicationIndex>(bytes).is_err()
        }),
        ("management.json", |bytes| {
            serde_json::from_slice::<BinaryManagement>(bytes).is_err()
        }),
        ("context.json", |bytes| {
            serde_json::from_slice::<ApplicationContext>(bytes).is_err()
        }),
        ("reply.json", |bytes| {
            serde_json::from_slice::<ApplicationReply>(bytes).is_err()
        }),
        ("distribution-index.json", |bytes| {
            serde_json::from_slice::<DistributionIndex>(bytes).is_err()
        }),
        ("bundle-manifest.json", |bytes| {
            serde_json::from_slice::<BundleManifest>(bytes).is_err()
        }),
    ];
    for (name, rejects) in cases {
        let mut value: Value = serde_json::from_slice(&corpus(name)).unwrap();
        value
            .as_object_mut()
            .unwrap()
            .insert("future".into(), Value::Bool(true));
        let bytes = serde_json::to_vec(&value).unwrap();
        assert!(rejects(&bytes), "{name} accepted an unknown root member");
    }
}

#[test]
fn registry_exposes_all_six_application_roots() {
    let ids = [
        FormatId::ApplicationIndex,
        FormatId::ApplicationManagement,
        FormatId::ApplicationContext,
        FormatId::ApplicationReply,
        FormatId::ApplicationDistributionIndex,
        FormatId::ApplicationBundleManifest,
    ];
    assert!(ids.iter().all(|id| id.epoch() == 1));
    assert!(
        ids.iter()
            .all(|id| id.unknown_fields() == vibe_wire::generated::format_id::UnknownFields::Deny)
    );
}
