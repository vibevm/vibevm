//! One cooperative fence for all registered participants of a saved plan.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#DOCUMENT-EDIT-BARRIER-LAW");
use super::*;
use std::{
    collections::BTreeSet,
    time::{Duration, Instant},
};
pub struct EditBarrier {
    pub(super) plan: EditPlan,
    pub(super) id: u64,
    pub(super) expires: Instant,
    pub(super) answers: Mutex<Answers>,
    pub(super) applying: Mutex<()>,
}
#[derive(Default)]
pub(super) struct Answers {
    pub acknowledged: BTreeSet<u64>,
    pub refused: bool,
    pub consumed: bool,
}
impl std::fmt::Debug for EditBarrier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EditBarrier")
            .field("plan", &self.plan)
            .field("barrier", &self.id)
            .finish()
    }
}
impl EditPlan {
    pub fn barrier(self, ttl: Duration) -> Result<EditBarrier, EditError> {
        let expires = Instant::now()
            .checked_add(ttl)
            .ok_or(EditError::InvalidPlan)?;
        let mut registry = self
            .authority
            .host
            .participants
            .lock()
            .map_err(|_| EditError::Unavailable)?;
        if registry.barrier.is_some() {
            return Err(EditError::Busy);
        };
        let live: BTreeSet<_> = registry
            .clients
            .iter()
            .filter_map(|(id, w)| {
                w.upgrade()
                    .filter(|c| c.connected.load(Ordering::Acquire))
                    .map(|_| *id)
            })
            .collect();
        let expected: BTreeSet<_> = self.participants.iter().map(|p| p.client_id).collect();
        if live != expected {
            return Err(EditError::Stale);
        };
        for p in &self.participants {
            validate(p, &self)?;
        }
        let id = registry
            .next_barrier
            .checked_add(1)
            .ok_or(EditError::Unavailable)?;
        registry.next_barrier = id;
        registry.barrier = Some(id);
        drop(registry);
        Ok(EditBarrier {
            plan: self,
            id,
            expires,
            answers: Mutex::new(Answers::default()),
            applying: Mutex::new(()),
        })
    }
}
fn validate(participant: &Participant, plan: &EditPlan) -> Result<(), EditError> {
    if !participant.client.connected.load(Ordering::Acquire) {
        return Err(EditError::Disconnected);
    };
    let state = participant
        .client
        .lock()
        .map_err(|_| EditError::Unavailable)?;
    validate_buffers(&state, participant, plan)
}
pub(super) fn validate_buffers(
    state: &super::super::buffers::ClientBuffers,
    participant: &Participant,
    plan: &EditPlan,
) -> Result<(), EditError> {
    if state.generation != participant.generation {
        return Err(EditError::Stale);
    };
    for (id, version) in &participant.versions {
        let buffer = state.open.get(id);
        if buffer.map(|b| b.version) != *version {
            return Err(EditError::Stale);
        };
        if buffer.is_some_and(|b| {
            b.bytes.as_ref() != plan.files[id].before.bytes.as_slice()
                || b.intent != BufferIntent::Replace
        }) {
            return Err(EditError::SaveRequired);
        };
    }
    Ok(())
}
impl DocumentClient {
    pub fn acknowledge_edit(&self, barrier: &EditBarrier, approve: bool) -> Result<(), EditError> {
        if !Arc::ptr_eq(&self.host, &barrier.plan.authority.host) {
            return Err(EditError::AuthorityRefused);
        };
        let p = barrier
            .plan
            .participants
            .iter()
            .find(|p| p.client_id == self.client)
            .ok_or(EditError::AuthorityRefused)?;
        barrier.check_time()?;
        let mut answers = barrier.answers.lock().map_err(|_| EditError::Unavailable)?;
        if answers.consumed {
            return Err(EditError::AlreadyApplied);
        };
        if !approve {
            answers.refused = true;
            return Err(EditError::Refused);
        };
        if answers.refused {
            return Err(EditError::Refused);
        };
        let mut state = self.state.lock().map_err(|_| EditError::Unavailable)?;
        validate_buffers(&state, p, &barrier.plan)?;
        for id in barrier.plan.files.keys() {
            state.fences.insert(*id, barrier.id);
        }
        answers.acknowledged.insert(self.client);
        Ok(())
    }
}
impl EditBarrier {
    pub(super) fn check_time(&self) -> Result<(), EditError> {
        if Instant::now() >= self.expires {
            Err(EditError::Expired)
        } else {
            Ok(())
        }
    }
    pub(super) fn check_participants(&self) -> Result<(), EditError> {
        self.check_time()?;
        for p in &self.plan.participants {
            validate(p, &self.plan)?;
        }
        Ok(())
    }
    pub(super) fn ready(&self) -> Result<(), EditError> {
        self.check_time()?;
        let answers = self.answers.lock().map_err(|_| EditError::Unavailable)?;
        if answers.consumed {
            return Err(EditError::AlreadyApplied);
        };
        if answers.refused {
            return Err(EditError::Refused);
        };
        if answers.acknowledged.len() != self.plan.participants.len() {
            return Err(EditError::AcknowledgementRequired);
        };
        for p in &self.plan.participants {
            validate(p, &self.plan)?;
        }
        Ok(())
    }
}
impl Drop for EditBarrier {
    fn drop(&mut self) {
        for p in &self.plan.participants {
            if let Ok(mut state) = p.client.lock() {
                state.fences.retain(|_, id| *id != self.id);
            }
        }
        if let Ok(mut registry) = self.plan.authority.host.participants.lock() {
            if registry.barrier == Some(self.id) {
                registry.barrier = None;
            }
        }
    }
}
