//! Frozen receipt reader refuses retained activation; successor preserves it.

#[test]
fn shared_physical_group_roundtrips_and_singleton_reader_refuses() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let value: serde_json::Value = serde_json::from_slice(
        &std::fs::read(root.join("formats/corpora/package-skills/e3/receipt.json")).unwrap(),
    )
    .unwrap();
    let receipt: vibe_wire::generated::package_skill::e3::receipt::PackageSkillReceipt =
        serde_json::from_value(value.clone()).unwrap();
    assert_eq!(receipt.schema, 4);
    assert_eq!(receipt.binding[0].target[0].agents, ["codex", "pi"]);
    assert_eq!(serde_json::to_value(receipt).unwrap(), value);
    assert!(
        serde_json::from_value::<
            vibe_wire::generated::package_skill::e2::receipt::PackageSkillReceipt,
        >(value)
        .is_err()
    );
}
#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-073#MANAGED-ACTIVATION")]
fn retained_activation_roundtrips_and_old_reader_refuses() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let value: serde_json::Value = serde_json::from_slice(
        &std::fs::read(root.join("formats/corpora/package-skills/e2/receipt.json")).unwrap(),
    )
    .unwrap();
    let receipt: vibe_wire::generated::package_skill::e2::receipt::PackageSkillReceipt =
        serde_json::from_value(value.clone()).unwrap();
    assert_eq!(receipt.schema, 3);
    assert!(!receipt.binding[0].activation["claude"].enabled);
    assert!(receipt.binding[0].target.is_empty());
    assert_eq!(serde_json::to_value(&receipt).unwrap(), value);
    assert!(
        serde_json::from_value::<vibe_wire::generated::package_skill_receipt::PackageSkillReceipt>(
            value
        )
        .is_err()
    );
    let legacy =
        std::fs::read(root.join("formats/corpora/package-skills/e1/receipt.json")).unwrap();
    let read: vibe_wire::generated::package_skill::e2::receipt::PackageSkillReceipt =
        serde_json::from_slice(&legacy).unwrap();
    assert!(read.binding[0].activation.is_empty());
}
