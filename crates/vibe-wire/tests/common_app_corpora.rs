//! Concrete emitted snapshots, read through the actual generated versions.
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;
use std::path::PathBuf;
use vibe_wire::generated::{
    application::{
        e1::{import_work::ImportWork, steps as old_steps},
        e2::{index as old_index, steps::StepsFile},
        e3::index::ApplicationIndex,
    },
    configuration::e1::{
        app_plan::AppPlan, description::Description, evaluation::Evaluation,
        input::ConfigurationSubmission,
    },
};
fn bytes(path: &str) -> Vec<u8> {
    std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../formats/corpora")
            .join(path),
    )
    .unwrap()
}
fn roundtrip<T: DeserializeOwned + Serialize>(path: &str) {
    let bytes = bytes(path);
    let raw: Value = serde_json::from_slice(&bytes).unwrap();
    let typed: T = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(serde_json::to_value(typed).unwrap(), raw, "{path}");
}
#[test]
fn ordinary_input_description_evaluation_and_plan_snapshots_roundtrip() {
    roundtrip::<ConfigurationSubmission>("configuration/e1/input.json");
    roundtrip::<Description>("configuration/e1/description.json");
    roundtrip::<Evaluation>("configuration/e1/evaluation.json");
    roundtrip::<AppPlan>("configuration/e1/app_plan.json");
}
#[test]
fn actual_index_steps_and_independent_cohort_snapshots_roundtrip() {
    roundtrip::<ApplicationIndex>("application/e3/index.json");
    roundtrip::<StepsFile>("application/e2/steps.json");
    roundtrip::<ImportWork>("application/e1/import_work.json");
    assert!(
        serde_json::from_slice::<old_index::ApplicationIndex>(&bytes("application/e3/index.json"))
            .is_err()
    );
    assert!(
        serde_json::from_slice::<old_steps::StepsFile>(&bytes("application/e2/steps.json"))
            .is_err()
    );
}
#[test]
fn new_durable_readers_remain_closed_at_their_nested_owners() {
    let mut index: Value = serde_json::from_slice(&bytes("application/e3/index.json")).unwrap();
    let row = index["installations"]
        .as_object_mut()
        .unwrap()
        .values_mut()
        .next()
        .unwrap();
    row["configuration"]["evaluation"]["input"]["intent"]["future"] = true.into();
    assert!(serde_json::from_value::<ApplicationIndex>(index).is_err());
    let mut steps: Value = serde_json::from_slice(&bytes("application/e2/steps.json")).unwrap();
    steps["operation"]["request"]["future"] = true.into();
    assert!(serde_json::from_value::<StepsFile>(steps).is_err());
    let mut cohort: Value =
        serde_json::from_slice(&bytes("application/e1/import_work.json")).unwrap();
    cohort["cohort"]["rows"][0]["source"]["future"] = true.into();
    assert!(serde_json::from_value::<ImportWork>(cohort).is_err());
}

#[test]
fn retirement_obligations_cannot_disappear_through_missing_or_null_wire_fields() {
    let source: Value = serde_json::from_slice(&bytes("application/e2/steps.json")).unwrap();
    assert_eq!(source["operation"]["retirements"], serde_json::json!([]));
    for missing in [true, false] {
        let mut bad = source.clone();
        if missing {
            bad["operation"]
                .as_object_mut()
                .unwrap()
                .remove("retirements");
        } else {
            bad["operation"]["retirements"] = Value::Null;
        }
        assert!(serde_json::from_value::<StepsFile>(bad).is_err());
    }
    let mut value = source;
    value["operation"]["retirements"] = serde_json::json!([{"key":"org.example/corpus@0.9.0","expectedRecord":"public-record-example","state":"queued"}]);
    assert!(serde_json::from_value::<StepsFile>(value.clone()).is_ok());
    value["operation"]["retirements"][0]
        .as_object_mut()
        .unwrap()
        .remove("expectedRecord");
    assert!(serde_json::from_value::<StepsFile>(value).is_err());
}
