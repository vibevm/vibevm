//! Cold, read-only ordered inspection for the 38 fake-only asset call records.
//! No production remote effect is exposed here.

use vibe_lifecycle::{LifecycleLease, SubmissionNamespace, SubmissionScope};
use vibe_wire::generated::publication_effect_plan::{
    PublicationEffectPlan, PublicationObservedRelease,
};
use vibe_wire::generated::publication_subject::PublicationSubject;

use crate::publication_effect_plan::PublicationEffectPlanExpectations;

use super::super::journal;
use super::super::{Error, Result};
use super::assets::{self, AssetAction, AssetLanded, AssetPending, PlannedAssetCall};
use super::{
    CallAction, CallInspection, CallObservation, Landed, decompose, inspect_calls, suffix,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum AssetInspection {
    Next {
        call: PlannedAssetCall,
        release_id: String,
        asset_id: Option<String>,
        before: CallObservation,
    },
    Uncertain {
        ordinal: usize,
    },
    /// All 38 asset calls landed; cleanup and finalize still have no adapter.
    Unavailable {
        landed: usize,
    },
}

fn future_records(lease: &LifecycleLease, run_id: &str, calls: &[PlannedAssetCall]) -> bool {
    calls.iter().any(|call| {
        ["pending", "landed"].into_iter().any(|phase| {
            !matches!(
                journal::read_bytes(lease, run_id, &assets::asset_suffix(call.ordinal, phase)),
                Ok(None)
            )
        })
    })
}

#[allow(clippy::too_many_arguments)]
pub(super) fn inspect_assets(
    lease: &LifecycleLease,
    namespace: &SubmissionNamespace,
    scope: &SubmissionScope,
    key: &str,
    run_id: &str,
    subject: &PublicationSubject,
    plan: &PublicationEffectPlan,
    expected: &PublicationEffectPlanExpectations<'_>,
) -> Result<AssetInspection> {
    // The prefix inspector includes C.1 Ready and the production cold vault +
    // approval checks. The cfg(test) fake lane is confined to its test capsule.
    let prefix = decompose(subject, plan, expected)?;
    match inspect_calls(
        lease, namespace, scope, key, run_id, subject, plan, expected,
    )? {
        CallInspection::Unavailable { landed, .. } if landed == prefix.calls.len() => {}
        CallInspection::Uncertain { ordinal } => return Ok(AssetInspection::Uncertain { ordinal }),
        _ => return Ok(AssetInspection::Unavailable { landed: 0 }),
    }
    let calls = assets::decompose_asset_calls(subject, plan, expected, run_id)?;
    let release_call = prefix
        .calls
        .last()
        .ok_or_else(|| Error::Refused("missing CreateDraft".into()))?;
    let Some(release_landed) =
        journal::read::<Landed>(lease, run_id, &suffix(release_call.ordinal, "landed"))?
    else {
        return Ok(AssetInspection::Uncertain {
            ordinal: calls[0].ordinal,
        });
    };
    let release_id = release_landed.receipt_id;
    let release_valid = matches!(&release_landed.after.release, PublicationObservedRelease::Present(row)
        if row.release_id == release_id && row.draft && row.inventory_complete
            && row.generation_ref == release_landed.after.generation_ref && row.assets.is_empty());
    if !assets::canonical_id(&release_id)
        || !release_valid
        || !matches!(release_call.action, CallAction::CreateDraft { .. })
        || calls[0].predecessor_release_call_id != release_call.call_id
    {
        return Ok(AssetInspection::Uncertain {
            ordinal: calls[0].ordinal,
        });
    }
    let mut prior = release_landed.after;
    let mut asset_id = None;
    for (index, call) in calls.iter().enumerate() {
        let pending_name = assets::asset_suffix(call.ordinal, "pending");
        let landed_name = assets::asset_suffix(call.ordinal, "landed");
        let pending = match journal::read::<AssetPending>(lease, run_id, &pending_name) {
            Ok(row) => row,
            Err(_) => {
                return Ok(AssetInspection::Uncertain {
                    ordinal: call.ordinal,
                });
            }
        };
        let Some(pending) = pending else {
            if !matches!(journal::read_bytes(lease, run_id, &landed_name), Ok(None))
                || future_records(lease, run_id, &calls[index + 1..])
            {
                return Ok(AssetInspection::Uncertain {
                    ordinal: call.ordinal,
                });
            }
            return Ok(AssetInspection::Next {
                call: call.clone(),
                release_id,
                asset_id,
                before: prior,
            });
        };
        if pending.call != *call
            || pending.run_id != run_id
            || pending.release_id != release_id
            || pending.asset_id != asset_id
            || !assets::same_observation(&pending.before, &prior)
        {
            return Ok(AssetInspection::Uncertain {
                ordinal: call.ordinal,
            });
        }
        let landed = match journal::read::<AssetLanded>(lease, run_id, &landed_name) {
            Ok(Some(row)) => row,
            _ => {
                return Ok(AssetInspection::Uncertain {
                    ordinal: call.ordinal,
                });
            }
        };
        if !assets::verify_transition(&pending, &landed) {
            return Ok(AssetInspection::Uncertain {
                ordinal: call.ordinal,
            });
        }
        asset_id = match call.action {
            AssetAction::UploadTemporary => Some(landed.receipt_id.clone()),
            AssetAction::RenameTemporary => None,
        };
        prior = landed.after;
    }
    Ok(AssetInspection::Unavailable {
        landed: calls.len(),
    })
}
