use super::*;
use vibe_wire::generated::publication_effect_plan::{
    PublicationObservedAsset, PublicationObservedLatestRelease,
    PublicationObservedLatestReleaseAbsent, PublicationObservedLatestReleasePresent,
    PublicationObservedReleasePresent, PublicationObservedTagPresent,
};

const SUBJECT: &str = include_str!(
    "../../../../formats/corpora/publication_subject/e1/valid/complete-five-targets.json"
);
const NEW: &str = include_str!(
    "../../../../formats/corpora/publication_effect_plan/e3/valid/new-draft-complete.json"
);

pub(super) fn expected() -> PublicationEffectPlanExpectations<'static> {
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
pub(super) fn prepared() -> (PublicationSubject, PublicationEffectPlan) {
    (
        serde_json::from_str(SUBJECT).unwrap(),
        serde_json::from_str(NEW).unwrap(),
    )
}
pub(super) fn scope() -> SubmissionScope {
    SubmissionScope {
        context: "fixture-host-context".into(),
        target: "github.com/vibevm/vibevm".into(),
        principal: "fixture-maintainer".into(),
    }
}

pub(super) struct FakeRemote {
    current: RemoteObservation,
    pub(super) calls: Vec<usize>,
    pub(super) lost_at: Option<usize>,
    drift_before: Option<usize>,
    observations: usize,
    pub(super) forbidden: bool,
    unchanged_objects_at: Option<usize>,
}
impl FakeRemote {
    pub(super) fn new(plan: &PublicationEffectPlan) -> Self {
        Self {
            current: RemoteObservation {
                generation_ref: plan.remote_generation_ref.clone(),
                tag: plan.observed_tag.clone(),
                release: plan.observed_release.clone(),
                latest_release: plan.observed_latest_release.clone(),
                last: None,
            },
            calls: Vec::new(),
            lost_at: None,
            drift_before: None,
            observations: 0,
            forbidden: false,
            unchanged_objects_at: None,
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
        let next_generation = if transition::effectful(partition) {
            format!("fake-generation-{}", index + 1)
        } else {
            self.current.generation_ref.clone()
        };
        let receipt = PartitionReceipt {
            index,
            partition_sha256: digest,
            resulting_generation_ref: next_generation,
            receipt_ref: format!("fake-receipt-{index}"),
        };
        self.calls.push(index);
        if self.unchanged_objects_at != Some(index) {
            match partition {
                PublicationPartition::Tag(step) => {
                    if matches!(step.action, vibe_wire::generated::publication_effect_plan::PublicationPartitionTagAction::Create) {
                        self.current.tag = PublicationObservedTag::Present(Box::new(PublicationObservedTagPresent {
                            reference: format!("refs/tags/{}", step.tag), object_kind: "commit".into(),
                            object_sha: step.desired_commit.clone(),
                        }));
                    }
                }
                PublicationPartition::Release(step) => {
                    self.current.release = PublicationObservedRelease::Present(Box::new(PublicationObservedReleasePresent {
                        assets: Vec::new(), draft: step.desired_draft,
                        prerelease: step.desired_prerelease,
                        target_commitish: step.desired_target_commit.clone(),
                        name: Some(step.desired_name.clone()),
                        body: Some(step.desired_body.clone()),
                        generation_ref: receipt.resulting_generation_ref.clone(), inventory_complete: true,
                        release_id: "700".into(), tag: step.desired_tag.clone(),
                    }));
                }
                PublicationPartition::PlatformAssets(step) => {
                    add_assets(&mut self.current.release, &step.assets, index, &receipt.resulting_generation_ref);
                }
                PublicationPartition::Cleanup(_) => {}
                PublicationPartition::FinalAssets(step) => {
                    add_assets(&mut self.current.release, &step.assets, index, &receipt.resulting_generation_ref);
                }
                PublicationPartition::Finalize(step) => {
                    if matches!(step.action, vibe_wire::generated::publication_effect_plan::PublicationPartitionFinalizeAction::Publish) {
                        let PublicationObservedRelease::Present(row) = &mut self.current.release else { panic!("fake draft absent") };
                        row.draft = false;
                        row.generation_ref = receipt.resulting_generation_ref.clone();
                        self.current.latest_release = PublicationObservedLatestRelease::Present(Box::new(
                            PublicationObservedLatestReleasePresent { release_id: row.release_id.clone(), tag: row.tag.clone() },
                        ));
                    }
                }
            }
        }
        self.current.generation_ref = receipt.resulting_generation_ref.clone();
        self.current.last = Some(receipt.clone());
        if self.lost_at == Some(index) {
            return Err("response lost after fake effect".into());
        }
        Ok(receipt)
    }
}

fn add_assets(
    release: &mut PublicationObservedRelease,
    assets: &[vibe_wire::generated::publication_effect_plan::PublicationAssetEffect],
    index: usize,
    generation: &str,
) {
    let PublicationObservedRelease::Present(row) = release else {
        panic!("fake draft absent")
    };
    for (offset, asset) in assets.iter().enumerate() {
        row.assets.push(PublicationObservedAsset {
            asset_id: (1000 + index * 10 + offset).to_string(),
            name: asset.name.clone(),
            size_bytes: asset.desired_size_bytes.clone(),
            sha256: asset.desired_sha256.clone(),
        });
    }
    row.generation_ref = generation.into();
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
        request: select(&lease, &selected, &subject, &plan, &expected(), None)
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
        if !transition::effectful(&plan.partitions[index]) {
            let landed = journal::read::<Step>(&lease, &run_id, &landed_name(index))
                .unwrap()
                .unwrap();
            assert_eq!(
                landed.resulting_generation_ref.as_deref(),
                Some(landed.expected_generation_ref.as_str()),
                "validated no-op partition {index} must not invent generation churn"
            );
        }
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
fn callback_and_generation_cannot_complete_unchanged_tag_release_or_assets() {
    let (subject, plan) = prepared();
    let effectful = plan
        .partitions
        .iter()
        .enumerate()
        .filter_map(|(index, partition)| {
            matches!(
                partition,
                PublicationPartition::Tag(_)
                    | PublicationPartition::Release(_)
                    | PublicationPartition::PlatformAssets(_)
                    | PublicationPartition::FinalAssets(_)
            )
            .then_some(index)
        })
        .collect::<Vec<_>>();
    assert!(!effectful.is_empty());
    for index in effectful {
        let project = tempfile::tempdir().unwrap();
        let selected = scope();
        let lease = LifecycleLease::acquire(project.path()).unwrap();
        let namespace = SubmissionReservations::issue_namespace(&lease, &selected, 60_000).unwrap();
        let mut fake = FakeRemote::new(&plan);
        fake.unchanged_objects_at = Some(index);
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
            pending: Some(stopped),
            uncertain: true,
        } = first
        else {
            panic!("unchanged objects at partition {index} falsely completed");
        };
        assert_eq!((landed, stopped), (index, index));
        assert_eq!(fake.calls, (0..=index).collect::<Vec<_>>());
        assert!(
            journal::read::<Step>(&lease, &run_id, &landed_name(index))
                .unwrap()
                .is_none()
        );
        if index + 1 < plan.partitions.len() {
            assert!(
                journal::read::<Step>(&lease, &run_id, &pending_name(index + 1))
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
            pending: Some(next), uncertain: true } if same == run_id && count == index && next == index)
        );
        assert_eq!(fake.calls.len(), index + 1);
    }
}

#[test]
fn callback_and_generation_cannot_complete_unchanged_publish_visibility() {
    let (subject, original) = prepared();
    let mut publish_expected = expected();
    publish_expected.publish = true;
    let remote = crate::publication_effect_plan::CapturedPublicationRemote {
        generation_ref: original.remote_generation_ref,
        tag: original.observed_tag,
        release: original.observed_release,
        observed_latest_release: original.observed_latest_release,
    };
    let plan = crate::publication_effect_plan::plan_from_captured_remote(
        &subject,
        &publish_expected,
        &remote,
    )
    .unwrap()
    .plan;
    let visibility_index = plan.partitions.len() - 1;
    let project = tempfile::tempdir().unwrap();
    let selected = scope();
    let lease = LifecycleLease::acquire(project.path()).unwrap();
    let namespace = SubmissionReservations::issue_namespace(&lease, &selected, 60_000).unwrap();
    let mut fake = FakeRemote::new(&plan);
    fake.unchanged_objects_at = Some(visibility_index);
    let first = submit_fake_full_plan(
        &lease,
        &namespace,
        &selected,
        "key-a",
        &subject,
        &plan,
        &publish_expected,
        &mut fake,
    )
    .unwrap();
    assert!(
        matches!(first, Inspection::Partial { landed, pending: Some(index),
        uncertain: true, .. } if landed == visibility_index && index == visibility_index)
    );
    assert_eq!(fake.calls.len(), plan.partitions.len());
    fake.forbidden = true;
    let replay = submit_fake_full_plan(
        &lease,
        &namespace,
        &selected,
        "key-a",
        &subject,
        &plan,
        &publish_expected,
        &mut fake,
    )
    .unwrap();
    assert!(
        matches!(replay, Inspection::Partial { landed, pending: Some(index),
        uncertain: true, .. } if landed == visibility_index && index == visibility_index)
    );
}

#[test]
fn fake_full_plan_refuses_foreign_latest_before_reservation_or_effect() {
    let (subject, plan) = prepared();
    let baseline = FakeRemote::new(&plan).current;
    assert!(initial_matches(&baseline, &plan));
    for foreign in [
        PublicationObservedLatestRelease::Absent(Box::new(
            PublicationObservedLatestReleaseAbsent {},
        )),
        PublicationObservedLatestRelease::Present(Box::new(
            PublicationObservedLatestReleasePresent {
                release_id: "102".into(),
                tag: "v1.0.1".into(),
            },
        )),
        PublicationObservedLatestRelease::Present(Box::new(
            PublicationObservedLatestReleasePresent {
                release_id: "101".into(),
                tag: "v1.0.2".into(),
            },
        )),
    ] {
        let project = tempfile::tempdir().unwrap();
        let selected = scope();
        let lease = LifecycleLease::acquire(project.path()).unwrap();
        let namespace = SubmissionReservations::issue_namespace(&lease, &selected, 60_000).unwrap();
        let mut fake = FakeRemote::new(&plan);
        fake.current.latest_release = foreign;
        assert!(!initial_matches(&fake.current, &plan));
        assert!(matches!(
            submit_fake_full_plan(
                &lease,
                &namespace,
                &selected,
                "latest-drift",
                &subject,
                &plan,
                &expected(),
                &mut fake,
            ),
            Err(Error::Refused(_))
        ));
        assert!(fake.calls.is_empty());
    }
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
