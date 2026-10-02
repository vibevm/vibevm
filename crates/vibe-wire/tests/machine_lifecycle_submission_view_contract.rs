//! Strict registered sampled evidence, never authority.
use vibe_wire::generated::{format_id::FormatId, machine::e1::lifecycle_submission_view::*};
#[path = "wire_support/mod.rs"]
mod wire_support;
const ROOT: &str = "formats/corpora/machine-lifecycle-submission-view/e1";
#[test]
fn five_status_arms_and_report_variants_roundtrip_registered_epoch() {
    for name in [
        "starting",
        "running",
        "recovery-required",
        "result-available",
        "inspection-unavailable",
        "result-report-unavailable",
        "active-phase",
        "uncertain-ok-report",
    ] {
        let v: LifecycleSubmissionView =
            wire_support::round_trip(&format!("{ROOT}/valid/{name}.json"));
        assert_eq!(v.schema, 1);
        assert!(
            serde_json::to_value(v)
                .unwrap()
                .get("phases")
                .unwrap()
                .is_array()
        );
    }
    assert!(
        FormatId::ALL
            .iter()
            .any(|f| f.id() == "machine-lifecycle-submission-view" && f.epoch() == 1)
    );
}
#[test]
fn structural_corpus_refuses_unknown_missing_and_wrong_types() {
    for entry in std::fs::read_dir(
        wire_support::repo_root()
            .join(ROOT)
            .join("structural-invalid"),
    )
    .unwrap()
    {
        let p = entry.unwrap().path();
        let b = std::fs::read(&p).unwrap();
        assert!(
            serde_json::from_slice::<LifecycleSubmissionView>(&b).is_err(),
            "{}",
            p.display()
        );
    }
}
#[test]
fn every_nullable_field_requires_presence_and_accepts_null() {
    let v = wire_support::read_json(&format!("{ROOT}/valid/starting.json"));
    for key in [
        "bound_scope",
        "run_id",
        "last_phase",
        "phase_unavailable_reason",
        "known_contribution_total",
        "observed_terminal_work_count",
    ] {
        let mut r = v.clone();
        r[key] = serde_json::Value::Null;
        assert!(serde_json::from_value::<LifecycleSubmissionView>(r.clone()).is_ok());
        r.as_object_mut().unwrap().remove(key);
        assert!(serde_json::from_value::<LifecycleSubmissionView>(r).is_err());
    }
    let mut r = v;
    r["last_phase"] = serde_json::json!({"phase":"validate","status":null});
    assert!(serde_json::from_value::<LifecycleSubmissionView>(r.clone()).is_ok());
    r["last_phase"].as_object_mut().unwrap().remove("status");
    assert!(serde_json::from_value::<LifecycleSubmissionView>(r).is_err());
}
#[test]
fn every_status_arm_refuses_extra_and_missing_payload() {
    for name in [
        "starting",
        "running",
        "recovery-required",
        "result-available",
        "inspection-unavailable",
    ] {
        let v = wire_support::read_json(&format!("{ROOT}/valid/{name}.json"));
        let mut r = v.clone();
        r["status"]["extra"] = true.into();
        assert!(serde_json::from_value::<LifecycleSubmissionView>(r).is_err());
        for key in v["status"]
            .as_object()
            .unwrap()
            .keys()
            .filter(|k| k.as_str() != "state")
        {
            let mut r = v.clone();
            r["status"].as_object_mut().unwrap().remove(key);
            assert!(serde_json::from_value::<LifecycleSubmissionView>(r).is_err());
        }
    }
}
