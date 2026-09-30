use super::tests::{FakeRemote, expected, prepared, scope};
use super::*;

#[test]
fn missing_terminal_result_after_all_landed_is_uncertain_and_duplicate_is_read_only() {
    let project = tempfile::tempdir().unwrap();
    let (subject, plan) = prepared();
    let selected = scope();
    let lease = LifecycleLease::acquire(project.path()).unwrap();
    let namespace = SubmissionReservations::issue_namespace(&lease, &selected, 60_000).unwrap();
    let mut fake = FakeRemote::new(&plan);
    let first = submit_fake_full_plan(
        &lease,
        &namespace,
        &selected,
        "key-a",
        &subject,
        &plan,
        &expected(),
        &mut fake,
    )
    .unwrap();
    let Inspection::Completed { run_id, .. } = first else {
        panic!("first run incomplete")
    };
    std::fs::remove_file(project.path().join(format!(
        ".vibe/publication-operations/{run_id}.full-plan-result.json"
    )))
    .unwrap();
    fake.forbidden = true;
    let replay = submit_fake_full_plan(
        &lease,
        &namespace,
        &selected,
        "key-a",
        &subject,
        &plan,
        &expected(),
        &mut fake,
    )
    .unwrap();
    assert!(matches!(replay, Inspection::Partial { run_id: same, landed,
        pending: None, uncertain: true } if same == run_id && landed == plan.partitions.len()));
    assert_eq!(fake.calls.len(), plan.partitions.len());
}

#[test]
fn tampered_post_observation_cannot_complete_even_with_recomputed_record_digests() {
    let project = tempfile::tempdir().unwrap();
    let (subject, plan) = prepared();
    let selected = scope();
    let lease = LifecycleLease::acquire(project.path()).unwrap();
    let namespace = SubmissionReservations::issue_namespace(&lease, &selected, 60_000).unwrap();
    let mut fake = FakeRemote::new(&plan);
    let first = submit_fake_full_plan(
        &lease,
        &namespace,
        &selected,
        "key-a",
        &subject,
        &plan,
        &expected(),
        &mut fake,
    )
    .unwrap();
    let Inspection::Completed { run_id, .. } = first else {
        panic!("first run incomplete")
    };

    // Keep the callback, generation and fake last-marker intact. Re-hash the
    // modified witness and terminal summary so only transition replay detects
    // that the planned tag was never present in the retained after-state.
    let mut first_step = journal::read::<Step>(&lease, &run_id, &landed_name(0))
        .unwrap()
        .unwrap();
    let observed = first_step.post_observation.as_mut().unwrap();
    observed.tag = plan.observed_tag.clone();
    first_step.post_observation_sha256 = Some(sha(&serde_json::to_vec(observed).unwrap()));
    let first_path = project.path().join(format!(
        ".vibe/publication-operations/{run_id}.full-plan-{}.json",
        landed_name(0)
    ));
    std::fs::write(&first_path, serde_json::to_vec(&first_step).unwrap()).unwrap();
    let hashes = (0..plan.partitions.len())
        .map(|index| {
            sha(&journal::read_bytes(&lease, &run_id, &landed_name(index))
                .unwrap()
                .unwrap())
        })
        .collect::<Vec<_>>();
    let mut result = journal::read::<FinalResult>(&lease, &run_id, "result")
        .unwrap()
        .unwrap();
    result.landed_sha256 = sha(&serde_json::to_vec(&hashes).unwrap());
    let result_path = project.path().join(format!(
        ".vibe/publication-operations/{run_id}.full-plan-result.json"
    ));
    std::fs::write(result_path, serde_json::to_vec(&result).unwrap()).unwrap();

    fake.forbidden = true;
    let replay = submit_fake_full_plan(
        &lease,
        &namespace,
        &selected,
        "key-a",
        &subject,
        &plan,
        &expected(),
        &mut fake,
    )
    .unwrap();
    assert!(
        matches!(replay, Inspection::Partial { run_id: same, landed: 0,
        pending: Some(0), uncertain: true } if same == run_id)
    );
    assert_eq!(fake.calls.len(), plan.partitions.len());
}
