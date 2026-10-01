use super::assets::{self, AssetAction, AssetLanded, AssetPending, FirstPairInspection};
use super::*;
use crate::publication_full_plan::{
    RemoteObservation, prepare,
    tests::{expected, prepared, scope},
};
use vibe_lifecycle::SubmissionReservations;
use vibe_wire::generated::publication_effect_plan::{
    PublicationObservedAsset, PublicationObservedReleasePresent, PublicationObservedTagPresent,
};

struct Ready {
    _root: tempfile::TempDir,
    lease: LifecycleLease,
    namespace: SubmissionNamespace,
    scope: SubmissionScope,
    run_id: String,
    subject: PublicationSubject,
    plan: PublicationEffectPlan,
    host: CallObservation,
}

fn publish_prefix(
    lease: &LifecycleLease,
    run_id: &str,
    schedule: &CallSchedule,
    plan: &PublicationEffectPlan,
) -> CallObservation {
    let mut before = CallObservation {
        generation_ref: plan.remote_generation_ref.clone(),
        tag: plan.observed_tag.clone(),
        release: plan.observed_release.clone(),
    };
    for call in &schedule.calls {
        let pending = Pending {
            format: PENDING_FORMAT.into(),
            run_id: run_id.into(),
            call: call.clone(),
            expected_observation_sha256: sha(&serde_json::to_vec(&before).unwrap()),
            before: before.clone(),
        };
        journal::publish(lease, run_id, &suffix(call.ordinal, "pending"), &pending).unwrap();
        let mut after = before.clone();
        after.generation_ref = format!("fake-generation-{}", call.ordinal + 1);
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
            _ => panic!("new-draft fixture has only create calls"),
        };
        assert!(verified_transition(call, &before, &after, &receipt));
        let landed = Landed {
            format: LANDED_FORMAT.into(),
            run_id: run_id.into(),
            pending_sha256: sha(&serde_json::to_vec(&pending).unwrap()),
            receipt_id: receipt,
            after_observation_sha256: sha(&serde_json::to_vec(&after).unwrap()),
            after: after.clone(),
        };
        journal::publish(lease, run_id, &suffix(call.ordinal, "landed"), &landed).unwrap();
        before = after;
    }
    before
}

impl Ready {
    fn new() -> Self {
        let (subject, plan) = prepared();
        let root = tempfile::tempdir().unwrap();
        let lease = LifecycleLease::acquire(root.path()).unwrap();
        let scope = scope();
        let namespace = SubmissionReservations::issue_namespace(&lease, &scope, 60_000).unwrap();
        let selected =
            super::super::select(&lease, &scope, &subject, &plan, &expected(), None).unwrap();
        let observed = RemoteObservation {
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
            "asset-key",
            &subject,
            &plan,
            &expected(),
            selected,
            || Ok(observed),
        )
        .unwrap() else {
            panic!("expected fresh C.1 Ready")
        };
        let prefix = decompose(&subject, &plan, &expected()).unwrap();
        let host = publish_prefix(&lease, &run_id, &prefix, &plan);
        Self {
            _root: root,
            lease,
            namespace,
            scope,
            run_id,
            subject,
            plan,
            host,
        }
    }
    fn inspect(&self) -> FirstPairInspection {
        assets::inspect_first_pair(
            &self.lease,
            &self.namespace,
            &self.scope,
            "asset-key",
            &self.run_id,
            &self.subject,
            &self.plan,
            &expected(),
        )
        .unwrap()
    }
    fn calls(&self) -> Vec<assets::PlannedAssetCall> {
        assets::decompose_asset_calls(&self.subject, &self.plan, &expected(), &self.run_id).unwrap()
    }
}

/// Test-only fake host. It models independent observation of the exact draft
/// and one tiny asset; it is not a production GitHub adapter or payload proof.
fn drive(
    world: &Ready,
    host: &mut CallObservation,
    lose_response: bool,
) -> Result<FirstPairInspection> {
    drive_with_drift(world, host, lose_response, false)
}

fn drive_with_drift(
    world: &Ready,
    host: &mut CallObservation,
    lose_response: bool,
    corrupt_observation: bool,
) -> Result<FirstPairInspection> {
    let current = world.inspect();
    let (call, release_id, asset_id, before) = match current {
        FirstPairInspection::NextUpload { release_id, before } => {
            (world.calls()[0].clone(), release_id, None, before)
        }
        FirstPairInspection::NextRename {
            release_id,
            asset_id,
            before,
        } => (world.calls()[1].clone(), release_id, Some(asset_id), before),
        other => return Ok(other),
    };
    // Reobservation must match the exact prior Landed generation, release ID
    // and complete inventory before any Pending or fake effect.
    if *host != before {
        return Err(Error::Refused("asset preflight remote drift".into()));
    }
    let pending = AssetPending {
        format: "vibevm.publication.asset-call-pending/e1".into(),
        run_id: world.run_id.clone(),
        call: call.clone(),
        release_id: release_id.clone(),
        asset_id: asset_id.clone(),
        before_sha256: sha(&serde_json::to_vec(&before)?),
        before,
    };
    journal::publish(
        &world.lease,
        &world.run_id,
        &assets::asset_suffix(call.ordinal, "pending"),
        &pending,
    )?;
    let mut after = host.clone();
    after.generation_ref = format!("asset-generation-{}", call.ordinal);
    let PublicationObservedRelease::Present(release) = &mut after.release else {
        unreachable!()
    };
    release.generation_ref = after.generation_ref.clone();
    let receipt_id = asset_id.unwrap_or_else(|| "701".into());
    let row = PublicationObservedAsset {
        asset_id: receipt_id.clone(),
        name: match call.action {
            AssetAction::UploadTemporary => call.temporary_name.clone(),
            AssetAction::RenameTemporary => call.canonical_name.clone(),
        },
        sha256: call.sha256.clone(),
        size_bytes: call.size_bytes.to_string(),
    };
    release.assets = vec![row];
    *host = after;
    if lose_response {
        return Ok(world.inspect());
    }
    // A separate fake remote observation after the callback; callback success
    // cannot by itself establish Landed. Tests may inject foreign inventory.
    let mut observed = host.clone();
    if corrupt_observation {
        let PublicationObservedRelease::Present(release) = &mut observed.release else {
            panic!()
        };
        release.assets[0].asset_id = "999".into();
    }
    let landed = AssetLanded {
        format: "vibevm.publication.asset-call-landed/e1".into(),
        run_id: world.run_id.clone(),
        pending_sha256: sha(&serde_json::to_vec(&pending)?),
        receipt_id,
        after_sha256: sha(&serde_json::to_vec(&observed)?),
        after: observed,
    };
    if !assets::verify_transition(&pending, &landed) {
        return Ok(FirstPairInspection::Uncertain);
    }
    journal::publish(
        &world.lease,
        &world.run_id,
        &assets::asset_suffix(call.ordinal, "landed"),
        &landed,
    )?;
    Ok(world.inspect())
}

#[test]
fn exact_nineteen_pairs_are_run_scoped_and_prefix_stays_unavailable() {
    let world = Ready::new();
    let calls = world.calls();
    let catalogue =
        crate::publication_payload::expected_upload_catalogue(&world.subject, &expected().subject)
            .unwrap();
    let prefix = decompose(&world.subject, &world.plan, &expected()).unwrap();
    assert_eq!(calls.len(), 38);
    assert_eq!(calls[0].ordinal, 2);
    assert_eq!(calls[37].ordinal, 39);
    assert!(calls.chunks_exact(2).all(|pair| matches!(
        pair[0].action,
        AssetAction::UploadTemporary
    ) && matches!(
        pair[1].action,
        AssetAction::RenameTemporary
    ) && pair[1].predecessor_asset_call_id.as_deref()
        == Some(pair[0].call_id.as_str())));
    assert!(
        calls
            .iter()
            .all(|call| call.temporary_name.contains(&world.run_id))
    );
    assert_eq!(
        calls
            .iter()
            .step_by(2)
            .map(|call| call.canonical_name.as_str())
            .collect::<Vec<_>>(),
        catalogue
            .entries
            .iter()
            .map(|entry| entry.name.as_str())
            .collect::<Vec<_>>()
    );
    assert!(
        calls
            .iter()
            .all(|call| call.predecessor_release_call_id == prefix.calls[1].call_id)
    );
    let another_run = assets::decompose_asset_calls(
        &world.subject,
        &world.plan,
        &expected(),
        "00000000000000000000000000000002",
    )
    .unwrap();
    assert_ne!(calls[0].temporary_name, another_run[0].temporary_name);
    assert_ne!(calls[0].call_id, another_run[0].call_id);
    assert_eq!(calls, world.calls());
    assert!(matches!(
        inspect_calls(
            &world.lease,
            &world.namespace,
            &world.scope,
            "asset-key",
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
fn first_fake_asset_pair_lands_only_after_exact_observations() {
    let world = Ready::new();
    let mut host = world.host.clone();
    assert!(matches!(
        drive(&world, &mut host, false).unwrap(),
        FirstPairInspection::NextRename { .. }
    ));
    assert_eq!(
        drive(&world, &mut host, false).unwrap(),
        FirstPairInspection::Landed
    );
    let PublicationObservedRelease::Present(release) = host.release else {
        panic!()
    };
    assert_eq!(release.assets[0].name, world.calls()[1].canonical_name);
    assert_eq!(release.assets[0].asset_id, "701");
}

#[test]
fn lost_upload_response_is_uncertain_and_never_renames_or_replays() {
    let world = Ready::new();
    let mut host = world.host.clone();
    assert_eq!(
        drive(&world, &mut host, true).unwrap(),
        FirstPairInspection::Uncertain
    );
    assert_eq!(
        drive(&world, &mut host, false).unwrap(),
        FirstPairInspection::Uncertain
    );
    assert!(
        journal::read_bytes(
            &world.lease,
            &world.run_id,
            &assets::asset_suffix(3, "pending")
        )
        .unwrap()
        .is_none()
    );
}

#[test]
fn foreign_post_observation_leaves_only_pending_and_no_rename() {
    let world = Ready::new();
    let mut host = world.host.clone();
    assert_eq!(
        drive_with_drift(&world, &mut host, false, true).unwrap(),
        FirstPairInspection::Uncertain
    );
    assert!(
        journal::read_bytes(
            &world.lease,
            &world.run_id,
            &assets::asset_suffix(2, "pending")
        )
        .unwrap()
        .is_some()
    );
    assert!(
        journal::read_bytes(
            &world.lease,
            &world.run_id,
            &assets::asset_suffix(2, "landed")
        )
        .unwrap()
        .is_none()
    );
    assert!(
        journal::read_bytes(
            &world.lease,
            &world.run_id,
            &assets::asset_suffix(3, "pending")
        )
        .unwrap()
        .is_none()
    );
    assert_eq!(
        drive(&world, &mut host, false).unwrap(),
        FirstPairInspection::Uncertain
    );
}

#[test]
fn foreign_release_or_inventory_refuses_before_pending() {
    let world = Ready::new();
    let mut foreign = world.host.clone();
    let PublicationObservedRelease::Present(release) = &mut foreign.release else {
        panic!()
    };
    release.release_id = "999".into();
    assert!(matches!(
        drive(&world, &mut foreign, false),
        Err(Error::Refused(_))
    ));
    assert!(
        journal::read_bytes(
            &world.lease,
            &world.run_id,
            &assets::asset_suffix(2, "pending")
        )
        .unwrap()
        .is_none()
    );

    let mut host = world.host.clone();
    assert!(matches!(
        drive(&world, &mut host, false).unwrap(),
        FirstPairInspection::NextRename { .. }
    ));
    let PublicationObservedRelease::Present(release) = &mut host.release else {
        panic!()
    };
    release.assets[0].asset_id = "999".into();
    assert!(matches!(
        drive(&world, &mut host, false),
        Err(Error::Refused(_))
    ));
    assert!(
        journal::read_bytes(
            &world.lease,
            &world.run_id,
            &assets::asset_suffix(3, "pending")
        )
        .unwrap()
        .is_none()
    );
}

#[test]
fn present_prior_and_nonempty_cleanup_refuse_pure_decomposition() {
    use vibe_wire::generated::publication_effect_plan::{
        PublicationPartition, PublicationPriorAsset, PublicationPriorAssetPresent,
    };
    let (subject, mut plan) = prepared();
    let PublicationPartition::PlatformAssets(platform) = &mut plan.partitions[2] else {
        panic!()
    };
    platform.assets[0].prior =
        PublicationPriorAsset::Present(Box::new(PublicationPriorAssetPresent {
            asset_id: "99".into(),
            sha256: platform.assets[0].desired_sha256.clone(),
        }));
    assert!(
        assets::decompose_asset_calls(
            &subject,
            &plan,
            &expected(),
            "00000000000000000000000000000001"
        )
        .is_err()
    );
    let (_, mut plan) = prepared();
    let PublicationPartition::Cleanup(cleanup) = &mut plan.partitions[7] else {
        panic!()
    };
    cleanup.assets.push(PublicationObservedAsset {
        asset_id: "99".into(),
        name: "foreign".into(),
        sha256: "sha256:".to_owned() + &"a".repeat(64),
        size_bytes: "1".into(),
    });
    assert!(
        assets::decompose_asset_calls(
            &subject,
            &plan,
            &expected(),
            "00000000000000000000000000000001"
        )
        .is_err()
    );
}

#[test]
fn future_asset_record_cannot_open_first_effect() {
    let world = Ready::new();
    journal::publish(
        &world.lease,
        &world.run_id,
        &assets::asset_suffix(3, "pending"),
        &serde_json::json!({"foreign": true}),
    )
    .unwrap();
    assert_eq!(world.inspect(), FirstPairInspection::Uncertain);
}
