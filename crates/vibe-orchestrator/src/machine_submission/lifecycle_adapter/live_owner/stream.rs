//! Bounded private history; generated values carry no admission authority.
use super::*;
use crate::machine_operation::stream_message::{self as validation, ExpectedBinding};
use std::collections::VecDeque;
use vibe_wire::generated::lifecycle_outcome_marker::LifecycleOutcomeMarkerEffectState;
pub(super) use vibe_wire::generated::machine::e1::operation_stream_message::PayloadStateEffectState as Effect;
pub(super) use vibe_wire::generated::machine::e1::operation_stream_message::PayloadStateObserved as Observed;
use vibe_wire::generated::machine::e1::operation_stream_message::*;

pub(super) const MAX_OBSERVERS: usize = 64;
const MAX_METADATA_SLOTS: usize = 128;

#[derive(Debug)]
pub(super) struct SubscriptionLimit;
impl std::fmt::Display for SubscriptionLimit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("live owner subscription limit")
    }
}
impl std::error::Error for SubscriptionLimit {}
#[derive(Debug, PartialEq, Eq)]
pub(in super::super) enum ReplayError {
    Binding,
    ScopeUnavailable(String),
    InvalidMessage(validation::StreamValidationError),
    WrongBody,
    FutureCursor,
    ReplyBudget,
}
impl std::fmt::Display for ReplayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "live replay refused: {self:?}")
    }
}
impl std::error::Error for ReplayError {}
enum Slot {
    Event(EventRecord, usize),
    Loss(u64, u64, GapReason),
}
pub(super) struct History {
    envelope: OperationStreamMessage,
    slots: VecDeque<Slot>,
    tip: u64,
    bytes: usize,
    count: usize,
    expired: bool,
    terminal_ref: Option<String>,
}
impl History {
    pub(super) fn new(scope: &SubmissionScope, run: &str, reservation: &str) -> Self {
        Self {
            envelope: OperationStreamMessage {
                schema: 1,
                context_id: scope.context.clone(),
                target_id: Some(scope.target.clone()),
                operation_id: FIXED_OPERATION.into(),
                run_id: run.into(),
                stream_id: format!("live:{run}:{reservation}"),
                body: Body::ReplayRequest(Box::new(BodyReplayRequest {
                    after_sequence: "0".into(),
                    max_events: 64,
                    max_bytes: 65536,
                })),
            },
            slots: VecDeque::new(),
            tip: 0,
            bytes: 0,
            count: 0,
            expired: false,
            terminal_ref: None,
        }
    }
    pub(super) fn expected(&self) -> ExpectedBinding<'_> {
        ExpectedBinding {
            context_id: &self.envelope.context_id,
            target_id: self.envelope.target_id.as_deref(),
            operation_id: &self.envelope.operation_id,
            run_id: &self.envelope.run_id,
            stream_id: &self.envelope.stream_id,
        }
    }
    pub(super) fn message(&self, body: Body) -> OperationStreamMessage {
        let mut message = self.envelope.clone();
        message.body = body;
        message
    }
    pub(super) fn state(
        &mut self,
        observed: Observed,
        effect_state: Effect,
        result_ref: Option<String>,
    ) {
        self.publish(Payload::State(Box::new(PayloadState {
            observed,
            effect_state,
            result_ref,
        })));
    }
    pub(super) fn publish(&mut self, payload: Payload) {
        let Some(next) = self.tip.checked_add(1) else {
            self.expire();
            return;
        };
        self.tip = next;
        if self.expired {
            return;
        }
        let event = EventRecord {
            sequence: next.to_string(),
            payload,
        };
        let message = self.message(Body::Event(Box::new(BodyEvent { event })));
        if let Err(error) = validation::validate(&message, self.expected()) {
            match error {
                validation::StreamValidationError::FrameLimit
                | validation::StreamValidationError::InvalidValue("diagnostic byte cap") => {
                    self.loss(next, GapReason::Oversize)
                }
                _ => self.expire(),
            }
            return;
        }
        let bytes = serde_json::to_vec(&message).unwrap().len();
        let Body::Event(row) = message.body else {
            unreachable!()
        };
        self.slots.push_back(Slot::Event(row.event, bytes));
        self.bytes += bytes;
        self.count += 1;
        while self.count > validation::MAX_REPLAY_EVENTS || self.bytes > validation::MAX_FRAME_BYTES
        {
            let index = self
                .slots
                .iter()
                .position(|s| matches!(s, Slot::Event(..)))
                .unwrap();
            let Slot::Event(event, bytes) = self.slots.remove(index).unwrap() else {
                unreachable!()
            };
            self.bytes -= bytes;
            self.count -= 1;
            let seq = event.sequence.parse().unwrap();
            // Preserve the original loss causes instead of relabelling holes.
            if index > 0 {
                if let Slot::Loss(_, to, GapReason::Evicted) = &mut self.slots[index - 1] {
                    *to = seq;
                } else {
                    self.slots
                        .insert(index, Slot::Loss(seq, seq, GapReason::Evicted));
                }
            } else {
                self.slots
                    .push_front(Slot::Loss(seq, seq, GapReason::Evicted));
            }
        }
        self.bound_metadata();
    }
    fn loss(&mut self, seq: u64, reason: GapReason) {
        if let Some(Slot::Loss(_, to, prior)) = self.slots.back_mut() {
            if *prior == reason && to.checked_add(1) == Some(seq) {
                *to = seq;
                return;
            }
        }
        self.slots.push_back(Slot::Loss(seq, seq, reason));
        self.bound_metadata();
    }
    fn bound_metadata(&mut self) {
        if self.slots.len() > MAX_METADATA_SLOTS {
            self.expire();
        }
    }
    pub(super) fn expire(&mut self) {
        self.slots.clear();
        self.bytes = 0;
        self.count = 0;
        self.expired = true;
    }
    fn replay(
        &self,
        request: &OperationStreamMessage,
    ) -> std::result::Result<OperationStreamMessage, ReplayError> {
        validation::validate(request, self.expected()).map_err(ReplayError::InvalidMessage)?;
        let Body::ReplayRequest(request) = &request.body else {
            return Err(ReplayError::WrongBody);
        };
        let after: u64 = request.after_sequence.parse().unwrap();
        if after > self.tip {
            return Err(ReplayError::FutureCursor);
        }
        let mut reply = BodyReplayReply {
            after_sequence: after.to_string(),
            events: vec![],
            expired: self.expired,
            gap: None,
            newest_sequence: Some(self.tip.to_string()),
            terminal_ref: self.terminal_ref.clone(),
        };
        let mut next = after.checked_add(1);
        if !self.expired {
            for slot in &self.slots {
                match slot {
                    Slot::Loss(from, to, reason)
                        if next.is_some_and(|n| n >= *from && n <= *to) =>
                    {
                        if !reply.events.is_empty() || reply.gap.is_some() {
                            break;
                        }
                        reply.gap = Some(Gap {
                            from_sequence: next.unwrap().to_string(),
                            to_sequence: to.to_string(),
                            reason: reason.clone(),
                            terminal_ref: self.terminal_ref.clone(),
                        });
                        next = to.checked_add(1);
                    }
                    Slot::Event(event, _) if Some(event.sequence.parse().unwrap()) == next => {
                        if reply.events.len() >= usize::from(request.max_events) {
                            break;
                        }
                        reply.events.push(event.clone());
                        let message = self.message(Body::ReplayReply(Box::new(reply.clone())));
                        if serde_json::to_vec(&message).unwrap().len() > request.max_bytes as usize
                            || validation::validate(&message, self.expected()).is_err()
                        {
                            reply.events.pop();
                            break;
                        }
                        next = next.and_then(|n| n.checked_add(1));
                    }
                    _ => {}
                }
            }
        }
        let message = self.message(Body::ReplayReply(Box::new(reply)));
        if serde_json::to_vec(&message).unwrap().len() > request.max_bytes as usize
            || validation::validate(&message, self.expected()).is_err()
        {
            return Err(ReplayError::ReplyBudget);
        }
        Ok(message)
    }
}
impl LiveOwner {
    pub(super) fn check_stream_binding(
        &self,
        binding: &Binding,
        trusted: &HostCoordinates,
    ) -> std::result::Result<(), ReplayError> {
        if !Weak::ptr_eq(&Arc::downgrade(&self.shared), &binding.owner)
            || &binding.coordinates != trusted
        {
            return Err(ReplayError::Binding);
        }
        binding
            .physical_root
            .check(&trusted.root)
            .map_err(|error| ReplayError::ScopeUnavailable(error.to_string()))?;
        let record = self.shared.record.lock().unwrap();
        if !record
            .binding
            .as_ref()
            .is_some_and(|b| same_binding(b, binding))
        {
            return Err(ReplayError::Binding);
        }
        Ok(())
    }
    pub(in super::super) fn replay(
        &self,
        binding: &Binding,
        trusted: &HostCoordinates,
        request: &OperationStreamMessage,
    ) -> std::result::Result<OperationStreamMessage, ReplayError> {
        // DTO validation precedes trusted scope comparison; neither admits work.
        {
            let history = binding.history.lock().unwrap();
            validation::validate(request, history.expected())
                .map_err(ReplayError::InvalidMessage)?;
        }
        self.check_stream_binding(binding, trusted)?;
        binding.history.lock().unwrap().replay(request)
    }
    pub(in super::super) fn expire_history(
        &self,
        binding: &Binding,
        trusted: &HostCoordinates,
    ) -> std::result::Result<(), ReplayError> {
        self.check_stream_binding(binding, trusted)?;
        binding.history.lock().unwrap().expire();
        Ok(())
    }
}
pub(super) fn publish_terminal(shared: &Arc<Shared>) {
    let binding = shared.record.lock().unwrap().binding.clone();
    let Some(binding) = binding else { return };
    // Fresh cold inspection validates C1 and marker/report after lease release.
    let (observed, effect, reference) = match inspect(shared) {
        Ok(Observation::ResultAvailable { run_id, result }) => {
            let effect = match result.marker.effect_state {
                LifecycleOutcomeMarkerEffectState::Landed => Effect::Completed,
                LifecycleOutcomeMarkerEffectState::None => Effect::NoEffects,
                LifecycleOutcomeMarkerEffectState::Uncertain => Effect::Uncertain,
            };
            (
                Observed::ResultAvailable,
                effect,
                Some(format!("lifecycle:{run_id}:{}", binding.reservation_ref)),
            )
        }
        _ => (Observed::OwnerStopped, Effect::Uncertain, None),
    };
    let mut history = binding.history.lock().unwrap();
    history.terminal_ref = reference.clone();
    history.state(observed, effect, reference);
}
#[cfg(test)]
mod tests;
