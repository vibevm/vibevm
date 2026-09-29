//! On-demand fake pure-planner exchange over the separate byte frame codec.
//! It neither starts the MCP server nor accepts an effectful operation.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-LOCAL-FRAME");

use std::io::{Read, Write};

use vibe_orchestrator::machine_operation::{
    MachineOperationError, MachineOperationService, PurePlannerAdmission,
    exchange::{PublishedPlannerStatus, PurePlannerExchange},
};
use vibe_wire::generated::machine::e1::{
    operation_envelope::OperationEnvelope,
    session_frame::{
        Body, BodyComplete, BodyRefusal, BodyRefusalReason, Outcome, OutcomeCompleteness,
        OutcomeEffectState, SessionFrame,
    },
};

use super::{
    framing::{FrameCodecError, read_frame, write_frame},
    state::{MachineSession, SessionDecision, SessionPhase, UnvalidatedRequest},
};

#[derive(Debug)]
pub enum RunnerError {
    Frame(FrameCodecError),
    Output {
        error: FrameCodecError,
        response: SessionFrame,
    },
    Service(MachineOperationError),
    EnvelopeSerialization(OperationEnvelope),
    /// Original unvalidated evidence stays in the error channel only.
    Diagnostic(PurePlannerExchange),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunnerStep {
    Replied,
    Closed,
}

pub struct MachineSessionRunner {
    session: MachineSession,
    service: MachineOperationService,
}

fn outcome(completeness: OutcomeCompleteness) -> Outcome {
    Outcome {
        effect_state: OutcomeEffectState::NoEffects,
        completeness,
        provenance_id: None,
    }
}

fn reply(request: &UnvalidatedRequest, body: Body) -> SessionFrame {
    SessionFrame {
        schema: 1,
        session_id: Some(request.session_id.clone()),
        request_id: Some(request.request_id.clone()),
        body,
    }
}

fn refused(
    request: &UnvalidatedRequest,
    reason: BodyRefusalReason,
    envelope_json: Option<String>,
    completeness: OutcomeCompleteness,
) -> SessionFrame {
    reply(
        request,
        Body::Refusal(Box::new(BodyRefusal {
            reason,
            envelope_json,
            outcome: outcome(completeness),
        })),
    )
}

fn serialize_validated(envelope: OperationEnvelope) -> Result<String, RunnerError> {
    serde_json::to_string(&envelope).map_err(|_| RunnerError::EnvelopeSerialization(envelope))
}

impl MachineSessionRunner {
    /// Caller composes the finite trusted session. The sole service route is
    /// the already reviewed pure planner, independently admitted here.
    pub fn new(session: MachineSession) -> Result<Self, MachineOperationError> {
        Ok(Self {
            session,
            service: MachineOperationService::pure_text_planner(PurePlannerAdmission::Admitted)?,
        })
    }

    /// One received frame becomes one reply, or EOF closes this connection.
    /// Malformed bytes stop before session/domain dispatch.
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-LOCAL-FRAME",
        r = 1
    )]
    pub fn step<R: Read, W: Write>(
        &mut self,
        reader: &mut R,
        writer: &mut W,
    ) -> Result<RunnerStep, RunnerError> {
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
                return Err(RunnerError::Frame(error));
            }
        };
        let response = match self.session.receive(frame) {
            SessionDecision::Reply(response) => response,
            SessionDecision::UnvalidatedRequest(request) => {
                match self.pure_planner_reply(&request) {
                    Ok(response) => response,
                    Err(error) => {
                        self.session.close();
                        return Err(error);
                    }
                }
            }
        };
        if let Err(error) = write_frame(writer, &response) {
            self.session.close();
            return Err(RunnerError::Output { error, response });
        }
        Ok(RunnerStep::Replied)
    }

    /// Drives only this supplied reader/writer until EOF. No listener or
    /// daemon is installed, and the caller owns the connection lifetime.
    pub fn run<R: Read, W: Write>(
        &mut self,
        reader: &mut R,
        writer: &mut W,
    ) -> Result<usize, RunnerError> {
        let mut replies = 0;
        loop {
            match self.step(reader, writer)? {
                RunnerStep::Replied => replies += 1,
                RunnerStep::Closed => return Ok(replies),
            }
        }
    }

    fn pure_planner_reply(
        &self,
        request: &UnvalidatedRequest,
    ) -> Result<SessionFrame, RunnerError> {
        if request.target_id.is_some() {
            return Ok(refused(
                request,
                BodyRefusalReason::InvalidRequest,
                None,
                OutcomeCompleteness::Unavailable,
            ));
        }
        let envelope: OperationEnvelope = match serde_json::from_str(&request.envelope_json) {
            Ok(envelope) => envelope,
            Err(_) => {
                return Ok(refused(
                    request,
                    BodyRefusalReason::InvalidRequest,
                    None,
                    OutcomeCompleteness::Unavailable,
                ));
            }
        };
        match self.service.exchange_pure_planner(envelope) {
            PurePlannerExchange::Published { status, envelope } => {
                let completeness = match status {
                    PublishedPlannerStatus::NeedsInput => OutcomeCompleteness::Partial,
                    PublishedPlannerStatus::Success | PublishedPlannerStatus::Refused => {
                        OutcomeCompleteness::Complete
                    }
                };
                Ok(reply(
                    request,
                    Body::Complete(Box::new(BodyComplete {
                        envelope_json: Some(serialize_validated(envelope)?),
                        outcome: outcome(completeness),
                    })),
                ))
            }
            PurePlannerExchange::Preflight { envelope, .. } => {
                let identified = envelope.operation_id.is_some();
                let payload = identified
                    .then(|| serialize_validated(envelope))
                    .transpose()?;
                Ok(refused(
                    request,
                    BodyRefusalReason::InvalidRequest,
                    payload,
                    if identified {
                        OutcomeCompleteness::Complete
                    } else {
                        OutcomeCompleteness::Unavailable
                    },
                ))
            }
            PurePlannerExchange::Unavailable { .. } => Ok(refused(
                request,
                BodyRefusalReason::Unavailable,
                None,
                OutcomeCompleteness::Unavailable,
            )),
            diagnostic @ PurePlannerExchange::Diagnostic { .. } => {
                Err(RunnerError::Diagnostic(diagnostic))
            }
        }
    }
}

#[cfg(test)]
mod tests;
