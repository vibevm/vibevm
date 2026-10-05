//! Explicitly host-granted fixed Deploy actions over the incumbent local frame.
//! Outer IDs select the retained route; inner owner IDs remain untouched.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-FIXED-DEPLOY-REATTACH");
use super::{
    framing::{FrameCodecError, read_frame, write_frame},
    runner::RunnerStep,
    state::{MachineSession, SessionBuildError, SessionDecision, SessionPhase, UnvalidatedRequest},
};
use crate::ServerContext;
use specmark::spec;
use std::{
    io::{Read, Write},
    path::PathBuf,
    sync::Arc,
};
use vibe_orchestrator::machine_operation::fixed_deploy::{
    FixedDeployHostError, FixedDeployRoute, FixedDeployRouteError,
};
use vibe_wire::generated::machine::e1::{operation_envelope as e, session_frame as f};

/// Construction errors do not create a host, namespace or provider.
#[derive(Debug, thiserror::Error)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-TRUSTED-CONTEXT")]
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-TRUSTED-CONTEXT",
    r = 1
)]
pub enum DeploySessionBuildError {
    #[error(
        "violates REQ spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-TRUSTED-CONTEXT: no explicit host grant; fix surface: embedding ServerContext"
    )]
    UnavailableHost,
    #[error(
        "violates REQ spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-TRUSTED-CONTEXT: {0}; fix surface: embedding physical root"
    )]
    Root(Box<FixedDeployHostError>),
    #[error(
        "violates REQ spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-TRUSTED-CONTEXT: {0:?}; fix surface: session binding"
    )]
    Session(SessionBuildError),
}
/// Output failure retains the exact typed response and its owner evidence.
#[derive(Debug, thiserror::Error)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-FIXED-DEPLOY-REATTACH")]
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-FIXED-DEPLOY-REATTACH",
    r = 1
)]
pub enum DeployRunnerError {
    #[error(
        "violates REQ spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-LOCAL-FRAME: {0:?}; fix surface: supplied frame bytes"
    )]
    Frame(FrameCodecError),
    #[error(
        "violates REQ spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-LOCAL-FRAME: {error:?}; fix surface: caller writer and retained response"
    )]
    Output {
        error: FrameCodecError,
        response: Box<f::SessionFrame>,
    },
    #[error(
        "violates REQ spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-OPERATION-VALIDATION: {0}; fix surface: fixed owner publication"
    )]
    Publication(Box<FixedDeployRouteError>),
    #[error(
        "violates REQ spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-OPERATION-VALIDATION: envelope serialization failed; fix surface: retained generated envelope"
    )]
    EnvelopeSerialization(Box<e::OperationEnvelope>),
    #[error(
        "violates REQ spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-OPERATION-VALIDATION: expected Result/Error; fix surface: retained generated envelope"
    )]
    InvalidPublication(Box<e::OperationEnvelope>),
}
/// Finite library-callable framed mode over one actual host and target scope.
/// EOF/output failure closes this connection; the retained host owns submitted
/// work and can be inspected through a fresh explicitly granted connection.
///
/// ```no_run
/// use std::{io::Cursor, sync::Arc};
/// use vibe_mcp::{ServerContext, machine_session::deploy::DeployMachineSessionRunner};
/// use vibe_orchestrator::machine_operation::fixed_deploy::FixedDeployRoute;
/// fn serve(context: ServerContext, route: Arc<FixedDeployRoute>) {
///     let context = context.with_fixed_deploy_route(route).unwrap();
///     let mut session = DeployMachineSessionRunner::new(context).unwrap();
///     let mut output = Vec::new();
///     assert_eq!(session.run(&mut Cursor::new(Vec::<u8>::new()), &mut output).unwrap(), 0);
/// }
/// ```
/// A shaped descriptor cannot provide the required private host binding:
/// ```
/// use vibe_mcp::{ServerContext, machine_session::deploy::DeployMachineSessionRunner};
/// assert!(DeployMachineSessionRunner::new(ServerContext::with_store_root("unbound", "unused")).is_err());
/// ```
pub struct DeployMachineSessionRunner {
    session: MachineSession,
    route: Arc<FixedDeployRoute>,
    root: PathBuf,
}
impl DeployMachineSessionRunner {
    /// Consume a context with an explicit fixed host grant; no ambient discovery.
    pub fn new(context: ServerContext) -> Result<Self, DeploySessionBuildError> {
        let route = context
            .fixed_deploy_bridge()
            .cloned()
            .ok_or(DeploySessionBuildError::UnavailableHost)?;
        route
            .compare_context_root(&context.project_root)
            .map_err(|error| DeploySessionBuildError::Root(Box::new(error)))?;
        Ok(Self {
            session: MachineSession::bound_fixed_deploy(&route)
                .map_err(DeploySessionBuildError::Session)?,
            route,
            root: context.project_root,
        })
    }
    /// One bounded generated frame gets one reply. Complete means publication,
    /// while nested owner truth describes the deploy owner's progress.
    pub fn step<R: Read, W: Write>(
        &mut self,
        reader: &mut R,
        writer: &mut W,
    ) -> Result<RunnerStep, DeployRunnerError> {
        if self.session.phase() == SessionPhase::Closed {
            return Ok(RunnerStep::Closed);
        }
        let frame = match read_frame(reader) {
            Ok(Some(frame)) => frame,
            Ok(None) => {
                self.session.close();
                return Ok(RunnerStep::Closed);
            }
            Err(error) => {
                self.session.close();
                return Err(DeployRunnerError::Frame(error));
            }
        };
        let response = match self.session.receive(frame) {
            SessionDecision::Reply(response) => response,
            SessionDecision::UnvalidatedRequest(request) => match self.reply(&request) {
                Ok(response) => response,
                Err(error) => {
                    self.session.close();
                    return Err(error);
                }
            },
        };
        if let Err(error) = write_frame(writer, &response) {
            self.session.close();
            return Err(DeployRunnerError::Output {
                error,
                response: Box::new(response),
            });
        }
        if self.session.correlation_exhausted() {
            self.session.close();
        }
        Ok(RunnerStep::Replied)
    }
    /// Drive only the caller-supplied reader/writer until EOF; no listener.
    pub fn run<R: Read, W: Write>(
        &mut self,
        reader: &mut R,
        writer: &mut W,
    ) -> Result<usize, DeployRunnerError> {
        let mut count = 0;
        loop {
            match self.step(reader, writer)? {
                RunnerStep::Replied => count += 1,
                RunnerStep::Closed => return Ok(count),
            }
        }
    }
    fn reply(&self, request: &UnvalidatedRequest) -> Result<f::SessionFrame, DeployRunnerError> {
        let envelope = FixedDeployRoute::exchange_local(
            Some(&self.route),
            request.envelope_json.as_bytes(),
            &self.root,
        )
        .map_err(|error| DeployRunnerError::Publication(Box::new(error)))?;
        let outcome = outcome(&envelope)?;
        let refused = matches!(envelope.body, e::Body::Error(_));
        let json = serde_json::to_string(&envelope)
            .map_err(|_| DeployRunnerError::EnvelopeSerialization(Box::new(envelope)))?;
        Ok(f::SessionFrame {
            schema: 1,
            session_id: Some(request.session_id.clone()),
            request_id: Some(request.request_id.clone()),
            body: if refused {
                f::Body::Refusal(Box::new(f::BodyRefusal {
                    reason: f::BodyRefusalReason::InvalidRequest,
                    envelope_json: Some(json),
                    outcome,
                }))
            } else {
                f::Body::Complete(Box::new(f::BodyComplete {
                    envelope_json: Some(json),
                    outcome,
                }))
            },
        })
    }
}
fn outcome(envelope: &e::OperationEnvelope) -> Result<f::Outcome, DeployRunnerError> {
    // exchange_local publishes only Result/Error, already checked against its
    // typed owner response. Mirror rather than fabricate outer completion.
    let (effect, completeness, provenance_id) = match &envelope.body {
        e::Body::Result(r) => (
            match r.effect_state {
                e::BodyResultEffectState::NoEffects => f::OutcomeEffectState::NoEffects,
                e::BodyResultEffectState::Completed => f::OutcomeEffectState::Completed,
                e::BodyResultEffectState::Uncertain => f::OutcomeEffectState::Uncertain,
            },
            match r.completeness {
                e::BodyResultCompleteness::Complete => f::OutcomeCompleteness::Complete,
                e::BodyResultCompleteness::Partial => f::OutcomeCompleteness::Partial,
                e::BodyResultCompleteness::Unavailable => f::OutcomeCompleteness::Unavailable,
            },
            r.provenance_id.clone(),
        ),
        e::Body::Error(r) => (
            match r.effect_state {
                e::BodyErrorEffectState::NoEffects => f::OutcomeEffectState::NoEffects,
                e::BodyErrorEffectState::Completed => f::OutcomeEffectState::Completed,
                e::BodyErrorEffectState::Uncertain => f::OutcomeEffectState::Uncertain,
            },
            match r.completeness {
                e::BodyErrorCompleteness::Complete => f::OutcomeCompleteness::Complete,
                e::BodyErrorCompleteness::Partial => f::OutcomeCompleteness::Partial,
                e::BodyErrorCompleteness::Unavailable => f::OutcomeCompleteness::Unavailable,
            },
            r.provenance_id.clone(),
        ),
        _ => {
            return Err(DeployRunnerError::InvalidPublication(Box::new(
                envelope.clone(),
            )));
        }
    };
    Ok(f::Outcome {
        effect_state: effect,
        completeness,
        provenance_id,
    })
}
#[cfg(all(test, target_os = "linux"))]
#[path = "deploy/tests.rs"]
mod tests;
