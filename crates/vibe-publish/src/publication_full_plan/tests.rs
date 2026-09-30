use super::*;

const SUBJECT: &str = include_str!(
    "../../../../formats/corpora/publication_subject/e1/valid/complete-five-targets.json"
);
const NEW: &str = include_str!(
    "../../../../formats/corpora/publication_effect_plan/e1/valid/new-draft-complete.json"
);

fn expected() -> PublicationEffectPlanExpectations<'static> {
    PublicationEffectPlanExpectations {
        subject: crate::publication_subject::PublicationSubjectExpectations {
            version: "1.1.0",
            repository: "vibevm/vibevm",
            smoke_policy_id: "distribution-smoke/e1",
            smoke_policy_epoch: 1,
        },
        host: "github.com",
        owner: "vibevm",
        repository: "vibevm",
        publish: false,
        allow_replace_published: false,
    }
}
fn prepared() -> (PublicationSubject, PublicationEffectPlan) {
    (
        serde_json::from_str(SUBJECT).unwrap(),
        serde_json::from_str(NEW).unwrap(),
    )
}
fn scope() -> SubmissionScope {
    SubmissionScope {
        context: "fixture-host-context".into(),
        target: "github.com/vibevm/vibevm".into(),
        principal: "fixture-maintainer".into(),
    }
}

struct FakeRemote {
    current: RemoteObservation,
    calls: Vec<usize>,
    lost_at: Option<usize>,
    drift_before: Option<usize>,
    observations: usize,
    forbidden: bool,
}
impl FakeRemote {
    fn new(plan: &PublicationEffectPlan) -> Self {
        Self {
            current: RemoteObservation {
                generation_ref: plan.remote_generation_ref.clone(),
                tag: plan.observed_tag.clone(),
                release: plan.observed_release.clone(),
                last: None,
            },
            calls: Vec::new(),
            lost_at: None,
            drift_before: None,
            observations: 0,
            forbidden: false,
        }
    }
}
impl FakePlanPort for FakeRemote {
    fn observe(&mut self) -> std::result::Result<RemoteObservation, String> {
        assert!(!self.forbidden, "duplicate observed fake remote");
        self.observations += 1;
        if let Some(index) = self.drift_before {
            if self.observations == 2 + 2 * index {
                self.current.generation_ref.push_str("-foreign");
            }
        }
        Ok(self.current.clone())
    }
    fn apply(
        &mut self,
        target: &PublicationRemoteTarget,
        index: usize,
        partition: &PublicationPartition,
        expected: &RemoteObservation,
    ) -> std::result::Result<PartitionReceipt, String> {
        assert!(!self.forbidden, "duplicate applied fake partition");
        assert_eq!(
            (
                target.host.as_str(),
                target.owner.as_str(),
                target.repository.as_str()
            ),
            ("github.com", "vibevm", "vibevm")
        );
        if &self.current != expected {
            return Err("fake conditional generation check failed".into());
        }
        let digest = sha(&serde_json::to_vec(partition).unwrap());
        let receipt = PartitionReceipt {
            index,
            partition_sha256: digest,
            resulting_generation_ref: format!("fake-generation-{}", index + 1),
            receipt_ref: format!("fake-receipt-{index}"),
        };
        self.calls.push(index);
        self.current.generation_ref = receipt.resulting_generation_ref.clone();
        self.current.last = Some(receipt.clone());
        if self.lost_at == Some(index) {
            return Err("response lost after fake effect".into());
        }
        Ok(receipt)
    }
}

struct ReadyChecked<'a> {
    inner: FakeRemote,
    lease: &'a LifecycleLease,
    namespace: &'a SubmissionNamespace,
    scope: &'a SubmissionScope,
    request: Vec<u8>,
}
impl FakePlanPort for ReadyChecked<'_> {
    fn observe(&mut self) -> std::result::Result<RemoteObservation, String> {
        self.inner.observe()
    }
    fn apply(
        &mut self,
        target: &PublicationRemoteTarget,
        index: usize,
        partition: &PublicationPartition,
        expected: &RemoteObservation,
    ) -> std::result::Result<PartitionReceipt, String> {
        let inspection = SubmissionReservations::inspect_existing_request(
            self.lease,
            self.namespace,
            self.scope,
            "key-a",
            &self.request,
        )
        .unwrap();
        let ExistingSubmissionInspection::Reserved {
            domain_run_id,
            journal_admission: JournalAdmission::Ready,
            ..
        } = inspection
        else {
            panic!("fake partition {index} preceded C.1 Ready");
        };
        assert!(
            journal::read::<Intent>(self.lease, &domain_run_id, "intent")
                .unwrap()
                .is_some()
        );
        assert!(
            journal::read::<Step>(self.lease, &domain_run_id, &pending_name(index))
                .unwrap()
                .is_some()
        );
        self.inner.apply(target, index, partition, expected)
    }
}

#[test]
fn full_plan_records_each_partition_and_replays_after_lease_exit_without_port_calls() {
    let project = tempfile::tempdir().unwrap();
    let (subject, plan) = prepared();
    let selected = scope();
    let lease = LifecycleLease::acquire(project.path()).unwrap();
    let namespace = SubmissionReservations::issue_namespace(&lease, &selected, 60_000).unwrap();
    let mut fake = ReadyChecked {
        inner: FakeRemote::new(&plan),
        lease: &lease,
        namespace: &namespace,
        scope: &selected,
        request: select(&selected, &subject, &plan, &expected())
            .unwrap()
            .request,
    };
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
    let Inspection::Completed {
        run_id,
        partitions,
        replayed: false,
    } = first
    else {
        panic!("complete fake plan did not finish");
    };
    assert_eq!(partitions, plan.partitions.len());
    assert_eq!(fake.inner.calls, (0..partitions).collect::<Vec<_>>());
    assert_eq!(
        SubmissionReservations::journal_admission(&lease, &namespace, &selected, "key-a", &run_id)
            .unwrap(),
        JournalAdmission::Ready
    );
    for index in 0..partitions {
        assert!(
            journal::read::<Step>(&lease, &run_id, &pending_name(index))
                .unwrap()
                .is_some()
        );
        assert!(
            journal::read::<Step>(&lease, &run_id, &landed_name(index))
                .unwrap()
                .is_some()
        );
    }
    assert!(
        journal::read::<FinalResult>(&lease, &run_id, "result")
            .unwrap()
            .is_some()
    );
    let mut fake = fake.inner;
    drop(lease);
    let lease = LifecycleLease::acquire(project.path()).unwrap();
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
    assert!(matches!(replay, Inspection::Completed { run_id: same,
        partitions: count, replayed: true } if same == run_id && count == partitions));
    assert_eq!(fake.calls.len(), partitions);
}

#[test]
fn lost_reply_at_every_partition_retains_exact_partial_and_never_reexecutes() {
    let (subject, plan) = prepared();
    for lost_at in 0..plan.partitions.len() {
        let project = tempfile::tempdir().unwrap();
        let selected = scope();
        let lease = LifecycleLease::acquire(project.path()).unwrap();
        let namespace = SubmissionReservations::issue_namespace(&lease, &selected, 60_000).unwrap();
        let mut fake = FakeRemote::new(&plan);
        fake.lost_at = Some(lost_at);
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
        let Inspection::Partial {
            run_id,
            landed,
            pending: Some(index),
            uncertain: true,
        } = first
        else {
            panic!("lost reply at {lost_at} did not retain typed uncertainty");
        };
        assert_eq!((landed, index), (lost_at, lost_at));
        assert_eq!(fake.calls, (0..=lost_at).collect::<Vec<_>>());
        assert!(
            journal::read::<Step>(&lease, &run_id, &pending_name(lost_at))
                .unwrap()
                .is_some()
        );
        assert!(
            journal::read::<Step>(&lease, &run_id, &landed_name(lost_at))
                .unwrap()
                .is_none()
        );
        if lost_at + 1 < plan.partitions.len() {
            assert!(
                journal::read::<Step>(&lease, &run_id, &pending_name(lost_at + 1))
                    .unwrap()
                    .is_none()
            );
        }
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
            matches!(replay, Inspection::Partial { run_id: same, landed: count,
            pending: Some(next), uncertain: true } if same == run_id && count == lost_at && next == lost_at)
        );
        assert_eq!(fake.calls.len(), lost_at + 1);
    }
}

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
fn incomplete_subject_and_drifted_generation_refuse_before_later_fake_effects() {
    let project = tempfile::tempdir().unwrap();
    let (mut subject, plan) = prepared();
    let selected = scope();
    let lease = LifecycleLease::acquire(project.path()).unwrap();
    let namespace = SubmissionReservations::issue_namespace(&lease, &selected, 60_000).unwrap();
    let mut fake = FakeRemote::new(&plan);
    fake.forbidden = true;
    subject.targets.pop();
    assert!(
        submit_fake_full_plan(
            &lease,
            &namespace,
            &selected,
            "key-a",
            &subject,
            &plan,
            &expected(),
            &mut fake
        )
        .is_err()
    );
    assert!(fake.calls.is_empty());

    let (subject, plan) = prepared();
    fake = FakeRemote::new(&plan);
    fake.drift_before = Some(3);
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
    assert!(matches!(
        first,
        Inspection::Partial {
            landed: 3,
            pending: Some(3),
            uncertain: true,
            ..
        }
    ));
    assert_eq!(fake.calls, vec![0, 1, 2]);
}
