use std::cell::RefCell;
use std::rc::Rc;

use super::CallObservation;
use super::noop_completion::suffix;
use super::{assets, sha};
use crate::github_release::{GithubGitObject, GithubGitRef, GithubRelease, GithubReleaseAsset};
use crate::publication_approval::ApprovalHome;
use crate::publication_full_plan::capability_tests::TinyPayloadWorld;
use crate::publication_full_plan::journal;
use crate::publication_full_plan::prepare;
use crate::publication_full_plan::remote_observation::{
    ObservationError, RemoteReadPort, RemoteSnapshot, normalize_fixed,
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

pub(super) struct World {
    pub(super) tiny: TinyPayloadWorld,
    pub(super) _root: tempfile::TempDir,
    pub(super) lease: LifecycleLease,
    pub(super) namespace: SubmissionNamespace,
    pub(super) scope: SubmissionScope,
    pub(super) home: ApprovalHome,
    pub(super) expected: crate::publication_effect_plan::PublicationEffectPlanExpectations<'static>,
    pub(super) snapshot: Rc<RefCell<RemoteSnapshot>>,
    pub(super) run_id: String,
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
    pub(super) fn new() -> Self {
        Self::new_with_prior_draft(false)
    }

    pub(super) fn new_with_prior_draft(replace: bool) -> Self {
        Self::build(replace, false)
    }

    pub(super) fn publish() -> Self {
        Self::build(false, true)
    }

    fn build(replace: bool, publish: bool) -> Self {
        let mut expected = expected();
        expected.publish = publish;
        let mut tiny = TinyPayloadWorld::new();
        if publish {
            let remote = crate::publication_effect_plan::CapturedPublicationRemote {
                generation_ref: tiny.plan.remote_generation_ref.clone(),
                tag: tiny.plan.observed_tag.clone(),
                release: tiny.plan.observed_release.clone(),
                observed_latest_release: tiny.plan.observed_latest_release.clone(),
            };
            tiny.plan = crate::publication_effect_plan::plan_from_captured_remote(
                &tiny.subject,
                &expected,
                &remote,
            )
            .unwrap()
            .plan;
        }
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
                &expected,
                &remote,
            )
            .unwrap()
            .plan;
        }
        let root = tempfile::tempdir().unwrap();
        let lease = LifecycleLease::acquire(root.path()).unwrap();
        let scope = scope();
        tiny.seed_approval_with(&lease, &scope, &expected);
        let namespace = SubmissionReservations::issue_namespace(&lease, &scope, 60_000).unwrap();
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
            crate::publication_full_plan::Inspection::JournalReady { .. }
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
        let prefix = super::decompose(&tiny.subject, &tiny.plan, &expected).unwrap();
        let mut prior = CallObservation {
            generation_ref: tiny.plan.remote_generation_ref.clone(),
            tag: tiny.plan.observed_tag.clone(),
            release: tiny.plan.observed_release.clone(),
            latest_release: tiny.plan.observed_latest_release.clone(),
        };
        for call in &prefix.calls {
            let pending = super::Pending {
                format: super::PENDING_FORMAT.into(),
                run_id: run_id.clone(),
                call: call.clone(),
                expected_observation_sha256: sha(&serde_json::to_vec(&prior).unwrap()),
                before: prior.clone(),
            };
            journal::publish(
                &lease,
                &run_id,
                &super::suffix(call.ordinal, "pending"),
                &pending,
            )
            .unwrap();
            let mut after = prior.clone();
            after.generation_ref = format!("prefix-{}", call.ordinal);
            let receipt = match &call.action {
                super::CallAction::RetainTag { .. } => {
                    after.generation_ref = prior.generation_ref.clone();
                    "tag-retained".to_owned()
                }
                super::CallAction::DeletePriorRelease { release_id, .. } => {
                    after.release = PublicationObservedRelease::Absent(Box::new(
                        vibe_wire::generated::publication_effect_plan::PublicationObservedReleaseAbsent {},
                    ));
                    release_id.clone()
                }
                super::CallAction::CreateTag { tag, commit } => {
                    after.tag =
                        PublicationObservedTag::Present(Box::new(PublicationObservedTagPresent {
                            reference: format!("refs/tags/{tag}"),
                            object_kind: "commit".into(),
                            object_sha: commit.clone(),
                        }));
                    "tag-created".to_owned()
                }
                super::CallAction::CreateDraft {
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
            assert!(super::verified_transition(call, &prior, &after, &receipt));
            let landed = super::Landed {
                format: super::LANDED_FORMAT.into(),
                run_id: run_id.clone(),
                pending_sha256: sha(&serde_json::to_vec(&pending).unwrap()),
                receipt_id: receipt,
                after_observation_sha256: sha(&serde_json::to_vec(&after).unwrap()),
                after: after.clone(),
            };
            journal::publish(
                &lease,
                &run_id,
                &super::suffix(call.ordinal, "landed"),
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

    pub(super) fn open(&self) -> SessionOpen<'_> {
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
    pub(super) fn record(&self, ordinal: usize, phase: &str) -> Option<Vec<u8>> {
        journal::read_bytes(&self.lease, &self.run_id, &suffix(ordinal, phase)).unwrap()
    }
}

pub(super) struct Reader {
    pub(super) snapshot: Rc<RefCell<RemoteSnapshot>>,
    pub(super) reads: usize,
    pub(super) fail_at: Option<usize>,
    pub(super) drift_at: Option<usize>,
    pub(super) latest_drift_from: Option<usize>,
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
