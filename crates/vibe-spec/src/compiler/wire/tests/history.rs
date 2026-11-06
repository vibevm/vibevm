use super::*;
use std::path::Path;

#[test]
fn historical_payloads_roundtrip_without_current_replay_or_synthetic_annotations() {
    let base =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../formats/corpora/compiler_ir/e1/valid");
    let mut count = 0;
    for entry in std::fs::read_dir(base).unwrap() {
        let bytes = std::fs::read(entry.unwrap().path()).unwrap();
        let old = inspect_historical_compiler_ir(&bytes).unwrap();
        assert_eq!(
            serde_json::to_value(old).unwrap(),
            serde_json::from_slice::<serde_json::Value>(&bytes).unwrap()
        );
        count += 1;
    }
    assert_eq!(count, 9);
}

#[test]
fn historical_map_duplicates_still_refuse_at_the_real_host_token_owner() {
    let base =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../formats/corpora/compiler_ir/e1/invalid");
    let mut count = 0;
    for entry in std::fs::read_dir(base).unwrap() {
        let path = entry.unwrap().path();
        if path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("duplicate_key_")
        {
            let error = inspect_historical_compiler_ir(&std::fs::read(path).unwrap()).unwrap_err();
            assert!(matches!(error, HistoricalCompilerIrError::Read { .. }));
            assert!(error.to_string().contains("duplicate"));
            count += 1;
        }
    }
    assert_eq!(count, 7);
}

#[test]
fn historical_source_shape_cannot_cross_current_execution_epoch() {
    let bytes = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../formats/corpora/compiler_ir/e1/valid/source_document.json"
    ));
    inspect_historical_compiler_ir(bytes).unwrap();
    assert!(matches!(
        super::super::decode(bytes),
        Err(super::super::IrWireError::Schema(1))
    ));
}
