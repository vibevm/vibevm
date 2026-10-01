use super::asset_history::{self, AssetInspection};
use super::assets::{self, AssetLanded, AssetPending};
use super::*;
use crate::publication_full_plan::{
    RemoteObservation, prepare,
    tests::{expected, prepared, scope},
};
use vibe_lifecycle::SubmissionReservations;
use vibe_wire::generated::publication_effect_plan::{
    PublicationObservedAsset, PublicationObservedReleasePresent, PublicationObservedTagPresent,
};

struct World {
    _root: tempfile::TempDir,
    lease: LifecycleLease,
    namespace: SubmissionNamespace,
    scope: SubmissionScope,
    run_id: String,
    subject: PublicationSubject,
    plan: PublicationEffectPlan,
    before_asset: CallObservation,
}

impl World {
    fn new() -> Self {
        let (subject, plan) = prepared();
        let root = tempfile::tempdir().unwrap();
        let lease = LifecycleLease::acquire(root.path()).unwrap();
        let scope = scope();
        let namespace = SubmissionReservations::issue_namespace(&lease, &scope, 60_000).unwrap();
        let selected =
            super::super::select(&lease, &scope, &subject, &plan, &expected(), None).unwrap();
        let initial = RemoteObservation {
            generation_ref: plan.remote_generation_ref.clone(),
            tag: plan.observed_tag.clone(),
            release: plan.observed_release.clone(),
            latest_release: plan.observed_latest_release.clone(),
            last: None,
        };
        let prepare::PreparedJournal::New { run_id, .. } = prepare::prepare_journal_fake(
            &lease,
            &namespace,
            &scope,
            "e2-key",
            &subject,
            &plan,
            &expected(),
            selected,
            || Ok(initial),
        )
        .unwrap() else {
            panic!("fresh C.1 Ready required")
        };
        let schedule = decompose(&subject, &plan, &expected()).unwrap();
        let mut before = CallObservation {
            generation_ref: plan.remote_generation_ref.clone(),
            tag: plan.observed_tag.clone(),
            release: plan.observed_release.clone(),
            latest_release: plan.observed_latest_release.clone(),
        };
        for call in &schedule.calls {
            let pending = Pending {
                format: PENDING_FORMAT.into(),
                run_id: run_id.clone(),
                call: call.clone(),
                expected_observation_sha256: sha(&serde_json::to_vec(&before).unwrap()),
                before: before.clone(),
            };
            journal::publish(&lease, &run_id, &suffix(call.ordinal, "pending"), &pending).unwrap();
            let mut after = before.clone();
            after.generation_ref = format!("prefix-e2-{}", call.ordinal);
            let receipt = match &call.action {
                CallAction::CreateTag { tag, commit } => {
                    after.tag =
                        PublicationObservedTag::Present(Box::new(PublicationObservedTagPresent {
                            reference: format!("refs/tags/{tag}"),
                            object_kind: "commit".into(),
                            object_sha: commit.clone(),
                        }));
                    "tag-created".to_owned()
                }
                CallAction::CreateDraft {
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
                _ => panic!("new draft prefix expected"),
            };
            assert!(verified_transition(call, &before, &after, &receipt));
            let landed = Landed {
                format: LANDED_FORMAT.into(),
                run_id: run_id.clone(),
                pending_sha256: sha(&serde_json::to_vec(&pending).unwrap()),
                receipt_id: receipt,
                after_observation_sha256: sha(&serde_json::to_vec(&after).unwrap()),
                after: after.clone(),
            };
            journal::publish(&lease, &run_id, &suffix(call.ordinal, "landed"), &landed).unwrap();
            before = after;
        }
        Self {
            _root: root,
            lease,
            namespace,
            scope,
            run_id,
            subject,
            plan,
            before_asset: before,
        }
    }

    fn calls(&self) -> Vec<assets::PlannedAssetCall> {
        assets::decompose_asset_calls(&self.subject, &self.plan, &expected(), &self.run_id).unwrap()
    }
    fn inspect(&self) -> AssetInspection {
        asset_history::inspect_assets(
            &self.lease,
            &self.namespace,
            &self.scope,
            "e2-key",
            &self.run_id,
            &self.subject,
            &self.plan,
            &expected(),
        )
        .unwrap()
    }
    fn asset_record(&self, ordinal: usize, phase: &str) -> Option<Vec<u8>> {
        journal::read_bytes(
            &self.lease,
            &self.run_id,
            &assets::asset_suffix(ordinal, phase),
        )
        .unwrap()
    }
}

fn asset_pending(
    world: &World,
    call: assets::PlannedAssetCall,
    before: CallObservation,
    asset_id: Option<&str>,
) -> AssetPending {
    AssetPending {
        format: "vibevm.publication.asset-call-pending/e2".into(),
        run_id: world.run_id.clone(),
        call,
        release_id: "700".into(),
        asset_id: asset_id.map(str::to_owned),
        before_sha256: sha(&serde_json::to_vec(&before).unwrap()),
        before,
    }
}

fn asset_landed(world: &World, pending: &AssetPending, mut after: CallObservation) -> AssetLanded {
    let receipt_id = "701".to_owned();
    after.generation_ref = format!("asset-e2-{}", pending.call.ordinal);
    let PublicationObservedRelease::Present(release) = &mut after.release else {
        panic!()
    };
    release.generation_ref = after.generation_ref.clone();
    let row = PublicationObservedAsset {
        asset_id: receipt_id.clone(),
        name: match pending.call.action {
            assets::AssetAction::UploadTemporary => pending.call.temporary_name.clone(),
            assets::AssetAction::RenameTemporary => pending.call.canonical_name.clone(),
        },
        sha256: pending.call.sha256.clone(),
        size_bytes: pending.call.size_bytes.to_string(),
    };
    match pending.call.action {
        assets::AssetAction::UploadTemporary => release.assets.push(row),
        assets::AssetAction::RenameTemporary => release.assets[0] = row,
    }
    AssetLanded {
        format: "vibevm.publication.asset-call-landed/e2".into(),
        run_id: world.run_id.clone(),
        pending_sha256: sha(&serde_json::to_vec(pending).unwrap()),
        receipt_id,
        after_sha256: sha(&serde_json::to_vec(&after).unwrap()),
        after,
    }
}

fn corrupt(mut landed: AssetLanded, field: &str) -> AssetLanded {
    let PublicationObservedRelease::Present(release) = &mut landed.after.release else {
        panic!()
    };
    match field {
        "target" => release.target_commitish = "b".repeat(40),
        "name" => release.name = Some("foreign".into()),
        "body" => release.body = Some("foreign".into()),
        "prerelease" => release.prerelease = !release.prerelease,
        _ => unreachable!(),
    }
    landed.after_sha256 = sha(&serde_json::to_vec(&landed.after).unwrap());
    landed
}

#[test]
fn upload_and_rename_metadata_drift_leave_pending_without_landed_or_future_call() {
    for phase in ["upload", "rename"] {
        for field in ["target", "name", "body", "prerelease"] {
            let world = World::new();
            let calls = world.calls();
            let upload = asset_pending(&world, calls[0].clone(), world.before_asset.clone(), None);
            journal::publish(
                &world.lease,
                &world.run_id,
                &assets::asset_suffix(upload.call.ordinal, "pending"),
                &upload,
            )
            .unwrap();
            let good_upload = asset_landed(&world, &upload, upload.before.clone());
            assert!(assets::verify_transition(&upload, &good_upload));
            if phase == "upload" {
                let bad = corrupt(good_upload, field);
                assert!(!assets::verify_transition(&upload, &bad), "upload {field}");
                assert_eq!(
                    world.inspect(),
                    AssetInspection::Uncertain {
                        ordinal: upload.call.ordinal
                    }
                );
                assert!(world.asset_record(upload.call.ordinal, "landed").is_none());
                assert!(world.asset_record(calls[1].ordinal, "pending").is_none());
                continue;
            }
            journal::publish(
                &world.lease,
                &world.run_id,
                &assets::asset_suffix(upload.call.ordinal, "landed"),
                &good_upload,
            )
            .unwrap();
            let rename = asset_pending(
                &world,
                calls[1].clone(),
                good_upload.after.clone(),
                Some("701"),
            );
            journal::publish(
                &world.lease,
                &world.run_id,
                &assets::asset_suffix(rename.call.ordinal, "pending"),
                &rename,
            )
            .unwrap();
            let good_rename = asset_landed(&world, &rename, rename.before.clone());
            assert!(assets::verify_transition(&rename, &good_rename));
            let bad = corrupt(good_rename, field);
            assert!(!assets::verify_transition(&rename, &bad), "rename {field}");
            assert_eq!(
                world.inspect(),
                AssetInspection::Uncertain {
                    ordinal: rename.call.ordinal
                }
            );
            assert!(world.asset_record(rename.call.ordinal, "landed").is_none());
            assert!(world.asset_record(calls[2].ordinal, "pending").is_none());
        }
    }
}
