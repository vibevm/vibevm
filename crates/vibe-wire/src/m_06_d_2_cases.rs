use crate::generated::boot_debugger::e1::{request::Request, result::Result};
use serde_json::json;
#[test]
fn preparation_commands_cannot_carry_a_grant_provider_or_path() {
    for action in ["preparation_plan", "prepare", "preparation_read"] {
        let mut value =
            json!({"schema":1,"context_id":"ctx","command":{"action":action,"plan_id":"retained"}});
        let request: Request = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(value, serde_json::to_value(request).unwrap());
        for field in ["grant", "provider", "cache", "source", "effects"] {
            value["command"][field] = json!("transport cannot grant");
            assert!(serde_json::from_value::<Request>(value.clone()).is_err());
            value["command"].as_object_mut().unwrap().remove(field);
        }
    }
}
#[test]
fn preparation_outcome_nullable_is_required_and_status_is_closed() {
    let value = json!({"schema":1,"context_id":"ctx","snapshot_id":"capture","result_id":"result","body":{"kind":"preparation","plan_id":"id","plan_json":"{}","outcome_json":null,"status":"needs_preparation"}});
    serde_json::from_value::<Result>(value.clone()).unwrap();
    let mut missing = value.clone();
    missing["body"]
        .as_object_mut()
        .unwrap()
        .remove("outcome_json");
    assert!(serde_json::from_value::<Result>(missing).is_err());
    let mut unknown = value;
    unknown["body"]["status"] = json!("silently_installed");
    assert!(serde_json::from_value::<Result>(unknown).is_err());
}
