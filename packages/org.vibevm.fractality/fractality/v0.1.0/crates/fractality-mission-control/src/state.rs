//! Shared daemon state and the single write path (plan D9).
//!
//! Every mutation goes through [`AppState::record`]: validate against the
//! current registry → append to the journal → fold into memory, all under
//! one lock. Replay at startup runs the *same fold*, so disk and memory
//! cannot drift. The lock is a plain `std::sync::Mutex` held only across
//! synchronous work — never across an await point.

use std::collections::HashMap;
use std::sync::Mutex;

use fractality_core::ids::{PodId, RunId};
use fractality_core::journal::{ApplyOutcome, Envelope, Event, apply};
use fractality_core::node::{NodeIdentity, ScopeInfo};
use fractality_core::run::{RunRecord, RunState};
use fractality_core::time::now_ms;
use specmark::spec;

use crate::journal_store::{JournalWriter, replay};
use crate::{Config, identity, scopes};

specmark::scope!("spec://fractality/PROP-001#architecture");

/// Live pod bookkeeping (never journaled: pods prove liveness by
/// re-registering and heartbeating, D3).
#[derive(Debug, Clone)]
pub struct PodRuntime {
    pub run_id: RunId,
    pub pod_pid: u32,
    pub last_heartbeat_ms: u64,
    /// A kill directive awaiting the pod's next heartbeat, with the
    /// moment it was armed (the sweeper escalates unclaimed kills to the
    /// pod-loss fallback). Never journaled — the authoritative `killed`
    /// record is; this is delivery bookkeeping.
    pub pending_kill: Option<(fractality_core::run::KillReason, u64)>,
}

pub struct Inner {
    pub journal: JournalWriter,
    pub runs: std::collections::BTreeMap<RunId, RunRecord>,
    pub pods: HashMap<PodId, PodRuntime>,
}

pub struct AppState {
    pub cfg: Config,
    pub bearer: String,
    pub mc_id: String,
    pub started_ts_ms: u64,
    pub node: NodeIdentity,
    pub scopes: Vec<ScopeInfo>,
    pub inner: Mutex<Inner>,
    /// Shutdown signal as *state* (watch, not Notify): late subscribers
    /// still observe a send that happened before their first poll.
    pub shutdown: tokio::sync::watch::Sender<bool>,
}

/// A `record()` refusal, mapped to an HTTP answer by the handlers.
#[derive(Debug, thiserror::Error, PartialEq)]
#[spec(implements = "spec://fractality/PROP-001#architecture")]
pub enum RecordError {
    #[error(
        "run {0} is not registered on this mission-control (violates spec://fractality/PROP-001#architecture; fix: register the run before referencing it)"
    )]
    UnknownRun(RunId),

    #[error(
        "run {run_id} cannot go {from} -> {to} (violates spec://fractality/PROP-001#architecture; fix: respect the state machine in fractality-core::run)"
    )]
    IllegalTransition {
        run_id: RunId,
        from: RunState,
        to: RunState,
    },

    #[error(
        "journal append failed: {0} (violates spec://fractality/PROP-001#architecture; fix: ensure the journal directory is writable and has space)"
    )]
    Journal(String),
}

impl AppState {
    /// Opens home state: replay the journal, detect identity, stamp the
    /// runs-scope beacon, mint the bearer.
    pub fn open(cfg: Config) -> Result<Self, String> {
        let mc_id = format!("mc-{}", ulid::Ulid::new());
        let node = identity::detect();
        let scopes = scopes::ensure_runs_scope(&cfg.home, &cfg.runs_root(), &mc_id)?;

        let (envelopes, report) = replay(&cfg.journal_dir())?;
        let mut runs = std::collections::BTreeMap::new();
        let mut refused = 0u64;
        for env in &envelopes {
            if apply(&mut runs, env) != ApplyOutcome::Applied {
                refused += 1;
            }
        }
        if report.skipped > 0 || refused > 0 || report.torn_tail {
            tracing::warn!(
                lines = report.lines,
                skipped = report.skipped,
                refused,
                torn_tail = report.torn_tail,
                "journal replay finished with anomalies"
            );
        } else {
            tracing::info!(lines = report.lines, runs = runs.len(), "journal replayed");
        }

        let journal = JournalWriter::open(&cfg.journal_dir())?;
        Ok(Self {
            cfg,
            bearer: fractality_mc_client::lock::mint_bearer(),
            mc_id,
            started_ts_ms: now_ms(),
            node,
            scopes,
            inner: Mutex::new(Inner {
                journal,
                runs,
                pods: HashMap::new(),
            }),
            shutdown: tokio::sync::watch::channel(false).0,
        })
    }

    /// Locks the inner state, riding through poison: a panicked holder
    /// cannot tear this state (the journal is flushed per event and every
    /// fold is a whole-record mutation), so wedging the daemon on poison
    /// would trade a survivable fault for a permanent outage.
    pub(crate) fn lock_inner(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// The single write path: validate → journal → fold.
    pub fn record(&self, event: Event) -> Result<RunRecord, RecordError> {
        let mut inner = self.lock_inner();
        Self::record_under_lock(&mut inner, event)
    }

    /// Atomically claims a queued run for launch (`queued -> starting`).
    /// The check and the journal write share one lock acquisition, so
    /// two concurrent admission ticks can never both claim the same run
    /// (the idempotent same-state path of `record` would let them).
    pub fn claim_queued(&self, run_id: RunId) -> bool {
        let mut inner = self.lock_inner();
        if inner.runs.get(&run_id).map(|r| r.state) != Some(RunState::Queued) {
            return false;
        }
        Self::record_under_lock(
            &mut inner,
            Event::State {
                run_id,
                state: RunState::Starting,
                detail: Some("admitted".to_owned()),
            },
        )
        .is_ok()
    }

    /// The write path body; callers hold the lock (directly or through
    /// [`Self::record`]).
    fn record_under_lock(inner: &mut Inner, event: Event) -> Result<RunRecord, RecordError> {
        // Validation happens against current state BEFORE the journal
        // sees the event — replay therefore only ever folds legal lines.
        if let Event::Registered { .. } = &event {
            // Registration needs no precondition.
        } else {
            let run_id = event.run_id();
            let run = inner
                .runs
                .get(&run_id)
                .ok_or(RecordError::UnknownRun(run_id))?;
            let target = match &event {
                Event::State { state, .. } => Some(*state),
                Event::Completed { exit_code, .. } => Some(match exit_code {
                    Some(0) => RunState::Completed,
                    _ => RunState::Failed,
                }),
                Event::Killed { .. } => Some(RunState::Killed),
                Event::Error { terminal: true, .. } => Some(RunState::Failed),
                Event::Question { .. } => Some(RunState::WaitingOnBoss),
                Event::Answer { .. } => Some(RunState::Running),
                _ => None,
            };
            // An answer strictly requires a parked run — the same-state
            // leniency below would otherwise let an answer land on a
            // running run and desynchronize validation from the fold.
            if matches!(&event, Event::Answer { .. }) && run.state != RunState::WaitingOnBoss {
                return Err(RecordError::IllegalTransition {
                    run_id,
                    from: run.state,
                    to: RunState::Running,
                });
            }
            if let Some(to) = target
                && run.state != to
                && !run.state.can_transition_to(to)
            {
                return Err(RecordError::IllegalTransition {
                    run_id,
                    from: run.state,
                    to,
                });
            }
        }

        let envelope = Envelope::now(event);
        inner
            .journal
            .append(&envelope)
            .map_err(RecordError::Journal)?;
        let outcome = apply(&mut inner.runs, &envelope);
        debug_assert_eq!(
            outcome,
            ApplyOutcome::Applied,
            "record() validated but apply refused — validation and fold diverged"
        );
        Ok(inner.runs[&envelope.event.run_id()].clone())
    }

    /// Registry read: all runs (ULID order = creation order, newest last),
    /// optional state filter, optional last-N limit.
    pub fn list_runs(&self, state: Option<RunState>, limit: Option<usize>) -> Vec<RunRecord> {
        let inner = self.lock_inner();
        let filtered: Vec<RunRecord> = inner
            .runs
            .values()
            .filter(|r| state.is_none_or(|s| r.state == s))
            .cloned()
            .collect();
        match limit {
            Some(n) if n < filtered.len() => filtered[filtered.len() - n..].to_vec(),
            _ => filtered,
        }
    }

    pub fn get_run(&self, id: RunId) -> Option<RunRecord> {
        self.lock_inner().runs.get(&id).cloned()
    }

    /// The call tree rooted at `id` (parent edges; single nodes until
    /// Phase 4 nests runs).
    pub fn tree(&self, id: RunId) -> Option<fractality_core::api::TreeNode> {
        let inner = self.lock_inner();
        fn build(
            runs: &std::collections::BTreeMap<RunId, RunRecord>,
            id: RunId,
        ) -> Option<fractality_core::api::TreeNode> {
            let run = runs.get(&id)?.clone();
            let children = runs
                .values()
                .filter(|r| r.parent == Some(id))
                .filter_map(|r| build(runs, r.run_id))
                .collect();
            Some(fractality_core::api::TreeNode { run, children })
        }
        build(&inner.runs, id)
    }

    pub fn counts(&self) -> (u64, u64) {
        let inner = self.lock_inner();
        let total = inner.runs.len() as u64;
        let open = inner
            .runs
            .values()
            .filter(|r| !r.state.is_terminal())
            .count() as u64;
        (total, open)
    }

    pub fn upsert_pod(&self, pod_id: PodId, runtime: PodRuntime) {
        self.lock_inner().pods.insert(pod_id, runtime);
    }

    /// Marks a heartbeat; false when the pod is unknown (it must
    /// re-register — e.g. after a daemon restart).
    pub fn pod_seen(&self, pod_id: PodId) -> bool {
        let mut inner = self.lock_inner();
        match inner.pods.get_mut(&pod_id) {
            Some(rt) => {
                rt.last_heartbeat_ms = now_ms();
                true
            }
            None => false,
        }
    }

    pub fn remove_pod(&self, pod_id: PodId) {
        self.lock_inner().pods.remove(&pod_id);
    }

    /// Arms a kill directive on the pod supervising `run_id`; the next
    /// heartbeat delivers it. Returns false when no live pod runtime is
    /// bound to that run (the caller escalates to the fallback).
    pub fn pend_kill(&self, run_id: RunId, reason: fractality_core::run::KillReason) -> bool {
        let mut inner = self.lock_inner();
        for rt in inner.pods.values_mut() {
            if rt.run_id == run_id {
                // First reason wins: a manual kill racing a budget kill
                // is still exactly one kill.
                if rt.pending_kill.is_none() {
                    rt.pending_kill = Some((reason, now_ms()));
                }
                return true;
            }
        }
        false
    }

    /// Takes the pending kill for delivery on a heartbeat answer.
    pub fn take_pending_kill(&self, pod_id: PodId) -> Option<fractality_core::run::KillReason> {
        self.lock_inner()
            .pods
            .get_mut(&pod_id)
            .and_then(|rt| rt.pending_kill.take().map(|(reason, _)| reason))
    }

    /// The call-tree ids rooted at `root`, root first (BFS). Used by the
    /// recursive kill; missing root yields an empty list.
    pub fn subtree_ids(&self, root: RunId) -> Vec<RunId> {
        let inner = self.lock_inner();
        if !inner.runs.contains_key(&root) {
            return Vec::new();
        }
        let mut out = vec![root];
        let mut frontier = vec![root];
        while let Some(parent) = frontier.pop() {
            for r in inner.runs.values() {
                if r.parent == Some(parent) {
                    out.push(r.run_id);
                    frontier.push(r.run_id);
                }
            }
        }
        out
    }

    /// Runs of a profile currently holding an admission slot: launched
    /// and not yet terminal (queued runs hold no slot).
    pub fn active_count(&self, profile: &str) -> usize {
        let inner = self.lock_inner();
        inner
            .runs
            .values()
            .filter(|r| r.profile == profile)
            .filter(|r| !r.state.is_terminal() && r.state != RunState::Queued)
            .count()
    }

    /// Queued spawn-requested runs in creation order (ULID order is
    /// admission FIFO).
    pub fn queued_candidates(&self) -> Vec<RunRecord> {
        let inner = self.lock_inner();
        inner
            .runs
            .values()
            .filter(|r| r.state == RunState::Queued && r.spawn_requested)
            .cloned()
            .collect()
    }
}
