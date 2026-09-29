//! Journal events and their wire form (plan D9).
//!
//! Mission-control's storage is an append-only JSONL journal: one
//! [`Envelope`] per line. Events carry cumulative snapshots where
//! idempotence matters (usage), and terminal facts ride typed events
//! (`completed` / `killed` / `escalated` / `error`) so replay never has to
//! reconstruct them from prose.
//!
//! The replay fold that turns this event stream into in-memory state lives
//! in [`crate::journal_fold`]; [`apply`] and [`ApplyOutcome`] are
//! re-exported here so `journal::apply` stays the one call site.

use serde::{Deserialize, Serialize};

use crate::ids::{PodId, RunId};
use crate::run::{Collected, KillReason, RunRecord, RunState, UsageTotals};
use crate::time::now_ms;

specmark::scope!("spec://fractality/PROP-001#architecture");

pub use crate::journal_fold::{ApplyOutcome, apply};

/// One journal line: a timestamp plus the event.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Envelope {
    pub ts_ms: u64,
    #[serde(flatten)]
    pub event: Event,
}

impl Envelope {
    /// Wraps an event with the current wall clock.
    pub fn now(event: Event) -> Self {
        Self {
            ts_ms: now_ms(),
            event,
        }
    }
}

/// The D9 event vocabulary. `event` is the JSONL discriminator tag.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum Event {
    /// A run was created from a packet; carries the full initial record.
    /// Boxed: this variant is ~5× the next largest, and events travel by
    /// value through channels and folds (clippy::large_enum_variant).
    Registered { run: Box<RunRecord> },
    /// A pod bound itself to the run (D3: one pod per run).
    PodAssigned {
        run_id: RunId,
        pod_id: PodId,
        pod_pid: u32,
    },
    /// The pod spawned the worker process.
    Spawned { run_id: RunId, worker_pid: u32 },
    /// A non-terminal state transition (terminal facts ride the typed
    /// events below).
    State {
        run_id: RunId,
        state: RunState,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        detail: Option<String>,
    },
    /// Cumulative usage snapshot (idempotent on replay).
    Usage { run_id: RunId, usage: UsageTotals },
    /// Collection settled: result provenance + acceptance verdicts
    /// (idempotent — the latest collection wins). Applies to terminal
    /// runs too: a killed worker's partial collection is still a fact.
    Collected {
        run_id: RunId,
        collected: Box<Collected>,
    },
    /// The worker parked on a question (D18): `running ->
    /// waiting_on_boss`, the question rides the record.
    Question { run_id: RunId, question: String },
    /// The boss answered: `waiting_on_boss -> running`, the answer rides
    /// the record for the broker to collect. `auto_rule` names the
    /// profile rule when mission-control answered without parking the
    /// boss (Ф5 — the D18 layer-2 slice); `None` is a human/boss answer.
    Answer {
        run_id: RunId,
        answer: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        auto_rule: Option<String>,
    },
    /// The worker exited. `Some(0)` completes the run; any other code —
    /// or a signal death (`None`) — fails it.
    Completed {
        run_id: RunId,
        exit_code: Option<i32>,
    },
    /// The run was killed (operator, budget, or pod loss).
    Killed { run_id: RunId, reason: KillReason },
    /// The run handed its task UP the tree (D-C3-6): `running |
    /// waiting_on_boss -> escalated`, a terminal OUTCOME, not a failure.
    /// `reason`/`needs` ride the record so the parent can act on the
    /// handed-up task as the escalation climbs the `parent` edges.
    Escalated {
        run_id: RunId,
        reason: String,
        needs: String,
    },
    /// An infrastructure error; `terminal` decides whether the run dies
    /// of it.
    Error {
        run_id: RunId,
        message: String,
        terminal: bool,
    },
}

impl Event {
    /// The run this event belongs to.
    pub fn run_id(&self) -> RunId {
        match self {
            Event::Registered { run } => run.run_id,
            Event::PodAssigned { run_id, .. }
            | Event::Spawned { run_id, .. }
            | Event::State { run_id, .. }
            | Event::Usage { run_id, .. }
            | Event::Collected { run_id, .. }
            | Event::Question { run_id, .. }
            | Event::Answer { run_id, .. }
            | Event::Completed { run_id, .. }
            | Event::Killed { run_id, .. }
            | Event::Escalated { run_id, .. }
            | Event::Error { run_id, .. } => *run_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(ts_ms: u64, event: Event) -> Envelope {
        Envelope { ts_ms, event }
    }

    const RUN: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAV";

    #[test]
    fn envelope_wire_form_is_flat_snake_case_jsonl() {
        let run_id: RunId = RUN.parse().expect("ulid");
        let e = env(
            42,
            Event::Killed {
                run_id,
                reason: KillReason::PodLost,
            },
        );
        let json = serde_json::to_string(&e).expect("serializes");
        assert_eq!(
            json,
            format!(r#"{{"ts_ms":42,"event":"killed","run_id":"{RUN}","reason":"pod_lost"}}"#)
        );
        let back: Envelope = serde_json::from_str(&json).expect("parses");
        assert_eq!(back, e);
    }

    #[test]
    fn escalated_wire_form_is_flat_snake_case_jsonl() {
        let run_id: RunId = RUN.parse().expect("ulid");
        let e = env(
            7,
            Event::Escalated {
                run_id,
                reason: "silo".into(),
                needs: "bigger window".into(),
            },
        );
        let json = serde_json::to_string(&e).expect("serializes");
        assert_eq!(
            json,
            format!(
                r#"{{"ts_ms":7,"event":"escalated","run_id":"{RUN}","reason":"silo","needs":"bigger window"}}"#
            )
        );
        let back: Envelope = serde_json::from_str(&json).expect("parses");
        assert_eq!(back, e);
    }
}
