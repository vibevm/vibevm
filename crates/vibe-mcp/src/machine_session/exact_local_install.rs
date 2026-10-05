//! Framed exact-local Install through one explicitly granted owner route.

use super::{
    fixed,
    framing::{FrameCodecError, read_frame, write_frame},
    runner::RunnerStep,
    state::{MachineSession, SessionBuildError, SessionDecision, SessionPhase, UnvalidatedRequest},
};
use crate::ServerContext;
use std::{
    io::{Read, Write},
    path::PathBuf,
    sync::Arc,
};
use vibe_orchestrator::{
    machine_operation::exact_local_install::{ExactLocalInstallRoute, ExactLocalInstallRouteError},
    machine_submission::ExactLocalInstallDecisionError,
};
use vibe_wire::generated::machine::e1::{operation_envelope as e, session_frame as f};

#[derive(Debug, thiserror::Error)]
pub enum ExactLocalInstallSessionBuildError {
    #[error("no explicitly granted exact-local Install route")]
    UnavailableHost,
    #[error("physical context root differs from exact-local Install route: {0}")]
    Root(ExactLocalInstallDecisionError),
    #[error("session binding refused: {0:?}")]
    Session(SessionBuildError),
}

#[derive(Debug, thiserror::Error)]
pub enum ExactLocalInstallRunnerError {
    #[error("framed input refused: {0:?}")]
    Frame(FrameCodecError),
    #[error("framed output failed: {error:?}")]
    Output {
        error: FrameCodecError,
        response: f::SessionFrame,
    },
    #[error("exact-local Install publication failed: {0}")]
    Publication(ExactLocalInstallRouteError),
    #[error("envelope serialization failed")]
    EnvelopeSerialization(e::OperationEnvelope),
    #[error("invalid owner envelope: {0}")]
    Outcome(#[from] fixed::FixedRunnerError),
}

/// One connection over an existing route. A fresh connection can share the
/// same route and C1 key; outer session IDs remain connection-local.
pub struct ExactLocalInstallMachineSessionRunner {
    session: MachineSession,
    route: Arc<ExactLocalInstallRoute>,
    root: PathBuf,
}
impl ExactLocalInstallMachineSessionRunner {
    pub fn new(context: ServerContext) -> Result<Self, ExactLocalInstallSessionBuildError> {
        let route = context
            .exact_local_install_bridge()
            .cloned()
            .ok_or(ExactLocalInstallSessionBuildError::UnavailableHost)?;
        route
            .compare_context_root(&context.project_root)
            .map_err(ExactLocalInstallSessionBuildError::Root)?;
        Ok(Self {
            session: MachineSession::bound_exact_local_install(&route)
                .map_err(ExactLocalInstallSessionBuildError::Session)?,
            route,
            root: context.project_root,
        })
    }

    pub fn step<R: Read, W: Write>(
        &mut self,
        reader: &mut R,
        writer: &mut W,
    ) -> Result<RunnerStep, ExactLocalInstallRunnerError> {
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
                return Err(ExactLocalInstallRunnerError::Frame(error));
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
            return Err(ExactLocalInstallRunnerError::Output { error, response });
        }
        Ok(RunnerStep::Replied)
    }

    pub fn run<R: Read, W: Write>(
        &mut self,
        reader: &mut R,
        writer: &mut W,
    ) -> Result<usize, ExactLocalInstallRunnerError> {
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
    ) -> Result<f::SessionFrame, ExactLocalInstallRunnerError> {
        let envelope = ExactLocalInstallRoute::exchange_local(
            Some(&self.route),
            request.envelope_json.as_bytes(),
            &self.root,
        )
        .map_err(ExactLocalInstallRunnerError::Publication)?;
        let outcome = fixed::outcome(&envelope)?;
        let refused = matches!(envelope.body, e::Body::Error(_));
        let json = serde_json::to_string(&envelope)
            .map_err(|_| ExactLocalInstallRunnerError::EnvelopeSerialization(envelope))?;
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

#[cfg(test)]
mod tests;
