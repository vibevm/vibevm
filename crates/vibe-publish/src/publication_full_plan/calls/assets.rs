//! Pure, run-bound asset call identities. The existing tag/release driver
//! deliberately remains unavailable at this frontier.

use serde::{Deserialize, Serialize};
use vibe_lifecycle::process::is_valid_run_id;
use vibe_wire::generated::publication_effect_plan::{
    PublicationAssetEffect, PublicationEffectPlan, PublicationPartition, PublicationPriorAsset,
};
use vibe_wire::generated::publication_subject::PublicationSubject;

use crate::publication_effect_plan::PublicationEffectPlanExpectations;
use crate::publication_payload::expected_upload_catalogue;

use super::super::journal;
use super::super::journal::sha;
use super::super::{Error, Result};
use super::{
    CallAction, CallInspection, CallObservation, Landed, PlannedCall, decompose, inspect_calls,
    suffix,
};
use vibe_lifecycle::{LifecycleLease, SubmissionNamespace, SubmissionScope};
use vibe_wire::generated::publication_effect_plan::{
    PublicationObservedAsset, PublicationObservedRelease,
};

const DOMAIN: &[u8] = b"vibevm.publication.asset-api-call/e1\0";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, tag = "kind", rename_all = "kebab-case")]
pub(super) enum AssetAction {
    UploadTemporary,
    RenameTemporary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct PlannedAssetCall {
    pub ordinal: usize,
    pub partition_ordinal: usize,
    pub call_id: String,
    pub semantic_plan_sha256: String,
    pub predecessor_release_call_id: String,
    pub predecessor_asset_call_id: Option<String>,
    pub action: AssetAction,
    pub temporary_name: String,
    pub canonical_name: String,
    pub media_type: String,
    pub size_bytes: u64,
    pub sha256: String,
}

fn frame(bytes: &mut Vec<u8>, value: &[u8]) {
    bytes.extend_from_slice(&(value.len() as u64).to_be_bytes());
    bytes.extend_from_slice(value);
}

fn identity(call: &PlannedAssetCall) -> Result<String> {
    let mut bytes = DOMAIN.to_vec();
    frame(
        &mut bytes,
        &serde_json::to_vec(&(
            &call.semantic_plan_sha256,
            call.ordinal,
            call.partition_ordinal,
            &call.predecessor_release_call_id,
            &call.predecessor_asset_call_id,
            &call.action,
            &call.temporary_name,
            &call.canonical_name,
            &call.media_type,
            call.size_bytes,
            &call.sha256,
        ))?,
    );
    Ok(sha(&bytes))
}

fn push_pair(
    calls: &mut Vec<PlannedAssetCall>,
    prefix: &[PlannedCall],
    partition_ordinal: usize,
    effect: &PublicationAssetEffect,
    run_id: &str,
    asset_index: usize,
) -> Result<()> {
    if !matches!(effect.prior, PublicationPriorAsset::Absent(_)) {
        return Err(Error::Refused("asset prior must be Absent".into()));
    }
    let release = prefix
        .last()
        .ok_or_else(|| Error::Refused("release call missing".into()))?;
    if !matches!(release.action, CallAction::CreateDraft { .. }) {
        return Err(Error::Refused("created-draft predecessor missing".into()));
    }
    // The temporary name cannot alias any canonical release asset name.
    // It is run-scoped and bounded by the validated C.1 run-id grammar.
    let temporary_name = format!(
        ".vibe-{run_id}-{asset_index:02}-{}",
        &effect.desired_sha256[7..23]
    );
    let size_bytes = effect
        .desired_size_bytes
        .parse()
        .map_err(|_| Error::Refused("invalid asset size".into()))?;
    let base = PlannedAssetCall {
        ordinal: prefix.len() + calls.len(),
        partition_ordinal,
        call_id: String::new(),
        semantic_plan_sha256: release.semantic_plan_sha256.clone(),
        predecessor_release_call_id: release.call_id.clone(),
        predecessor_asset_call_id: None,
        action: AssetAction::UploadTemporary,
        temporary_name,
        canonical_name: effect.name.clone(),
        media_type: effect.media_type.clone(),
        size_bytes,
        sha256: effect.desired_sha256.clone(),
    };
    let mut upload = base.clone();
    upload.call_id = identity(&upload)?;
    let mut rename = base;
    rename.ordinal += 1;
    rename.action = AssetAction::RenameTemporary;
    rename.predecessor_asset_call_id = Some(upload.call_id.clone());
    rename.call_id = identity(&rename)?;
    calls.extend([upload, rename]);
    Ok(())
}

/// Expand all 19 expected uploads into 38 exact calls without enabling a
/// production remote effect. The created draft is a symbolic call-id here;
/// its dynamic release ID is learned only from an independently Landed prefix.
pub(super) fn decompose_asset_calls(
    subject: &PublicationSubject,
    plan: &PublicationEffectPlan,
    expected: &PublicationEffectPlanExpectations<'_>,
    run_id: &str,
) -> Result<Vec<PlannedAssetCall>> {
    if !is_valid_run_id(run_id) {
        return Err(Error::Refused("invalid reserved run ID".into()));
    }
    let prefix = decompose(subject, plan, expected)?;
    let catalogue = expected_upload_catalogue(subject, &expected.subject).map_err(Error::io)?;
    let mut calls = Vec::with_capacity(38);
    let mut asset_index = 0;
    for (partition_ordinal, partition) in plan.partitions.iter().enumerate() {
        let assets = match partition {
            PublicationPartition::PlatformAssets(step) => Some(step.assets.as_slice()),
            PublicationPartition::FinalAssets(step) => Some(step.assets.as_slice()),
            PublicationPartition::Cleanup(step) if !step.assets.is_empty() => {
                return Err(Error::Refused("nonempty asset cleanup unsupported".into()));
            }
            _ => None,
        };
        if let Some(assets) = assets {
            for effect in assets {
                let wanted = catalogue
                    .entries
                    .get(asset_index)
                    .ok_or_else(|| Error::Refused("asset topology exceeds catalogue".into()))?;
                if effect.name != wanted.name
                    || effect.media_type != wanted.media_type
                    || effect.desired_sha256 != wanted.sha256
                    || effect.desired_size_bytes != wanted.size_bytes.to_string()
                {
                    return Err(Error::Refused(
                        "asset effect differs from exact catalogue".into(),
                    ));
                }
                push_pair(
                    &mut calls,
                    &prefix.calls,
                    partition_ordinal,
                    effect,
                    run_id,
                    asset_index,
                )?;
                asset_index += 1;
            }
        }
    }
    if asset_index != 19 || calls.len() != 38 {
        return Err(Error::Refused("unsupported asset topology".into()));
    }
    Ok(calls)
}

const PENDING_FORMAT: &str = "vibevm.publication.asset-call-pending/e1";
const LANDED_FORMAT: &str = "vibevm.publication.asset-call-landed/e1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct AssetPending {
    pub format: String,
    pub run_id: String,
    pub call: PlannedAssetCall,
    /// Dynamically frozen from CreateDraft Landed, never guessed from plan.
    pub release_id: String,
    /// Dynamically frozen from UploadTemporary Landed for rename.
    pub asset_id: Option<String>,
    pub before_sha256: String,
    pub before: CallObservation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct AssetLanded {
    pub format: String,
    pub run_id: String,
    pub pending_sha256: String,
    pub receipt_id: String,
    pub after_sha256: String,
    pub after: CallObservation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum FirstPairInspection {
    NextUpload {
        release_id: String,
        before: CallObservation,
    },
    NextRename {
        release_id: String,
        asset_id: String,
        before: CallObservation,
    },
    Landed,
    Uncertain,
    Unavailable,
}

pub(super) fn asset_suffix(ordinal: usize, phase: &str) -> String {
    format!("asset-{ordinal:03}-{phase}")
}

fn canonical_id(value: &str) -> bool {
    value
        .parse::<u64>()
        .ok()
        .is_some_and(|id| id > 0 && id.to_string() == value)
}

fn draft<'a>(
    observation: &'a CallObservation,
    release_id: &str,
) -> Option<&'a vibe_wire::generated::publication_effect_plan::PublicationObservedReleasePresent> {
    let PublicationObservedRelease::Present(value) = &observation.release else {
        return None;
    };
    (value.release_id == release_id
        && value.draft
        && value.inventory_complete
        && value.generation_ref == observation.generation_ref)
        .then_some(value)
}

pub(super) fn verify_transition(pending: &AssetPending, landed: &AssetLanded) -> bool {
    let call = &pending.call;
    if pending.format != PENDING_FORMAT
        || landed.format != LANDED_FORMAT
        || pending.run_id != landed.run_id
        || pending.before_sha256 != sha(&serde_json::to_vec(&pending.before).unwrap_or_default())
        || landed.pending_sha256 != sha(&serde_json::to_vec(pending).unwrap_or_default())
        || landed.after_sha256 != sha(&serde_json::to_vec(&landed.after).unwrap_or_default())
        || pending.before.tag != landed.after.tag
        || pending.before.generation_ref == landed.after.generation_ref
        || !canonical_id(&pending.release_id)
        || !canonical_id(&landed.receipt_id)
    {
        return false;
    }
    let (Some(before), Some(after)) = (
        draft(&pending.before, &pending.release_id),
        draft(&landed.after, &pending.release_id),
    ) else {
        return false;
    };
    match call.action {
        AssetAction::UploadTemporary => {
            pending.asset_id.is_none()
                && before.assets.is_empty()
                && after.assets
                    == vec![PublicationObservedAsset {
                        asset_id: landed.receipt_id.clone(),
                        name: call.temporary_name.clone(),
                        sha256: call.sha256.clone(),
                        size_bytes: call.size_bytes.to_string(),
                    }]
        }
        AssetAction::RenameTemporary => {
            pending.asset_id.as_deref() == Some(landed.receipt_id.as_str())
                && before.assets
                    == vec![PublicationObservedAsset {
                        asset_id: landed.receipt_id.clone(),
                        name: call.temporary_name.clone(),
                        sha256: call.sha256.clone(),
                        size_bytes: call.size_bytes.to_string(),
                    }]
                && after.assets
                    == vec![PublicationObservedAsset {
                        asset_id: landed.receipt_id.clone(),
                        name: call.canonical_name.clone(),
                        sha256: call.sha256.clone(),
                        size_bytes: call.size_bytes.to_string(),
                    }]
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn inspect_first_pair(
    lease: &LifecycleLease,
    namespace: &SubmissionNamespace,
    scope: &SubmissionScope,
    key: &str,
    run_id: &str,
    subject: &PublicationSubject,
    plan: &PublicationEffectPlan,
    expected: &PublicationEffectPlanExpectations<'_>,
) -> Result<FirstPairInspection> {
    // This uses the same strict C.1/vault/approval cold gate as the prefix.
    let prefix = decompose(subject, plan, expected)?;
    if !matches!(inspect_calls(lease, namespace, scope, key, run_id, subject, plan, expected)?,
        CallInspection::Unavailable { landed, .. } if landed == prefix.calls.len())
    {
        return Ok(FirstPairInspection::Unavailable);
    }
    let calls = decompose_asset_calls(subject, plan, expected, run_id)?;
    let release_call = prefix
        .calls
        .last()
        .ok_or_else(|| Error::Refused("missing CreateDraft".into()))?;
    let Some(release_landed) =
        journal::read::<Landed>(lease, run_id, &suffix(release_call.ordinal, "landed"))?
    else {
        return Ok(FirstPairInspection::Uncertain);
    };
    if !canonical_id(&release_landed.receipt_id)
        || draft(&release_landed.after, &release_landed.receipt_id).is_none()
        || !matches!(release_call.action, CallAction::CreateDraft { .. })
        || calls[0].predecessor_release_call_id != release_call.call_id
    {
        return Ok(FirstPairInspection::Uncertain);
    }
    let release_id = release_landed.receipt_id;
    let mut prior = release_landed.after;
    let mut asset_id = None;
    for call in calls.iter().take(2) {
        let pending_name = asset_suffix(call.ordinal, "pending");
        let landed_name = asset_suffix(call.ordinal, "landed");
        let pending = match journal::read::<AssetPending>(lease, run_id, &pending_name) {
            Ok(row) => row,
            Err(_) => return Ok(FirstPairInspection::Uncertain),
        };
        let Some(pending) = pending else {
            if !matches!(journal::read_bytes(lease, run_id, &landed_name), Ok(None))
                || calls
                    .iter()
                    .skip(call.ordinal + 1 - calls[0].ordinal)
                    .any(|future| {
                        ["pending", "landed"].into_iter().any(|phase| {
                            !matches!(
                                journal::read_bytes(
                                    lease,
                                    run_id,
                                    &asset_suffix(future.ordinal, phase)
                                ),
                                Ok(None)
                            )
                        })
                    })
            {
                return Ok(FirstPairInspection::Uncertain);
            }
            return Ok(match call.action {
                AssetAction::UploadTemporary => FirstPairInspection::NextUpload {
                    release_id,
                    before: prior,
                },
                AssetAction::RenameTemporary => FirstPairInspection::NextRename {
                    release_id,
                    asset_id: asset_id.expect("upload Landed supplied ID"),
                    before: prior,
                },
            });
        };
        if pending.call != *call
            || pending.run_id != run_id
            || pending.release_id != release_id
            || pending.before != prior
            || pending.asset_id != asset_id
        {
            return Ok(FirstPairInspection::Uncertain);
        }
        let landed = match journal::read::<AssetLanded>(lease, run_id, &landed_name) {
            Ok(Some(row)) => row,
            _ => return Ok(FirstPairInspection::Uncertain),
        };
        if !verify_transition(&pending, &landed) {
            return Ok(FirstPairInspection::Uncertain);
        }
        asset_id = Some(landed.receipt_id.clone());
        prior = landed.after;
    }
    if calls.iter().skip(2).any(|future| {
        ["pending", "landed"].into_iter().any(|phase| {
            !matches!(
                journal::read_bytes(lease, run_id, &asset_suffix(future.ordinal, phase)),
                Ok(None)
            )
        })
    }) {
        return Ok(FirstPairInspection::Uncertain);
    }
    Ok(FirstPairInspection::Landed)
}
