//! The boss-session cell (Campaign 2 D2/D3): registration, initiative
//! notes, and session-scoped reads over [`AppState`].
//!
//! Sessions are FACTS. The write path mirrors the run journal's law —
//! validate → append → fold under one lock — against the sibling
//! `sessions.jsonl`; policy (thresholds, cooldowns, texts) lives in the
//! initiative engine, never here.

use fractality_core::SessionRecord;
use fractality_core::api::ParkedQuestion;
use fractality_core::ids::SessionId;
use fractality_core::run::RunState;
use fractality_core::session::{
    InitiativeCounters, SessionApplyOutcome, SessionEnvelope, SessionEvent, SessionNote,
    apply_session,
};
use fractality_core::time::now_ms;
use specmark::spec;

use crate::state::AppState;

specmark::scope!("spec://fractality/PROP-001#architecture");

/// A session write-path refusal, mapped to an HTTP answer by handlers.
#[derive(Debug, thiserror::Error, PartialEq)]
#[spec(implements = "spec://fractality/PROP-001#architecture")]
pub enum SessionError {
    #[error(
        "session {0} is not registered on this mission-control (violates spec://fractality/PROP-001#architecture; fix: begin the session before noting facts on it)"
    )]
    UnknownSession(SessionId),

    #[error(
        "session journal append failed: {0} (violates spec://fractality/PROP-001#architecture; fix: ensure the journal directory is writable and has space)"
    )]
    Journal(String),
}

impl AppState {
    /// Begins (or resumes) a boss session. Idempotent per
    /// `(harness, external_id)` while such a session is open — Claude
    /// Code fires SessionStart on `startup|resume|clear|compact`, and
    /// all four must land on one record (D2). Returns the record and
    /// whether it was resumed.
    pub fn session_begin(
        &self,
        harness: &str,
        external_id: &str,
        cwd: camino::Utf8PathBuf,
    ) -> Result<(SessionRecord, bool), SessionError> {
        let mut inner = self.lock_inner();
        if let Some(existing) = inner
            .sessions
            .values()
            .find(|s| s.is_open() && s.harness == harness && s.external_id == external_id)
        {
            return Ok((existing.clone(), true));
        }
        let now = now_ms();
        let record = SessionRecord {
            session_id: SessionId::generate(),
            harness: harness.to_owned(),
            external_id: external_id.to_owned(),
            cwd,
            node_id: self.node.node_id.clone(),
            started_ts_ms: now,
            updated_ts_ms: now,
            ended_ts_ms: None,
            counters: InitiativeCounters::default(),
        };
        let envelope = SessionEnvelope::now(SessionEvent::SessionBegan {
            session: Box::new(record.clone()),
        });
        inner
            .session_journal
            .append(&envelope)
            .map_err(SessionError::Journal)?;
        let outcome = apply_session(&mut inner.sessions, &envelope);
        debug_assert_eq!(outcome, SessionApplyOutcome::Applied);
        Ok((record, false))
    }

    /// Records one initiative note on an open session (the D3 facts
    /// channel): validate → journal → fold, one lock.
    pub fn session_note(
        &self,
        session_id: SessionId,
        note: SessionNote,
    ) -> Result<SessionRecord, SessionError> {
        let mut inner = self.lock_inner();
        if !inner.sessions.contains_key(&session_id) {
            return Err(SessionError::UnknownSession(session_id));
        }
        let envelope = SessionEnvelope::now(SessionEvent::SessionNoted { session_id, note });
        inner
            .session_journal
            .append(&envelope)
            .map_err(SessionError::Journal)?;
        let outcome = apply_session(&mut inner.sessions, &envelope);
        debug_assert_eq!(outcome, SessionApplyOutcome::Applied);
        Ok(inner.sessions[&session_id].clone())
    }

    /// Marks the session ended (idempotent: the first end's timestamp
    /// wins; an echo still touches `updated_ts_ms`).
    pub fn session_end(&self, session_id: SessionId) -> Result<SessionRecord, SessionError> {
        let mut inner = self.lock_inner();
        if !inner.sessions.contains_key(&session_id) {
            return Err(SessionError::UnknownSession(session_id));
        }
        let envelope = SessionEnvelope::now(SessionEvent::SessionEnded { session_id });
        inner
            .session_journal
            .append(&envelope)
            .map_err(SessionError::Journal)?;
        let outcome = apply_session(&mut inner.sessions, &envelope);
        debug_assert_eq!(outcome, SessionApplyOutcome::Applied);
        Ok(inner.sessions[&session_id].clone())
    }

    pub fn get_session(&self, id: SessionId) -> Option<SessionRecord> {
        self.lock_inner().sessions.get(&id).cloned()
    }

    /// The registration-path attribution note (D2): a delegation fact
    /// zeroes the session's BD1 slate. Best-effort by design — an
    /// unknown or stale session id is logged and dropped, never an
    /// error (the stamp on the run already carries the label).
    pub fn note_delegation_best_effort(
        &self,
        origin: Option<SessionId>,
        run_id: fractality_core::ids::RunId,
    ) {
        if let Some(sid) = origin
            && let Err(e) = self.session_note(sid, SessionNote::Delegated { run_id })
        {
            tracing::warn!(session = %sid, error = %e, "delegation note dropped");
        }
    }

    /// Sessions in creation order (ULID order, newest last — D17),
    /// optionally only the open ones.
    pub fn list_sessions(&self, open_only: bool) -> Vec<SessionRecord> {
        self.lock_inner()
            .sessions
            .values()
            .filter(|s| !open_only || s.is_open())
            .cloned()
            .collect()
    }

    /// The session-scoped scoreboard facts: the record, the metrics
    /// bucket over the runs it originated, and their parked questions
    /// with ages (the D5 injection facts).
    pub fn session_metrics(
        &self,
        id: SessionId,
    ) -> Option<(
        SessionRecord,
        fractality_core::api::MetricsBucket,
        Vec<ParkedQuestion>,
    )> {
        let inner = self.lock_inner();
        let session = inner.sessions.get(&id)?.clone();
        let now = now_ms();
        let mut bucket = fractality_core::api::MetricsBucket::default();
        let mut parked = Vec::new();
        for run in inner.runs.values() {
            if run.origin_session != Some(id) {
                continue;
            }
            crate::metrics::fold_into(&mut bucket, run);
            if run.state == RunState::WaitingOnBoss
                && let Some(q) = &run.question
            {
                parked.push(ParkedQuestion {
                    run_id: run.run_id,
                    question: q.clone(),
                    waiting_ms: now.saturating_sub(run.updated_ts_ms),
                });
            }
        }
        Some((session, bucket, parked))
    }
}

#[cfg(test)]
mod tests {
    // The session write path is exercised end to end (journal on real
    // disk + fold + reads) by `tests/sessions.rs`; the fold's own unit
    // tests live with it in fractality-core. This cell keeps only the
    // seam-level compile guarantees.
}
