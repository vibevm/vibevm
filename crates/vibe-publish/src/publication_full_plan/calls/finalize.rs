//! Private one-call visibility journal. No live finalize transport exists.
//! Legacy is the selected callback value; it grants no latest drift policy.
use serde::{Deserialize, Serialize};
use vibe_wire::generated::publication_effect_plan::{
    PublicationObservedRelease, PublicationPartition, PublicationPartitionFinalizeAction,
    PublicationPartitionFinalizeMakeLatest,
};

use super::super::{Error, Result, journal};
use super::assets;
use super::noop_completion::{self, NoOpInspection};
use super::{CallObservation, sha};
use crate::publication_full_plan::session::ReadyRun;

const DOMAIN: &[u8] = b"vibevm.publication.finalize-call/e1\0";
const PENDING_FORMAT: &str = "vibevm.publication.finalize-pending/e1";
const LANDED_FORMAT: &str = "vibevm.publication.finalize-landed/e1";

/// Opaque lost/invalid response; neither state inspection nor this error
/// supplies a receipt or authorizes replay.
pub(super) struct UncertainResponse;
pub(super) trait FinalizePort {
    fn finalize(
        &self,
        release_id: &str,
        action: &PublicationPartitionFinalizeAction,
        make_latest: &PublicationPartitionFinalizeMakeLatest,
    ) -> std::result::Result<String, UncertainResponse>;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct FinalizeCall {
    pub ordinal: usize,
    pub call_id: String,
    semantic_plan_sha256: String,
    predecessor_call_id: String,
    partition_ordinal: usize,
    release_id: String,
    action: PublicationPartitionFinalizeAction,
    make_latest: PublicationPartitionFinalizeMakeLatest,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Pending {
    format: String,
    run_id: String,
    call: FinalizeCall,
    before_sha256: String,
    before: CallObservation,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Landed {
    format: String,
    run_id: String,
    pending_sha256: String,
    receipt_id: String,
    after_sha256: String,
    pub after: CallObservation,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum FinalizeInspection {
    Next {
        call: FinalizeCall,
    },
    Landed {
        call: FinalizeCall,
        after: CallObservation,
    },
    Uncertain,
    Unavailable,
}
pub(super) fn selected(session: &ReadyRun<'_>) -> bool {
    matches!(session.plan.partitions.last(), Some(PublicationPartition::Finalize(row))
        if row.action == PublicationPartitionFinalizeAction::Publish
            && row.make_latest == PublicationPartitionFinalizeMakeLatest::Legacy)
}
pub(super) fn suffix(ordinal: usize, phase: &str) -> String {
    format!("finalize-{ordinal:03}-{phase}")
}
pub(super) fn ordinal(session: &ReadyRun<'_>) -> Result<usize> {
    Ok(noop_completion::schedule(session)?
        .and_then(|rows| rows.last().map(|c| c.ordinal + 1))
        .unwrap_or(0))
}
fn retained(session: &ReadyRun<'_>) -> Result<Option<(FinalizeCall, CallObservation)>> {
    if !selected(session)
        || noop_completion::inspect_proved(session)? != NoOpInspection::CheckpointsLanded
    {
        return Ok(None);
    }
    let noops = noop_completion::schedule(session)?
        .ok_or_else(|| Error::Refused("missing cleanup".into()))?;
    let cleanup = &noops[0];
    let Some(last): Option<noop_completion::Landed> = journal::read(
        session.lease,
        &session.run_id,
        &noop_completion::suffix(cleanup.ordinal, "landed"),
    )?
    else {
        return Ok(None);
    };
    let PublicationObservedRelease::Present(release) = &last.after.release else {
        return Ok(None);
    };
    if !release.draft
        || !release.inventory_complete
        || release.assets.len() != 19
        || release.generation_ref != last.after.generation_ref
        || last.after.latest_release != session.plan.observed_latest_release
        || !assets::canonical_id(&release.release_id)
    {
        return Ok(None);
    }
    let mut call = FinalizeCall {
        ordinal: cleanup.ordinal + 1,
        call_id: String::new(),
        semantic_plan_sha256: session.terminal_proof().intent.semantic_plan_sha256.clone(),
        predecessor_call_id: cleanup.call_id.clone(),
        partition_ordinal: session.plan.partitions.len() - 1,
        release_id: release.release_id.clone(),
        action: PublicationPartitionFinalizeAction::Publish,
        make_latest: PublicationPartitionFinalizeMakeLatest::Legacy,
    };
    let mut bytes = DOMAIN.to_vec();
    for frame in [
        session.run_id.as_bytes().to_vec(),
        serde_json::to_vec(&call)?,
    ] {
        bytes.extend_from_slice(&(frame.len() as u64).to_be_bytes());
        bytes.extend_from_slice(&frame);
    }
    call.call_id = sha(&bytes);
    Ok(Some((call, last.after)))
}
fn verifies(before: &CallObservation, after: &CallObservation, receipt: &str) -> bool {
    let (PublicationObservedRelease::Present(a), PublicationObservedRelease::Present(b)) =
        (&before.release, &after.release)
    else {
        return false;
    };
    if !a.draft
        || b.draft
        || !a.inventory_complete
        || !b.inventory_complete
        || a.assets.len() != 19
        || b.assets.len() != 19
        || receipt != a.release_id
        || b.release_id != a.release_id
        || a.generation_ref != before.generation_ref
        || b.generation_ref != after.generation_ref
        || before.generation_ref == after.generation_ref
        || after.generation_ref.is_empty()
    {
        return false;
    }
    // Normalize only the approved visibility change and its consistent
    // generation. Exact metadata, tag, inventory and latest still compare.
    let mut expected = before.clone();
    expected.generation_ref = after.generation_ref.clone();
    let PublicationObservedRelease::Present(row) = &mut expected.release else {
        unreachable!()
    };
    row.draft = false;
    row.generation_ref = after.generation_ref.clone();
    assets::same_observation(&expected, after)
}
fn exists(session: &ReadyRun<'_>, ordinal: usize, phase: &str) -> bool {
    !matches!(
        journal::read_bytes(session.lease, &session.run_id, &suffix(ordinal, phase)),
        Ok(None)
    )
}
pub(super) fn inspect_proved(session: &ReadyRun<'_>) -> Result<FinalizeInspection> {
    if !selected(session) {
        return Ok(FinalizeInspection::Unavailable);
    }
    let ordinal = ordinal(session)?;
    let Some((call, prior)) = retained(session)? else {
        return Ok(
            if exists(session, ordinal, "pending") || exists(session, ordinal, "landed") {
                FinalizeInspection::Uncertain
            } else {
                FinalizeInspection::Unavailable
            },
        );
    };
    // Reject a second finalize call or legacy noop finalize occupying this ordinal.
    if ["pending", "landed"]
        .into_iter()
        .any(|phase| exists(session, ordinal + 1, phase))
    {
        return Ok(FinalizeInspection::Uncertain);
    }
    let pending =
        match journal::read::<Pending>(session.lease, &session.run_id, &suffix(ordinal, "pending"))
        {
            Ok(Some(row)) => row,
            Ok(None) if !exists(session, ordinal, "landed") => {
                return Ok(FinalizeInspection::Next { call });
            }
            _ => return Ok(FinalizeInspection::Uncertain),
        };
    if pending.format != PENDING_FORMAT
        || pending.run_id != session.run_id
        || pending.call != call
        || pending.before_sha256 != sha(&serde_json::to_vec(&pending.before)?)
        || !assets::same_observation(&pending.before, &prior)
    {
        return Ok(FinalizeInspection::Uncertain);
    }
    let landed =
        match journal::read::<Landed>(session.lease, &session.run_id, &suffix(ordinal, "landed")) {
            Ok(Some(row)) => row,
            _ => return Ok(FinalizeInspection::Uncertain),
        };
    if landed.format != LANDED_FORMAT
        || landed.run_id != session.run_id
        || landed.pending_sha256 != sha(&serde_json::to_vec(&pending)?)
        || landed.after_sha256 != sha(&serde_json::to_vec(&landed.after)?)
        || !verifies(&pending.before, &landed.after, &landed.receipt_id)
    {
        return Ok(FinalizeInspection::Uncertain);
    }
    Ok(FinalizeInspection::Landed {
        call,
        after: landed.after,
    })
}
pub(super) fn drive_one(
    session: &ReadyRun<'_>,
    reader: &mut dyn crate::publication_full_plan::remote_observation::RemoteReadPort,
    port: &dyn FinalizePort,
) -> Result<FinalizeInspection> {
    let call = match inspect_proved(session)? {
        FinalizeInspection::Next { call } => call,
        other => return Ok(other),
    };
    let Some((_, prior)) = retained(session)? else {
        return Ok(FinalizeInspection::Uncertain);
    };
    let before = noop_completion::observed(reader, &session.subject.source.commit)
        .map_err(|_| Error::Refused("finalize before-image unavailable before Pending".into()))?;
    if !assets::same_observation(&before, &prior) {
        return Err(Error::Refused(
            "finalize precondition drifted before Pending".into(),
        ));
    }
    session.recheck_small()?;
    let pending = Pending {
        format: PENDING_FORMAT.into(),
        run_id: session.run_id.clone(),
        call: call.clone(),
        before_sha256: sha(&serde_json::to_vec(&before)?),
        before,
    };
    let pending_bytes = match journal::publish(
        session.lease,
        &session.run_id,
        &suffix(call.ordinal, "pending"),
        &pending,
    ) {
        Ok(bytes) => bytes,
        Err(_) => return Ok(FinalizeInspection::Uncertain),
    };
    let pending_unchanged = || {
        matches!(journal::read_bytes(session.lease, &session.run_id,
            &suffix(call.ordinal, "pending")), Ok(Some(bytes)) if bytes == pending_bytes)
    };
    if session.recheck_small().is_err() || !pending_unchanged() {
        return Ok(FinalizeInspection::Uncertain);
    }
    let receipt_id = match port.finalize(&call.release_id, &call.action, &call.make_latest) {
        Ok(id) => id,
        Err(_) => return Ok(FinalizeInspection::Uncertain),
    };
    let after = match noop_completion::observed(reader, &session.subject.source.commit) {
        Ok(row) => row,
        Err(_) => return Ok(FinalizeInspection::Uncertain),
    };
    if !verifies(&pending.before, &after, &receipt_id)
        || session.recheck_small().is_err()
        || !pending_unchanged()
    {
        return Ok(FinalizeInspection::Uncertain);
    }
    let landed = Landed {
        format: LANDED_FORMAT.into(),
        run_id: session.run_id.clone(),
        pending_sha256: sha(&pending_bytes),
        receipt_id,
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
        return Ok(FinalizeInspection::Uncertain);
    }
    inspect_proved(session)
}
#[cfg(test)]
#[path = "finalize_tests.rs"]
mod tests;
