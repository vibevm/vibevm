use super::tests::{FakeRemote, expected, prepared, scope};
use super::*;
use vibe_wire::generated::publication_effect_plan::PublicationObservedAsset;

#[test]
fn nonempty_cleanup_refuses_before_c1_or_any_fake_remote_method() {
    let project = tempfile::tempdir().unwrap();
    let (subject, mut plan) = prepared();
    let index = plan
        .partitions
        .iter()
        .position(|step| matches!(step, PublicationPartition::Cleanup(_)))
        .unwrap();
    let PublicationPartition::Cleanup(cleanup) = &mut plan.partitions[index] else {
        unreachable!()
    };
    cleanup.assets.push(PublicationObservedAsset {
        asset_id: "999".into(),
        name: "stale.tmp".into(),
        size_bytes: "1".into(),
        sha256: format!("sha256:{}", "a".repeat(64)),
    });
    assert!(!transition::supports(&plan.partitions[index]));
    let selected = scope();
    let lease = LifecycleLease::acquire(project.path()).unwrap();
    let namespace = SubmissionReservations::issue_namespace(&lease, &selected, 60_000).unwrap();
    let mut fake = FakeRemote::new(&plan);
    fake.forbidden = true;
    let result = submit_fake_full_plan(
        &lease,
        &namespace,
        &selected,
        "key-a",
        &subject,
        &plan,
        &expected(),
        &mut fake,
    );
    assert!(
        matches!(result, Err(Error::Refused(reason)) if reason.contains(&format!("partition {index}")))
    );
    assert!(fake.calls.is_empty());
    assert!(
        lease
            .publication_journal_project()
            .dir_if_present(&[".vibe", "publication-operations"])
            .unwrap()
            .is_none()
    );
}

#[test]
fn replacement_release_without_old_id_deletion_proof_is_not_fake_admitted() {
    let project = tempfile::tempdir().unwrap();
    let (subject, _) = prepared();
    let plan: PublicationEffectPlan = serde_json::from_str(include_str!(
        "../../../../formats/corpora/publication_effect_plan/e1/valid/replace-known-draft.json"
    ))
    .unwrap();
    let index = plan
        .partitions
        .iter()
        .position(|step| matches!(step, PublicationPartition::Release(_)))
        .unwrap();
    assert!(!transition::supports(&plan.partitions[index]));
    let selected = scope();
    let lease = LifecycleLease::acquire(project.path()).unwrap();
    let namespace = SubmissionReservations::issue_namespace(&lease, &selected, 60_000).unwrap();
    let mut fake = FakeRemote::new(&plan);
    fake.forbidden = true;
    let result = submit_fake_full_plan(
        &lease,
        &namespace,
        &selected,
        "key-a",
        &subject,
        &plan,
        &expected(),
        &mut fake,
    );
    assert!(
        matches!(result, Err(Error::Refused(reason)) if reason.contains(&format!("partition {index}")))
    );
    assert!(fake.calls.is_empty());
}
