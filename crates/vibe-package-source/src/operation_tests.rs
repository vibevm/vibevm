//! Pure construction and shared demand binding do not initialize transports.
use super::operation::SourceOperation;
use crate::transport::TransportInputs;
use std::collections::BTreeMap;
use std::{sync::Arc, time::Duration};
use vibe_core::GlobalRegistrySource;
use vibe_registry::source_candidates::SourceChoiceMode;

fn operation(root: &std::path::Path, body: &str) -> SourceOperation {
    let file = root.join("registry.toml");
    std::fs::write(&file, body).unwrap();
    SourceOperation::new(
        GlobalRegistrySource::deferred(Some(file), Default::default()),
        Default::default(),
        Default::default(),
        TransportInputs {
            cwd: root.into(),
            acquisition_root: root.join("must-not-exist"),
            environment: BTreeMap::new(),
        },
        None,
    )
    .unwrap()
}
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#LOCKED-SOURCE")]
fn construction_and_progress_attach_do_not_observe_policy_or_tools() {
    let root = tempfile::tempdir().unwrap();
    let operation = operation(root.path(), "malformed TOML");
    assert_eq!(operation.source_choice_mode(), SourceChoiceMode::Deferred);
    let backend = operation.backend();
    let _public = backend.anonymized_for_public().unwrap();
    assert!(!root.path().join("must-not-exist").exists());
    assert!(operation.admission().is_err());
    assert!(!root.path().join("must-not-exist").exists());
}
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#SOURCE-POLICY")]
fn payload_and_catalog_share_one_frozen_admission_identity() {
    let root = tempfile::tempdir().unwrap();
    let operation = operation(
        root.path(),
        "[source_policy]\nexclude_hosting=['host:blocked.invalid']\n",
    );
    let http = operation.http_client(Duration::from_secs(1));
    let binding = operation.admission().unwrap();
    assert!(Arc::ptr_eq(&binding, &http.admission().unwrap()));
    std::fs::write(root.path().join("registry.toml"), "").unwrap();
    assert!(
        http.admission()
            .unwrap()
            .check_address("https://blocked.invalid/pkg")
            .is_err()
    );
    assert_eq!(
        operation.source_choice_mode(),
        SourceChoiceMode::CandidateRequired
    );
    assert!(!root.path().join("must-not-exist").exists());
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#SOURCE-POLICY-LAW")]
fn optional_http_binding_keeps_neutral_legacy_and_bound_identity() {
    let root = tempfile::tempdir().unwrap();
    let neutral = operation(root.path(), "");
    assert!(neutral.http_admission().unwrap().is_none());
    let preference = operation(root.path(), "[source_policy]\nprefer_hosting=['github']\n");
    assert!(preference.http_admission().unwrap().is_none());
    let bound = operation(
        root.path(),
        "[source_policy]\nexclude_hosting=['host:blocked.invalid']\n",
    );
    assert!(Arc::ptr_eq(
        &bound.admission().unwrap(),
        &bound.http_admission().unwrap().unwrap()
    ));
}
