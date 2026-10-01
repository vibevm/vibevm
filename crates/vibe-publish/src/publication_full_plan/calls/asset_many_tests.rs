use super::asset_history::{self, AssetInspection};
use super::assets::{self, AssetAction, AssetLanded, AssetPending};
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
    host: CallObservation,
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
            latest_release_id: None,
            last: None,
        };
        let prepare::PreparedJournal::New { run_id, .. } = prepare::prepare_journal_fake(
            &lease,
            &namespace,
            &scope,
            "many-key",
            &subject,
            &plan,
            &expected(),
            selected,
            || Ok(initial),
        )
        .unwrap() else {
            panic!("fresh C.1 run required")
        };
        let schedule = decompose(&subject, &plan, &expected()).unwrap();
        let mut before = CallObservation {
            generation_ref: plan.remote_generation_ref.clone(),
            tag: plan.observed_tag.clone(),
            release: plan.observed_release.clone(),
        };
        for call in &schedule.calls {
            let pending = Pending {
                format: PENDING_FORMAT.into(),
                run_id: run_id.clone(),
                call: call.clone(),
                before: before.clone(),
                expected_observation_sha256: sha(&serde_json::to_vec(&before).unwrap()),
            };
            journal::publish(&lease, &run_id, &suffix(call.ordinal, "pending"), &pending).unwrap();
            let mut after = before.clone();
            after.generation_ref = format!("prefix-generation-{}", call.ordinal);
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
                CallAction::CreateDraft { tag, .. } => {
                    after.release = PublicationObservedRelease::Present(Box::new(
                        PublicationObservedReleasePresent {
                            assets: vec![],
                            draft: true,
                            generation_ref: after.generation_ref.clone(),
                            inventory_complete: true,
                            release_id: "700".into(),
                            tag: tag.clone(),
                        },
                    ));
                    "700".to_owned()
                }
                _ => panic!("new-draft fixture expected"),
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
            host: before,
        }
    }

    fn inspect(&self) -> AssetInspection {
        asset_history::inspect_assets(
            &self.lease,
            &self.namespace,
            &self.scope,
            "many-key",
            &self.run_id,
            &self.subject,
            &self.plan,
            &expected(),
        )
        .unwrap()
    }
    fn record(&self, ordinal: usize, phase: &str) -> Option<Vec<u8>> {
        journal::read_bytes(
            &self.lease,
            &self.run_id,
            &assets::asset_suffix(ordinal, phase),
        )
        .unwrap()
    }
}

/// One fake API call. All payloads are metadata-only fixture values, never
/// distribution ZIP bytes. The production module exposes no effect port.
fn drive_one(
    world: &World,
    host: &mut CallObservation,
    lose_response: bool,
) -> Result<AssetInspection> {
    drive_one_with_corruption(world, host, lose_response, false, false, false)
}

fn drive_one_with_corruption(
    world: &World,
    host: &mut CallObservation,
    lose_response: bool,
    duplicate_prior_id: bool,
    reorder_prior: bool,
    change_digest: bool,
) -> Result<AssetInspection> {
    let AssetInspection::Next {
        call,
        release_id,
        asset_id,
        before,
    } = world.inspect()
    else {
        return Ok(world.inspect());
    };
    let ordinal = call.ordinal;
    // Independent pre-effect observation, including generation and full list.
    let observed_before = host.clone();
    if !assets::same_observation(&observed_before, &before) {
        return Err(Error::Refused("asset remote precondition drift".into()));
    }
    let pending = AssetPending {
        format: "vibevm.publication.asset-call-pending/e1".into(),
        run_id: world.run_id.clone(),
        call: call.clone(),
        release_id,
        asset_id: asset_id.clone(),
        before_sha256: sha(&serde_json::to_vec(&before)?),
        before,
    };
    journal::publish(
        &world.lease,
        &world.run_id,
        &assets::asset_suffix(ordinal, "pending"),
        &pending,
    )?;
    // The fake callback changes only the target draft's one asset row.
    let receipt_id = asset_id.unwrap_or_else(|| (701 + call.asset_index).to_string());
    host.generation_ref = format!("asset-generation-{ordinal}");
    let PublicationObservedRelease::Present(release) = &mut host.release else {
        unreachable!()
    };
    release.generation_ref = host.generation_ref.clone();
    let row = PublicationObservedAsset {
        asset_id: receipt_id.clone(),
        name: match call.action {
            AssetAction::UploadTemporary => call.temporary_name.clone(),
            AssetAction::RenameTemporary => call.canonical_name.clone(),
        },
        sha256: call.sha256.clone(),
        size_bytes: call.size_bytes.to_string(),
    };
    match call.action {
        AssetAction::UploadTemporary => release.assets.push(row),
        AssetAction::RenameTemporary => {
            let prior = release
                .assets
                .iter_mut()
                .find(|asset| asset.asset_id == receipt_id)
                .expect("preflight frozen uploaded ID");
            *prior = row;
        }
    }
    if reorder_prior {
        release.assets[..call.asset_index].reverse();
    }
    if lose_response {
        return Ok(world.inspect());
    }
    // Separate read of the observed post-state before publishing Landed.
    let mut observed_after = host.clone();
    if duplicate_prior_id {
        let PublicationObservedRelease::Present(release) = &mut observed_after.release else {
            panic!()
        };
        let first_id = release.assets[0].asset_id.clone();
        release.assets[call.asset_index].asset_id = first_id;
    }
    if change_digest {
        let PublicationObservedRelease::Present(release) = &mut observed_after.release else {
            panic!()
        };
        let row = release
            .assets
            .iter_mut()
            .find(|row| row.asset_id == receipt_id)
            .unwrap();
        row.sha256 = "sha256:".to_owned() + &"f".repeat(64);
    }
    let landed = AssetLanded {
        format: "vibevm.publication.asset-call-landed/e1".into(),
        run_id: world.run_id.clone(),
        pending_sha256: sha(&serde_json::to_vec(&pending)?),
        receipt_id,
        after_sha256: sha(&serde_json::to_vec(&observed_after)?),
        after: observed_after,
    };
    if !assets::verify_transition(&pending, &landed) {
        return Ok(AssetInspection::Uncertain { ordinal });
    }
    journal::publish(
        &world.lease,
        &world.run_id,
        &assets::asset_suffix(ordinal, "landed"),
        &landed,
    )?;
    Ok(world.inspect())
}

fn drive_until(world: &World, host: &mut CallObservation, target: usize) -> AssetInspection {
    loop {
        match world.inspect() {
            AssetInspection::Next { ref call, .. } if call.ordinal < target => {
                let next = drive_one(world, host, false).unwrap();
                assert!(!matches!(next, AssetInspection::Uncertain { .. }));
            }
            other => return other,
        }
    }
}

#[test]
fn middle_and_last_pair_preserve_every_prior_canonical_asset() {
    let world = World::new();
    let mut host = world.host.clone();
    let AssetInspection::Next { call, before, .. } = drive_until(&world, &mut host, 20) else {
        panic!()
    };
    assert_eq!((call.ordinal, call.asset_index), (20, 9));
    let PublicationObservedRelease::Present(release) = before.release else {
        panic!()
    };
    assert_eq!(release.assets.len(), 9);
    let AssetInspection::Next { call, before, .. } = drive_until(&world, &mut host, 38) else {
        panic!()
    };
    assert_eq!((call.ordinal, call.asset_index), (38, 18));
    let PublicationObservedRelease::Present(release) = before.release else {
        panic!()
    };
    assert_eq!(release.assets.len(), 18);
    assert!(
        matches!(drive_one(&world, &mut host, false).unwrap(), AssetInspection::Next { call, .. } if call.ordinal == 39)
    );
    assert_eq!(
        drive_one(&world, &mut host, false).unwrap(),
        AssetInspection::Unavailable { landed: 38 }
    );
    let PublicationObservedRelease::Present(release) = host.release else {
        panic!()
    };
    assert_eq!(release.assets.len(), 19);
    assert!(matches!(
        inspect_calls(
            &world.lease,
            &world.namespace,
            &world.scope,
            "many-key",
            &world.run_id,
            &world.subject,
            &world.plan,
            &expected()
        )
        .unwrap(),
        CallInspection::Unavailable { landed: 2, .. }
    ));
}

#[test]
fn late_lost_response_never_replays_or_opens_next_call() {
    let world = World::new();
    let mut host = world.host.clone();
    assert!(
        matches!(drive_until(&world, &mut host, 22), AssetInspection::Next { call, .. } if call.ordinal == 22)
    );
    assert_eq!(
        drive_one(&world, &mut host, true).unwrap(),
        AssetInspection::Uncertain { ordinal: 22 }
    );
    assert!(world.record(22, "pending").is_some());
    assert!(world.record(22, "landed").is_none());
    assert!(world.record(23, "pending").is_none());
    assert_eq!(
        drive_one(&world, &mut host, false).unwrap(),
        AssetInspection::Uncertain { ordinal: 22 }
    );
}

#[test]
fn late_lost_rename_response_never_opens_following_upload() {
    let world = World::new();
    let mut host = world.host.clone();
    assert!(
        matches!(drive_until(&world, &mut host, 23), AssetInspection::Next { call, .. } if call.ordinal == 23)
    );
    assert_eq!(
        drive_one(&world, &mut host, true).unwrap(),
        AssetInspection::Uncertain { ordinal: 23 }
    );
    assert!(world.record(23, "pending").is_some());
    assert!(world.record(23, "landed").is_none());
    assert!(world.record(24, "pending").is_none());
    assert_eq!(
        drive_one(&world, &mut host, false).unwrap(),
        AssetInspection::Uncertain { ordinal: 23 }
    );
}

#[test]
fn later_generation_or_inventory_drift_refuses_before_pending() {
    let world = World::new();
    let mut host = world.host.clone();
    assert!(
        matches!(drive_until(&world, &mut host, 16), AssetInspection::Next { call, .. } if call.ordinal == 16)
    );
    host.generation_ref.push_str("-foreign");
    assert!(matches!(
        drive_one(&world, &mut host, false),
        Err(Error::Refused(_))
    ));
    assert!(world.record(16, "pending").is_none());
    let mut host = match world.inspect() {
        AssetInspection::Next { before, .. } => before,
        _ => panic!(),
    };
    let PublicationObservedRelease::Present(release) = &mut host.release else {
        panic!()
    };
    release.assets.pop();
    assert!(matches!(
        drive_one(&world, &mut host, false),
        Err(Error::Refused(_))
    ));
    assert!(world.record(16, "pending").is_none());
    let mut host = match world.inspect() {
        AssetInspection::Next { before, .. } => before,
        _ => panic!(),
    };
    let PublicationObservedRelease::Present(release) = &mut host.release else {
        panic!()
    };
    release.assets[0].sha256 = "sha256:".to_owned() + &"f".repeat(64);
    assert!(matches!(
        drive_one(&world, &mut host, false),
        Err(Error::Refused(_))
    ));
    assert!(world.record(16, "pending").is_none());
}

#[test]
fn duplicate_asset_id_in_middle_post_observation_cannot_land() {
    let world = World::new();
    let mut host = world.host.clone();
    assert!(
        matches!(drive_until(&world, &mut host, 12), AssetInspection::Next { call, .. } if call.ordinal == 12)
    );
    assert_eq!(
        drive_one_with_corruption(&world, &mut host, false, true, false, false).unwrap(),
        AssetInspection::Uncertain { ordinal: 12 }
    );
    assert!(world.record(12, "pending").is_some());
    assert!(world.record(12, "landed").is_none());
    assert!(world.record(13, "pending").is_none());
}

#[test]
fn reordered_prior_assets_remain_valid_across_upload_and_rename() {
    let world = World::new();
    let mut host = world.host.clone();
    assert!(
        matches!(drive_until(&world, &mut host, 12), AssetInspection::Next { call, .. } if call.ordinal == 12)
    );
    assert!(matches!(
        drive_one_with_corruption(&world, &mut host, false, false, true, false).unwrap(),
        AssetInspection::Next { call, .. } if call.ordinal == 13
    ));
    // Provider listing order changes once more before rename preflight.
    let PublicationObservedRelease::Present(release) = &mut host.release else {
        panic!()
    };
    release.assets.reverse();
    assert!(matches!(drive_one(&world, &mut host, false).unwrap(),
        AssetInspection::Next { call, .. } if call.ordinal == 14));
    assert!(matches!(drive_one(&world, &mut host, false).unwrap(),
        AssetInspection::Next { call, .. } if call.ordinal == 15));
}

#[test]
fn changed_middle_asset_digest_in_post_observation_cannot_land() {
    let world = World::new();
    let mut host = world.host.clone();
    assert!(
        matches!(drive_until(&world, &mut host, 12), AssetInspection::Next { call, .. } if call.ordinal == 12)
    );
    assert_eq!(
        drive_one_with_corruption(&world, &mut host, false, false, false, true).unwrap(),
        AssetInspection::Uncertain { ordinal: 12 }
    );
    assert!(world.record(12, "pending").is_some());
    assert!(world.record(12, "landed").is_none());
    assert!(world.record(13, "pending").is_none());
}

#[test]
fn future_middle_call_record_blocks_earlier_asset_call() {
    let world = World::new();
    journal::publish(
        &world.lease,
        &world.run_id,
        &assets::asset_suffix(20, "pending"),
        &serde_json::json!({"foreign": true}),
    )
    .unwrap();
    assert_eq!(world.inspect(), AssetInspection::Uncertain { ordinal: 2 });
    assert!(world.record(2, "pending").is_none());
}
