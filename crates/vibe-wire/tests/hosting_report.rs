//! Report labels are open and optional; durable source evidence remains closed.
use serde_json::json;
use vibe_wire::generated::{install::e2::plan::InstallPlan, shared::HostingView};

#[test]
fn old_live_rows_stay_unknown_and_future_report_labels_are_readable() {
    let old: InstallPlan = serde_json::from_value(
        json!({"command":"install:plan","packages":[{"package":"example/a","version":"1.0.0"}]}),
    )
    .unwrap();
    assert_eq!(old.packages[0].source_hosting, None);
    let mut future = serde_json::to_value(old).unwrap();
    future["packages"][0]["source_hosting"] = json!({"selected_origin":"future-provider","differing_acquisition_route":"another-provider","future_fact":true});
    let parsed: InstallPlan = serde_json::from_value(future).unwrap();
    let hosting = parsed.packages[0].source_hosting.as_ref().unwrap();
    assert_eq!(hosting.selected_origin.as_deref(), Some("future-provider"));
    assert_eq!(
        hosting.differing_acquisition_route.as_deref(),
        Some("another-provider")
    );
    assert!(
        !serde_json::to_string(&parsed)
            .unwrap()
            .contains("future_fact")
    );
    let empty: HostingView = serde_json::from_value(json!({})).unwrap();
    assert_eq!(serde_json::to_value(empty).unwrap(), json!({}));
}

#[test]
fn hosting_is_not_a_new_durable_source_evidence_field() {
    use vibe_wire::generated::shared::SourceEvidence;
    for key in ["source_hosting", "installed_hosting", "selected_origin"] {
        assert!(serde_json::from_value::<SourceEvidence>(json!({key:"github"})).is_err());
    }
}
