//! Two independent read-only call checkpoints after all 38 asset effects.
//! Empty Cleanup and RetainDraft perform no remote write. They remain
//! separately Pending/Landed and a lost read never grants replay or completion.
//! The remote port observes latest ID/tag, but these checkpoints alone
//! cannot establish the full Finalize transition or FinalResult.

use serde::{Deserialize, Serialize};
use vibe_wire::generated::publication_effect_plan::{
    PublicationObservedRelease, PublicationPartition, PublicationPartitionFinalizeAction,
    PublicationPartitionFinalizeMakeLatest,
};

use crate::publication_effect_plan::publication_effect_plan_digest;
use crate::publication_full_plan::remote_observation::{RemoteReadPort, observe_twice};
use crate::publication_full_plan::session::ReadyRun;

use super::super::journal;
use super::super::{Error, Result};
use super::asset_history::{self, AssetInspection};
use super::assets::{self, AssetLanded};
use super::{CallObservation, sha};

const DOMAIN: &[u8] = b"vibevm.publication.read-only-call/e2\0";
const PENDING_FORMAT: &str = "vibevm.publication.noop-pending/e2";
const LANDED_FORMAT: &str = "vibevm.publication.noop-landed/e2";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum Action {
    EmptyCleanup,
    RetainDraft,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct NoOpCall {
    ordinal: usize,
    partition_ordinal: usize,
    call_id: String,
    semantic_plan_sha256: String,
    predecessor_call_id: String,
    action: Action,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Pending {
    format: String,
    run_id: String,
    call: NoOpCall,
    before_sha256: String,
    before: CallObservation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Landed {
    format: String,
    run_id: String,
    pending_sha256: String,
    receipt_ref: String,
    after_sha256: String,
    after: CallObservation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::publication_full_plan) enum NoOpInspection {
    Next { step: usize },
    Uncertain { step: usize },
    CheckpointsLanded,
    Unavailable,
}

fn suffix(ordinal: usize, phase: &str) -> String {
    format!("noop-{ordinal:03}-{phase}")
}

fn call_id(
    run_id: &str,
    plan_sha: &str,
    ordinal: usize,
    partition: usize,
    predecessor: &str,
    action: &Action,
) -> Result<String> {
    let mut bytes = DOMAIN.to_vec();
    for value in [
        run_id.as_bytes().to_vec(),
        plan_sha.as_bytes().to_vec(),
        (ordinal as u64).to_be_bytes().to_vec(),
        (partition as u64).to_be_bytes().to_vec(),
        predecessor.as_bytes().to_vec(),
        serde_json::to_vec(action)?,
    ] {
        bytes.extend_from_slice(&(value.len() as u64).to_be_bytes());
        bytes.extend_from_slice(&value);
    }
    Ok(sha(&bytes))
}

fn schedule(session: &ReadyRun<'_>) -> Result<Option<[NoOpCall; 2]>> {
    let cleanup_index = 2 + session.subject.targets.len();
    let Some(PublicationPartition::Cleanup(cleanup)) = session.plan.partitions.get(cleanup_index)
    else {
        return Ok(None);
    };
    let Some(PublicationPartition::Finalize(finalize)) = session.plan.partitions.last() else {
        return Ok(None);
    };
    if !cleanup.assets.is_empty()
        || finalize.action != PublicationPartitionFinalizeAction::RetainDraft
        || finalize.make_latest != PublicationPartitionFinalizeMakeLatest::Unchanged
    {
        return Ok(None);
    }
    let assets = assets::decompose_asset_calls(
        session.subject,
        session.plan,
        session.expected,
        &session.run_id,
    )?;
    let last = assets
        .last()
        .ok_or_else(|| Error::Refused("asset call catalogue empty".into()))?;
    let plan_sha = publication_effect_plan_digest(session.plan, session.subject, session.expected)
        .map_err(Error::io)?;
    let cleanup_ordinal = last.ordinal + 1;
    let cleanup_action = Action::EmptyCleanup;
    let cleanup_call = NoOpCall {
        ordinal: cleanup_ordinal,
        partition_ordinal: cleanup_index,
        call_id: call_id(
            &session.run_id,
            &plan_sha,
            cleanup_ordinal,
            cleanup_index,
            &last.call_id,
            &cleanup_action,
        )?,
        semantic_plan_sha256: plan_sha.clone(),
        predecessor_call_id: last.call_id.clone(),
        action: cleanup_action,
    };
    let finalize_ordinal = cleanup_ordinal + 1;
    let finalize_action = Action::RetainDraft;
    let finalize_call = NoOpCall {
        ordinal: finalize_ordinal,
        partition_ordinal: session.plan.partitions.len() - 1,
        call_id: call_id(
            &session.run_id,
            &plan_sha,
            finalize_ordinal,
            session.plan.partitions.len() - 1,
            &cleanup_call.call_id,
            &finalize_action,
        )?,
        semantic_plan_sha256: plan_sha,
        predecessor_call_id: cleanup_call.call_id.clone(),
        action: finalize_action,
    };
    Ok(Some([cleanup_call, finalize_call]))
}

fn prior_asset_after(session: &ReadyRun<'_>) -> Result<Option<CallObservation>> {
    let calls = assets::decompose_asset_calls(
        session.subject,
        session.plan,
        session.expected,
        &session.run_id,
    )?;
    let last = calls
        .last()
        .ok_or_else(|| Error::Refused("asset call catalogue empty".into()))?;
    Ok(journal::read::<AssetLanded>(
        session.lease,
        &session.run_id,
        &assets::asset_suffix(last.ordinal, "landed"),
    )?
    .map(|row| row.after))
}

fn receipt(action: &Action) -> &'static str {
    match action {
        Action::EmptyCleanup => "read-only-empty-cleanup",
        Action::RetainDraft => "read-only-retain-draft",
    }
}

fn still_draft(observation: &CallObservation) -> bool {
    matches!(&observation.release, PublicationObservedRelease::Present(row)
        if row.draft && row.inventory_complete && row.assets.len() == 19)
}

fn future_record(session: &ReadyRun<'_>, calls: &[NoOpCall]) -> bool {
    calls.iter().any(|call| {
        ["pending", "landed"].into_iter().any(|phase| {
            !matches!(
                journal::read_bytes(session.lease, &session.run_id, &suffix(call.ordinal, phase)),
                Ok(None)
            )
        })
    })
}

pub(in crate::publication_full_plan) fn inspect_proved(
    session: &ReadyRun<'_>,
) -> Result<NoOpInspection> {
    if !matches!(
        asset_history::inspect_assets_proved(session)?,
        AssetInspection::Unavailable { landed: 38 }
    ) {
        return Ok(NoOpInspection::Unavailable);
    }
    let Some(calls) = schedule(session)? else {
        return Ok(NoOpInspection::Unavailable);
    };
    let Some(mut prior) = prior_asset_after(session)? else {
        return Ok(NoOpInspection::Uncertain { step: 0 });
    };
    if !still_draft(&prior) {
        return Ok(NoOpInspection::Uncertain { step: 0 });
    }
    for (step, call) in calls.iter().enumerate() {
        let pending_name = suffix(call.ordinal, "pending");
        let landed_name = suffix(call.ordinal, "landed");
        let pending = match journal::read::<Pending>(session.lease, &session.run_id, &pending_name)
        {
            Ok(row) => row,
            Err(_) => return Ok(NoOpInspection::Uncertain { step }),
        };
        let Some(pending) = pending else {
            if !matches!(
                journal::read_bytes(session.lease, &session.run_id, &landed_name),
                Ok(None)
            ) || future_record(session, &calls[step + 1..])
            {
                return Ok(NoOpInspection::Uncertain { step });
            }
            return Ok(NoOpInspection::Next { step });
        };
        if pending.format != PENDING_FORMAT
            || pending.run_id != session.run_id
            || pending.call != *call
            || pending.before_sha256 != sha(&serde_json::to_vec(&pending.before)?)
            || !assets::same_observation(&pending.before, &prior)
        {
            return Ok(NoOpInspection::Uncertain { step });
        }
        let landed = match journal::read::<Landed>(session.lease, &session.run_id, &landed_name) {
            Ok(Some(row)) => row,
            _ => return Ok(NoOpInspection::Uncertain { step }),
        };
        if landed.format != LANDED_FORMAT
            || landed.run_id != session.run_id
            || landed.pending_sha256 != sha(&serde_json::to_vec(&pending)?)
            || landed.receipt_ref != receipt(&call.action)
            || landed.after_sha256 != sha(&serde_json::to_vec(&landed.after)?)
            || !assets::same_observation(&pending.before, &landed.after)
            || !still_draft(&landed.after)
        {
            return Ok(NoOpInspection::Uncertain { step });
        }
        prior = landed.after;
    }
    if ["pending", "landed"].into_iter().any(|phase| {
        !matches!(
            journal::read_bytes(
                session.lease,
                &session.run_id,
                &suffix(calls[1].ordinal + 1, phase)
            ),
            Ok(None)
        )
    }) {
        return Ok(NoOpInspection::Uncertain { step: 2 });
    }
    Ok(NoOpInspection::CheckpointsLanded)
}

fn observed(
    reader: &mut dyn RemoteReadPort,
    source_commit: &str,
) -> std::result::Result<CallObservation, ()> {
    let remote = observe_twice(reader, source_commit).map_err(|_| ())?;
    Ok(CallObservation {
        generation_ref: remote.generation_ref,
        tag: remote.tag,
        release: remote.release,
        latest_release: remote.observed_latest_release,
    })
}

/// One no-op checkpoint. The separate Before and After each require two
/// independent complete read epochs; a missing response after Pending leaves
/// the run uncertain and never authorizes a repeated checkpoint.
pub(super) fn drive_one_read_only(
    session: &ReadyRun<'_>,
    reader: &mut dyn RemoteReadPort,
) -> Result<NoOpInspection> {
    let step = match inspect_proved(session)? {
        NoOpInspection::Next { step } => step,
        other => return Ok(other),
    };
    let Some(calls) = schedule(session)? else {
        return Ok(NoOpInspection::Unavailable);
    };
    let call = &calls[step];
    let prior = if step == 0 {
        prior_asset_after(session)?
    } else {
        journal::read::<Landed>(
            session.lease,
            &session.run_id,
            &suffix(calls[0].ordinal, "landed"),
        )?
        .map(|row| row.after)
    };
    let Some(prior) = prior else {
        return Ok(NoOpInspection::Uncertain { step });
    };
    let before = observed(reader, &session.subject.source.commit)
        .map_err(|_| Error::Refused("read-only before-image unavailable before Pending".into()))?;
    if !assets::same_observation(&before, &prior) || !still_draft(&before) {
        return Err(Error::Refused(
            "read-only remote precondition drifted before Pending".into(),
        ));
    }
    session.recheck_small()?;
    let pending = Pending {
        format: PENDING_FORMAT.into(),
        run_id: session.run_id.clone(),
        call: call.clone(),
        before_sha256: sha(&serde_json::to_vec(&before)?),
        before: before.clone(),
    };
    if journal::publish(
        session.lease,
        &session.run_id,
        &suffix(call.ordinal, "pending"),
        &pending,
    )
    .is_err()
    {
        return Ok(NoOpInspection::Uncertain { step });
    }
    if session.recheck_small().is_err() {
        return Ok(NoOpInspection::Uncertain { step });
    }
    let after = match observed(reader, &session.subject.source.commit) {
        Ok(row) => row,
        Err(_) => return Ok(NoOpInspection::Uncertain { step }),
    };
    if !assets::same_observation(&before, &after)
        || !still_draft(&after)
        || session.recheck_small().is_err()
    {
        return Ok(NoOpInspection::Uncertain { step });
    }
    let landed = Landed {
        format: LANDED_FORMAT.into(),
        run_id: session.run_id.clone(),
        pending_sha256: sha(&serde_json::to_vec(&pending)?),
        receipt_ref: receipt(&call.action).into(),
        after_sha256: sha(&serde_json::to_vec(&after)?),
        after,
    };
    if journal::publish(
        session.lease,
        &session.run_id,
        &suffix(call.ordinal, "landed"),
        &landed,
    )
    .is_err()
    {
        return Ok(NoOpInspection::Uncertain { step });
    }
    inspect_proved(session)
}

#[cfg(test)]
#[path = "noop_completion_tests.rs"]
mod tests;
