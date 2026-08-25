//! One canonical envelope/fingerprint/state/handler transition.

use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use specmark::spec;
use thiserror::Error;
use vibe_wire::generated::lifecycle::e1::context::{Context, Project, World};
use vibe_wire::generated::lifecycle::e1::reply::ReplyStatus;
use vibe_wire::generated::lifecycle_state::{
    ExecutionRecord, ExecutionRecordStatus, StateArtifact,
};

use crate::handlers::{HandlerRuntime, HandlerStreams};
use crate::{
    ContributionOutcome, DispatchError, ExecutionSession, FingerprintError, HandlerExecution,
    LifecycleStateError, LifecycleStateStore, RunMetadata, fingerprint_handler_execution,
    preparation_error_fingerprint_for_identity,
};

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

#[derive(Debug, Clone)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#REPLY-SHAPE")]
pub struct ExecutionTransition {
    pub envelope: Context,
    pub status: ExecutionRecordStatus,
    pub message: Option<String>,
    pub artifacts: Vec<StateArtifact>,
    pub streams: HandlerStreams,
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

#[derive(Debug, Error)]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#FAILURE-BY-PHASE")]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#PHASE-STATE-HOME")]
pub enum LifecycleRunError {
    #[error(
        "lifecycle envelope preparation failed for `{key}`: {source} \
         (governed by spec://org.vibevm.core/vibevm/common/PROP-054#ENVELOPE-LAW; \
          fix: correct the named execution's configuration and rerun)"
    )]
    Envelope {
        key: String,
        #[source]
        source: DispatchError,
    },
    #[error(
        "lifecycle fingerprint preparation failed for `{key}`: {source} \
         (governed by spec://org.vibevm.core/vibevm/common/PROP-054#PHASE-FINGERPRINT; \
          fix: correct the named execution's declared inputs and rerun)"
    )]
    Fingerprint {
        key: String,
        #[source]
        source: FingerprintError,
    },
    #[error(transparent)]
    Dispatch(#[from] DispatchError),
    #[error(
        "{source} (failed lifecycle transition checkpointed; governed by \
         spec://org.vibevm.core/vibevm/common/PROP-054#OBS-RUN-RECORD; \
         fix: correct the named handler and rerun)"
    )]
    FailedTransition {
        transition: Box<FailedExecutionTransition>,
        #[source]
        source: Box<DispatchError>,
    },
    #[error(transparent)]
    State(#[from] LifecycleStateError),
    #[error(
        "{primary}; also failed to checkpoint lifecycle failure for `{key}`: {checkpoint} \
         (governed by spec://org.vibevm.core/vibevm/common/PROP-054#PHASE-STATE-HOME; \
          fix: restore a writable .vibe cache and rerun)"
    )]
    Checkpoint {
        key: String,
        primary: String,
        checkpoint: Box<LifecycleStateError>,
        transition: Option<Box<FailedExecutionTransition>>,
        dispatch: Option<Box<DispatchError>>,
    },
    #[error(
        "lifecycle run has not been bound to a selected project/world \
         (governed by spec://org.vibevm.core/vibevm/common/PROP-054#ENVELOPE-LAW; \
          fix: bind the selected project/world before executing a contribution)"
    )]
    Unbound,
    #[error(
        "state checkpoint was requested from the state-blind clean runner \
         (governed by spec://org.vibevm.core/vibevm/common/PROP-054#PHASE-STATE-HOME; \
          fix: use tracked LifecycleRun::begin for freshness-aware execution)"
    )]
    UntrackedCheckpoint,
}

impl LifecycleRunError {
    #[must_use]
    pub fn is_durable_soft_post(&self) -> bool {
        match self {
            Self::FailedTransition { source, .. } => source.is_durable_soft_post(),
            _ => false,
        }
    }

    #[must_use]
    pub fn dispatch_error(&self) -> Option<&DispatchError> {
        match self {
            Self::Dispatch(error) => Some(error),
            Self::FailedTransition { source, .. } => Some(source.as_ref()),
            Self::Checkpoint {
                dispatch: Some(error),
                ..
            } => Some(error.as_ref()),
            _ => None,
        }
    }

    #[must_use]
    pub fn failed_transition(&self) -> Option<&FailedExecutionTransition> {
        match self {
            Self::FailedTransition { transition, .. } => Some(transition.as_ref()),
            Self::Checkpoint {
                transition: Some(transition),
                ..
            } => Some(transition.as_ref()),
            _ => None,
        }
    }
}

/// Mutable state for one complete lifecycle invocation.
#[derive(Debug)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM")]
pub struct LifecycleRun {
    session: Option<ExecutionSession>,
    state: Option<LifecycleStateStore>,
    force: bool,
}

impl LifecycleRun {
    pub fn begin(
        workspace_root: &Path,
        project: Project,
        world: World,
        metadata: RunMetadata,
        state_chain: Vec<String>,
    ) -> Result<Self, LifecycleRunError> {
        let state = LifecycleStateStore::begin(
            workspace_root,
            metadata.requested.clone(),
            state_chain,
            metadata.started.clone(),
        )?;
        Ok(Self {
            force: metadata.force,
            session: Some(ExecutionSession::new(project, world, metadata)),
            state: Some(state),
        })
    }

    /// Create the state-blind clean runner. It uses the same handler
    /// transition but never reads or rewrites the erasable freshness cache.
    #[must_use]
    pub fn untracked(project: Project, world: World, metadata: RunMetadata) -> Self {
        Self {
            force: metadata.force,
            session: Some(ExecutionSession::new(project, world, metadata)),
            state: None,
        }
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
        if self.state.is_none() {
            return self.dispatch_untracked(execution, envelope, runtime);
        }
        let fingerprint = match fingerprint_handler_execution(execution, &envelope) {
            Ok(fingerprint) => fingerprint,
            Err(source) => {
                let failure = LifecycleRunError::Fingerprint {
                    key: key.clone(),
                    source,
                };
                return Err(self.checkpoint_preparation_failure(execution, phase, started, failure));
            }
        };
        if reuse == ExecutionReuse::FreshnessAware
            && !self.force
            && let Some(prior) = self
                .state
                .as_ref()
                .and_then(|state| state.reusable_record(&key, &fingerprint))
                .cloned()
        {
            self.session
                .as_mut()
                .ok_or(LifecycleRunError::Unbound)?
                .hydrate_artifacts(phase, &prior.artifacts);
            self.state
                .as_mut()
                .ok_or(LifecycleRunError::UntrackedCheckpoint)?
                .checkpoint(
                    key,
                    ExecutionRecord {
                        artifacts: prior.artifacts.clone(),
                        duration_ms: 0,
                        fingerprint,
                        phase: phase.into(),
                        status: ExecutionRecordStatus::Fresh,
                    },
                )?;
            return Ok(ExecutionTransition {
                envelope,
                status: ExecutionRecordStatus::Fresh,
                message: None,
                artifacts: prior.artifacts,
                streams: HandlerStreams::default(),
            });
        }

        let dispatched = self
            .session
            .as_mut()
            .ok_or(LifecycleRunError::Unbound)?
            .dispatch_execution(execution, envelope.clone(), runtime);
        match dispatched {
            Ok(outcome) => self.checkpoint_success(key, phase, fingerprint, started, outcome),
            Err(source) => {
                let record = ExecutionRecord {
                    artifacts: Vec::new(),
                    duration_ms: elapsed_ms(started),
                    fingerprint,
                    phase: phase.into(),
                    status: ExecutionRecordStatus::Fail,
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
    ) -> Result<ExecutionTransition, LifecycleRunError> {
        let status = match outcome.reply.status {
            ReplyStatus::Ok => ExecutionRecordStatus::Ok,
            ReplyStatus::Skip => ExecutionRecordStatus::Skip,
            ReplyStatus::Fail => unreachable!("dispatch rejects fail replies"),
        };
        let artifacts = outcome
            .reply
            .artifacts
            .iter()
            .map(|artifact| StateArtifact {
                id: artifact.id.clone(),
                kind: artifact.kind.clone(),
                path: artifact.path.clone(),
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
                },
            )?;
        Ok(ExecutionTransition {
            envelope: outcome.envelope,
            status,
            message: outcome.reply.message,
            artifacts,
            streams: outcome.streams,
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
    ) -> Result<ExecutionTransition, LifecycleRunError> {
        let dispatched = self
            .session
            .as_mut()
            .ok_or(LifecycleRunError::Unbound)?
            .dispatch_execution(execution, envelope.clone(), runtime);
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
                })
                .collect(),
            streams: outcome.streams,
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
