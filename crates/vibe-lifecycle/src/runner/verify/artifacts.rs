//! The artifact half: the durable baseline against a verify-instant
//! re-observation.
//!
//! The universe here is deliberately NOT the phase prefix. It is every
//! artifact this invocation accumulated — the session's own registry, in the
//! order it accumulated them — because an install-stage slot execution
//! produces artifacts that no `RitualPlan` row names, and a comparison that
//! walked only the phase plan would let them vanish from the evidence.
//!
//! The baseline is the exact durable row the same invocation checkpointed or
//! preserved for that id, remembered at its write. Never a scan of old state:
//! a stale record carrying the same id is not this run's owner, and guessing
//! between them would be a coin toss over what `matched` means. No baseline is
//! `unavailable`, said out loud.
//!
//! The current reading is taken NOW, physically, through the same observer the
//! runner uses, and recorded into the invocation's own map. That ordering is
//! the whole of E5: a file mutated after its producer ran is compared against
//! what the producer saw, not against itself.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#EVIDENCE-ARTIFACT-WITNESS");

use specmark::spec;
use vibe_wire::generated::lifecycle_state::StateArtifact;
use vibe_wire::generated::shared::{ArtifactWitness, EvidenceStatus};

use crate::artifacts::observe::{ArtifactObserver, WitnessOutcome, WitnessRefusal};

use super::{LifecycleRun, LifecycleRunError, witness};

/// The baseline is absent — the honest word for a pre-R7.5 row, one whose
/// production-time observation was refused, or an id no execution of this
/// invocation owns.
const UNWITNESSED: &str = "artifact-unwitnessed";

/// Re-observe every artifact this invocation accumulated and compare it
/// against its own baseline.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#EVIDENCE-ARTIFACT-WITNESS")]
pub(super) fn rows(
    run: &mut LifecycleRun,
    project_root: &str,
) -> Result<Vec<ArtifactWitness>, LifecycleRunError> {
    // The registry is copied out first: the observation below needs the run
    // mutably, and the accumulation order is what canonical row order means.
    let accumulated: Vec<(String, String, String)> = run
        .session
        .as_ref()
        .ok_or(LifecycleRunError::Unbound)?
        .artifacts()
        .iter()
        .map(|artifact| {
            (
                artifact.id.clone(),
                artifact.kind.clone(),
                artifact.path.clone(),
            )
        })
        .collect();
    let observer = ArtifactObserver::new(project_root);
    let mut rows = Vec::with_capacity(accumulated.len());
    for (id, kind, path) in accumulated {
        // The published row carries the PORTABLE half of the identity. A
        // recorded path this project cannot locate is not an evidence outcome
        // — it is a row that cannot be published at all, so it refuses rather
        // than inventing a wire-safe spelling. The diagnostic names the id and
        // never the absolute path.
        let relative = match crate::artifacts::eligible_relative(&id, &path, project_root) {
            Ok(Some(relative)) => relative.to_string(),
            Ok(None) | Err(_) => {
                return Err(LifecycleRunError::Verification {
                    reason: format!(
                        "the recorded artifact `{}` is not a path below the selected project \
                         and cannot be published as evidence",
                        preview(&id),
                    ),
                });
            }
        };
        run.observe_artifact(&observer, &id, &path);
        // Taken back OUT of the invocation's map rather than from the returned
        // value: what verify compares is exactly what the run holds as its
        // current half, so the two can never drift apart.
        let outcome =
            run.artifact_observation(&id)
                .ok_or_else(|| LifecycleRunError::Verification {
                    reason: format!(
                        "the artifact `{}` was observed but its outcome was not retained",
                        preview(&id),
                    ),
                })?;
        rows.push(row(
            &id,
            &kind,
            relative,
            run.artifact_baseline(&id),
            outcome,
        ));
    }
    Ok(rows)
}

/// One comparison. The baseline decides first: with none, no current reading
/// can make this row a pass, and the reason names the absence rather than
/// whatever the current observation happened to say.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#EVIDENCE-OUTCOME-VOCABULARY")]
fn row(
    id: &str,
    kind: &str,
    path: String,
    baseline: Option<&StateArtifact>,
    outcome: &WitnessOutcome,
) -> ArtifactWitness {
    let mut wire = ArtifactWitness {
        id: id.to_string(),
        kind: kind.to_string(),
        measured: None,
        measured_run_id: None,
        observed: None,
        path,
        reason_code: None,
        status: EvidenceStatus::Unavailable,
    };
    let observed = match outcome {
        WitnessOutcome::Measured(current) => Some(witness(current)),
        WitnessOutcome::Refused(_) => None,
    };
    let Some(measured) = baseline.and_then(|row| row.witness.as_ref()) else {
        wire.observed = observed;
        wire.reason_code = Some(UNWITNESSED.to_string());
        return wire;
    };
    wire.measured = Some(witness(measured));
    wire.measured_run_id = baseline.and_then(|row| row.measured_run_id.clone());
    match outcome {
        WitnessOutcome::Measured(_) => {
            wire.observed = observed;
            wire.status = if wire.measured == wire.observed {
                EvidenceStatus::Matched
            } else {
                EvidenceStatus::Stale
            };
        }
        // Strict absence is the ONE refusal that is not instability: the
        // object is owed and gone, which is a different fact from "this run
        // could not establish one safe comparable object".
        WitnessOutcome::Refused(WitnessRefusal::Absent) => {
            wire.status = EvidenceStatus::Missing;
            wire.reason_code = Some("artifact-absent".to_string());
        }
        WitnessOutcome::Refused(cause) => {
            wire.status = EvidenceStatus::Unstable;
            wire.reason_code = Some(refusal(*cause).to_string());
        }
    }
    wire
}

/// The closed artifact reason vocabulary.
const fn refusal(cause: WitnessRefusal) -> &'static str {
    match cause {
        WitnessRefusal::Absent => "artifact-absent",
        WitnessRefusal::NotRegular => "artifact-not-regular",
        WitnessRefusal::Linked => "artifact-linked",
        WitnessRefusal::Aliased => "artifact-aliased",
        WitnessRefusal::Outside => "artifact-outside-project",
        WitnessRefusal::Malformed => "artifact-path-malformed",
        WitnessRefusal::Moved => "artifact-moved",
        WitnessRefusal::Torn => "artifact-torn",
        WitnessRefusal::Unbounded => "artifact-unbounded",
        WitnessRefusal::Io => "artifact-io",
    }
}

/// A refusal quotes handler-supplied text, so bound it — an id, never a
/// machine path and never a file body.
fn preview(value: &str) -> String {
    const LIMIT: usize = 120;
    if value.chars().count() <= LIMIT {
        return value.to_string();
    }
    value.chars().take(LIMIT).collect::<String>() + "…"
}
