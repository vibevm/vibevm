//! One canonical envelope/fingerprint/state/handler transition.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use specmark::spec;
use vibe_wire::generated::lifecycle::e1::context::{Context, Project, RunAgentMode, World};
use vibe_wire::generated::lifecycle::e1::reply::ReplyStatus;
use vibe_wire::generated::lifecycle_state::{
    ExecutionRecord, ExecutionRecordStatus, StateArtifact, StateInputMeasurement,
};

use crate::agent::PreparedAgent;
use crate::delegation::Delegation;
use crate::handlers::{HandlerRuntime, HandlerStreams};
use crate::lease::LifecycleLease;
use crate::state::prepare_handler_execution_with;
use crate::{
    ContributionOutcome, DispatchError, ExecutionSession, HandlerExecution, LifecycleStateStore,
    RunMetadata, preparation_error_fingerprint_for_identity,
};

mod observations;

/// A shared run is passed through install's slot callbacks and rebound after
/// the durable-world barrier before normal phase dispatch.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM")]
pub type LifecycleRunHandle = Arc<Mutex<LifecycleRun>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#PHASE-STATE-HOME")]
pub enum ExecutionReuse {
    FreshnessAware,
    Always,
}

/// The two pre-dispatch state values the hosted branch either checkpoints on
/// satisfaction or deliberately drops on park. One value keeps that private
/// transition below the argument-count ceiling and makes their shared
/// observation epoch explicit.
#[derive(Debug)]
pub(super) struct PreparedRecordEvidence {
    fingerprint: String,
    input_measurement: Option<StateInputMeasurement>,
}

#[derive(Debug, Clone)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#REPLY-SHAPE")]
pub struct ExecutionTransition {
    pub envelope: Context,
    pub status: ExecutionRecordStatus,
    pub message: Option<String>,
    pub artifacts: Vec<StateArtifact>,
    pub streams: HandlerStreams,
    /// The typed handoff, present exactly when this transition parked an
    /// agent execution for the hosting agent (`status == delegated`).
    pub delegation: Option<Delegation>,
}

#[derive(Debug, Clone)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#OBS-RUN-RECORD")]
pub struct FailedExecutionTransition {
    pub envelope: Context,
    pub message: String,
    pub streams: HandlerStreams,
}

impl ExecutionTransition {
    #[must_use]
    pub fn is_fresh(&self) -> bool {
        self.status == ExecutionRecordStatus::Fresh
    }
}

/// Mutable state for one complete lifecycle invocation.
#[derive(Debug)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM")]
pub struct LifecycleRun {
    session: Option<ExecutionSession>,
    state: Option<LifecycleStateStore>,
    force: bool,
    run_id: String,
    parked: Option<Delegation>,
    /// The CURRENT half of the artifact comparison, keyed by artifact id.
    /// Transient by construction — see [`observations`] for why folding it
    /// into the durable baseline would kill E5.
    artifact_observations: BTreeMap<String, crate::artifacts::observe::WitnessOutcome>,
    /// The workspace's mutation lease, retained for the run's whole life on
    /// BOTH the tracked and the untracked path — the same `Arc` share of the
    /// ONE acquisition the command boundary made, never a reacquisition.
    /// Retention-only on purpose (hence the underscore): nothing reads it
    /// through the run; holding it here is what keeps a parked or wiping run
    /// from releasing workspace ownership while its rows are still being
    /// written.
    _lease: Arc<LifecycleLease>,
}

impl LifecycleRun {
    pub fn begin(
        lease: Arc<LifecycleLease>,
        project: Project,
        world: World,
        metadata: RunMetadata,
        state_chain: Vec<String>,
    ) -> Result<Self, LifecycleRunError> {
        let state = LifecycleStateStore::begin(
            lease.clone(),
            metadata.requested.clone(),
            state_chain,
            metadata.started.clone(),
            metadata.run_id.clone(),
            metadata.selected.clone(),
            metadata.trace_compile,
        )?;
        let run_id = metadata.run_id.clone();
        Ok(Self {
            force: metadata.force,
            run_id,
            parked: None,
            artifact_observations: BTreeMap::new(),
            session: Some(ExecutionSession::new(project, world, metadata)),
            state: Some(state),
            _lease: lease,
        })
    }

    /// Create the state-blind clean runner. It uses the same handler
    /// transition but never reads or rewrites the erasable freshness cache —
    /// but it RETAINS the lease proof exactly as the tracked runner does:
    /// an untracked clean mutates the tree, so it owns the workspace for its
    /// whole life too.
    #[must_use]
    pub fn untracked(
        lease: Arc<LifecycleLease>,
        project: Project,
        world: World,
        metadata: RunMetadata,
    ) -> Self {
        let run_id = metadata.run_id.clone();
        Self {
            force: metadata.force,
            run_id,
            parked: None,
            artifact_observations: BTreeMap::new(),
            session: Some(ExecutionSession::new(project, world, metadata)),
            state: None,
            _lease: lease,
        }
    }

    /// The typed handoff of the first execution this invocation parked, if
    /// any. One invocation parks at most one agent row — the chain stops
    /// there — so this is a single value, not a log.
    #[must_use]
    pub fn parked_delegation(&self) -> Option<&Delegation> {
        self.parked.as_ref()
    }

    pub fn shared(self) -> LifecycleRunHandle {
        Arc::new(Mutex::new(self))
    }

    pub fn rebind_world(
        &mut self,
        project: Project,
        world: World,
    ) -> Result<(), LifecycleRunError> {
        self.session
            .as_mut()
            .ok_or(LifecycleRunError::Unbound)?
            .rebind_world(project, world);
        Ok(())
    }

    pub fn retain_execution_prefix(
        &mut self,
        prefix: &str,
        keep: &BTreeSet<String>,
    ) -> Result<(), LifecycleRunError> {
        self.state
            .as_mut()
            .ok_or(LifecycleRunError::UntrackedCheckpoint)?
            .retain_prefixed(prefix, keep)?;
        Ok(())
    }

    pub fn execute_one(
        &mut self,
        execution: &HandlerExecution,
        phase: &str,
        reuse: ExecutionReuse,
        runtime: &HandlerRuntime<'_>,
    ) -> Result<ExecutionTransition, LifecycleRunError> {
        let started = Instant::now();
        let key = execution.key();
        let envelope = match self
            .session
            .as_ref()
            .ok_or(LifecycleRunError::Unbound)?
            .envelope_for_execution(phase, execution)
        {
            Ok(envelope) => envelope,
            Err(source) => {
                let failure = LifecycleRunError::Envelope {
                    key: key.clone(),
                    source,
                };
                return Err(self.checkpoint_preparation_failure(execution, phase, started, failure));
            }
        };
        // Credential-free preparation FIRST: an agent row's contract, its
        // provider-pinned address and the exact resolved prompt bytes are all
        // decided before the freshness question is asked, so the same bytes
        // that answer it are the bytes the paid call later uses.
        let prepared = match crate::agent::prepare(runtime.agent, execution.row(), &envelope) {
            Ok(prepared) => prepared,
            Err(source) => {
                let failure = LifecycleRunError::AgentPreparation {
                    key: key.clone(),
                    source: Box::new(source),
                };
                return Err(self.checkpoint_preparation_failure(execution, phase, started, failure));
            }
        };
        if self.state.is_none() {
            return self.dispatch_untracked(execution, envelope, runtime, prepared.as_ref());
        }
        // ONE preparation: the fingerprint, its declaration sibling and the
        // input measurement below all come from this single pre-dispatch
        // observation. Nothing here is recomputed after the handler may have
        // changed its own inputs (PROP-054 `##EVIDENCE-MEASUREMENT-CARRIAGE`).
        let prepared_inputs =
            match prepare_handler_execution_with(execution, &envelope, prepared.as_ref()) {
                Ok(prepared_inputs) => prepared_inputs,
                Err(source) => {
                    let failure = LifecycleRunError::Fingerprint {
                        key: key.clone(),
                        source,
                    };
                    return Err(
                        self.checkpoint_preparation_failure(execution, phase, started, failure)
                    );
                }
            };
        // The durable measurement this invocation attributes to itself —
        // present only when the declared scope was actually measured, never
        // a partial digest and never a copy of a prior row's claim.
        let measurement = prepared_inputs.state_measurement(&key, phase, &self.run_id);
        let fingerprint = prepared_inputs.fingerprint;
        // A HOSTED agent row is freshness-aware whatever the caller asked for.
        //
        // `Always` exists so a slot contribution re-runs every time its slot is
        // touched — correct for a script, and unbounded for a handoff: "run it
        // again" would mean re-parking a row whose declared outputs this very
        // engine already recorded, so a target carrying TWO ordered agent rows
        // could never converge. Row A is satisfied, row B parks, and the next
        // pass re-parks A, forever. The evidence accepted here is the engine's
        // own reusable record for this exact execution at this exact
        // fingerprint — never coincidental files, which the handoff branch
        // below still refuses.
        let hosted_agent = prepared.is_some() && envelope.run.agent_mode == RunAgentMode::Agent;
        let prior = ((reuse == ExecutionReuse::FreshnessAware || hosted_agent) && !self.force)
            .then(|| {
                self.state
                    .as_ref()
                    .and_then(|state| state.reusable_record(&key, &fingerprint))
                    .cloned()
            })
            .flatten();
        let prior = prior.filter(|prior| {
            // An agent row's outputs are the whole point of the execution, so
            // "the inputs did not change" is only half the question: a deleted,
            // emptied, relinked or contract-mismatched output is not fresh, it
            // is missing work. The probe is credential-free and provider-free.
            if let Some(prepared) = prepared.as_ref() {
                // The COMPLETE recorded rows, not their ids: a tampered path
                // or kind must not survive into the hydrated envelope, where a
                // later contribution would treat it as real.
                return crate::agent::probe_outputs(
                    Path::new(&envelope.project.root),
                    prepared.contract(),
                    &prior.artifacts,
                );
            }
            let vibe_core::manifest::ExtensionHandler::Builtin { name } =
                &execution.row().declaration().handler
            else {
                return true;
            };
            !crate::BuiltinRegistry::is_package_binding(name, execution.row())
                || runtime
                    .package_binding
                    .probe(&key, &prior.artifacts)
                    .unwrap_or(false)
        });
        if let Some(prior) = prior {
            // A fresh skip is NOT a producer. It re-observes the current
            // object into this invocation's transient map and checkpoints the
            // prior rows byte-for-byte — witness, run id and absence alike.
            //
            // Overwriting the baseline with the current reading is exactly the
            // defect E5 names: verify would then compare W2 against W2 and
            // report `matched` after an external mutation. The mirror move is
            // just as wrong — a current success may not upgrade a legacy
            // unwitnessed row into a baseline nobody produced.
            let observer = crate::artifacts::observe::ArtifactObserver::new(&envelope.project.root);
            let artifacts = prior.artifacts.clone();
            for artifact in &artifacts {
                self.observe_artifact(&observer, &artifact.id, &artifact.path);
            }
            self.session
                .as_mut()
                .ok_or(LifecycleRunError::Unbound)?
                .hydrate_artifacts(phase, &artifacts);
            self.state
                .as_mut()
                .ok_or(LifecycleRunError::UntrackedCheckpoint)?
                .checkpoint(
                    key,
                    ExecutionRecord {
                        artifacts: artifacts.clone(),
                        duration_ms: 0,
                        fingerprint,
                        phase: phase.into(),
                        status: ExecutionRecordStatus::Fresh,
                        tasks: Vec::new(),
                        scope: None,
                        // A fresh skip IS an observation: the fingerprint
                        // pass above re-walked the declared inputs, so the
                        // CURRENT invocation checkpoints its own measurement
                        // under its own run id — never a copy of the prior
                        // row's claim. A refused current observation writes
                        // `None` here and drops the old claim honestly.
                        input_measurement: measurement,
                    },
                )?;
            return Ok(ExecutionTransition {
                envelope,
                status: ExecutionRecordStatus::Fresh,
                message: None,
                artifacts,
                streams: HandlerStreams::default(),
                delegation: None,
            });
        }

        // The hosted handoff, owned by the ENGINE and never by handler reply
        // vocabulary: an agent row in resolved agent mode parks here, after
        // credential-free preparation and the ordinary reusable-success probe
        // and before any provider dispatch. `AgentBackend::complete` is
        // unreachable below this branch, so a parked execution spends nothing.
        if let Some(prepared) = prepared.as_ref()
            && envelope.run.agent_mode == RunAgentMode::Agent
        {
            return self.delegated_transition(
                execution,
                envelope,
                prepared,
                phase,
                PreparedRecordEvidence {
                    fingerprint,
                    input_measurement: measurement,
                },
                started,
            );
        }

        let dispatched = self
            .session
            .as_mut()
            .ok_or(LifecycleRunError::Unbound)?
            .dispatch_execution(execution, envelope.clone(), runtime, prepared.as_ref());
        match dispatched {
            Ok(outcome) => {
                self.checkpoint_success(key, phase, fingerprint, started, outcome, measurement)
            }
            Err(source) => {
                let record = ExecutionRecord {
                    artifacts: Vec::new(),
                    duration_ms: elapsed_ms(started),
                    fingerprint,
                    phase: phase.into(),
                    status: ExecutionRecordStatus::Fail,
                    tasks: Vec::new(),
                    scope: None,
                    input_measurement: None,
                };
                let state = self
                    .state
                    .as_mut()
                    .ok_or(LifecycleRunError::UntrackedCheckpoint)?;
                match state.checkpoint(key.clone(), record) {
                    Ok(()) => Err(failed_transition(envelope, source)),
                    Err(checkpoint) => {
                        let failed = failed_transition(envelope, source);
                        let LifecycleRunError::FailedTransition { transition, source } = failed
                        else {
                            unreachable!("failed_transition returns the typed variant")
                        };
                        Err(LifecycleRunError::Checkpoint {
                            key,
                            primary: source.to_string(),
                            checkpoint: Box::new(checkpoint),
                            transition: Some(transition),
                            dispatch: Some(source),
                        })
                    }
                }
            }
        }
    }

    fn checkpoint_success(
        &mut self,
        key: String,
        phase: &str,
        fingerprint: String,
        started: Instant,
        outcome: ContributionOutcome,
        measurement: Option<StateInputMeasurement>,
    ) -> Result<ExecutionTransition, LifecycleRunError> {
        let status = match outcome.reply.status {
            ReplyStatus::Ok => ExecutionRecordStatus::Ok,
            ReplyStatus::Skip => ExecutionRecordStatus::Skip,
            ReplyStatus::Fail => unreachable!("dispatch rejects fail replies"),
        };
        // The reply NAMES the artifact; the witness is what the filesystem
        // says is actually there. Probed after the handler returned and before
        // the one execution-record checkpoint below, so the witness rides that
        // same transaction — no second file, no second crash window.
        //
        // This IS a production boundary, so the one observation becomes both
        // the durable baseline and this invocation's current reading.
        let observer =
            crate::artifacts::observe::ArtifactObserver::new(&outcome.envelope.project.root);
        let artifacts = outcome
            .reply
            .artifacts
            .iter()
            .map(|artifact| {
                let outcome = self.observe_artifact(&observer, &artifact.id, &artifact.path);
                crate::artifacts::observe::state_row(
                    &self.run_id,
                    artifact.id.clone(),
                    artifact.kind.clone(),
                    artifact.path.clone(),
                    &outcome,
                )
            })
            .collect::<Vec<_>>();
        self.state
            .as_mut()
            .ok_or(LifecycleRunError::UntrackedCheckpoint)?
            .checkpoint(
                key,
                ExecutionRecord {
                    artifacts: artifacts.clone(),
                    duration_ms: elapsed_ms(started),
                    fingerprint,
                    phase: phase.into(),
                    status: status.clone(),
                    tasks: Vec::new(),
                    scope: None,
                    // The pre-dispatch measurement of THIS invocation, under
                    // its own run id (PROP-054 `##EVIDENCE-MEASUREMENT-CARRIAGE`).
                    input_measurement: measurement,
                },
            )?;
        Ok(ExecutionTransition {
            envelope: outcome.envelope,
            status,
            message: outcome.reply.message,
            artifacts,
            streams: outcome.streams,
            delegation: None,
        })
    }

    fn checkpoint_preparation_failure(
        &mut self,
        execution: &HandlerExecution,
        phase: &str,
        started: Instant,
        primary: LifecycleRunError,
    ) -> LifecycleRunError {
        let key = execution.key();
        let record = ExecutionRecord {
            artifacts: Vec::new(),
            duration_ms: elapsed_ms(started),
            fingerprint: preparation_error_fingerprint_for_identity(&execution.key(), phase),
            phase: phase.into(),
            status: ExecutionRecordStatus::Fail,
            tasks: Vec::new(),
            scope: None,
            input_measurement: None,
        };
        let Some(state) = self.state.as_mut() else {
            return primary;
        };
        match state.checkpoint(key.clone(), record) {
            Ok(()) => primary,
            Err(checkpoint) => LifecycleRunError::Checkpoint {
                key,
                primary: primary.to_string(),
                checkpoint: Box::new(checkpoint),
                transition: None,
                dispatch: None,
            },
        }
    }

    fn dispatch_untracked(
        &mut self,
        execution: &HandlerExecution,
        envelope: Context,
        runtime: &HandlerRuntime<'_>,
        prepared: Option<&PreparedAgent>,
    ) -> Result<ExecutionTransition, LifecycleRunError> {
        let dispatched = self
            .session
            .as_mut()
            .ok_or(LifecycleRunError::Unbound)?
            .dispatch_execution(execution, envelope.clone(), runtime, prepared);
        let outcome = match dispatched {
            Ok(outcome) => outcome,
            Err(source) => return Err(failed_transition(envelope, source)),
        };
        let status = match outcome.reply.status {
            ReplyStatus::Ok => ExecutionRecordStatus::Ok,
            ReplyStatus::Skip => ExecutionRecordStatus::Skip,
            ReplyStatus::Fail => unreachable!("dispatch rejects fail replies"),
        };
        Ok(ExecutionTransition {
            envelope: outcome.envelope,
            status,
            message: outcome.reply.message,
            artifacts: outcome
                .reply
                .artifacts
                .into_iter()
                .map(|artifact| StateArtifact {
                    id: artifact.id,
                    kind: artifact.kind,
                    path: artifact.path,
                    witness: None,
                    measured_run_id: None,
                })
                .collect(),
            streams: outcome.streams,
            delegation: None,
        })
    }
}

fn failed_transition(envelope: Context, source: DispatchError) -> LifecycleRunError {
    let transition = FailedExecutionTransition {
        envelope,
        message: source.to_string(),
        streams: source.streams().cloned().unwrap_or_default(),
    };
    LifecycleRunError::FailedTransition {
        transition: Box::new(transition),
        source: Box::new(source),
    }
}

fn elapsed_ms(started: Instant) -> u32 {
    u32::try_from(started.elapsed().as_millis()).unwrap_or(u32::MAX)
}

#[path = "runner/error.rs"]
mod error;
pub use error::LifecycleRunError;

#[path = "runner/hosted.rs"]
mod hosted;

#[path = "runner/owed.rs"]
mod owed;
pub use owed::{REMOVED_DECLARATION, REMOVED_SLOT_DECLARATION, UNKNOWN_PROVENANCE};

#[cfg(test)]
#[path = "runner/tests.rs"]
mod hosted_tests;
