use std::path::PathBuf;

use vibe_wire::generated::distribution_smoke_report::{
    DistributionSmokeReport, DistributionSmokeTargetStatus,
};
use vibe_wire::generated::publication_subject::PublicationSubject;

use super::validate_distribution_smoke_report_bytes;

fn corpus(category: &str, name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../formats/corpora/distribution_smoke_report/e1")
        .join(category)
        .join(format!("{name}.json"));
    std::fs::read(path).unwrap()
}

fn subject() -> PublicationSubject {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../formats/corpora/publication_subject/e1/valid/complete-five-targets.json");
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

fn report() -> DistributionSmokeReport {
    serde_json::from_slice(&corpus("valid", "five-unavailable")).unwrap()
}

fn assert_refused(report: &DistributionSmokeReport) {
    let bytes = serde_json::to_vec(report).unwrap();
    validate_distribution_smoke_report_bytes(&bytes, &subject()).unwrap_err();
}

#[test]
fn five_unavailable_rows_are_exact_inspectable_evidence_data() {
    let bytes = corpus("valid", "five-unavailable");
    let digest = validate_distribution_smoke_report_bytes(&bytes, &subject()).unwrap();
    assert_eq!(digest, crate::sha256_digest(&bytes));
    assert_eq!(report().targets.len(), 5);
    assert!(
        report()
            .targets
            .iter()
            .all(|row| row.tested_sha256.is_none())
    );
}

#[test]
fn strict_generated_reader_rejects_three_structural_errors() {
    for name in ["unknown-root", "unknown-status", "missing-bundle-digest"] {
        let bytes = corpus("structural-invalid", name);
        assert!(serde_json::from_slice::<DistributionSmokeReport>(&bytes).is_err());
        assert!(validate_distribution_smoke_report_bytes(&bytes, &subject()).is_err());
    }
}

#[test]
fn wrong_target_order_or_bundle_digest_refuses() {
    let mut value = report();
    value.targets.swap(0, 1);
    assert_refused(&value);
    let mut value = report();
    value.targets[0].target = "foreign-target".into();
    assert_refused(&value);
    let mut value = report();
    value.targets[0].bundle_sha256 = format!("sha256:{}", "f".repeat(64));
    assert_refused(&value);
}

#[test]
fn passed_requires_native_runner_measured_binary_and_actual_success_shape() {
    let mut value = report();
    let row = &mut value.targets[2];
    row.status = DistributionSmokeTargetStatus::Passed;
    row.tested_sha256 = Some(row.bundle_sha256.clone());
    row.executable_sha256 = Some(subject().targets[2].components[0].sha256.clone());
    row.exit_code = Some(0);
    row.stdout = "vibe 1.1.0\n".into();
    row.elapsed_ms = "12".into();
    assert_refused(&value);
    value.targets[2].runner_ref = Some("system-native/linux/x86_64/gnu".into());
    assert_refused(&value); // the unavailable reason is still contradictory
    value.targets[2].reason = "exact staged native executable returned vibe 1.1.0".into();
    validate_distribution_smoke_report_bytes(&serde_json::to_vec(&value).unwrap(), &subject())
        .unwrap();
    value.targets[2].output_truncated = true;
    assert_refused(&value);
    value.targets[2].output_truncated = false;
    value.targets[2].status = DistributionSmokeTargetStatus::Failed;
    assert_refused(&value);
}

#[test]
fn unavailable_cannot_claim_a_tested_digest_or_process_output() {
    let mut value = report();
    value.targets[0].tested_sha256 = Some(value.targets[0].bundle_sha256.clone());
    assert_refused(&value);
    value.targets[0].tested_sha256 = None;
    value.targets[0].stdout = "vibe 1.1.0\n".into();
    assert_refused(&value);
}

#[test]
fn captured_output_and_duration_are_bounded() {
    let mut value = report();
    value.targets[0].stdout = "x".repeat(16 * 1024 + 1);
    assert_refused(&value);
    value.targets[0].stdout.clear();
    value.targets[0].elapsed_ms = "60001".into();
    assert_refused(&value);
}
