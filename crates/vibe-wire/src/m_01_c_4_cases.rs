//! C4 generated slot-record epoch: the parser issue class is closed.

#[test]
fn schema_two_decodes_copied_unconverted_issues_and_refuses_unknown_class() {
    let mut value = serde_json::json!({
        "schema": 2,
        "source_hash": format!("sha256:{}", "1".repeat(64)),
        "spec_format": "xml",
        "converter_recipe": "specdoc/4",
        "derived_hash": format!("sha256:{}", "2".repeat(64)),
        "file": [{
            "path": "vibevm/vibespecs/DOC.md",
            "sha256": "a".repeat(64),
            "source": "vibevm/vibespecs/DOC.md",
            "disposition": "copied-unconverted",
            "conversion_failure": { "issues": [{
                "class": "Stranded", "line": 3, "message": "status has no carrier"
            }] }
        }]
    });
    let decoded: crate::generated::slot_record::SlotRecord =
        serde_json::from_value(value.clone()).unwrap();
    assert_eq!(decoded.file.len(), 1);
    assert_eq!(
        decoded.file[0]
            .conversion_failure
            .as_ref()
            .unwrap()
            .issues
            .len(),
        1
    );
    value["file"][0]["conversion_failure"]["issues"][0]["class"] = "Invented".into();
    assert!(serde_json::from_value::<crate::generated::slot_record::SlotRecord>(value).is_err());
}
