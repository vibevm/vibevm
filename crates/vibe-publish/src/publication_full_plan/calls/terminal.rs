//! Private retained completion for the individual call chain. This is a
//! historical journal result, never a statement about current GitHub state.

use serde::{Deserialize, Serialize};
use vibe_wire::generated::publication_effect_plan::PublicationObservedRelease;

use crate::github_release::GithubReleaseClient;
use crate::publication_full_plan::github_remote::GithubRemoteReadPort;
use crate::publication_full_plan::remote_observation::{RemoteReadPort, observe_twice};
use crate::publication_full_plan::session::ReadyRun;
use crate::publication_payload::vault::{VaultInspection, inspect_payload_vault};

use super::super::{Error, Result, journal};
use super::asset_history::{self, AssetInspection};
use super::assets;
use super::finalize::{self, FinalizeInspection};
use super::noop_completion::{self, NoOpInspection};
use super::{CallInspection, CallObservation, decompose, inspect_calls_proved, sha, suffix};

const PENDING_FORMAT: &str = "vibevm.publication.call-terminal-pending/e1";
const RESULT_FORMAT: &str = "vibevm.publication.call-terminal-result/e1";
const CHAIN_DOMAIN: &[u8] = b"vibevm.publication.call-chain/e1\0";
const PENDING_NAME: &str = "call-terminal-pending";
const RESULT_NAME: &str = "call-terminal-result";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RecordHash {
    ordinal: usize,
    call_id: String,
    pending_sha256: String,
    landed_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ChainEvidence {
    calls: usize,
    chain_sha256: String,
    final_observation: CallObservation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Pending {
    format: String,
    run_id: String,
    c1_plan_sha256: String,
    request_sha256: String,
    semantic_plan_sha256: String,
    root_correlation_token: String,
    calls: usize,
    chain_sha256: String,
    final_observation_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TerminalResult {
    format: String,
    run_id: String,
    pending_sha256: String,
    calls: usize,
    chain_sha256: String,
    final_observation_sha256: String,
    final_observation: CallObservation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::publication_full_plan) enum TerminalInspection {
    NotReady,
    Ready { calls: usize },
    Uncertain,
    Completed { calls: usize },
}

enum Progress {
    NotReady,
    Uncertain,
    Complete(ChainEvidence),
}

fn record(
    session: &ReadyRun<'_>,
    ordinal: usize,
    call_id: &str,
    pending_name: &str,
    landed_name: &str,
) -> Result<Option<RecordHash>> {
    let Some(pending) = journal::read_bytes(session.lease, &session.run_id, pending_name)? else {
        return Ok(None);
    };
    let Some(landed) = journal::read_bytes(session.lease, &session.run_id, landed_name)? else {
        return Ok(None);
    };
    Ok(Some(RecordHash {
        ordinal,
        call_id: call_id.into(),
        pending_sha256: sha(&pending),
        landed_sha256: sha(&landed),
    }))
}

fn collect(session: &ReadyRun<'_>) -> Result<Progress> {
    // Known finalize history cannot hide behind earlier incomplete records,
    // or be ignored by a plan that selected only RetainDraft.
    let publish = finalize::selected(session);
    let ordinal = finalize::ordinal(session)?;
    let first = if publish {
        ordinal
    } else {
        ordinal.saturating_sub(1)
    };
    let has_record = (first..=ordinal + 1).any(|index| {
        ["pending", "landed"].into_iter().any(|phase| {
            !matches!(
                journal::read_bytes(
                    session.lease,
                    &session.run_id,
                    &finalize::suffix(index, phase)
                ),
                Ok(None)
            )
        })
    });
    if has_record
        && (!publish
            || !matches!(
                finalize::inspect_proved(session)?,
                FinalizeInspection::Landed { .. }
            ))
    {
        return Ok(Progress::Uncertain);
    }
    let prefix = decompose(session.subject, session.plan, session.expected)?;
    match inspect_calls_proved(session)? {
        CallInspection::Unavailable { landed, .. } if landed == prefix.calls.len() => {}
        CallInspection::Uncertain { .. } => return Ok(Progress::Uncertain),
        _ => return Ok(Progress::NotReady),
    }
    match asset_history::inspect_assets_proved(session)? {
        AssetInspection::Unavailable { landed: 38 } => {}
        AssetInspection::Uncertain { .. } => return Ok(Progress::Uncertain),
        _ => return Ok(Progress::NotReady),
    }
    match noop_completion::inspect_proved(session)? {
        NoOpInspection::CheckpointsLanded => {}
        NoOpInspection::Uncertain { .. } => return Ok(Progress::Uncertain),
        _ => return Ok(Progress::NotReady),
    }
    let asset_calls = assets::decompose_asset_calls(
        session.subject,
        session.plan,
        session.expected,
        &session.run_id,
    )?;
    let Some(noops) = noop_completion::schedule(session)? else {
        return Ok(Progress::Uncertain);
    };
    let mut rows = Vec::with_capacity(prefix.calls.len() + asset_calls.len() + noops.len());
    for call in &prefix.calls {
        let Some(row) = record(
            session,
            call.ordinal,
            &call.call_id,
            &suffix(call.ordinal, "pending"),
            &suffix(call.ordinal, "landed"),
        )?
        else {
            return Ok(Progress::Uncertain);
        };
        rows.push(row);
    }
    for call in &asset_calls {
        let Some(row) = record(
            session,
            call.ordinal,
            &call.call_id,
            &assets::asset_suffix(call.ordinal, "pending"),
            &assets::asset_suffix(call.ordinal, "landed"),
        )?
        else {
            return Ok(Progress::Uncertain);
        };
        rows.push(row);
    }
    for call in &noops {
        let Some(row) = record(
            session,
            call.ordinal,
            &call.call_id,
            &noop_completion::suffix(call.ordinal, "pending"),
            &noop_completion::suffix(call.ordinal, "landed"),
        )?
        else {
            return Ok(Progress::Uncertain);
        };
        rows.push(row);
    }
    let final_observation = if finalize::selected(session) {
        let (call, after) = match finalize::inspect_proved(session)? {
            FinalizeInspection::Landed { call, after } => (call, after),
            FinalizeInspection::Next { .. } | FinalizeInspection::Unavailable => {
                return Ok(Progress::NotReady);
            }
            FinalizeInspection::Uncertain => return Ok(Progress::Uncertain),
        };
        let Some(row) = record(
            session,
            call.ordinal,
            &call.call_id,
            &finalize::suffix(call.ordinal, "pending"),
            &finalize::suffix(call.ordinal, "landed"),
        )?
        else {
            return Ok(Progress::Uncertain);
        };
        rows.push(row);
        after
    } else {
        let Some(last): Option<noop_completion::Landed> = journal::read(
            session.lease,
            &session.run_id,
            &noop_completion::suffix(noops.last().unwrap().ordinal, "landed"),
        )?
        else {
            return Ok(Progress::Uncertain);
        };
        last.after
    };
    if rows
        .iter()
        .enumerate()
        .any(|(ordinal, row)| row.ordinal != ordinal)
    {
        return Ok(Progress::Uncertain);
    }
    let valid_final = matches!(&final_observation.release,
        PublicationObservedRelease::Present(release)
            if release.draft == !finalize::selected(session) && release.inventory_complete && release.assets.len() == 19);
    if !valid_final || final_observation.latest_release != session.plan.observed_latest_release {
        return Ok(Progress::Uncertain);
    }
    let mut bytes = CHAIN_DOMAIN.to_vec();
    bytes.extend_from_slice(&serde_json::to_vec(&(
        &session.run_id,
        &session.terminal_proof().intent.semantic_plan_sha256,
        &rows,
    ))?);
    Ok(Progress::Complete(ChainEvidence {
        calls: rows.len(),
        chain_sha256: sha(&bytes),
        final_observation,
    }))
}

fn expected_pending(session: &ReadyRun<'_>, evidence: &ChainEvidence) -> Result<Pending> {
    let proof = session.terminal_proof();
    Ok(Pending {
        format: PENDING_FORMAT.into(),
        run_id: session.run_id.clone(),
        c1_plan_sha256: proof.plan_sha256.clone(),
        request_sha256: proof.request_sha256.clone(),
        semantic_plan_sha256: proof.intent.semantic_plan_sha256.clone(),
        root_correlation_token: proof.capsule.root_correlation_token.clone(),
        calls: evidence.calls,
        chain_sha256: evidence.chain_sha256.clone(),
        final_observation_sha256: sha(&serde_json::to_vec(&evidence.final_observation)?),
    })
}

pub(in crate::publication_full_plan) fn inspect_proved(
    session: &ReadyRun<'_>,
) -> Result<TerminalInspection> {
    let pending_bytes = match journal::read_bytes(session.lease, &session.run_id, PENDING_NAME) {
        Ok(value) => value,
        Err(_) => return Ok(TerminalInspection::Uncertain),
    };
    let result_bytes = match journal::read_bytes(session.lease, &session.run_id, RESULT_NAME) {
        Ok(value) => value,
        Err(_) => return Ok(TerminalInspection::Uncertain),
    };
    let evidence = match collect(session) {
        Ok(Progress::Complete(value)) => value,
        Ok(Progress::Uncertain) | Err(_) => return Ok(TerminalInspection::Uncertain),
        Ok(Progress::NotReady) => {
            return Ok(if pending_bytes.is_some() || result_bytes.is_some() {
                TerminalInspection::Uncertain
            } else {
                TerminalInspection::NotReady
            });
        }
    };
    let Some(pending_bytes) = pending_bytes else {
        return Ok(if result_bytes.is_some() {
            TerminalInspection::Uncertain
        } else {
            TerminalInspection::Ready {
                calls: evidence.calls,
            }
        });
    };
    let expected = expected_pending(session, &evidence)?;
    let Ok(pending) = serde_json::from_slice::<Pending>(&pending_bytes) else {
        return Ok(TerminalInspection::Uncertain);
    };
    if pending != expected {
        return Ok(TerminalInspection::Uncertain);
    }
    let Some(result_bytes) = result_bytes else {
        return Ok(TerminalInspection::Uncertain);
    };
    let Ok(result) = serde_json::from_slice::<TerminalResult>(&result_bytes) else {
        return Ok(TerminalInspection::Uncertain);
    };
    if result
        != (TerminalResult {
            format: RESULT_FORMAT.into(),
            run_id: session.run_id.clone(),
            pending_sha256: sha(&pending_bytes),
            calls: evidence.calls,
            chain_sha256: evidence.chain_sha256,
            final_observation_sha256: expected.final_observation_sha256,
            final_observation: evidence.final_observation,
        })
    {
        return Ok(TerminalInspection::Uncertain);
    }
    Ok(TerminalInspection::Completed {
        calls: result.calls,
    })
}

fn observe(
    session: &ReadyRun<'_>,
    reader: &mut dyn RemoteReadPort,
) -> std::result::Result<CallObservation, ()> {
    let remote = observe_twice(reader, &session.subject.source.commit).map_err(|_| ())?;
    Ok(CallObservation {
        generation_ref: remote.generation_ref,
        tag: remote.tag,
        release: remote.release,
        latest_release: remote.observed_latest_release,
    })
}

/// Local terminal seal only. No remote write or retry occurs here.
pub(super) fn seal_inner(
    session: &ReadyRun<'_>,
    reader: &mut dyn RemoteReadPort,
) -> Result<TerminalInspection> {
    let current = inspect_proved(session)?;
    if !matches!(current, TerminalInspection::Ready { .. }) {
        return Ok(current);
    }
    let evidence = match collect(session) {
        Ok(Progress::Complete(value)) => value,
        _ => return Ok(TerminalInspection::Uncertain),
    };
    let Some(before) = observe(session, reader).ok() else {
        return Ok(TerminalInspection::Uncertain);
    };
    if !assets::same_observation(&before, &evidence.final_observation) {
        return Ok(TerminalInspection::Uncertain);
    }
    session.recheck_small()?;
    let pending = expected_pending(session, &evidence)?;
    if journal::publish(session.lease, &session.run_id, PENDING_NAME, &pending).is_err() {
        return Ok(TerminalInspection::Uncertain);
    }
    let Some(after) = observe(session, reader).ok() else {
        return Ok(TerminalInspection::Uncertain);
    };
    if !assets::same_observation(&after, &evidence.final_observation)
        || session.recheck_small().is_err()
        || !matches!(collect(session), Ok(Progress::Complete(ref now)) if now == &evidence)
    {
        return Ok(TerminalInspection::Uncertain);
    }
    let proof = session.terminal_proof();
    let vault = inspect_payload_vault(
        session.lease,
        session.namespace,
        session.scope,
        session.key,
        &session.run_id,
        session.subject,
        &session.expected.subject,
    );
    if !matches!(vault, VaultInspection::Complete {
        run_id: ref run, catalogue_digest: ref catalogue, manifest_sha256: ref manifest,
    } if run == &session.run_id && catalogue == &proof.capsule.upload_catalogue_sha256
        && Some(manifest) == proof.capsule.vault_manifest_sha256.as_ref())
    {
        return Ok(TerminalInspection::Uncertain);
    }
    let result = TerminalResult {
        format: RESULT_FORMAT.into(),
        run_id: session.run_id.clone(),
        pending_sha256: sha(&serde_json::to_vec(&pending)?),
        calls: evidence.calls,
        chain_sha256: evidence.chain_sha256,
        final_observation_sha256: pending.final_observation_sha256,
        final_observation: evidence.final_observation,
    };
    let _ = journal::publish(session.lease, &session.run_id, RESULT_NAME, &result);
    Ok(inspect_proved(session).unwrap_or(TerminalInspection::Uncertain))
}

/// Only a fixed-repository source-owned reader can seal a production run.
/// This private method has no CLI or public caller.
#[allow(dead_code)]
pub(super) fn seal_fixed(
    session: &ReadyRun<'_>,
    client: &GithubReleaseClient,
) -> Result<TerminalInspection> {
    let mut reader = GithubRemoteReadPort::new_fixed(client).map_err(Error::io)?;
    seal_inner(session, &mut reader)
}

#[cfg(test)]
pub(super) fn seal(
    session: &ReadyRun<'_>,
    reader: &mut dyn RemoteReadPort,
) -> Result<TerminalInspection> {
    seal_inner(session, reader)
}
