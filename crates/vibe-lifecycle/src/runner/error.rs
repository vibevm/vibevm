//! Why one lifecycle transition refused, and how that refusal reads.
//!
//! Split out of `runner.rs` so the transition algorithm and its diagnostic
//! vocabulary are separate cells: the enum is long by design (every variant
//! carries its own remediation), and it grew a hosted-handoff variant in
//! R7.3.

use specmark::spec;
use thiserror::Error;

use crate::agent::AgentError;
use crate::handlers::HandlerError;
use crate::{DispatchError, FingerprintError, LifecycleStateError};

use super::FailedExecutionTransition;

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
    #[error(
        "lifecycle agent preparation failed for `{key}`: {source} \
         (governed by spec://org.vibevm.core/vibevm/common/PROP-054#PHASE-FINGERPRINT; \
          fix: correct the named execution's contract or prompt and rerun)"
    )]
    AgentPreparation {
        key: String,
        #[source]
        source: Box<AgentError>,
    },
    #[error(
        "parking lifecycle execution `{key}` for the hosting agent failed: {source} \
         (governed by spec://org.vibevm.core/vibevm/common/PROP-054#AGENT-HANDSHAKE; \
          fix: apply the inner failure's remediation and rerun the stopped lifecycle)"
    )]
    DelegationPark {
        key: String,
        #[source]
        source: Box<crate::delegation::DelegationError>,
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
    /// The comparison could not be assembled into a member this wire may
    /// publish. Deliberately NOT one of the five evidence words: those name
    /// what an honest observation SAW, and there is no observation here — a
    /// state row that cannot be located under the selected project, or an
    /// assembled member that breaks its own relational law, is a defect in
    /// what verify was handed, not a verdict about the project's work.
    #[error(
        "lifecycle verification evidence could not be assembled: {reason} \
         (governed by spec://org.vibevm.core/vibevm/common/PROP-054#EVIDENCE-WIRE-AND-SURFACES; \
          fix: rebuild `.vibe/lifecycle.toml` by rerunning the lifecycle with --force)"
    )]
    Verification { reason: String },
}

impl LifecycleRunError {
    #[must_use]
    pub fn is_durable_soft_post(&self) -> bool {
        match self {
            Self::FailedTransition { source, .. } => source.is_durable_soft_post(),
            _ => false,
        }
    }

    /// The typed agent refusal, whether it came from the credential-free
    /// preparation or from the dispatched paid half.
    #[must_use]
    pub fn agent_error(&self) -> Option<&AgentError> {
        if let Self::AgentPreparation { source, .. } = self {
            return Some(source.as_ref());
        }
        match self.dispatch_error()? {
            DispatchError::Handler { error, .. } => match error.as_ref() {
                HandlerError::Agent { error, .. } => Some(error.as_ref()),
                _ => None,
            },
            _ => None,
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

#[cfg(test)]
mod diagnostic_tests {
    use super::{AgentError, LifecycleRunError};

    /// Every run refusal is one sentence. A multi-line `#[error]` literal
    /// written without a `\` continuation bakes the next line's source
    /// indentation into the message — and no `contains` assertion on a
    /// fragment either side of that seam would ever notice.
    #[test]
    fn run_refusals_render_as_single_spaced_sentences() {
        let rendered = LifecycleRunError::AgentPreparation {
            key: "org.demo/tools#produce".into(),
            source: Box::new(AgentError::Contract {
                reason: "`config.outputs` is absent".into(),
            }),
        }
        .to_string();

        assert!(
            rendered.starts_with(
                "lifecycle agent preparation failed for `org.demo/tools#produce`: the declared \
                 output contract is invalid: `config.outputs` is absent (governed by"
            ),
            "{rendered}"
        );
        assert!(
            rendered.ends_with("fix: correct the named execution's contract or prompt and rerun)"),
            "the remediation must survive intact: {rendered}"
        );
        assert!(
            !rendered.contains("  "),
            "a run of spaces is baked source indentation: {rendered}",
        );
    }
}
