use std::path::PathBuf;

use vibe_wire::generated::publication_subject::PublicationSmokeProbeStatus;
use vibe_wire::generated::publication_subject::PublicationSubject;

use super::{
    PublicationSubjectExpectations, publication_subject_digest, require_apply_ready_default_smoke,
    validate_publication_subject,
};

fn corpus(category: &str, name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../formats/corpora/publication_subject/e1")
        .join(category)
        .join(format!("{name}.json"));
    std::fs::read(path).unwrap()
}

fn parsed(category: &str, name: &str) -> PublicationSubject {
    serde_json::from_slice(&corpus(category, name)).unwrap()
}

fn expected() -> PublicationSubjectExpectations<'static> {
    PublicationSubjectExpectations {
        version: "1.1.0",
        repository: "vibevm/vibevm",
        smoke_policy_id: "distribution-smoke/e1",
        smoke_policy_epoch: 1,
    }
}

#[test]
fn generated_subject_refuses_four_structural_corpus_errors() {
    for name in [
        "unknown-root",
        "wrong-size-type",
        "unknown-smoke-status",
        "missing-lock-witness",
    ] {
        assert!(
            serde_json::from_slice::<PublicationSubject>(&corpus("structural-invalid", name))
                .is_err(),
            "{name} unexpectedly decoded"
        );
    }
}

#[test]
fn semantic_subject_refuses_incomplete_duplicate_or_mismatched_evidence() {
    for name in [
        "incomplete-target-set",
        "duplicate-target",
        "tested-hash-mismatch",
    ] {
        let candidate = parsed("semantic-invalid", name);
        validate_publication_subject(&candidate, &expected()).unwrap_err();
    }
}

#[test]
fn skipped_failed_and_unavailable_remain_distinct_subjects_but_not_default_ready() {
    let complete = parsed("valid", "complete-five-targets");
    let passed = publication_subject_digest(&complete, &expected()).unwrap();
    require_apply_ready_default_smoke(&complete, &expected()).unwrap();
    let skipped = parsed("readiness-invalid", "skipped-smoke");
    validate_publication_subject(&skipped, &expected()).unwrap();
    let skipped_digest = publication_subject_digest(&skipped, &expected()).unwrap();
    assert_ne!(skipped_digest, passed);
    let refusal = require_apply_ready_default_smoke(&skipped, &expected()).unwrap_err();
    assert!(
        refusal
            .to_string()
            .contains("separate explicit versioned policy rule"),
        "skip is inspectable but never an implicit pass: {refusal}"
    );
    let mut failed = skipped.clone();
    failed.smoke.probes[0].status = PublicationSmokeProbeStatus::Failed;
    let failed_digest = publication_subject_digest(&failed, &expected()).unwrap();
    assert_ne!(failed_digest, skipped_digest);
    assert!(require_apply_ready_default_smoke(&failed, &expected()).is_err());
    let mut unavailable = skipped;
    unavailable.smoke.probes[0].status = PublicationSmokeProbeStatus::Unavailable;
    let unavailable_digest = publication_subject_digest(&unavailable, &expected()).unwrap();
    assert_ne!(unavailable_digest, failed_digest);
    assert_ne!(unavailable_digest, skipped_digest);
    assert!(require_apply_ready_default_smoke(&unavailable, &expected()).is_err());
}

#[test]
fn exact_subject_digest_is_stable_and_changes_with_the_committed_lock() {
    let complete = parsed("valid", "complete-five-targets");
    let changed_lock = parsed("valid", "changed-committed-lock");
    validate_publication_subject(&complete, &expected()).unwrap();
    validate_publication_subject(&changed_lock, &expected()).unwrap();
    let first = publication_subject_digest(&complete, &expected()).unwrap();
    assert_eq!(
        first,
        publication_subject_digest(&complete, &expected()).unwrap()
    );
    assert_ne!(
        first,
        publication_subject_digest(&changed_lock, &expected()).unwrap()
    );
    assert_eq!(first.len(), 71);
    assert!(first.starts_with("sha256:"));
}

#[test]
fn changed_bundle_or_target_cannot_reuse_an_approved_subject_identity() {
    let original = parsed("valid", "complete-five-targets");
    let initial = publication_subject_digest(&original, &expected()).unwrap();
    let mut changed_bundle = original.clone();
    changed_bundle.targets[0].bundle.sha256 = format!("sha256:{}", "f".repeat(64));
    changed_bundle.smoke.probes[0].tested_sha256 =
        Some(changed_bundle.targets[0].bundle.sha256.clone());
    assert_ne!(
        initial,
        publication_subject_digest(&changed_bundle, &expected()).unwrap()
    );
    let mut changed_target = original;
    changed_target.targets.swap(0, 1);
    assert!(publication_subject_digest(&changed_target, &expected()).is_err());
}

#[test]
fn subject_validation_and_digest_have_no_filesystem_or_network_capability() {
    let _validator: fn(
        &PublicationSubject,
        &PublicationSubjectExpectations<'_>,
    ) -> Result<(), super::PublicationSubjectError> = validate_publication_subject;
    let _digest: fn(
        &PublicationSubject,
        &PublicationSubjectExpectations<'_>,
    ) -> Result<String, super::PublicationSubjectError> = publication_subject_digest;
    let home = tempfile::tempdir().unwrap();
    let before = std::fs::read_dir(home.path()).unwrap().count();
    let candidate = parsed("valid", "complete-five-targets");
    publication_subject_digest(&candidate, &expected()).unwrap();
    assert_eq!(std::fs::read_dir(home.path()).unwrap().count(), before);
}
