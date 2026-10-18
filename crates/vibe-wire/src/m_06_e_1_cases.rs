use crate::generated::boot_debugger::e1::result as r;
use serde_json::json;
#[test]
fn comparison_extension_is_optional_strict_and_preserves_evidence() {
    let schema: serde_json::Value = serde_json::from_str(include_str!(
        "../../../schemas/boot_debugger/e1/result.jtd.json"
    ))
    .unwrap();
    assert_eq!(
        schema["definitions"]["body"]["mapping"]["preview"]["optionalProperties"]["comparison_json"]
            ["type"],
        "string"
    );
    let input = json!({"artifact":null,"baseline_world":"b","candidate_catalog":"catalog","candidate_world":"c","compiler_carrier_json":null,"measurement_json":null,"comparison_json":"{\"baseline\":\"b\",\"candidate\":\"c\"}","missing":[],"placements":[],"scenario_id":"scenario","status":"complete","target":"os:linux"});
    let body: r::BodyPreview = serde_json::from_value(input.clone()).unwrap();
    assert_eq!(serde_json::to_value(body).unwrap(), input);
    let mut unknown = input.clone();
    unknown["invented_cost"] = json!(0);
    assert!(serde_json::from_value::<r::BodyPreview>(unknown).is_err());
    let mut legacy = input;
    legacy.as_object_mut().unwrap().remove("comparison_json");
    let body: r::BodyPreview = serde_json::from_value(legacy.clone()).unwrap();
    assert!(body.comparison_json.is_none());
    assert_eq!(serde_json::to_value(body).unwrap(), legacy);
}
