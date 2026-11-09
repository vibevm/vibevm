//! Frozen readers reject new provenance-bearing worlds; current readers retain facts.
use serde::Serialize;
use serde::de::DeserializeOwned;
fn bytes(path: &str) -> Vec<u8> {
    std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../formats/corpora")
            .join(path),
    )
    .unwrap()
}
fn roundtrip<T: DeserializeOwned + Serialize>(path: &str) {
    let raw = bytes(path);
    let value: T = serde_json::from_slice(&raw).unwrap();
    assert_eq!(
        serde_json::to_value(value).unwrap(),
        serde_json::from_slice::<serde_json::Value>(&raw).unwrap()
    );
}
#[test]
fn new_source_records_roundtrip_and_frozen_readers_refuse() {
    use vibe_wire::generated::application::{e1, e2, e3, e4, e5};
    roundtrip::<e5::index::ApplicationIndex>("application/e5/index.json");
    roundtrip::<e4::index::ApplicationIndex>("application/e4/index.json");
    roundtrip::<e3::steps::StepsFile>("application/e3/steps.json");
    roundtrip::<e3::import_work::ImportWork>("application/e3/import_work.json");
    roundtrip::<e3::legacy_import_work::ImportWork>("application/e3/legacy_import_work.json");
    assert!(
        serde_json::from_slice::<e3::index::ApplicationIndex>(&bytes("application/e4/index.json"))
            .is_err()
    );
    // The exposed payload directories and the slot off switch are index5's
    // own world (PROP-059 `#path-directories`, `#slot-off`).
    assert!(
        serde_json::from_slice::<e4::index::ApplicationIndex>(&bytes("application/e5/index.json"))
            .is_err()
    );
    assert!(
        serde_json::from_slice::<e2::steps::StepsFile>(&bytes("application/e3/steps.json"))
            .is_err()
    );
    assert!(
        serde_json::from_slice::<e2::import_work::ImportWork>(&bytes(
            "application/e3/import_work.json"
        ))
        .is_err()
    );
    assert!(
        serde_json::from_slice::<e1::import_work::ImportWork>(&bytes(
            "application/e3/legacy_import_work.json"
        ))
        .is_err()
    );
}
