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

use super::super::journal::sha;
use super::super::{Error, Result};
use super::{CallAction, CallObservation, PlannedCall, decompose};
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
    pub asset_index: usize,
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
            call.asset_index,
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
        asset_index,
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

pub(super) fn canonical_id(value: &str) -> bool {
    value
        .parse::<u64>()
        .ok()
        .is_some_and(|id| id > 0 && id.to_string() == value)
}

pub(super) fn draft<'a>(
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

fn inventory_map(
    rows: &[PublicationObservedAsset],
) -> Option<std::collections::BTreeMap<&str, &PublicationObservedAsset>> {
    let mut ids = std::collections::BTreeMap::new();
    let mut names = std::collections::BTreeSet::new();
    for row in rows {
        if !canonical_id(&row.asset_id)
            || !names.insert(row.name.as_str())
            || ids.insert(row.asset_id.as_str(), row).is_some()
        {
            return None;
        }
    }
    Some(ids)
}

/// Equivalent exact remote state despite provider listing order. Generation,
/// release identity and every unique asset ID/name/digest/size still match.
pub(super) fn same_observation(left: &CallObservation, right: &CallObservation) -> bool {
    if left.generation_ref != right.generation_ref || left.tag != right.tag {
        return false;
    }
    match (&left.release, &right.release) {
        (PublicationObservedRelease::Absent(_), PublicationObservedRelease::Absent(_)) => true,
        (PublicationObservedRelease::Present(a), PublicationObservedRelease::Present(b)) => {
            a.release_id == b.release_id
                && a.tag == b.tag
                && a.draft == b.draft
                && a.target_commitish == b.target_commitish
                && a.name == b.name
                && a.body == b.body
                && a.prerelease == b.prerelease
                && a.inventory_complete == b.inventory_complete
                && a.generation_ref == b.generation_ref
                && matches!((inventory_map(&a.assets), inventory_map(&b.assets)),
                    (Some(left), Some(right)) if left == right)
        }
        _ => false,
    }
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
    if before.target_commitish != after.target_commitish
        || before.name != after.name
        || before.body != after.body
        || before.prerelease != after.prerelease
    {
        return false;
    }
    let (Some(before_map), Some(after_map)) =
        (inventory_map(&before.assets), inventory_map(&after.assets))
    else {
        return false;
    };
    let index = call.asset_index;
    let wanted = |name: &str| PublicationObservedAsset {
        asset_id: landed.receipt_id.clone(),
        name: name.into(),
        sha256: call.sha256.clone(),
        size_bytes: call.size_bytes.to_string(),
    };
    let preserved = before_map
        .iter()
        .all(|(id, row)| after_map.get(id).is_some_and(|observed| *observed == *row));
    match call.action {
        AssetAction::UploadTemporary => {
            pending.asset_id.is_none()
                && before.assets.len() == index
                && after.assets.len() == index + 1
                && !before_map.contains_key(landed.receipt_id.as_str())
                && preserved
                && after_map
                    .get(landed.receipt_id.as_str())
                    .is_some_and(|row| **row == wanted(&call.temporary_name))
        }
        AssetAction::RenameTemporary => {
            pending.asset_id.as_deref() == Some(landed.receipt_id.as_str())
                && before.assets.len() == index + 1
                && after.assets.len() == index + 1
                && before_map
                    .get(landed.receipt_id.as_str())
                    .is_some_and(|row| **row == wanted(&call.temporary_name))
                && after_map
                    .get(landed.receipt_id.as_str())
                    .is_some_and(|row| **row == wanted(&call.canonical_name))
                && before_map
                    .iter()
                    .filter(|(id, _)| **id != landed.receipt_id.as_str())
                    .all(|(id, row)| after_map.get(id).is_some_and(|observed| *observed == *row))
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
    use super::asset_history::{AssetInspection, inspect_assets};
    Ok(
        match inspect_assets(
            lease, namespace, scope, key, run_id, subject, plan, expected,
        )? {
            AssetInspection::Next {
                call,
                release_id,
                asset_id,
                before,
            } if call.asset_index == 0 => match call.action {
                AssetAction::UploadTemporary => {
                    FirstPairInspection::NextUpload { release_id, before }
                }
                AssetAction::RenameTemporary => FirstPairInspection::NextRename {
                    release_id,
                    asset_id: asset_id.expect("upload Landed supplied ID"),
                    before,
                },
            },
            AssetInspection::Next { .. } | AssetInspection::Unavailable { landed: 38 } => {
                FirstPairInspection::Landed
            }
            AssetInspection::Uncertain { .. } => FirstPairInspection::Uncertain,
            AssetInspection::Unavailable { .. } => FirstPairInspection::Unavailable,
        },
    )
}
