//! Shared registered-participant commitments for replacement and absence.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#DOCUMENT-EDIT-BARRIER-LAW");
use super::*;
use std::{
    collections::BTreeSet,
    time::{Duration, Instant},
};
pub(super) type ExpectedBuffers = BTreeMap<ResourceId, Option<Arc<[u8]>>>;
pub(super) fn collect(
    authority: &Arc<DocumentEditAuthority>,
    capture: &AnalysisCapture,
    expected: &ExpectedBuffers,
) -> Result<Vec<Participant>, EditError> {
    let registry = authority
        .host
        .participants
        .lock()
        .map_err(|_| EditError::Unavailable)?;
    if registry.barrier.is_some() {
        return Err(EditError::Busy);
    }
    let capturer = registry
        .clients
        .get(&capture.client)
        .and_then(|w| w.upgrade())
        .ok_or(EditError::Disconnected)?;
    if !capturer.connected.load(Ordering::Acquire) {
        return Err(EditError::Disconnected);
    }
    if capturer
        .lock()
        .map_err(|_| EditError::Unavailable)?
        .generation
        != capture.generation
    {
        return Err(EditError::Stale);
    }
    let mut participants = vec![];
    for (id, weak) in &registry.clients {
        let Some(client) = weak.upgrade() else {
            continue;
        };
        if !client.connected.load(Ordering::Acquire) {
            continue;
        }
        let state = client.lock().map_err(|_| EditError::Unavailable)?;
        let versions = expected
            .keys()
            .map(|resource| (*resource, state.open.get(resource).map(|b| b.version)))
            .collect();
        let participant = Participant {
            client_id: *id,
            client: client.clone(),
            versions,
            generation: state.generation,
        };
        validate_buffers(&state, &participant, expected)?;
        drop(state);
        participants.push(participant);
    }
    Ok(participants)
}
pub(super) fn validate_buffers(
    state: &super::super::buffers::ClientBuffers,
    participant: &Participant,
    expected: &ExpectedBuffers,
) -> Result<(), EditError> {
    if state.generation != participant.generation {
        return Err(EditError::Stale);
    }
    for (id, version) in &participant.versions {
        let buffer = state.open.get(id);
        if buffer.map(|b| b.version) != *version {
            return Err(EditError::Stale);
        }
        if let Some(buffer) = buffer {
            if expected[id].as_ref().is_none_or(|bytes| {
                buffer.bytes.as_ref() != bytes.as_ref() || buffer.intent != BufferIntent::Replace
            }) {
                return Err(EditError::SaveRequired);
            }
        }
    }
    Ok(())
}
pub(super) fn validate(
    participant: &Participant,
    expected: &ExpectedBuffers,
) -> Result<(), EditError> {
    if !participant.client.connected.load(Ordering::Acquire) {
        return Err(EditError::Disconnected);
    }
    let state = participant
        .client
        .lock()
        .map_err(|_| EditError::Unavailable)?;
    validate_buffers(&state, participant, expected)
}
pub(super) fn reserve(
    authority: &Arc<DocumentEditAuthority>,
    participants: &[Participant],
    expected: &ExpectedBuffers,
    ttl: Duration,
) -> Result<(u64, Instant), EditError> {
    let expires = Instant::now()
        .checked_add(ttl)
        .ok_or(EditError::InvalidPlan)?;
    let mut registry = authority
        .host
        .participants
        .lock()
        .map_err(|_| EditError::Unavailable)?;
    if registry.barrier.is_some() {
        return Err(EditError::Busy);
    }
    let live: BTreeSet<_> = registry
        .clients
        .iter()
        .filter_map(|(id, w)| {
            w.upgrade()
                .filter(|c| c.connected.load(Ordering::Acquire))
                .map(|_| *id)
        })
        .collect();
    let required: BTreeSet<_> = participants.iter().map(|p| p.client_id).collect();
    if live != required {
        return Err(EditError::Stale);
    }
    for p in participants {
        validate(p, expected)?
    }
    let id = registry
        .next_barrier
        .checked_add(1)
        .ok_or(EditError::Unavailable)?;
    registry.next_barrier = id;
    registry.barrier = Some(id);
    Ok((id, expires))
}
pub(super) fn time(expires: Instant) -> Result<(), EditError> {
    if Instant::now() >= expires {
        Err(EditError::Expired)
    } else {
        Ok(())
    }
}
pub(super) fn ready(
    expires: Instant,
    answers: &Mutex<super::barrier::Answers>,
    participants: &[Participant],
    expected: &ExpectedBuffers,
) -> Result<(), EditError> {
    time(expires)?;
    let answers = answers.lock().map_err(|_| EditError::Unavailable)?;
    if answers.consumed {
        return Err(EditError::AlreadyApplied);
    }
    if answers.refused {
        return Err(EditError::Refused);
    }
    if answers.acknowledged.len() != participants.len() {
        return Err(EditError::AcknowledgementRequired);
    }
    for p in participants {
        validate(p, expected)?
    }
    Ok(())
}
pub(super) fn acknowledge(
    client: &DocumentClient,
    authority: &Arc<DocumentEditAuthority>,
    participants: &[Participant],
    expected: &ExpectedBuffers,
    id: u64,
    expires: Instant,
    answers: &Mutex<super::barrier::Answers>,
    approve: bool,
) -> Result<(), EditError> {
    if !Arc::ptr_eq(&client.host, &authority.host) {
        return Err(EditError::AuthorityRefused);
    }
    let p = participants
        .iter()
        .find(|p| p.client_id == client.client)
        .ok_or(EditError::AuthorityRefused)?;
    time(expires)?;
    let mut answers = answers.lock().map_err(|_| EditError::Unavailable)?;
    if answers.consumed {
        return Err(EditError::AlreadyApplied);
    }
    if !approve {
        answers.refused = true;
        return Err(EditError::Refused);
    }
    if answers.refused {
        return Err(EditError::Refused);
    }
    let mut state = client.state.lock().map_err(|_| EditError::Unavailable)?;
    validate_buffers(&state, p, expected)?;
    for resource in expected.keys() {
        state.fences.insert(*resource, id);
    }
    answers.acknowledged.insert(client.client);
    Ok(())
}
pub(super) fn release(
    authority: &Arc<DocumentEditAuthority>,
    participants: &[Participant],
    id: u64,
    invalidate: bool,
) {
    for p in participants {
        if let Ok(mut state) = p.client.lock() {
            state.fences.retain(|_, value| *value != id);
            if invalidate {
                let _ = state.advance();
            }
        }
    }
    if let Ok(mut registry) = authority.host.participants.lock() {
        if registry.barrier == Some(id) {
            registry.barrier = None
        }
    }
}
/// Coordination only: publication still requires the caller's separate finite
/// scaffold authority and current capability-relative absence validation.
pub struct AbsentDocumentPlan {
    authority: Arc<DocumentEditAuthority>,
    expected: ExpectedBuffers,
    participants: Vec<Participant>,
}
impl std::fmt::Debug for AbsentDocumentPlan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AbsentDocumentPlan")
            .field("resources", &self.expected.keys().collect::<Vec<_>>())
            .field("participants", &self.participants.len())
            .finish()
    }
}
#[derive(Debug)]
pub struct AbsentDocumentBarrier {
    plan: AbsentDocumentPlan,
    id: u64,
    expires: Instant,
    answers: Mutex<super::barrier::Answers>,
}
impl DocumentEditAuthority {
    pub fn prepare_absent(
        self: &Arc<Self>,
        capture: &AnalysisCapture,
        resources: &[ResourceId],
    ) -> Result<AbsentDocumentPlan, EditError> {
        self.prepare_absent_with_saved(capture, resources, &[])
    }
    /// Saved read preconditions share the same participant/version fence;
    /// they are never replacement or creation authorization.
    pub fn prepare_absent_with_saved(
        self: &Arc<Self>,
        capture: &AnalysisCapture,
        resources: &[ResourceId],
        saved: &[ResourceId],
    ) -> Result<AbsentDocumentPlan, EditError> {
        if capture.context != self.host.host {
            return Err(EditError::AuthorityRefused);
        }
        let mut expected = BTreeMap::new();
        for id in resources {
            if !self.resources.contains(id) || expected.insert(*id, None).is_some() {
                return Err(EditError::AuthorityRefused);
            }
            let resource = capture.resource(*id).ok_or(EditError::InvalidPlan)?;
            if resource.disk_observation().is_some()
                || resource.underlying_observation().is_some()
                || resource.raw().is_some()
                || resource.buffer_version().is_some()
                || !matches!(resource.disposition(), ResourceDisposition::Disk)
            {
                return Err(EditError::SaveRequired);
            }
            if self
                .host
                .disk(*id)
                .map_err(|_| EditError::Unavailable)?
                .is_some()
            {
                return Err(EditError::Stale);
            }
        }
        if expected.is_empty() {
            return Err(EditError::InvalidPlan);
        }
        for id in saved {
            if !self.resources.contains(id) || expected.contains_key(id) {
                return Err(EditError::AuthorityRefused);
            }
            let row = capture.resource(*id).ok_or(EditError::InvalidPlan)?;
            let disk = row.disk_observation().ok_or(EditError::SaveRequired)?;
            if row.raw() != Some(disk.bytes.as_slice())
                || !matches!(
                    row.disposition(),
                    ResourceDisposition::Disk | ResourceDisposition::Replaced
                )
            {
                return Err(EditError::SaveRequired);
            }
            if self
                .host
                .disk(*id)
                .map_err(|_| EditError::Unavailable)?
                .as_ref()
                != Some(disk)
            {
                return Err(EditError::Stale);
            }
            expected.insert(*id, Some(Arc::from(disk.bytes.as_slice())));
        }
        let participants = collect(self, capture, &expected)?;
        Ok(AbsentDocumentPlan {
            authority: self.clone(),
            expected,
            participants,
        })
    }
}
impl AbsentDocumentPlan {
    pub fn barrier(self, ttl: Duration) -> Result<AbsentDocumentBarrier, EditError> {
        let (id, expires) = reserve(&self.authority, &self.participants, &self.expected, ttl)?;
        Ok(AbsentDocumentBarrier {
            plan: self,
            id,
            expires,
            answers: Mutex::new(super::barrier::Answers::default()),
        })
    }
}
impl DocumentClient {
    pub fn acknowledge_absent_edit(
        &self,
        barrier: &AbsentDocumentBarrier,
        approve: bool,
    ) -> Result<(), EditError> {
        acknowledge(
            self,
            &barrier.plan.authority,
            &barrier.plan.participants,
            &barrier.plan.expected,
            barrier.id,
            barrier.expires,
            &barrier.answers,
            approve,
        )
    }
}
impl AbsentDocumentBarrier {
    pub fn ready(&self) -> Result<(), EditError> {
        ready(
            self.expires,
            &self.answers,
            &self.plan.participants,
            &self.plan.expected,
        )
    }
    pub fn check_participants(&self) -> Result<(), EditError> {
        time(self.expires)?;
        for p in &self.plan.participants {
            validate(p, &self.plan.expected)?
        }
        Ok(())
    }
    pub fn consume(&self) -> Result<(), EditError> {
        self.ready()?;
        self.answers
            .lock()
            .map_err(|_| EditError::Unavailable)?
            .consumed = true;
        Ok(())
    }
    pub fn release_with_outcome(&self, changed: bool) {
        release(
            &self.plan.authority,
            &self.plan.participants,
            self.id,
            changed,
        )
    }
}
impl Drop for AbsentDocumentBarrier {
    fn drop(&mut self) {
        release(
            &self.plan.authority,
            &self.plan.participants,
            self.id,
            false,
        )
    }
}
