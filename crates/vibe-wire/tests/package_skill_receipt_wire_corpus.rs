use std::path::PathBuf;

use vibe_wire::generated::package_skill_receipt::PackageSkillReceipt;

#[test]
fn strict_receipt_corpus_round_trips_without_losing_ownership_rows() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("formats/corpora/package-skills/e1/receipt.json");
    let authored: serde_json::Value =
        serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let receipt: PackageSkillReceipt = serde_json::from_value(authored.clone()).unwrap();
    assert_eq!(receipt.schema, 2);
    assert_eq!(receipt.binding[0].target[0].file[0].path, "SKILL.md");
    assert_eq!(serde_json::to_value(&receipt).unwrap(), authored);

    let mut unknown = authored;
    unknown["binding"][0]["unknown"] = serde_json::json!(true);
    assert!(serde_json::from_value::<PackageSkillReceipt>(unknown).is_err());

    // An applying intent carries its transaction nonce; without one the
    // strict shape refuses to parse.
    let mut applying = serde_json::to_value(&receipt).unwrap();
    let key = applying["binding"][0]["key"].clone();
    let binding = applying["binding"].clone();
    applying["applying"] = serde_json::json!({
        "key": key,
        "nonce": "0123456789abcdef0123456789abcdef",
        "binding": binding
    });
    let applying: PackageSkillReceipt = serde_json::from_value(applying).unwrap();
    assert!(applying.applying.is_some());
    assert_eq!(
        applying.applying.as_ref().unwrap().nonce,
        "0123456789abcdef0123456789abcdef"
    );

    let mut nonceless = serde_json::to_value(&receipt).unwrap();
    nonceless["applying"] = serde_json::json!({
        "key": nonceless["binding"][0]["key"].clone(),
        "binding": nonceless["binding"].clone()
    });
    assert!(serde_json::from_value::<PackageSkillReceipt>(nonceless).is_err());
}
