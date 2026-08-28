//! Engine-owned verify reconciliation: the completed current-plan prefix,
//! compared against what this invocation can see NOW (PROP-054
//! `##VERIFY-CURRENT-PREFIX`, `##EVIDENCE-WIRE-AND-SURFACES`; R7.5 P2/A5).
//!
//! Two halves meet here, and keeping them apart is the whole correctness of
//! the member. The **prior** half is durable: the input measurement and the
//! artifact witness the executions of the completed prefix checkpointed. The
//! **current** half is taken here, at the verify instant: the declaration is
//! reconstructed from the plan row that is live now, the declared input scope
//! is re-walked, and every accumulated artifact is physically re-observed.
//! Nothing durable is rewritten — this cell holds `&mut self` only to record
//! the verify-instant artifact observation into the invocation's own map, and
//! it never touches the state store, which is why an externally mutated output
//! cannot come to compare against itself.
//!
//! The reconstruction is deliberately a REPLAY rather than a cache. Each
//! prefix row's declaration is rebuilt against the artifact registry that row
//! actually met — its predecessors' outputs hydrated from their exact durable
//! records, its own outputs not yet in the set — so an agent row's
//! credential-free preparation sees the same world it saw when it ran. A
//! remembered fingerprint would be cheaper and would answer a different
//! question: whether the declaration was the same THEN, not whether it is the
//! same NOW.
//!
//! Refusals are typed rows, never command failures: a scope that could not be
//! observed is `unstable`, an unwitnessed legacy row is `unavailable`. Only
//! two things escape as errors — a state row this project cannot locate at
//! all, and a member that breaks its own wire law — because neither is an
//! observation about the project's work.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#VERIFY-CURRENT-PREFIX");

use specmark::spec;
use vibe_wire::behaviour::verification_evidence::{EVIDENCE_EPOCH, validate};
use vibe_wire::generated::lifecycle_state::{ExecutionRecord, StateDigestWitness};
use vibe_wire::generated::shared::{
    ArtifactWitness, DigestWitness, EvidenceRun, EvidenceStatus, InputMeasurement, Timestamp,
    VerificationEvidence,
};

use crate::agent::AgentBackend;
use crate::execution::HandlerExecution;
use crate::{ExecutableContribution, ExtensionRegistryRow};

use super::{LifecycleRun, LifecycleRunError};

mod artifacts;
mod id;
mod inputs;

#[cfg(test)]
mod tests;

/// One prefix row, with the durable record it left behind cloned out of the
/// store so the comparison can proceed without holding a borrow of the run.
pub(super) struct Selected<'a> {
    pub(super) phase: &'a str,
    pub(super) row: &'a ExtensionRegistryRow,
    pub(super) key: String,
    pub(super) record: Option<ExecutionRecord>,
}

impl LifecycleRun {
    /// Reconcile the completed current-plan prefix into ONE generated
    /// verification-evidence member.
    ///
    /// `prefix` is exactly the executions that completed BEFORE the verify
    /// boundary, in canonical plan order; verify and later rows are not
    /// evidence producers and must not appear. It selects the DECLARED-INPUT
    /// half only — the artifact half is this invocation's whole accumulation,
    /// slot-stage outputs included. `agent` is the credential-free backend the
    /// declaration reconstruction resolves prompts through — it is never asked
    /// to complete anything, so this reconciliation spends nothing.
    /// `observed_at` is injected: the engine reads no clock.
    ///
    /// Tracked-only. The state-blind clean runner has no durable half to
    /// compare against and refuses as it refuses every other state request.
    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#VERIFY-CURRENT-PREFIX")]
    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#EVIDENCE-WIRE-AND-SURFACES")]
    pub fn reconcile_verification(
        &mut self,
        prefix: &[ExecutableContribution],
        agent: &dyn AgentBackend,
        observed_at: Timestamp,
    ) -> Result<VerificationEvidence, LifecycleRunError> {
        let selected = self.select(prefix)?;
        let run = self.evidence_run()?;
        let project_root = self
            .session
            .as_ref()
            .ok_or(LifecycleRunError::Unbound)?
            .project_root()
            .to_string();
        let inputs = inputs::rows(self, &selected, agent)?;
        // The artifact half deliberately does NOT take the prefix: its
        // universe is everything this invocation accumulated, which includes
        // install-stage slot outputs no phase plan names.
        let artifacts = artifacts::rows(self, &project_root)?;
        let status = root_status(&inputs, &artifacts);
        let mut member = VerificationEvidence {
            artifacts,
            evidence: EVIDENCE_EPOCH,
            // Filled below: the id is a digest over every OTHER member, so it
            // cannot be part of its own material.
            evidence_id: String::new(),
            inputs,
            observed_at,
            run,
            status,
        };
        member.evidence_id = id::evidence_id(&member);
        validate(&member).map_err(|error| LifecycleRunError::Verification {
            reason: format!("the assembled member breaks its own wire law: {error}"),
        })?;
        Ok(member)
    }

    /// The prefix, paired with each row's durable record. Cloning the record
    /// ends the store borrow here, so the halves below can be built without
    /// the reconciler ever holding the state open while it observes the tree.
    fn select<'a>(
        &self,
        prefix: &'a [ExecutableContribution],
    ) -> Result<Vec<Selected<'a>>, LifecycleRunError> {
        let state = self
            .state
            .as_ref()
            .ok_or(LifecycleRunError::UntrackedCheckpoint)?;
        Ok(prefix
            .iter()
            .map(|contribution| {
                let key = HandlerExecution::from_row(&contribution.row).key();
                Selected {
                    phase: contribution.phase.as_str(),
                    row: &contribution.row,
                    record: state.prior(&key).cloned(),
                    key,
                }
            })
            .collect())
    }

    /// The run header, restated on the evidence wire from the CURRENT run
    /// metadata — the requested full chain this invocation was asked for,
    /// including a leading `clean` when one was composed, never the narrower
    /// spelling the state header keeps for its own purposes.
    fn evidence_run(&self) -> Result<EvidenceRun, LifecycleRunError> {
        let metadata = self
            .session
            .as_ref()
            .ok_or(LifecycleRunError::Unbound)?
            .metadata();
        Ok(EvidenceRun {
            chain: metadata.chain.clone(),
            requested: metadata.requested.clone(),
            run_id: metadata.run_id.clone(),
            selected: metadata.selected.clone(),
            started: metadata.started.clone(),
        })
    }
}

/// The root never speaks for itself: with no rows it is `unavailable`, with
/// rows it is the worst of them.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#EVIDENCE-OUTCOME-VOCABULARY")]
fn root_status(inputs: &[InputMeasurement], artifacts: &[ArtifactWitness]) -> EvidenceStatus {
    inputs
        .iter()
        .map(|row| &row.status)
        .chain(artifacts.iter().map(|row| &row.status))
        .max_by_key(|status| severity(status))
        .cloned()
        .unwrap_or(EvidenceStatus::Unavailable)
}

/// `unstable > missing > stale > unavailable > matched`.
const fn severity(status: &EvidenceStatus) -> u8 {
    match status {
        EvidenceStatus::Matched => 0,
        EvidenceStatus::Unavailable => 1,
        EvidenceStatus::Stale => 2,
        EvidenceStatus::Missing => 3,
        EvidenceStatus::Unstable => 4,
    }
}

/// The durable witness as the shared wire spells it. Two records, one claim:
/// the state twin is strict-reader shaped and the wire twin is the permissive
/// shared vocabulary, and this is the only place the two meet.
fn witness(state: &StateDigestWitness) -> DigestWitness {
    DigestWitness {
        algorithm: state.algorithm.clone(),
        bytes: state.bytes.clone(),
        digest: state.digest.clone(),
        files: state.files,
    }
}
