use std::cell::RefCell;
use std::rc::Rc;

use crate::github_release::{GithubGitObject, GithubGitRef, GithubRelease, GithubReleaseAsset};
use crate::publication_effect_plan::{
    PublicationEffectPlanExpectations, plan_from_captured_remote,
};
use crate::publication_full_plan::capability_tests::TinyPayloadWorld;
use crate::publication_full_plan::prepare;
use crate::publication_full_plan::remote_observation::{
    ObservationError, RemoteSnapshot, normalize_fixed,
};
use crate::publication_full_plan::session::{ReadyRun, SessionOpen};
use crate::publication_full_plan::tests::{expected, scope};
use vibe_lifecycle::{
    LifecycleLease, SubmissionNamespace, SubmissionReservations, SubmissionScope,
};
use vibe_safefs::Project;

use super::*;

#[derive(Clone)]
struct FakeState {
    snapshot: RemoteSnapshot,
    reads: usize,
    writes: Vec<&'static str>,
    lost_delete: bool,
    wrong_reply: bool,
    wrong_post_body: bool,
    drift_second_read: bool,
    latest_after_call: Option<crate::github_release::LatestReleaseObservation>,
}

fn tag(commit: &str) -> GithubGitRef {
    GithubGitRef {
        reference: "refs/tags/v1.1.0".into(),
        object: GithubGitObject {
            sha: commit.into(),
            kind: "commit".into(),
            url: String::new(),
        },
    }
}

fn old_release(commit: &str) -> GithubRelease {
    GithubRelease {
        id: 123,
        tag_name: "v1.1.0".into(),
        target_commitish: commit.into(),
        name: Some("prior draft".into()),
        body: Some("prior release notes".into()),
        draft: true,
        prerelease: false,
        html_url: "https://github.com/vibevm/vibevm/releases/tag/v1.1.0".into(),
        upload_url: "https://uploads.github.com/repos/vibevm/vibevm/releases/123/assets".into(),
    }
}

fn initial(commit: &str, retain_tag: bool, replace_release: bool) -> FakeState {
    let release = replace_release.then(|| old_release(commit));
    FakeState {
        snapshot: RemoteSnapshot {
            latest: crate::github_release::LatestReleaseObservation::Present {
                release_id: 101,
                tag: "v1.0.1".into(),
            },
            tag: (retain_tag || replace_release).then(|| tag(commit)),
            release,
            assets: replace_release.then(|| {
                vec![GithubReleaseAsset {
                    id: 44,
                    name: "prior.zip".into(),
                    size: 3,
                    digest: Some(format!("sha256:{}", "a".repeat(64))),
                    browser_download_url: "https://github.com/old/prior.zip".into(),
                    content_type: "application/zip".into(),
                }]
            }),
            streamed_digests: Vec::new(),
        },
        reads: 0,
        writes: Vec::new(),
        lost_delete: false,
        wrong_reply: false,
        wrong_post_body: false,
        drift_second_read: false,
        latest_after_call: None,
    }
}

struct Reader(Rc<RefCell<FakeState>>);
impl RemoteReadPort for Reader {
    fn read_complete(&mut self) -> std::result::Result<RemoteSnapshot, ObservationError> {
        let mut state = self.0.borrow_mut();
        state.reads += 1;
        let mut snapshot = state.snapshot.clone();
        if state.drift_second_read && state.reads % 2 == 0 {
            if let Some(release) = &mut snapshot.release {
                release.html_url.push_str("?foreign=1");
            } else if let Some(tag) = &mut snapshot.tag {
                tag.reference.push_str("-foreign");
            } else {
                return Err(ObservationError::Drift);
            }
        }
        Ok(snapshot)
    }
}

struct Transport(Rc<RefCell<FakeState>>);
impl CallTransport for Transport {
    fn create_tag(&self, commit: &str) -> std::result::Result<TagResponseEvidence, String> {
        let mut state = self.0.borrow_mut();
        state.writes.push("create-tag");
        state.snapshot.tag = Some(tag(commit));
        if let Some(latest) = state.latest_after_call.clone() {
            state.snapshot.latest = latest;
        }
        Ok(TagResponseEvidence {
            reference: "refs/tags/v1.1.0".into(),
            object_sha: if state.wrong_reply {
                "b".repeat(40)
            } else {
                commit.into()
            },
        })
    }

    fn delete_release(
        &self,
        release_id: u64,
    ) -> std::result::Result<DeleteReleaseResponseEvidence, String> {
        let mut state = self.0.borrow_mut();
        state.writes.push("delete-release");
        state.snapshot.release = None;
        state.snapshot.assets = None;
        if state.lost_delete {
            Err("response lost after remote deletion".into())
        } else {
            Ok(DeleteReleaseResponseEvidence { release_id })
        }
    }

    fn create_draft(
        &self,
        selected: &DraftCallFields,
    ) -> std::result::Result<DraftResponseEvidence, String> {
        let mut state = self.0.borrow_mut();
        state.writes.push("create-draft");
        state.snapshot.release = Some(GithubRelease {
            id: 700,
            tag_name: selected.tag.clone(),
            target_commitish: selected.target_commit.clone(),
            name: Some(selected.name.clone()),
            body: Some(if state.wrong_post_body {
                "foreign successor body".into()
            } else {
                selected.body.clone()
            }),
            draft: selected.draft,
            prerelease: selected.prerelease,
            html_url: "https://github.com/vibevm/vibevm/releases/tag/v1.1.0".into(),
            upload_url: "https://uploads.github.com/repos/vibevm/vibevm/releases/700/assets".into(),
        });
        state.snapshot.assets = Some(Vec::new());
        Ok(DraftResponseEvidence {
            release_id: 700,
            tag: selected.tag.clone(),
            target_commitish: selected.target_commit.clone(),
            draft: selected.draft,
            name: selected.name.clone(),
            body: selected.body.clone(),
            prerelease: selected.prerelease,
        })
    }
}

struct Fixture {
    tiny: TinyPayloadWorld,
    _root: tempfile::TempDir,
    lease: LifecycleLease,
    namespace: SubmissionNamespace,
    scope: SubmissionScope,
    home: crate::publication_approval::ApprovalHome,
    expected: PublicationEffectPlanExpectations<'static>,
    state: Rc<RefCell<FakeState>>,
}

impl Fixture {
    fn new(retain_tag: bool, replace_release: bool) -> Self {
        let mut tiny = TinyPayloadWorld::new();
        let state = initial(&tiny.subject.source.commit, retain_tag, replace_release);
        let captured =
            normalize_fixed(state.snapshot.clone(), &tiny.subject.source.commit).unwrap();
        tiny.plan = plan_from_captured_remote(&tiny.subject, &expected(), &captured)
            .unwrap()
            .plan;
        let root = tempfile::tempdir().unwrap();
        let lease = LifecycleLease::acquire(root.path()).unwrap();
        let scope = scope();
        tiny.seed_approval(&lease, &scope);
        let namespace = SubmissionReservations::issue_namespace(&lease, &scope, 60_000).unwrap();
        let output = Project::open(tiny.output.path()).unwrap();
        let result = prepare::prepare_local_full_plan(
            &lease,
            &namespace,
            &scope,
            "driver-key",
            &tiny.subject,
            &tiny.plan,
            &expected(),
            prepare::trusted(
                tiny.settings.path(),
                &lease,
                &scope,
                &tiny.subject,
                &tiny.plan,
                &output,
                tiny.finals(),
            ),
        )
        .unwrap();
        assert!(matches!(
            result,
            crate::publication_full_plan::Inspection::JournalReady { .. }
        ));
        let home = tiny.approval_home();
        Self {
            tiny,
            _root: root,
            lease,
            namespace,
            scope,
            home,
            expected: expected(),
            state: Rc::new(RefCell::new(state)),
        }
    }

    fn session(&self) -> ReadyRun<'_> {
        match ReadyRun::open(
            &self.lease,
            &self.namespace,
            &self.scope,
            "driver-key",
            &self.tiny.subject,
            &self.tiny.plan,
            &self.expected,
            &self.home,
        )
        .unwrap()
        {
            SessionOpen::Ready(session) => session,
            _ => panic!("fixed complete fixture should open ReadyRun"),
        }
    }

    fn drive(&self, session: &ReadyRun<'_>) -> Result<CallInspection> {
        let mut reader = Reader(self.state.clone());
        let transport = Transport(self.state.clone());
        drive_one(session, &mut reader, &transport)
    }
}

#[test]
fn retain_tag_is_read_only_checkpoint_then_exact_draft_lands() {
    let fixture = Fixture::new(true, false);
    let session = fixture.session();
    assert!(matches!(
        fixture.drive(&session).unwrap(),
        CallInspection::Next { ordinal: 1 }
    ));
    assert!(fixture.state.borrow().writes.is_empty());
    assert!(matches!(
        fixture.drive(&session).unwrap(),
        CallInspection::Unavailable { landed: 2, .. }
    ));
    assert_eq!(fixture.state.borrow().writes, ["create-draft"]);
}

#[test]
fn exact_create_tag_then_draft_each_have_independent_pending_and_landed() {
    let fixture = Fixture::new(false, false);
    let session = fixture.session();
    assert!(matches!(
        fixture.drive(&session).unwrap(),
        CallInspection::Next { ordinal: 1 }
    ));
    assert_eq!(fixture.state.borrow().writes, ["create-tag"]);
    assert!(matches!(
        fixture.drive(&session).unwrap(),
        CallInspection::Unavailable { landed: 2, .. }
    ));
    assert_eq!(
        fixture.state.borrow().writes,
        ["create-tag", "create-draft"]
    );
}

#[test]
fn lost_delete_response_retains_pending_and_duplicate_never_writes_again() {
    let fixture = Fixture::new(true, true);
    let session = fixture.session();
    assert!(matches!(
        fixture.drive(&session).unwrap(),
        CallInspection::Next { ordinal: 1 }
    ));
    fixture.state.borrow_mut().lost_delete = true;
    assert!(matches!(
        fixture.drive(&session).unwrap(),
        CallInspection::Uncertain { ordinal: 1 }
    ));
    let before = fixture.state.borrow().writes.clone();
    assert_eq!(before, ["delete-release"]);
    assert!(matches!(
        fixture.drive(&session).unwrap(),
        CallInspection::Uncertain { ordinal: 1 }
    ));
    assert_eq!(fixture.state.borrow().writes, before);
}

#[test]
fn foreign_generation_refuses_before_pending_or_transport() {
    let fixture = Fixture::new(true, true);
    let session = fixture.session();
    fixture
        .state
        .borrow_mut()
        .snapshot
        .release
        .as_mut()
        .unwrap()
        .html_url
        .push_str("?foreign=1");
    assert!(fixture.drive(&session).is_err());
    assert!(fixture.state.borrow().writes.is_empty());
    assert!(matches!(
        inspect_calls_proved(&session).unwrap(),
        CallInspection::Next { ordinal: 0 }
    ));
}

#[test]
fn old_call_record_without_latest_cannot_resume_as_e3() {
    let fixture = Fixture::new(false, false);
    let session = fixture.session();
    let schedule = decompose(session.subject, session.plan, session.expected).unwrap();
    let before = CallObservation {
        generation_ref: session.plan.remote_generation_ref.clone(),
        tag: session.plan.observed_tag.clone(),
        release: session.plan.observed_release.clone(),
        latest_release: session.plan.observed_latest_release.clone(),
    };
    let pending = Pending {
        format: PENDING_FORMAT.into(),
        run_id: session.run_id.clone(),
        call: schedule.calls[0].clone(),
        expected_observation_sha256: sha(&serde_json::to_vec(&before).unwrap()),
        before,
    };
    let mut old = serde_json::to_value(pending).unwrap();
    old["format"] = "vibevm.publication.api-call-pending/e1".into();
    old["before"]
        .as_object_mut()
        .unwrap()
        .remove("latest_release");
    journal::publish(session.lease, &session.run_id, &suffix(0, "pending"), &old).unwrap();
    assert!(matches!(
        inspect_calls_proved(&session).unwrap(),
        CallInspection::Uncertain { ordinal: 0 }
    ));
    assert!(fixture.state.borrow().writes.is_empty());
}

#[test]
fn latest_id_or_tag_drift_refuses_before_pending_and_after_effect_never_lands() {
    use crate::github_release::LatestReleaseObservation;
    for foreign in [
        LatestReleaseObservation::Present {
            release_id: 102,
            tag: "v1.0.1".into(),
        },
        LatestReleaseObservation::Present {
            release_id: 101,
            tag: "v1.0.2".into(),
        },
    ] {
        let before = Fixture::new(false, false);
        let session = before.session();
        before.state.borrow_mut().snapshot.latest = foreign.clone();
        assert!(before.drive(&session).is_err());
        assert!(before.state.borrow().writes.is_empty());
        assert!(matches!(
            inspect_calls_proved(&session).unwrap(),
            CallInspection::Next { ordinal: 0 }
        ));

        let after = Fixture::new(false, false);
        let session = after.session();
        after.state.borrow_mut().latest_after_call = Some(foreign);
        assert!(matches!(
            after.drive(&session).unwrap(),
            CallInspection::Uncertain { ordinal: 0 }
        ));
        assert_eq!(after.state.borrow().writes, ["create-tag"]);
        assert!(matches!(
            inspect_calls_proved(&session).unwrap(),
            CallInspection::Uncertain { ordinal: 0 }
        ));
        assert!(matches!(
            after.drive(&session).unwrap(),
            CallInspection::Uncertain { ordinal: 0 }
        ));
        assert_eq!(after.state.borrow().writes, ["create-tag"]);
    }
}

#[test]
fn wrong_callback_or_wrong_successor_body_keeps_pending_uncertain() {
    let callback = Fixture::new(false, false);
    let session = callback.session();
    callback.state.borrow_mut().wrong_reply = true;
    assert!(matches!(
        callback.drive(&session).unwrap(),
        CallInspection::Uncertain { ordinal: 0 }
    ));
    assert_eq!(callback.state.borrow().writes, ["create-tag"]);

    let body = Fixture::new(true, false);
    let session = body.session();
    assert!(matches!(
        body.drive(&session).unwrap(),
        CallInspection::Next { ordinal: 1 }
    ));
    body.state.borrow_mut().wrong_post_body = true;
    assert!(matches!(
        body.drive(&session).unwrap(),
        CallInspection::Uncertain { ordinal: 1 }
    ));
    assert_eq!(body.state.borrow().writes, ["create-draft"]);
}

#[test]
fn independent_before_epoch_drift_refuses_without_pending() {
    let fixture = Fixture::new(true, false);
    let session = fixture.session();
    fixture.state.borrow_mut().drift_second_read = true;
    assert!(fixture.drive(&session).is_err());
    assert!(fixture.state.borrow().writes.is_empty());
    assert!(matches!(
        inspect_calls_proved(&session).unwrap(),
        CallInspection::Next { ordinal: 0 }
    ));
}
