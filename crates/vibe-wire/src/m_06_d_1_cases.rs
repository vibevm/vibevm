use crate::generated::boot_debugger::e1::{request as q, result as r};
use serde_json::json;

#[test]
fn preview_is_strict_and_preserves_expected_worlds() {
    let input = json!({"schema":1,"context_id":"host","command":{"action":"preview","scenario_id":"scenario","expected_baseline":"baseline","expected_candidate":"candidate","expected_catalog":"catalog","loading_overrides":{}}});
    let request: q::Request = serde_json::from_value(input.clone()).unwrap();
    assert_eq!(serde_json::to_value(&request).unwrap(), input);
    let mut unknown = input.clone();
    unknown["command"]["install"] = json!(true);
    assert!(serde_json::from_value::<q::Request>(unknown).is_err());
    let mut missing = input;
    missing["command"]
        .as_object_mut()
        .unwrap()
        .remove("expected_catalog");
    assert!(serde_json::from_value::<q::Request>(missing).is_err());
}
#[test]
fn incomplete_is_distinct_from_zero_measurement() {
    let result = json!({"schema":1,"context_id":"host","snapshot_id":null,"result_id":"result","body":{"kind":"preview","scenario_id":"scenario","baseline_world":"baseline","candidate_world":"candidate","candidate_catalog":"catalog","target":"os:linux","status":"incomplete","missing":["native stage: unavailable"],"placements":[],"artifact":null,"compiler_carrier_json":null,"measurement_json":null}});
    let parsed: r::Result = serde_json::from_value(result.clone()).unwrap();
    assert_eq!(serde_json::to_value(parsed).unwrap(), result);
    let mut unknown = result;
    unknown["body"]["native_invoked"] = json!(false);
    assert!(serde_json::from_value::<r::Result>(unknown).is_err());
}
