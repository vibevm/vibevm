use std::cell::RefCell;
use std::rc::Rc;

use super::*;
use crate::github_release::{GithubGitObject, GithubGitRef, GithubRelease, GithubReleaseAsset};
use crate::publication_approval::ApprovalHome;
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
use vibe_wire::generated::publication_effect_plan::{
    PublicationObservedAsset, PublicationObservedRelease, PublicationObservedReleasePresent,
    PublicationObservedTag, PublicationObservedTagPresent,
};

struct World {
    tiny: TinyPayloadWorld,
    _root: tempfile::TempDir,
    lease: LifecycleLease,
    namespace: SubmissionNamespace,
    scope: SubmissionScope,
    home: ApprovalHome,
    expected: crate::publication_effect_plan::PublicationEffectPlanExpectations<'static>,
    snapshot: Rc<RefCell<RemoteSnapshot>>,
    run_id: String,
}

fn github_snapshot(
    subject_commit: &str,
    release: &PublicationObservedReleasePresent,
) -> RemoteSnapshot {
    RemoteSnapshot {
        latest: crate::github_release::LatestReleaseObservation::Present {
            release_id: 101,
            tag: "v1.0.1".into(),
        },
        tag: Some(GithubGitRef {
            reference: "refs/tags/v1.1.0".into(),
            object: GithubGitObject {
                sha: subject_commit.into(),
                kind: "commit".into(),
                url: String::new(),
            },
        }),
        release: Some(GithubRelease {
            id: 700,
            tag_name: "v1.1.0".into(),
            target_commitish: release.target_commitish.clone(),
            name: release.name.clone(),
            body: release.body.clone(),
            draft: release.draft,
            prerelease: release.prerelease,
            html_url: String::new(),
            upload_url: String::new(),
        }),
        assets: Some(
            release
                .assets
                .iter()
                .map(|asset| GithubReleaseAsset {
                    id: asset.asset_id.parse().unwrap(),
                    name: asset.name.clone(),
                    size: asset.size_bytes.parse().unwrap(),
                    digest: Some(asset.sha256.clone()),
                    browser_download_url: String::new(),
                    content_type: "application/octet-stream".into(),
                })
                .collect(),
        ),
        streamed_digests: Vec::new(),
    }
}

impl World {
    fn new() -> Self {
        Self::new_with_prior_draft(false)
    }

    fn new_with_prior_draft(replace: bool) -> Self {
        let mut tiny = TinyPayloadWorld::new();
        if replace {
            let prior: vibe_wire::generated::publication_effect_plan::PublicationEffectPlan =
                serde_json::from_str(include_str!(
                    "../../../../../formats/corpora/publication_effect_plan/e3/valid/replace-known-draft.json"
                )).unwrap();
            let remote = crate::publication_effect_plan::CapturedPublicationRemote {
                generation_ref: prior.remote_generation_ref,
                tag: prior.observed_tag,
                release: prior.observed_release,
                observed_latest_release: prior.observed_latest_release,
            };
            tiny.plan = crate::publication_effect_plan::plan_from_captured_remote(
                &tiny.subject,
                &expected(),
                &remote,
            )
            .unwrap()
            .plan;
        }
        let root = tempfile::tempdir().unwrap();
        let lease = LifecycleLease::acquire(root.path()).unwrap();
        let scope = scope();
        tiny.seed_approval(&lease, &scope);
        let namespace = SubmissionReservations::issue_namespace(&lease, &scope, 60_000).unwrap();
        let expected = expected();
        let output = Project::open(tiny.output.path()).unwrap();
        assert!(matches!(
            prepare::prepare_local_full_plan(
                &lease,
                &namespace,
                &scope,
                "noop-key",
                &tiny.subject,
                &tiny.plan,
                &expected,
                prepare::trusted(
                    tiny.settings.path(),
                    &lease,
                    &scope,
                    &tiny.subject,
                    &tiny.plan,
                    &output,
                    tiny.finals()
                ),
            )
            .unwrap(),
            super::super::super::Inspection::JournalReady { .. }
        ));
        let home = tiny.approval_home();
        let run_id = match ReadyRun::open(
            &lease,
            &namespace,
            &scope,
            "noop-key",
            &tiny.subject,
            &tiny.plan,
            &expected,
            &home,
        )
        .unwrap()
        {
            SessionOpen::Ready(run) => run.run_id,
            _ => panic!("sealed ReadyRun absent"),
        };
        let prefix = super::super::decompose(&tiny.subject, &tiny.plan, &expected).unwrap();
        let mut prior = CallObservation {
            generation_ref: tiny.plan.remote_generation_ref.clone(),
            tag: tiny.plan.observed_tag.clone(),
            release: tiny.plan.observed_release.clone(),
            latest_release: tiny.plan.observed_latest_release.clone(),
        };
        for call in &prefix.calls {
            let pending = super::super::Pending {
                format: super::super::PENDING_FORMAT.into(),
                run_id: run_id.clone(),
                call: call.clone(),
                expected_observation_sha256: sha(&serde_json::to_vec(&prior).unwrap()),
                before: prior.clone(),
            };
            journal::publish(
                &lease,
                &run_id,
                &super::super::suffix(call.ordinal, "pending"),
                &pending,
            )
            .unwrap();
            let mut after = prior.clone();
            after.generation_ref = format!("prefix-{}", call.ordinal);
            let receipt = match &call.action {
                super::super::CallAction::RetainTag { .. } => {
                    after.generation_ref = prior.generation_ref.clone();
                    "tag-retained".to_owned()
                }
                super::super::CallAction::DeletePriorRelease { release_id, .. } => {
                    after.release = PublicationObservedRelease::Absent(Box::new(
                        vibe_wire::generated::publication_effect_plan::PublicationObservedReleaseAbsent {},
                    ));
                    release_id.clone()
                }
                super::super::CallAction::CreateTag { tag, commit } => {
                    after.tag =
                        PublicationObservedTag::Present(Box::new(PublicationObservedTagPresent {
                            reference: format!("refs/tags/{tag}"),
                            object_kind: "commit".into(),
                            object_sha: commit.clone(),
                        }));
                    "tag-created".to_owned()
                }
                super::super::CallAction::CreateDraft {
                    tag,
                    target_commit,
                    name,
                    body,
                    prerelease,
                } => {
                    after.release = PublicationObservedRelease::Present(Box::new(
                        PublicationObservedReleasePresent {
                            assets: vec![],
                            draft: true,
                            generation_ref: after.generation_ref.clone(),
                            inventory_complete: true,
                            release_id: "700".into(),
                            tag: tag.clone(),
                            target_commitish: target_commit.clone(),
                            name: Some(name.clone()),
                            body: Some(body.clone()),
                            prerelease: *prerelease,
                        },
                    ));
                    "700".to_owned()
                }
            };
            assert!(super::super::verified_transition(
                call, &prior, &after, &receipt
            ));
            let landed = super::super::Landed {
                format: super::super::LANDED_FORMAT.into(),
                run_id: run_id.clone(),
                pending_sha256: sha(&serde_json::to_vec(&pending).unwrap()),
                receipt_id: receipt,
                after_observation_sha256: sha(&serde_json::to_vec(&after).unwrap()),
                after: after.clone(),
            };
            journal::publish(
                &lease,
                &run_id,
                &super::super::suffix(call.ordinal, "landed"),
                &landed,
            )
            .unwrap();
            prior = after;
        }
        let asset_calls =
            assets::decompose_asset_calls(&tiny.subject, &tiny.plan, &expected, &run_id).unwrap();
        let mut snapshot = None;
        for call in &asset_calls {
            let pending = assets::AssetPending {
                format: assets::PENDING_FORMAT.into(),
                run_id: run_id.clone(),
                call: call.clone(),
                release_id: "700".into(),
                asset_id: matches!(call.action, assets::AssetAction::RenameTemporary)
                    .then(|| (701 + call.asset_index).to_string()),
                before_sha256: sha(&serde_json::to_vec(&prior).unwrap()),
                before: prior.clone(),
            };
            journal::publish(
                &lease,
                &run_id,
                &assets::asset_suffix(call.ordinal, "pending"),
                &pending,
            )
            .unwrap();
            let mut after = prior.clone();
            after.generation_ref = format!("asset-{}", call.ordinal);
            let PublicationObservedRelease::Present(release) = &mut after.release else {
                panic!()
            };
            release.generation_ref = after.generation_ref.clone();
            let receipt_id = (701 + call.asset_index).to_string();
            let asset = PublicationObservedAsset {
                asset_id: receipt_id.clone(),
                name: match call.action {
                    assets::AssetAction::UploadTemporary => call.temporary_name.clone(),
                    assets::AssetAction::RenameTemporary => call.canonical_name.clone(),
                },
                sha256: call.sha256.clone(),
                size_bytes: call.size_bytes.to_string(),
            };
            match call.action {
                assets::AssetAction::UploadTemporary => release.assets.push(asset),
                assets::AssetAction::RenameTemporary => release.assets[call.asset_index] = asset,
            }
            if call.ordinal == asset_calls.last().unwrap().ordinal {
                let observed = github_snapshot(&tiny.subject.source.commit, release);
                let captured =
                    normalize_fixed(observed.clone(), &tiny.subject.source.commit).unwrap();
                after.generation_ref = captured.generation_ref.clone();
                let PublicationObservedRelease::Present(release) = &mut after.release else {
                    panic!()
                };
                release.generation_ref = captured.generation_ref;
                snapshot = Some(observed);
            }
            let landed = assets::AssetLanded {
                format: assets::LANDED_FORMAT.into(),
                run_id: run_id.clone(),
                pending_sha256: sha(&serde_json::to_vec(&pending).unwrap()),
                receipt_id,
                after_sha256: sha(&serde_json::to_vec(&after).unwrap()),
                after: after.clone(),
            };
            assert!(assets::verify_transition(&pending, &landed));
            journal::publish(
                &lease,
                &run_id,
                &assets::asset_suffix(call.ordinal, "landed"),
                &landed,
            )
            .unwrap();
            prior = after;
        }
        Self {
            tiny,
            _root: root,
            lease,
            namespace,
            scope,
            home,
            expected,
            snapshot: Rc::new(RefCell::new(snapshot.unwrap())),
            run_id,
        }
    }

    fn open(&self) -> SessionOpen<'_> {
        ReadyRun::open(
            &self.lease,
            &self.namespace,
            &self.scope,
            "noop-key",
            &self.tiny.subject,
            &self.tiny.plan,
            &self.expected,
            &self.home,
        )
        .unwrap()
    }
    fn record(&self, ordinal: usize, phase: &str) -> Option<Vec<u8>> {
        journal::read_bytes(&self.lease, &self.run_id, &suffix(ordinal, phase)).unwrap()
    }
}

struct Reader {
    snapshot: Rc<RefCell<RemoteSnapshot>>,
    reads: usize,
    fail_at: Option<usize>,
    drift_at: Option<usize>,
    latest_drift_from: Option<usize>,
}
impl RemoteReadPort for Reader {
    fn read_complete(&mut self) -> std::result::Result<RemoteSnapshot, ObservationError> {
        self.reads += 1;
        if self.fail_at == Some(self.reads) {
            return Err(ObservationError::Unavailable("EOF fixture".into()));
        }
        let mut row = self.snapshot.borrow().clone();
        if self.drift_at == Some(self.reads) {
            row.release.as_mut().unwrap().body = Some("foreign".into());
        }
        if self
            .latest_drift_from
            .is_some_and(|first| self.reads >= first)
        {
            row.latest = crate::github_release::LatestReleaseObservation::Present {
                release_id: 102,
                tag: "v1.0.1".into(),
            };
        }
        Ok(row)
    }
}

#[test]
fn two_read_only_checkpoints_land_and_survive_cold_attach_without_completion_claim() {
    let world = World::new();
    let SessionOpen::Ready(session) = world.open() else {
        panic!()
    };
    assert_eq!(
        inspect_proved(&session).unwrap(),
        NoOpInspection::Next { step: 0 }
    );
    let mut reader = Reader {
        snapshot: world.snapshot.clone(),
        reads: 0,
        fail_at: None,
        drift_at: None,
        latest_drift_from: None,
    };
    assert_eq!(
        drive_one_read_only(&session, &mut reader).unwrap(),
        NoOpInspection::Next { step: 1 }
    );
    assert_eq!(
        drive_one_read_only(&session, &mut reader).unwrap(),
        NoOpInspection::CheckpointsLanded
    );
    let calls = schedule(&session).unwrap().unwrap();
    assert!(world.record(calls[0].ordinal, "landed").is_some());
    assert!(world.record(calls[1].ordinal, "landed").is_some());
    drop(session);
    let SessionOpen::Ready(cold) = world.open() else {
        panic!("not a full FinalResult")
    };
    assert_eq!(
        inspect_proved(&cold).unwrap(),
        NoOpInspection::CheckpointsLanded
    );
    let before = reader.reads;
    assert_eq!(
        drive_one_read_only(&cold, &mut reader).unwrap(),
        NoOpInspection::CheckpointsLanded
    );
    assert_eq!(
        reader.reads, before,
        "cold attach replayed read-only checkpoint"
    );
}

#[test]
fn drift_before_cleanup_refuses_without_pending() {
    let world = World::new();
    world.snapshot.borrow_mut().release.as_mut().unwrap().body = Some("foreign".into());
    let SessionOpen::Ready(session) = world.open() else {
        panic!()
    };
    let calls = schedule(&session).unwrap().unwrap();
    let mut reader = Reader {
        snapshot: world.snapshot.clone(),
        reads: 0,
        fail_at: None,
        drift_at: None,
        latest_drift_from: None,
    };
    assert!(matches!(
        drive_one_read_only(&session, &mut reader),
        Err(Error::Refused(_))
    ));
    assert!(world.record(calls[0].ordinal, "pending").is_none());
}

#[test]
fn out_of_order_or_post_pending_remote_drift_cannot_advance() {
    let world = World::new();
    let SessionOpen::Ready(session) = world.open() else {
        panic!()
    };
    let calls = schedule(&session).unwrap().unwrap();
    journal::publish(
        &world.lease,
        &world.run_id,
        &suffix(calls[1].ordinal, "pending"),
        &serde_json::json!({"foreign": "future finalize"}),
    )
    .unwrap();
    assert_eq!(
        inspect_proved(&session).unwrap(),
        NoOpInspection::Uncertain { step: 0 }
    );
    assert!(world.record(calls[0].ordinal, "pending").is_none());

    let world = World::new();
    let SessionOpen::Ready(session) = world.open() else {
        panic!()
    };
    let calls = schedule(&session).unwrap().unwrap();
    let mut reader = Reader {
        snapshot: world.snapshot.clone(),
        reads: 0,
        fail_at: None,
        drift_at: Some(4),
        latest_drift_from: None,
    };
    assert_eq!(
        drive_one_read_only(&session, &mut reader).unwrap(),
        NoOpInspection::Uncertain { step: 0 }
    );
    assert!(world.record(calls[0].ordinal, "pending").is_some());
    assert!(world.record(calls[0].ordinal, "landed").is_none());
    drop(session);
    assert!(matches!(world.open(), SessionOpen::Uncertain { .. }));
}

#[test]
fn latest_drift_after_noop_pending_never_lands_or_replays() {
    let world = World::new();
    let SessionOpen::Ready(session) = world.open() else {
        panic!()
    };
    let calls = schedule(&session).unwrap().unwrap();
    let mut reader = Reader {
        snapshot: world.snapshot.clone(),
        reads: 0,
        fail_at: None,
        drift_at: None,
        latest_drift_from: Some(3),
    };
    assert_eq!(
        drive_one_read_only(&session, &mut reader).unwrap(),
        NoOpInspection::Uncertain { step: 0 }
    );
    assert!(world.record(calls[0].ordinal, "pending").is_some());
    assert!(world.record(calls[0].ordinal, "landed").is_none());
    let reads = reader.reads;
    assert_eq!(
        drive_one_read_only(&session, &mut reader).unwrap(),
        NoOpInspection::Uncertain { step: 0 }
    );
    assert_eq!(reader.reads, reads);
    drop(session);
    assert!(matches!(world.open(), SessionOpen::Uncertain { .. }));
}

#[test]
fn eof_or_lost_reply_after_pending_is_uncertain_and_cold_attach_never_replays() {
    for failed_step in 0..2 {
        let world = World::new();
        let SessionOpen::Ready(session) = world.open() else {
            panic!()
        };
        let mut reader = Reader {
            snapshot: world.snapshot.clone(),
            reads: 0,
            fail_at: None,
            drift_at: None,
            latest_drift_from: None,
        };
        if failed_step == 1 {
            assert_eq!(
                drive_one_read_only(&session, &mut reader).unwrap(),
                NoOpInspection::Next { step: 1 }
            );
        }
        let calls = schedule(&session).unwrap().unwrap();
        reader.fail_at = Some(reader.reads + 3); // after Pending, first of two post reads
        assert_eq!(
            drive_one_read_only(&session, &mut reader).unwrap(),
            NoOpInspection::Uncertain { step: failed_step }
        );
        assert!(
            world
                .record(calls[failed_step].ordinal, "pending")
                .is_some()
        );
        assert!(world.record(calls[failed_step].ordinal, "landed").is_none());
        drop(session);
        let SessionOpen::Uncertain { .. } = world.open() else {
            panic!("pending was not retained")
        };
        assert!(world.record(calls[1].ordinal, "landed").is_none());
    }
}

#[path = "terminal_tests.rs"]
mod terminal_tests;
