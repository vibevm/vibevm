//! Boss sessions and their journal fold (Campaign 2, plan D2/D3).
//!
//! A session is mission-control's record of one boss conversation in some
//! harness ("claude-code" today; the label is data, never a code path —
//! invariant I4). Sessions carry the initiative counters the scoreboard
//! and the nudge engine read; runs point back via
//! [`crate::run::RunRecord::origin_session`]. Session events ride their
//! own journal file (`sessions.jsonl`) with their own fold, so the run
//! journal's replay stays byte-for-byte untouched; the only cross-link is
//! the `origin_session` label on runs, and a dangling label is harmless
//! by design (facts, not references).

use camino::Utf8PathBuf;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::ids::{RunId, SessionId};

specmark::scope!("spec://fractality/PROP-001#model");

/// The environment variable a harness adapter exports at session start
/// (via the hook's `CLAUDE_ENV_FILE` on Claude Code) and every
/// `fractality` CLI invocation reads to stamp `origin_session` onto the
/// runs it creates. Never whitelisted into worker environments (I1):
/// worker-side spawns attribute through `FRACTALITY_RUN_ID` parenting.
pub const BOSS_SESSION_ENV: &str = "FRACTALITY_BOSS_SESSION";

/// Initiative counters folded from session notes. All facts, no policy:
/// thresholds and cooldowns live in the engine, not here (D3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct InitiativeCounters {
    /// Work-shaped tool events since the last delegation — the BD1
    /// "clean slate" counter the nudge threshold reads.
    pub work_tools_since_delegation: u64,
    pub work_tools_total: u64,
    /// Total milliseconds spent in work-shaped tools (F21: PostToolUse
    /// serves `duration_ms`), so nudges can weigh time, not just count.
    pub work_tool_ms_total: u64,
    /// Runs this session delegated through the fabric.
    pub delegations: u64,
    pub nudges_sent: u64,
    /// Stop-time parked-question alerts emitted (D5: once per question).
    pub question_alerts: u64,
}

/// Everything mission-control knows about one boss session. Registry
/// entry, `session_began` journal snapshot, and wire shape at once —
/// one type, no drift (the RunRecord precedent).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionRecord {
    pub session_id: SessionId,
    /// Harness label, e.g. `claude-code` (I4: data, not a code path).
    pub harness: String,
    /// The harness's own session identifier (Claude Code session UUID).
    pub external_id: String,
    pub cwd: Utf8PathBuf,
    pub node_id: String,
    pub started_ts_ms: u64,
    pub updated_ts_ms: u64,
    /// Set when the harness reported the session end; an open session
    /// has `None`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ended_ts_ms: Option<u64>,
    #[serde(default)]
    pub counters: InitiativeCounters,
    /// When the last nudge was injected (the engine's cooldown anchor,
    /// D5). Envelope time via the fold — deterministic on replay.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_nudge_ts_ms: Option<u64>,
    /// Runs whose parked question was already alerted at a Stop (D5:
    /// once per question). Bounded by the number of parked runs.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub alerted_runs: Vec<RunId>,
}

impl SessionRecord {
    pub fn is_open(&self) -> bool {
        self.ended_ts_ms.is_none()
    }
}

/// One `sessions.jsonl` line: a timestamp plus the event.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionEnvelope {
    pub ts_ms: u64,
    #[serde(flatten)]
    pub event: SessionEvent,
}

impl SessionEnvelope {
    /// Wraps an event with the current wall clock.
    pub fn now(event: SessionEvent) -> Self {
        Self {
            ts_ms: crate::time::now_ms(),
            event,
        }
    }
}

/// The session-journal event vocabulary (D3, additive by design).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum SessionEvent {
    /// A boss session was registered; carries the full initial record.
    SessionBegan { session: Box<SessionRecord> },
    /// An initiative fact was observed on an open session.
    SessionNoted {
        session_id: SessionId,
        note: SessionNote,
    },
    /// The harness reported the session end.
    SessionEnded { session_id: SessionId },
}

impl SessionEvent {
    pub fn session_id(&self) -> SessionId {
        match self {
            SessionEvent::SessionBegan { session } => session.session_id,
            SessionEvent::SessionNoted { session_id, .. }
            | SessionEvent::SessionEnded { session_id } => *session_id,
        }
    }
}

/// The facts a session accumulates (BD1/BD5: counters in MC, never in
/// harness-side state files).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SessionNote {
    /// The boss used a work-shaped tool itself (adapter-reported).
    WorkTool {
        tool: String,
        #[serde(default)]
        duration_ms: u64,
    },
    /// A run was delegated through the fabric — zeroes the BD1 slate.
    Delegated { run_id: RunId },
    /// The engine injected a nudge (the reason names the trigger).
    NudgeSent { reason: String },
    /// A Stop-time parked-question alert was emitted for this run.
    QuestionAlert { run_id: RunId },
}

/// Outcome of folding one envelope into the session map.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionApplyOutcome {
    Applied,
    /// The event references a session the map does not hold — nothing
    /// was mutated (journal gap or foreign-home event).
    UnknownSession(SessionId),
}

/// Folds one envelope into the session map. The live write path and
/// startup replay call exactly this function (the D9 discipline).
pub fn apply_session(
    sessions: &mut BTreeMap<SessionId, SessionRecord>,
    envelope: &SessionEnvelope,
) -> SessionApplyOutcome {
    let ts = envelope.ts_ms;
    match &envelope.event {
        SessionEvent::SessionBegan { session } => {
            sessions.insert(session.session_id, (**session).clone());
            SessionApplyOutcome::Applied
        }
        SessionEvent::SessionNoted { session_id, note } => {
            let Some(s) = sessions.get_mut(session_id) else {
                return SessionApplyOutcome::UnknownSession(*session_id);
            };
            let c = &mut s.counters;
            match note {
                SessionNote::WorkTool { duration_ms, .. } => {
                    c.work_tools_since_delegation += 1;
                    c.work_tools_total += 1;
                    c.work_tool_ms_total += duration_ms;
                }
                SessionNote::Delegated { .. } => {
                    c.delegations += 1;
                    // BD1: choosing the right path cleans the slate.
                    c.work_tools_since_delegation = 0;
                }
                SessionNote::NudgeSent { .. } => {
                    c.nudges_sent += 1;
                    s.last_nudge_ts_ms = Some(ts);
                }
                SessionNote::QuestionAlert { run_id } => {
                    c.question_alerts += 1;
                    if !s.alerted_runs.contains(run_id) {
                        s.alerted_runs.push(*run_id);
                    }
                }
            }
            s.updated_ts_ms = ts;
            SessionApplyOutcome::Applied
        }
        SessionEvent::SessionEnded { session_id } => {
            let Some(s) = sessions.get_mut(session_id) else {
                return SessionApplyOutcome::UnknownSession(*session_id);
            };
            // Idempotent: the first end wins (a re-sent end is a fact
            // echo, not a new fact).
            if s.ended_ts_ms.is_none() {
                s.ended_ts_ms = Some(ts);
            }
            s.updated_ts_ms = ts;
            SessionApplyOutcome::Applied
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SID: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAV";
    const RUN: &str = "01BX5ZZKBKACTAV9WEVGEMMVRY";

    fn record(id: &str) -> SessionRecord {
        SessionRecord {
            session_id: id.parse().expect("fixed ulid"),
            harness: "claude-code".into(),
            external_id: "cc-uuid-1".into(),
            cwd: "proj".into(),
            node_id: "node-1".into(),
            started_ts_ms: 1,
            updated_ts_ms: 1,
            ended_ts_ms: None,
            last_nudge_ts_ms: None,
            alerted_runs: Vec::new(),
            counters: InitiativeCounters::default(),
        }
    }

    fn env(ts_ms: u64, event: SessionEvent) -> SessionEnvelope {
        SessionEnvelope { ts_ms, event }
    }

    #[test]
    fn work_tools_accumulate_and_a_delegation_cleans_the_slate() {
        let sid: SessionId = SID.parse().expect("ulid");
        let mut sessions = BTreeMap::new();
        apply_session(
            &mut sessions,
            &env(
                1,
                SessionEvent::SessionBegan {
                    session: Box::new(record(SID)),
                },
            ),
        );
        for ts in 2..=4 {
            let outcome = apply_session(
                &mut sessions,
                &env(
                    ts,
                    SessionEvent::SessionNoted {
                        session_id: sid,
                        note: SessionNote::WorkTool {
                            tool: "Bash".into(),
                            duration_ms: 100,
                        },
                    },
                ),
            );
            assert_eq!(outcome, SessionApplyOutcome::Applied);
        }
        let s = &sessions[&sid];
        assert_eq!(s.counters.work_tools_since_delegation, 3);
        assert_eq!(s.counters.work_tools_total, 3);
        assert_eq!(s.counters.work_tool_ms_total, 300);

        apply_session(
            &mut sessions,
            &env(
                5,
                SessionEvent::SessionNoted {
                    session_id: sid,
                    note: SessionNote::Delegated {
                        run_id: RUN.parse().expect("ulid"),
                    },
                },
            ),
        );
        let s = &sessions[&sid];
        assert_eq!(s.counters.delegations, 1);
        assert_eq!(s.counters.work_tools_since_delegation, 0, "BD1 slate");
        assert_eq!(s.counters.work_tools_total, 3, "history survives");
        assert_eq!(s.updated_ts_ms, 5, "fold uses envelope time");
    }

    #[test]
    fn end_is_idempotent_and_keeps_the_first_timestamp() {
        let sid: SessionId = SID.parse().expect("ulid");
        let mut sessions = BTreeMap::new();
        apply_session(
            &mut sessions,
            &env(
                1,
                SessionEvent::SessionBegan {
                    session: Box::new(record(SID)),
                },
            ),
        );
        apply_session(
            &mut sessions,
            &env(7, SessionEvent::SessionEnded { session_id: sid }),
        );
        apply_session(
            &mut sessions,
            &env(9, SessionEvent::SessionEnded { session_id: sid }),
        );
        let s = &sessions[&sid];
        assert_eq!(s.ended_ts_ms, Some(7), "first end wins");
        assert_eq!(s.updated_ts_ms, 9, "the echo still touches");
        assert!(!s.is_open());
    }

    #[test]
    fn notes_for_unknown_sessions_are_reported_not_applied() {
        let sid: SessionId = SID.parse().expect("ulid");
        let mut sessions = BTreeMap::new();
        let outcome = apply_session(
            &mut sessions,
            &env(
                1,
                SessionEvent::SessionNoted {
                    session_id: sid,
                    note: SessionNote::NudgeSent {
                        reason: "threshold".into(),
                    },
                },
            ),
        );
        assert_eq!(outcome, SessionApplyOutcome::UnknownSession(sid));
    }

    #[test]
    fn envelope_wire_form_is_flat_snake_case_jsonl() {
        let sid: SessionId = SID.parse().expect("ulid");
        let e = env(42, SessionEvent::SessionEnded { session_id: sid });
        let json = serde_json::to_string(&e).expect("serializes");
        assert_eq!(
            json,
            format!(r#"{{"ts_ms":42,"event":"session_ended","session_id":"{SID}"}}"#)
        );
        let back: SessionEnvelope = serde_json::from_str(&json).expect("parses");
        assert_eq!(back, e);
    }

    #[test]
    fn note_wire_form_uses_kind_tags() {
        let sid: SessionId = SID.parse().expect("ulid");
        let e = env(
            1,
            SessionEvent::SessionNoted {
                session_id: sid,
                note: SessionNote::WorkTool {
                    tool: "Edit".into(),
                    duration_ms: 250,
                },
            },
        );
        let json = serde_json::to_string(&e).expect("serializes");
        assert_eq!(
            json,
            format!(
                r#"{{"ts_ms":1,"event":"session_noted","session_id":"{SID}","note":{{"kind":"work_tool","tool":"Edit","duration_ms":250}}}}"#
            )
        );
    }
}
