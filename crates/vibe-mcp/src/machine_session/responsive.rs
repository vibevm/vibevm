//! Poll-driven observation of one already admitted host-owned operation.
//! The embedding host authenticates the connected port and retains its owning
//! job independently. No roots, admission, listener, or submit grammar live here.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-OPERATION-STREAM");

use super::{
    incremental::{MAX_FRAME_BYTES, ReadPoll, TcpFramePort, WritePoll},
    state::{MachineSession, SessionBuildError, SessionDecision, SessionPhase, UnvalidatedRequest},
};
use std::collections::VecDeque;
use vibe_orchestrator::{
    machine_operation::stream_message::{self, ExpectedBinding},
    machine_submission::{
        BindingSummary, FixedOperationStatus, HostControlCapability, ObservationHandle,
    },
};
use vibe_wire::generated::machine::e1::{
    operation_stream_message as stream, session_frame as frame,
};

pub const MAX_DELIVERY_FRAMES: usize = 16;
pub const MAX_DELIVERY_BYTES: usize = 1024 * 1024;
const REPLY_RESERVATION: usize = MAX_FRAME_BYTES + 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PollStatus {
    Open,
    Closed,
}

/// Terminal connection diagnostics retain the actual incumbent identity.
/// A full output queue cannot promise delivery of an additional wire diagnostic.
#[derive(Debug)]
pub enum SessionError {
    Overloaded {
        binding: BindingSummary,
    },
    Transport {
        binding: BindingSummary,
        reason: String,
    },
    Owner {
        binding: BindingSummary,
        reason: String,
    },
}

pub struct ResponsiveSession {
    session: MachineSession,
    reader: ObservationHandle,
    control: Option<HostControlCapability>,
    port: Option<TcpFramePort>,
    pending: VecDeque<(frame::SessionFrame, usize)>,
    delivery_bytes: usize,
    active_bytes: usize,
    cursor: u64,
    expired: bool,
    detached: bool,
}

fn outcome(completeness: frame::OutcomeCompleteness) -> frame::Outcome {
    // This is the observed operation's effect state, not a claim that reading
    // replay caused effects. Neither replay nor an acknowledgment proves that
    // the concurrently running operation completed; its retained domain result
    // and the typed control reply carry the narrower authoritative truth.
    frame::Outcome {
        effect_state: frame::OutcomeEffectState::Uncertain,
        completeness,
        provenance_id: None,
    }
}

// Count before allocating serialized envelope storage. This also caps escaped
// outer serialization and includes its four-byte codec prefix in accounting.
fn serialized_size<T: serde::Serialize>(value: &T, cap: usize) -> Result<usize, ()> {
    struct Counter {
        size: usize,
        cap: usize,
    }
    impl std::io::Write for Counter {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.size = self
                .size
                .checked_add(bytes.len())
                .filter(|n| *n <= self.cap)
                .ok_or_else(|| std::io::Error::other("responsive serialization cap"))?;
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut counter = Counter { size: 0, cap };
    serde_json::to_writer(&mut counter, value).map_err(|_| ())?;
    Ok(counter.size)
}
fn envelope(message: &stream::OperationStreamMessage) -> Result<String, ()> {
    serialized_size(message, stream_message::MAX_FRAME_BYTES)?;
    serde_json::to_string(message).map_err(|_| ())
}

impl ResponsiveSession {
    /// Takes only a sealed real reader, optional opaque host control, and an
    /// already authenticated/admitted connected nonblocking TcpFramePort.
    pub fn new(
        reader: ObservationHandle,
        control: Option<HostControlCapability>,
        port: TcpFramePort,
    ) -> Result<Self, SessionBuildError> {
        if let Some(capability) = &control {
            capability
                .validate_observer(&reader)
                .map_err(|_| SessionBuildError::ObserverUnavailable)?;
        }
        // A sealed but already detached handle must not advertise replay.
        // This bounded read verifies the incumbent subscription without effects.
        let summary = reader.summary();
        let probe = stream::OperationStreamMessage {
            schema: 1,
            context_id: summary.context_id().to_owned(),
            target_id: summary.target_id().map(str::to_owned),
            operation_id: summary.operation_id().to_owned(),
            run_id: summary.run_id().to_owned(),
            stream_id: summary.stream_id().to_owned(),
            body: stream::Body::ReplayRequest(Box::new(stream::BodyReplayRequest {
                after_sequence: "0".to_owned(),
                max_events: 1,
                max_bytes: 16 * 1024,
            })),
        };
        reader
            .replay(&probe)
            .map_err(|_| SessionBuildError::ObserverUnavailable)?;
        let session = MachineSession::bound_observer(&reader, control.is_some())?;
        Ok(Self {
            session,
            reader,
            control,
            port: Some(port),
            pending: VecDeque::new(),
            delivery_bytes: 0,
            active_bytes: 0,
            cursor: 0,
            expired: false,
            detached: false,
        })
    }

    /// Fresh facade inspection remains the marker/report authority; callers
    /// must not convert a replay terminal reference to a completed result.
    pub fn inspect(&self) -> Result<FixedOperationStatus, SessionError> {
        self.reader.inspect().map_err(|e| SessionError::Owner {
            binding: self.reader.summary().clone(),
            reason: e.to_string(),
        })
    }
    pub fn queued_frames(&self) -> usize {
        self.pending.len() + usize::from(self.active_bytes != 0)
    }
    pub fn queued_bytes(&self) -> usize {
        self.delivery_bytes
    }

    pub fn detach(&mut self) {
        self.reader.detach();
        self.detached = true;
        self.port.take();
        self.session.close();
        self.pending.clear();
        self.delivery_bytes = 0;
        self.active_bytes = 0;
    }
    fn overloaded(&mut self) -> SessionError {
        let error = SessionError::Overloaded {
            binding: self.reader.summary().clone(),
        };
        self.detach();
        error
    }
    fn transport(&mut self, reason: String) -> SessionError {
        let error = SessionError::Transport {
            binding: self.reader.summary().clone(),
            reason,
        };
        self.detach();
        error
    }
    fn owner_error(&mut self, reason: String) -> SessionError {
        let error = SessionError::Owner {
            binding: self.reader.summary().clone(),
            reason,
        };
        self.detach();
        error
    }
    fn can_reserve(&self) -> bool {
        self.queued_frames() < MAX_DELIVERY_FRAMES
            && self.delivery_bytes <= MAX_DELIVERY_BYTES - REPLY_RESERVATION
    }
    fn queue(&mut self, frame: frame::SessionFrame) -> Result<(), SessionError> {
        let size = serialized_size(&frame, MAX_FRAME_BYTES).map_err(|_| self.overloaded())? + 4;
        if self.queued_frames() >= MAX_DELIVERY_FRAMES
            || size > MAX_DELIVERY_BYTES - self.delivery_bytes
        {
            return Err(self.overloaded());
        }
        self.delivery_bytes += size;
        self.pending.push_back((frame, size));
        Ok(())
    }
    fn expected(&self) -> ExpectedBinding<'_> {
        let summary = self.reader.summary();
        ExpectedBinding {
            context_id: summary.context_id(),
            target_id: summary.target_id(),
            operation_id: summary.operation_id(),
            run_id: summary.run_id(),
            stream_id: summary.stream_id(),
        }
    }
    fn stream_request(&self, body: stream::Body) -> stream::OperationStreamMessage {
        let summary = self.reader.summary();
        stream::OperationStreamMessage {
            schema: 1,
            context_id: summary.context_id().to_owned(),
            target_id: summary.target_id().map(str::to_owned),
            operation_id: summary.operation_id().to_owned(),
            run_id: summary.run_id().to_owned(),
            stream_id: summary.stream_id().to_owned(),
            body,
        }
    }
    fn refusal(
        &self,
        request: &UnvalidatedRequest,
        reason: frame::BodyRefusalReason,
    ) -> frame::SessionFrame {
        // NoEffects describes this rejected request only. It is not an effect
        // statement about the independently owned, potentially running job.
        frame::SessionFrame {
            schema: 1,
            session_id: Some(request.session_id.clone()),
            request_id: Some(request.request_id.clone()),
            body: frame::Body::Refusal(Box::new(frame::BodyRefusal {
                reason,
                envelope_json: None,
                outcome: frame::Outcome {
                    effect_state: frame::OutcomeEffectState::NoEffects,
                    completeness: frame::OutcomeCompleteness::Unavailable,
                    provenance_id: None,
                },
            })),
        }
    }
    fn dispatch(
        &mut self,
        request: UnvalidatedRequest,
    ) -> Result<frame::SessionFrame, SessionError> {
        let message = match stream_message::parse(request.envelope_json.as_bytes(), self.expected())
        {
            Ok(message) => message,
            Err(_) => return Ok(self.refusal(&request, frame::BodyRefusalReason::InvalidRequest)),
        };
        let response = match &message.body {
            stream::Body::ReplayRequest(_) => self.reader.replay(&message),
            stream::Body::ControlRequest(control) => {
                // Correlate the inner request too, before invoking the owner.
                if control.request_id != request.request_id {
                    return Ok(self.refusal(&request, frame::BodyRefusalReason::InvalidRequest));
                }
                if let Some(capability) = &self.control {
                    capability.control(&mut self.reader, &message)
                } else {
                    let detach = control.action == stream::BodyControlRequestAction::Detach;
                    let status = if detach {
                        self.reader.detach();
                        self.detached = true;
                        stream::BodyControlReplyStatus::Detached
                    } else {
                        stream::BodyControlReplyStatus::Unavailable
                    };
                    Ok(self.stream_request(stream::Body::ControlReply(Box::new(
                        stream::BodyControlReply {
                            action: if detach {
                                stream::BodyControlReplyAction::Detach
                            } else {
                                stream::BodyControlReplyAction::Cancel
                            },
                            request_id: control.request_id.clone(),
                            status,
                            result_ref: None,
                            operation_effect_state:
                                stream::BodyControlReplyOperationEffectState::Uncertain,
                        },
                    ))))
                }
            }
            _ => return Ok(self.refusal(&request, frame::BodyRefusalReason::InvalidRequest)),
        };
        // Scope/owner refusals never turn into a fabricated success.
        let response = match response {
            Ok(response) => response,
            Err(_) => return Ok(self.refusal(&request, frame::BodyRefusalReason::Unavailable)),
        };
        if let stream::Body::ControlReply(reply) = &response.body {
            if reply.status == stream::BodyControlReplyStatus::Detached {
                self.detached = true;
            }
        }
        let envelope_json = envelope(&response).map_err(|_| self.overloaded())?;
        Ok(frame::SessionFrame {
            schema: 1,
            session_id: Some(request.session_id),
            request_id: Some(request.request_id),
            body: frame::Body::Complete(Box::new(frame::BodyComplete {
                envelope_json: Some(envelope_json),
                outcome: outcome(frame::OutcomeCompleteness::Complete),
            })),
        })
    }

    fn auto_event(&mut self) -> Result<(), SessionError> {
        if self.session.phase() != SessionPhase::Initialized
            || self.detached
            || self.expired
            || !self.can_reserve()
        {
            return Ok(());
        }
        let request = self.stream_request(stream::Body::ReplayRequest(Box::new(
            stream::BodyReplayRequest {
                after_sequence: self.cursor.to_string(),
                max_events: 1,
                max_bytes: 16 * 1024,
            },
        )));
        let replay = self
            .reader
            .replay(&request)
            .map_err(|e| self.owner_error(e.to_string()))?;
        let stream::Body::ReplayReply(row) = &replay.body else {
            return Err(self.owner_error("owner returned non-replay reply".to_owned()));
        };
        let mut cursor = self.cursor;
        let (event_id, message) = if row.gap.is_some() || row.expired {
            if let Some(gap) = &row.gap {
                cursor = gap
                    .to_sequence
                    .parse()
                    .map_err(|_| self.owner_error("invalid owner gap".to_owned()))?;
            }
            if let Some(event) = row.events.last() {
                cursor = event
                    .sequence
                    .parse()
                    .map_err(|_| self.owner_error("invalid owner sequence".to_owned()))?;
            }
            let identity = row
                .gap
                .as_ref()
                .map(|g| format!("gap:{}:{}", g.from_sequence, g.to_sequence))
                .unwrap_or_else(|| format!("expired:{}", self.cursor));
            (format!("{}:{identity}", replay.stream_id), replay.clone())
        } else if let Some(event) = row.events.first() {
            cursor = event
                .sequence
                .parse()
                .map_err(|_| self.owner_error("invalid owner sequence".to_owned()))?;
            (
                format!("{}:{}", replay.stream_id, event.sequence),
                self.stream_request(stream::Body::Event(Box::new(stream::BodyEvent {
                    event: event.clone(),
                }))),
            )
        } else {
            return Ok(());
        };
        let envelope_json = envelope(&message).map_err(|_| self.overloaded())?;
        let expired = row.expired;
        self.queue(frame::SessionFrame {
            schema: 1,
            session_id: Some(self.session.session_id().to_owned()),
            request_id: None,
            body: frame::Body::Event(Box::new(frame::BodyEvent {
                event_id,
                envelope_json: Some(envelope_json),
                outcome: outcome(frame::OutcomeCompleteness::Partial),
            })),
        })?;
        // Commit the subscription cursor only once its complete frame is queued.
        self.cursor = cursor;
        self.expired = expired;
        Ok(())
    }

    /// At most one inbound frame, one optional automatic event/prefix batch,
    /// and 4 KiB of socket IO in each direction. Blocked output still permits
    /// inbound queries/control while a whole bounded reply can be reserved.
    pub fn poll(&mut self) -> Result<PollStatus, SessionError> {
        let Some(port) = &mut self.port else {
            return Ok(PollStatus::Closed);
        };
        let received = port
            .poll_read()
            .map_err(|e| self.transport(format!("{e:?}")))?;
        match received {
            ReadPoll::Closed | ReadPoll::Terminal => {
                self.detach();
                return Ok(PollStatus::Closed);
            }
            ReadPoll::Pending => {}
            ReadPoll::Frame(frame) => {
                // No owner control call can occur without a reserved reply slot
                // and worst-case codec bytes (including an active partial frame).
                if !self.can_reserve() {
                    return Err(self.overloaded());
                }
                let decision = self.session.receive(frame);
                if self.session.correlation_exhausted() {
                    return Err(self.overloaded());
                }
                let response = match decision {
                    SessionDecision::Reply(response) => response,
                    SessionDecision::UnvalidatedRequest(request) => self.dispatch(request)?,
                };
                self.queue(response)?;
            }
        }
        self.auto_event()?;
        if self.active_bytes == 0 {
            if let Some((frame, size)) = self.pending.pop_front() {
                self.port
                    .as_mut()
                    .expect("open port")
                    .enqueue(&frame)
                    .map_err(|e| self.transport(format!("{e:?}")))?;
                self.active_bytes = size;
            }
        }
        let written = self
            .port
            .as_mut()
            .expect("open port")
            .poll_write()
            .map_err(|e| self.transport(format!("{e:?}")))?;
        match written {
            WritePoll::Complete => {
                self.delivery_bytes -= self.active_bytes;
                self.active_bytes = 0;
            }
            WritePoll::Terminal => {
                self.detach();
                return Ok(PollStatus::Closed);
            }
            WritePoll::Idle | WritePoll::Pending => {}
        }
        Ok(PollStatus::Open)
    }
}
impl Drop for ResponsiveSession {
    fn drop(&mut self) {
        self.detach();
    }
}

#[cfg(test)]
mod tests;
