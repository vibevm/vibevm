//! Actual prior generated reader compatibility, not a guessed strict facade.
mod generated {
    pub use vibe_wire::generated::shared;
}
#[allow(dead_code, unused_imports)]
#[path = "fixtures/update-report-pre-retirement/mod.rs"]
mod old;
use serde_json::json;
use vibe_wire::generated::update_report::UpdateReport;
fn legacy() -> serde_json::Value {
    json!({"ok":true,"command":"update","project":"/synthetic/project","scope":"all","packages":[],"packages_resolved":1,"version_bumps":[],"materialised":[],"skipped":[],"pruned":[],"nodes_regenerated":[],"complete":true,"unchanged":true,"hooks":[]})
}
fn without_hooks(mut v: serde_json::Value) -> serde_json::Value {
    v.as_object_mut().unwrap().remove("hooks");
    v
}
#[test]
fn old_json_is_byte_value_stable_without_retirement_field() {
    let value = legacy();
    let before: old::UpdateReport = serde_json::from_value(value.clone()).unwrap();
    let after: UpdateReport = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(serde_json::to_value(before).unwrap(), value);
    assert_eq!(
        serde_json::to_value(after).unwrap(),
        without_hooks(value.clone())
    );
}
#[test]
fn old_reader_accepts_but_does_not_understand_retained_custody() {
    for state in ["published_backup_retained", "restored_container_retained"] {
        let mut value = legacy();
        value["retained_backups"] = json!([{"slot":"vibevm/vibedeps/org.example.leaf/1.0.0","path":"/synthetic/backup","state":state}]);
        let current: UpdateReport = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(
            serde_json::to_value(current).unwrap(),
            without_hooks(value.clone())
        );
        let prior: old::UpdateReport = serde_json::from_value(value).unwrap();
        assert!(
            serde_json::to_value(prior)
                .unwrap()
                .get("retained_backups")
                .is_none(),
            "old reader silently loses the recovery instruction"
        );
    }
}
#[test]
fn new_custody_state_is_closed_and_empty_is_omitted() {
    let mut value = legacy();
    value["retained_backups"] = json!([]);
    assert!(
        serde_json::to_value(serde_json::from_value::<UpdateReport>(value.clone()).unwrap())
            .unwrap()
            .get("retained_backups")
            .is_none()
    );
    value["retained_backups"] = json!([{"slot":"leaf","path":"/synthetic","state":"uncertain"}]);
    assert!(serde_json::from_value::<UpdateReport>(value).is_err());
}
