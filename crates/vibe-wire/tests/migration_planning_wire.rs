//! Registered neutral migration roots exercise generated strict shape readers.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-TRANSPORT");
use serde::{Serialize, de::DeserializeOwned};
use std::path::PathBuf;
use vibe_wire::generated::migration::e1::{
    plan_error::PlanError, plan_report::PlanReport, plan_request::PlanRequest,
};
fn corpus<T: DeserializeOwned + Serialize>(family: &str, valid: bool) {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../formats/corpora")
        .join(family)
        .join("e1")
        .join(if valid { "valid" } else { "structural-invalid" });
    let mut count = 0;
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_none_or(|e| e != "json") {
            continue;
        }
        let value: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        let parsed = serde_json::from_value::<T>(value.clone());
        if valid {
            let parsed = parsed.unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            assert_eq!(
                serde_json::to_value(parsed).unwrap(),
                value,
                "{}",
                path.display()
            );
        } else {
            assert!(parsed.is_err(), "{} must refuse", path.display());
        }
        count += 1;
    }
    assert!(count > 0, "nonempty corpus required");
}
#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-TRANSPORT",
    r = 1
)]
fn request_valid() {
    corpus::<PlanRequest>("migration-plan-request", true);
}
#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-TRANSPORT",
    r = 1
)]
fn request_invalid() {
    corpus::<PlanRequest>("migration-plan-request", false);
}
#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-TRANSPORT",
    r = 1
)]
fn report_valid() {
    corpus::<PlanReport>("migration-plan-report", true);
}
#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-TRANSPORT",
    r = 1
)]
fn report_invalid() {
    corpus::<PlanReport>("migration-plan-report", false);
}
#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-TRANSPORT",
    r = 1
)]
fn error_valid() {
    corpus::<PlanError>("migration-plan-error", true);
}
#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-TRANSPORT",
    r = 1
)]
fn error_invalid() {
    corpus::<PlanError>("migration-plan-error", false);
}
