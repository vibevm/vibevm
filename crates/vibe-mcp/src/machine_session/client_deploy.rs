//! Existing finite framing and session identity over the typed client owner.
use super::{
    framing::{FrameCodecError, read_frame, write_frame},
    runner::RunnerStep,
    state::{MachineSession, SessionDecision, SessionPhase, UnvalidatedRequest},
};
use crate::ServerContext;
use std::{
    io::{Read, Write},
    path::PathBuf,
    sync::Arc,
};
use vibe_orchestrator::machine_operation::client_deploy::ClientDeployRoute;
use vibe_wire::generated::machine::e1::{operation_envelope as e, session_frame as f};
#[derive(Debug)]
pub enum ClientDeployRunnerError {
    Frame(FrameCodecError),
    Output {
        error: FrameCodecError,
        response: Box<f::SessionFrame>,
    },
    Publication(Box<String>),
    EnvelopeSerialization(Box<e::OperationEnvelope>),
    InvalidPublication(Box<e::OperationEnvelope>),
}
pub struct ClientDeployMachineSessionRunner {
    session: MachineSession,
    route: Arc<ClientDeployRoute>,
    root: PathBuf,
}
impl ClientDeployMachineSessionRunner {
    /// Consume a context with an explicit fixed host grant; no ambient discovery.
    pub fn new(context: ServerContext) -> Result<Self, String> {
        let route = context
            .client_deploy_route()
            .cloned()
            .ok_or("client deployment host unavailable")?;
        route.compare_context_root(&context.project_root)?;
        Ok(Self {
            session: MachineSession::bound_client_deploy(&route).map_err(|e| format!("{e:?}"))?,
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
    ) -> Result<RunnerStep, ClientDeployRunnerError> {
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
                return Err(ClientDeployRunnerError::Frame(error));
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
            return Err(ClientDeployRunnerError::Output {
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
    ) -> Result<usize, ClientDeployRunnerError> {
        let mut count = 0;
        loop {
            match self.step(reader, writer)? {
                RunnerStep::Replied => count += 1,
                RunnerStep::Closed => return Ok(count),
            }
        }
    }
    fn reply(
        &self,
        request: &UnvalidatedRequest,
    ) -> Result<f::SessionFrame, ClientDeployRunnerError> {
        let envelope = ClientDeployRoute::exchange_local(
            Some(&self.route),
            request.envelope_json.as_bytes(),
            &self.root,
        )
        .map_err(|error| ClientDeployRunnerError::Publication(Box::new(error)))?;
        let outcome = outcome(&envelope)?;
        let refused = matches!(envelope.body, e::Body::Error(_));
        let json = serde_json::to_string(&envelope)
            .map_err(|_| ClientDeployRunnerError::EnvelopeSerialization(Box::new(envelope)))?;
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
fn outcome(envelope: &e::OperationEnvelope) -> Result<f::Outcome, ClientDeployRunnerError> {
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
            return Err(ClientDeployRunnerError::InvalidPublication(Box::new(
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
