//! The hosted handoff branch of `LifecycleRun::execute_one`.
//!
//! Delegation is ENGINE-owned, never handler-reply vocabulary: script, binary
//! and native handlers keep the `ok|fail|skip` reply and can never park the
//! engine. This cell holds the one branch that can — an agent row in resolved
//! agent mode — placed after credential-free preparation, fingerprinting and
//! the ordinary reusable-success probe, and before any provider dispatch, so
//! a parked execution provably spends nothing.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#AGENT-HANDSHAKE");

use std::time::Instant;

use specmark::spec;
use vibe_wire::generated::lifecycle::e1::context::Context;
use vibe_wire::generated::lifecycle_state::{
    ExecutionRecord, ExecutionRecordScope, ExecutionRecordStatus,
};

use crate::agent::PreparedAgent;
use crate::delegation::Delegation;
use crate::handlers::HandlerStreams;
use crate::{ExecutionTransition, HandlerExecution, LifecycleRun, LifecycleRunError};

use super::{PreparedRecordEvidence, elapsed_ms};

impl LifecycleRun {
    /// The hosted handoff transition for one agent execution in resolved
    /// agent mode. Reached only after preparation, fingerprinting and the
    /// ordinary reusable-success probe — so a first invocation may never be
    /// satisfied by coincidental pre-existing outputs, and only a prior
    /// **delegated record for this same execution with this exact
    /// fingerprint** may be satisfied by existing outputs at all.
    ///
    /// Satisfied resume: checkpoint `ok`, clear tasks, hydrate exactly the
    /// planned rows, remove the state-owned task (a notice on failure — never
    /// a reason to erase outputs or downgrade the execution) and continue the
    /// chain with zero provider spend. Otherwise: publish the task FIRST,
    /// then checkpoint `delegated`, and stop the chain at this row.
    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#AGENT-HANDSHAKE")]
    pub(super) fn delegated_transition(
        &mut self,
        execution: &HandlerExecution,
        envelope: Context,
        prepared: &PreparedAgent,
        phase: &str,
        evidence: PreparedRecordEvidence,
        started: Instant,
    ) -> Result<ExecutionTransition, LifecycleRunError> {
        let PreparedRecordEvidence {
            fingerprint,
            input_measurement: measurement,
        } = evidence;
        let key = execution.key();
        let root = std::path::PathBuf::from(&envelope.project.root);
        if !self.force {
            let satisfied = self
                .state
                .as_ref()
                .and_then(|state| state.prior(&key))
                .filter(|prior| {
                    prior.status == ExecutionRecordStatus::Delegated
                        && prior.fingerprint == fingerprint
                })
                .filter(|prior| {
                    crate::agent::probe_outputs(&root, prepared.contract(), &prior.artifacts)
                });
            if let Some(prior) = satisfied {
                // Owned BEFORE the store is touched: everything below borrows
                // `self` mutably, and the record this resume is judged against
                // must not be a live borrow into the map being rewritten.
                let prior = prior.clone();
                // The host's bytes enter durable state ONLY here, so this IS
                // the acceptance boundary: the witness taken now becomes the
                // baseline, under the adopting run id, and never an inherited
                // one (a parked row's planned rows carry none by construction).
                let observer =
                    crate::artifacts::observe::ArtifactObserver::new(&envelope.project.root);
                let artifacts = prior
                    .artifacts
                    .iter()
                    .map(|artifact| {
                        let outcome =
                            self.observe_artifact(&observer, &artifact.id, &artifact.path);
                        crate::artifacts::observe::state_row(
                            &self.run_id,
                            artifact.id.clone(),
                            artifact.kind.clone(),
                            artifact.path.clone(),
                            &outcome,
                        )
                    })
                    .collect::<Vec<_>>();
                self.session
                    .as_mut()
                    .ok_or(LifecycleRunError::Unbound)?
                    .hydrate_artifacts(phase, &artifacts);
                self.state
                    .as_mut()
                    .ok_or(LifecycleRunError::UntrackedCheckpoint)?
                    .checkpoint(
                        key.clone(),
                        ExecutionRecord {
                            artifacts: artifacts.clone(),
                            duration_ms: elapsed_ms(started),
                            fingerprint,
                            phase: phase.into(),
                            status: ExecutionRecordStatus::Ok,
                            tasks: Vec::new(),
                            // A satisfied row is no longer delegated, so it
                            // carries no scope tag either.
                            scope: None,
                            // The resume invocation's own pre-probe
                            // measurement — `execute_one` walked the declared
                            // inputs before this branch — attributed to the
                            // adopting run id, never a copied prior claim.
                            input_measurement: measurement,
                        },
                    )?;
                // The success checkpoint is durable; only now may the exact
                // task this run owns for this execution go — cleanup proves
                // ownership by recomputing the path, not by recognising a
                // plausible spelling. A cleanup failure is a notice.
                let notice = prior
                    .tasks
                    .first()
                    .map(|task| crate::delegation::cleanup_task(&root, &self.run_id, &key, task))
                    .transpose()
                    .err();
                let mut message = format!(
                    "resumed from the hosting agent's outputs; {} declared output(s) accepted; no \
                     provider spend",
                    artifacts.len(),
                );
                if let Some(notice) = notice {
                    message.push_str(&format!("; NOTE: {notice}"));
                }
                return Ok(ExecutionTransition {
                    envelope,
                    status: ExecutionRecordStatus::Ok,
                    message: Some(message),
                    artifacts,
                    streams: HandlerStreams::default(),
                    delegation: None,
                });
            }
        }
        // Park: publish the task document first — state must never point at a
        // task that was not durably published — then checkpoint the handoff.
        let planned = prepared
            .contract()
            .planned_state_rows(&envelope.project.root);
        let task =
            crate::delegation::publish_task(&root, &self.run_id, &key, phase, prepared, &envelope)
                .map_err(|source| {
                    let failure = LifecycleRunError::DelegationPark {
                        key: key.clone(),
                        source: Box::new(source),
                    };
                    self.checkpoint_preparation_failure(execution, phase, started, failure)
                })?;
        self.state
            .as_mut()
            .ok_or(LifecycleRunError::UntrackedCheckpoint)?
            .checkpoint(
                key,
                ExecutionRecord {
                    artifacts: planned.clone(),
                    duration_ms: elapsed_ms(started),
                    fingerprint,
                    phase: phase.into(),
                    status: ExecutionRecordStatus::Delegated,
                    tasks: vec![task.clone()],
                    // The ENGINE records which plan owns this park, so a later
                    // reconciliation never has to guess it by parsing the
                    // execution key or a task filename.
                    scope: Some(if execution.slot_target().is_some() {
                        ExecutionRecordScope::Slot
                    } else {
                        ExecutionRecordScope::Phase
                    }),
                    // A parked row has executed nothing; the resume
                    // re-measures before it accepts the row, so no
                    // measurement is attributed to a run that produced no
                    // work.
                    input_measurement: None,
                },
            )?;
        let handoff = Delegation {
            resume: format!("vibe {}", envelope.run.requested),
            run_id: self.run_id.clone(),
            tasks: vec![task],
        };
        self.parked = Some(handoff.clone());
        Ok(ExecutionTransition {
            envelope,
            status: ExecutionRecordStatus::Delegated,
            message: Some(format!(
                "parked for the hosting agent; {} declared output(s) awaited; resume with `{}`",
                planned.len(),
                handoff.resume,
            )),
            artifacts: planned,
            streams: HandlerStreams::default(),
            delegation: Some(handoff),
        })
    }
}
