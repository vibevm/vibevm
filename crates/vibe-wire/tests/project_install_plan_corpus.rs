//! The current generated contract matches live packages rows; the declared e1 stays frozen.
use vibe_wire::generated::format_id::FormatId;
use vibe_wire::generated::install::e2::plan::InstallPlan;

#[test]
fn observed_and_legacy_live_project_events_roundtrip() {
    for file in ["legacy-live.json", "observed.json", "in-place.json"] {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../formats/corpora/install-plan/e2")
            .join(file);
        let bytes = std::fs::read(path).unwrap();
        let parsed: InstallPlan = serde_json::from_slice(&bytes).unwrap();
        if file == "legacy-live.json" {
            assert!(parsed.packages[0].source_evidence.is_none());
        }
        assert_eq!(
            serde_json::to_value(parsed).unwrap(),
            serde_json::from_slice::<serde_json::Value>(&bytes).unwrap()
        );
    }
    assert_eq!(FormatId::CliInstallPlan.epoch(), 2);
    assert_eq!(FormatId::CliInstallPlanV1.epoch(), 1);
}

#[test]
fn frozen_declared_reader_still_reads_its_own_shape_and_not_live_packages() {
    let old: vibe_wire::generated::install_plan::InstallPlan =
        serde_json::from_str(r#"{"command":"install:plan","plans":[]}"#).unwrap();
    assert!(old.plans.is_empty());
    assert!(
        serde_json::from_str::<vibe_wire::generated::install_plan::InstallPlan>(
            r#"{"command":"install:plan","packages":[]}"#
        )
        .is_err()
    );
    let live: InstallPlan =
        serde_json::from_str(r#"{"command":"install:plan","packages":[],"future":true}"#).unwrap();
    assert!(live.packages.is_empty());
}
