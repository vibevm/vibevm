use super::*;
use crate::publication_effect_plan::{CapturedPublicationRemote, plan_from_captured_remote};
use crate::publication_subject::PublicationSubjectExpectations;

const SUBJECT: &str = include_str!(
    "../../../../formats/corpora/publication_subject/e1/valid/complete-five-targets.json"
);
const NEW: &str = include_str!(
    "../../../../formats/corpora/publication_effect_plan/e3/valid/new-draft-complete.json"
);
const REPLACE: &str = include_str!(
    "../../../../formats/corpora/publication_effect_plan/e3/valid/replace-known-draft.json"
);

fn expected() -> PublicationEffectPlanExpectations<'static> {
    PublicationEffectPlanExpectations {
        subject: PublicationSubjectExpectations {
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
fn prepared() -> (
    PublicationSubject,
    PublicationEffectPlan,
    PublicationObservedTag,
) {
    let subject = serde_json::from_str(SUBJECT).unwrap();
    let plan = serde_json::from_str(NEW).unwrap();
    let replace: PublicationEffectPlan = serde_json::from_str(REPLACE).unwrap();
    assert!(matches!(
        replace.observed_tag,
        PublicationObservedTag::Present(_)
    ));
    (subject, plan, replace.observed_tag)
}
fn scope() -> SubmissionScope {
    SubmissionScope {
        context: "fixture-host-context".into(),
        target: "github.com/vibevm/vibevm".into(),
        principal: "fixture-maintainer".into(),
    }
}

struct FakeTag {
    observed: CapturedPublicationRemote,
    post_tag: PublicationObservedTag,
    creates: usize,
    observes: usize,
    lose_reply: bool,
    drift_on_second_read: bool,
    fail_second_read: bool,
    forbid_calls: bool,
}
impl FakeTag {
    fn new(plan: &PublicationEffectPlan, post_tag: PublicationObservedTag) -> Self {
        Self {
            observed: CapturedPublicationRemote {
                generation_ref: plan.remote_generation_ref.clone(),
                tag: plan.observed_tag.clone(),
                release: plan.observed_release.clone(),
                observed_latest_release: plan.observed_latest_release.clone(),
            },
            post_tag,
            creates: 0,
            observes: 0,
            lose_reply: false,
            drift_on_second_read: false,
            fail_second_read: false,
            forbid_calls: false,
        }
    }
}
impl TagCreatePort for FakeTag {
    fn observe(&mut self) -> std::result::Result<CapturedPublicationRemote, String> {
        assert!(!self.forbid_calls, "duplicate touched fake remote");
        self.observes += 1;
        if self.fail_second_read && self.observes == 2 {
            return Err("fake observation unavailable after Ready".into());
        }
        if self.drift_on_second_read && self.observes == 2 {
            self.observed.generation_ref.push_str("-drift");
        }
        Ok(self.observed.clone())
    }
    fn create_tag(
        &mut self,
        target: &PublicationRemoteTarget,
        expected: &CapturedPublicationRemote,
        tag: &str,
        commit: &str,
    ) -> std::result::Result<TagCreateReceipt, String> {
        assert!(!self.forbid_calls, "duplicate created tag");
        assert_eq!(
            (
                target.host.as_str(),
                target.owner.as_str(),
                target.repository.as_str()
            ),
            ("github.com", "vibevm", "vibevm")
        );
        if &self.observed != expected {
            return Err("fake conditional create saw another remote generation".into());
        }
        self.creates += 1;
        self.observed.tag = self.post_tag.clone();
        self.observed.generation_ref = "observed-after-tag-create".into();
        if self.lose_reply {
            return Err("reply lost after fake effect".into());
        }
        Ok(TagCreateReceipt {
            tag_ref: format!("refs/tags/{tag}"),
            commit: commit.into(),
        })
    }
}

struct ReadyChecked<'a> {
    inner: FakeTag,
    lease: &'a LifecycleLease,
    namespace: &'a SubmissionNamespace,
    scope: &'a SubmissionScope,
    request: Vec<u8>,
}
impl TagCreatePort for ReadyChecked<'_> {
    fn observe(&mut self) -> std::result::Result<CapturedPublicationRemote, String> {
        self.inner.observe()
    }
    fn create_tag(
        &mut self,
        target: &PublicationRemoteTarget,
        expected: &CapturedPublicationRemote,
        tag: &str,
        commit: &str,
    ) -> std::result::Result<TagCreateReceipt, String> {
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
            panic!("fake effect preceded C.1 Ready");
        };
        assert!(
            read::<Intent>(self.lease, &domain_run_id, "intent")
                .unwrap()
                .is_some()
        );
        assert!(
            read::<Partition>(self.lease, &domain_run_id, "pending")
                .unwrap()
                .is_some()
        );
        self.inner.create_tag(target, expected, tag, commit)
    }
}

#[test]
fn fake_tag_create_is_ready_before_effect_and_replays_after_lease_exit() {
    let project = tempfile::tempdir().unwrap();
    let (subject, plan, after_tag) = prepared();
    let selected = scope();
    let lease = LifecycleLease::acquire(project.path()).unwrap();
    let namespace = SubmissionReservations::issue_namespace(&lease, &selected, 60_000).unwrap();
    let mut fake = ReadyChecked {
        inner: FakeTag::new(&plan, after_tag),
        lease: &lease,
        namespace: &namespace,
        scope: &selected,
        request: fixed_tag(&selected, &subject, &plan, &expected())
            .unwrap()
            .request,
    };
    let first = submit_fake_tag_create(
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
    let PublicationTagInspection::Completed {
        run_id,
        replayed: false,
        ..
    } = first
    else {
        panic!("first fake tag did not complete");
    };
    assert_eq!(fake.inner.creates, 1);
    assert_eq!(
        SubmissionReservations::journal_admission(&lease, &namespace, &selected, "key-a", &run_id)
            .unwrap(),
        JournalAdmission::Ready
    );
    assert!(read::<Intent>(&lease, &run_id, "intent").unwrap().is_some());
    assert!(
        read::<Partition>(&lease, &run_id, "pending")
            .unwrap()
            .is_some()
    );
    assert!(
        read::<Partition>(&lease, &run_id, "landed")
            .unwrap()
            .is_some()
    );
    assert!(
        read::<ResultRecord>(&lease, &run_id, "result")
            .unwrap()
            .is_some()
    );
    let mut fake = fake.inner;
    drop(lease);
    let lease = LifecycleLease::acquire(project.path()).unwrap();
    fake.forbid_calls = true;
    let replay = submit_fake_tag_create(
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
        matches!(replay, PublicationTagInspection::Completed { run_id: same,
        replayed: true, .. } if same == run_id)
    );
    assert_eq!(fake.creates, 1);
}

#[test]
fn lost_fake_remote_reply_is_uncertain_and_duplicate_never_replays_effect() {
    let project = tempfile::tempdir().unwrap();
    let (subject, plan, after_tag) = prepared();
    let selected = scope();
    let lease = LifecycleLease::acquire(project.path()).unwrap();
    let namespace = SubmissionReservations::issue_namespace(&lease, &selected, 60_000).unwrap();
    let mut fake = FakeTag::new(&plan, after_tag);
    fake.lose_reply = true;
    let first = submit_fake_tag_create(
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
    let PublicationTagInspection::Uncertain {
        run_id,
        pending: true,
        landed: false,
    } = first
    else {
        panic!("lost response was not uncertain");
    };
    assert_eq!(fake.creates, 1);
    assert!(
        read::<Partition>(&lease, &run_id, "pending")
            .unwrap()
            .is_some()
    );
    assert!(
        read::<ResultRecord>(&lease, &run_id, "result")
            .unwrap()
            .is_none()
    );
    drop(lease);
    let lease = LifecycleLease::acquire(project.path()).unwrap();
    fake.forbid_calls = true;
    let replay = submit_fake_tag_create(
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
        matches!(replay, PublicationTagInspection::Uncertain { run_id: same,
        pending: true, landed: false } if same == run_id)
    );
    assert_eq!(fake.creates, 1);
}

#[test]
fn remote_generation_drift_after_ready_refuses_before_fake_effect() {
    let project = tempfile::tempdir().unwrap();
    let (subject, plan, after_tag) = prepared();
    let selected = scope();
    let lease = LifecycleLease::acquire(project.path()).unwrap();
    let namespace = SubmissionReservations::issue_namespace(&lease, &selected, 60_000).unwrap();
    let mut fake = FakeTag::new(&plan, after_tag);
    fake.drift_on_second_read = true;
    let first = submit_fake_tag_create(
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
    let PublicationTagInspection::RefusedBeforeEffect { run_id } = first else {
        panic!("pre-effect drift was not refused");
    };
    assert_eq!(fake.creates, 0);
    assert_eq!(
        SubmissionReservations::journal_admission(&lease, &namespace, &selected, "key-a", &run_id)
            .unwrap(),
        JournalAdmission::Ready
    );
    fake.forbid_calls = true;
    let replay = submit_fake_tag_create(
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
        matches!(replay, PublicationTagInspection::RefusedBeforeEffect { run_id: same } if same == run_id)
    );
}

#[test]
fn failed_second_observation_retains_run_and_never_calls_fake_effect() {
    let project = tempfile::tempdir().unwrap();
    let (subject, plan, after_tag) = prepared();
    let selected = scope();
    let lease = LifecycleLease::acquire(project.path()).unwrap();
    let namespace = SubmissionReservations::issue_namespace(&lease, &selected, 60_000).unwrap();
    let mut fake = FakeTag::new(&plan, after_tag);
    fake.fail_second_read = true;
    let first = submit_fake_tag_create(
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
    let PublicationTagInspection::Uncertain {
        run_id,
        pending: true,
        landed: false,
    } = first
    else {
        panic!("failed pre-effect read did not retain uncertain run");
    };
    assert_eq!(fake.creates, 0);
    assert_eq!(
        SubmissionReservations::journal_admission(&lease, &namespace, &selected, "key-a", &run_id)
            .unwrap(),
        JournalAdmission::Ready
    );
    fake.forbid_calls = true;
    let replay = submit_fake_tag_create(
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
        matches!(replay, PublicationTagInspection::Uncertain { run_id: same, .. } if same == run_id)
    );
}

#[test]
fn malformed_empty_partition_plan_refuses_without_panic_or_port_call() {
    let project = tempfile::tempdir().unwrap();
    let (subject, mut plan, after_tag) = prepared();
    plan.partitions.clear();
    let selected = scope();
    let lease = LifecycleLease::acquire(project.path()).unwrap();
    let namespace = SubmissionReservations::issue_namespace(&lease, &selected, 60_000).unwrap();
    let mut fake = FakeTag::new(&plan, after_tag);
    fake.forbid_calls = true;
    assert!(
        submit_fake_tag_create(
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
}

#[test]
fn same_key_changed_remote_generation_is_c1_conflict_before_fake_port() {
    let project = tempfile::tempdir().unwrap();
    let (subject, plan, after_tag) = prepared();
    let selected = scope();
    let lease = LifecycleLease::acquire(project.path()).unwrap();
    let namespace = SubmissionReservations::issue_namespace(&lease, &selected, 60_000).unwrap();
    let mut fake = FakeTag::new(&plan, after_tag);
    fake.lose_reply = true;
    assert!(matches!(
        submit_fake_tag_create(
            &lease,
            &namespace,
            &selected,
            "key-a",
            &subject,
            &plan,
            &expected(),
            &mut fake
        )
        .unwrap(),
        PublicationTagInspection::Uncertain { .. }
    ));
    let mut remote = CapturedPublicationRemote {
        generation_ref: plan.remote_generation_ref.clone(),
        tag: plan.observed_tag.clone(),
        release: plan.observed_release.clone(),
        observed_latest_release: plan.observed_latest_release.clone(),
    };
    remote.generation_ref.push_str("-next");
    let changed = plan_from_captured_remote(&subject, &expected(), &remote).unwrap();
    fake.forbid_calls = true;
    assert!(matches!(
        submit_fake_tag_create(
            &lease,
            &namespace,
            &selected,
            "key-a",
            &subject,
            &changed.plan,
            &expected(),
            &mut fake
        ),
        Err(PublicationOperationError::Submission(
            vibe_lifecycle::SubmissionError::KeyConflict
        ))
    ));
    assert_eq!(fake.creates, 1);
}

#[test]
fn missing_or_corrupt_pending_never_replays_fake_effect() {
    let project = tempfile::tempdir().unwrap();
    let (subject, plan, after_tag) = prepared();
    let selected = scope();
    let lease = LifecycleLease::acquire(project.path()).unwrap();
    let namespace = SubmissionReservations::issue_namespace(&lease, &selected, 60_000).unwrap();
    let mut fake = FakeTag::new(&plan, after_tag);
    let first = submit_fake_tag_create(
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
    let PublicationTagInspection::Completed { run_id, .. } = first else {
        panic!("fake tag was not created");
    };
    let pending = project.path().join(format!(
        ".vibe/publication-operations/{run_id}.tag-pending.json"
    ));
    fake.forbid_calls = true;
    std::fs::remove_file(&pending).unwrap();
    let missing = submit_fake_tag_create(
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
        matches!(missing, PublicationTagInspection::RecoveryRequired { run_id: same } if same == run_id)
    );
    std::fs::write(&pending, b"{").unwrap();
    let corrupt = submit_fake_tag_create(
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
        matches!(corrupt, PublicationTagInspection::Uncertain { run_id: same, pending: true, .. } if same == run_id)
    );
    assert_eq!(fake.creates, 1);
}

#[test]
fn corrupt_landed_or_result_never_replays_fake_effect() {
    let project = tempfile::tempdir().unwrap();
    let (subject, plan, after_tag) = prepared();
    let selected = scope();
    let lease = LifecycleLease::acquire(project.path()).unwrap();
    let namespace = SubmissionReservations::issue_namespace(&lease, &selected, 60_000).unwrap();
    let mut fake = FakeTag::new(&plan, after_tag);
    let first = submit_fake_tag_create(
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
    let PublicationTagInspection::Completed { run_id, .. } = first else {
        panic!("fake tag was not created");
    };
    let home = project.path().join(".vibe/publication-operations");
    let landed = home.join(format!("{run_id}.tag-landed.json"));
    let result = home.join(format!("{run_id}.tag-result.json"));
    let landed_bytes = std::fs::read(&landed).unwrap();
    fake.forbid_calls = true;
    std::fs::write(&landed, b"{").unwrap();
    let corrupt_landed = submit_fake_tag_create(
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
        matches!(corrupt_landed, PublicationTagInspection::Uncertain { run_id: same, landed: true, .. } if same == run_id)
    );
    std::fs::write(&landed, landed_bytes).unwrap();
    std::fs::write(&result, b"{").unwrap();
    let corrupt_result = submit_fake_tag_create(
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
        matches!(corrupt_result, PublicationTagInspection::Uncertain { run_id: same, landed: true, .. } if same == run_id)
    );
    assert_eq!(fake.creates, 1);
}
