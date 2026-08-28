//! The declared-input half: the CURRENT declaration and manifest, compared
//! against the measurement the execution checkpointed.
//!
//! The walk visits EVERY prefix row, not only the ones that declare inputs,
//! because the replay's artifact registry is what makes a reconstruction
//! faithful: a row is prepared against its predecessors' outputs and not yet
//! against its own, exactly as it was when it ran. So each row is prepared
//! first and hydrated second, from its own exact durable record.
//!
//! A preparation defect — an envelope that will not build, a prompt that will
//! not resolve, a pattern that will not compile — is an `Err`, never an
//! evidence word: the five words report what an observation SAW, and a
//! reconstruction that could not be performed saw nothing. What IS an
//! observation is the manifest's own typed refusal, and that becomes
//! `unstable` (with a prior) or `unavailable` (without one).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#VERIFY-CURRENT-PREFIX");

use specmark::spec;
use vibe_wire::generated::lifecycle_state::StateInputMeasurement;
use vibe_wire::generated::shared::{EvidenceStatus, InputMeasurement};

use crate::agent::AgentBackend;
use crate::execution::HandlerExecution;
use crate::state::{
    InputRefusal, ManifestOutcome, PreparedFingerprint, prepare_handler_execution_with,
};

use super::{LifecycleRun, LifecycleRunError, Selected, witness};

/// The declaration this row is measured against is no longer the one that was
/// measured — the spec's own word for that arm.
const DECLARATION_CHANGED: &str = "input-declaration-changed";
/// No measurement is attributable to this declaration at all.
const UNWITNESSED: &str = "input-unwitnessed";

/// Reconstruct and compare every inputs-declaring row of the prefix.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#VERIFY-CURRENT-PREFIX")]
pub(super) fn rows(
    run: &LifecycleRun,
    selected: &[Selected<'_>],
    agent: &dyn AgentBackend,
) -> Result<Vec<InputMeasurement>, LifecycleRunError> {
    let mut replay = run
        .session
        .as_ref()
        .ok_or(LifecycleRunError::Unbound)?
        .empty_replay();
    let mut rows = Vec::new();
    for entry in selected {
        // Selection is the CURRENT declaration's word: a row that declares no
        // inputs is `unavailable` by having no row at all, never an empty
        // matched set. A durable measurement under a declaration that no
        // longer declares inputs is not evidence about current work.
        if entry.row.declaration().inputs.is_some() {
            let prepared = reconstruct(&replay, entry, agent)?;
            rows.push(row(entry, &prepared)?);
        }
        // Only NOW does this row's own output become part of the registry the
        // next reconstruction meets.
        if let Some(record) = entry.record.as_ref() {
            replay.hydrate_artifacts(entry.phase, &record.artifacts);
        }
    }
    Ok(rows)
}

/// One row's current declaration and manifest, from the exact preparation the
/// engine performs before dispatch — same envelope law, same prompt
/// resolution, same one-walk manifest.
fn reconstruct(
    replay: &crate::ExecutionSession,
    entry: &Selected<'_>,
    agent: &dyn AgentBackend,
) -> Result<PreparedFingerprint, LifecycleRunError> {
    let handler = HandlerExecution::from_row(entry.row);
    let envelope = replay
        .envelope_for_execution(entry.phase, &handler)
        .map_err(|source| LifecycleRunError::Envelope {
            key: entry.key.clone(),
            source,
        })?;
    let prepared = crate::agent::prepare(agent, handler.row(), &envelope).map_err(|source| {
        LifecycleRunError::AgentPreparation {
            key: entry.key.clone(),
            source: Box::new(source),
        }
    })?;
    prepare_handler_execution_with(&handler, &envelope, prepared.as_ref()).map_err(|source| {
        LifecycleRunError::Fingerprint {
            key: entry.key.clone(),
            source,
        }
    })
}

/// The per-status matrix, in its one true order: a refused current observation
/// answers first (there is nothing to compare), then whether the declaration
/// is still the one that was measured, then the digests themselves.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#EVIDENCE-OUTCOME-VOCABULARY")]
fn row(
    entry: &Selected<'_>,
    prepared: &PreparedFingerprint,
) -> Result<InputMeasurement, LifecycleRunError> {
    let manifest =
        prepared
            .input_manifest
            .as_ref()
            .ok_or_else(|| LifecycleRunError::Verification {
                reason: format!(
                    "execution `{}` declares inputs but its reconstruction produced no manifest",
                    entry.key,
                ),
            })?;
    let prior = entry
        .record
        .as_ref()
        .and_then(|record| record.input_measurement.as_ref());
    let mut wire = InputMeasurement {
        declaration_fingerprint: prepared.declaration_fingerprint.clone(),
        execution: entry.key.clone(),
        measured: None,
        measured_run_id: None,
        observed: None,
        patterns: manifest.patterns.clone(),
        phase: entry.phase.to_string(),
        reason_code: None,
        status: EvidenceStatus::Unavailable,
    };
    let observed = manifest.measured().map(witness);
    let Some(prior) = prior else {
        // No attributable measurement. A safe current reading may still ride
        // along — it is honest context — but the reason names the absent
        // baseline, which is what `unavailable` is about.
        wire.observed = observed;
        wire.reason_code = Some(UNWITNESSED.to_string());
        return Ok(wire);
    };
    wire.measured = Some(witness(&prior.witness));
    wire.measured_run_id = Some(prior.measured_run_id.clone());
    if let ManifestOutcome::Refused(cause) = &manifest.outcome {
        wire.status = EvidenceStatus::Unstable;
        wire.reason_code = Some(refusal(*cause).to_string());
        return Ok(wire);
    }
    wire.observed = observed;
    if changed(&wire, prior) {
        wire.status = EvidenceStatus::Stale;
        wire.reason_code = Some(DECLARATION_CHANGED.to_string());
        return Ok(wire);
    }
    wire.status = if wire.measured == wire.observed {
        EvidenceStatus::Matched
    } else {
        // A digest mismatch under an unchanged declaration needs no reason:
        // the two witnesses on the row ARE the reason, and inventing a second
        // spelling for what the reader can already see would be noise.
        EvidenceStatus::Stale
    };
    Ok(wire)
}

/// Whether the identity this row was measured under is still the identity the
/// current declaration names. All four members bind: a measurement recorded
/// for another execution, phase, declaration or pattern list is a measurement
/// of other work.
fn changed(wire: &InputMeasurement, prior: &StateInputMeasurement) -> bool {
    prior.declaration_fingerprint != wire.declaration_fingerprint
        || prior.patterns != wire.patterns
        || prior.execution != wire.execution
        || prior.phase != wire.phase
}

/// The closed reason vocabulary for a refused current observation — one code
/// per typed cause, so `unstable` never means merely "something went wrong".
const fn refusal(cause: InputRefusal) -> &'static str {
    match cause {
        InputRefusal::NonRegular => "input-non-regular",
        InputRefusal::Aliased => "input-aliased",
        InputRefusal::Open => "input-open",
        InputRefusal::Read => "input-read",
        InputRefusal::Unstable => "input-unstable",
        InputRefusal::Disagree => "input-disagree",
    }
}
