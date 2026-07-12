//! The replay fold (plan D9): folding a journal event stream into the
//! in-memory run map.
//!
//! [`apply`] is the single fold step, shared verbatim by mission-control's
//! startup replay and its live write path, so the two can never disagree.
//! Split from [`crate::journal`] (the event vocabulary + wire form) along
//! the fold/vocabulary seam; `journal::apply` and `journal::ApplyOutcome`
//! still resolve through the re-export there.
//!
//! Timestamps inside the fold come from the envelope, never from the
//! clock — replay is deterministic by construction.

use std::collections::BTreeMap;

use crate::ids::RunId;
use crate::journal::{Envelope, Event};
use crate::run::{EscalationRecord, PodBinding, RunRecord, RunState};

specmark::scope!("spec://fractality/PROP-001#architecture");

/// Outcome of folding one envelope into the registry map.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApplyOutcome {
    Applied,
    /// The event references a run the map does not hold (journal gap or
    /// foreign-home event) — nothing was mutated.
    UnknownRun(RunId),
    /// The event asked for a transition the state machine forbids —
    /// nothing was mutated. Write paths validate before journaling, so
    /// replay hitting this means the journal itself is damaged.
    IllegalTransition {
        run_id: RunId,
        from: RunState,
        to: RunState,
    },
}

/// Folds one envelope into the run map. Both mission-control's live write
/// path and startup replay call exactly this function.
pub fn apply(runs: &mut BTreeMap<RunId, RunRecord>, envelope: &Envelope) -> ApplyOutcome {
    let ts = envelope.ts_ms;
    match &envelope.event {
        Event::Registered { run } => {
            runs.insert(run.run_id, (**run).clone());
            ApplyOutcome::Applied
        }
        Event::PodAssigned {
            run_id,
            pod_id,
            pod_pid,
        } => with_run(runs, *run_id, |r| {
            r.pod = Some(PodBinding {
                pod_id: *pod_id,
                pod_pid: *pod_pid,
            });
            r.updated_ts_ms = ts;
            ApplyOutcome::Applied
        }),
        Event::Spawned { run_id, worker_pid } => with_run(runs, *run_id, |r| {
            r.worker_pid = Some(*worker_pid);
            r.updated_ts_ms = ts;
            ApplyOutcome::Applied
        }),
        Event::State { run_id, state, .. } => with_run(runs, *run_id, |r| {
            if r.state == *state {
                // Idempotent re-assertion (heartbeat echo); not an error.
                r.updated_ts_ms = ts;
                return ApplyOutcome::Applied;
            }
            if !r.state.can_transition_to(*state) {
                return ApplyOutcome::IllegalTransition {
                    run_id: *run_id,
                    from: r.state,
                    to: *state,
                };
            }
            r.state = *state;
            if *state == RunState::Starting && r.started_ts_ms.is_none() {
                // The budget wall-clock anchor: when the run left the
                // queue. Envelope time, so replay stays deterministic.
                r.started_ts_ms = Some(ts);
            }
            r.updated_ts_ms = ts;
            ApplyOutcome::Applied
        }),
        Event::Usage { run_id, usage } => with_run(runs, *run_id, |r| {
            r.usage = *usage;
            r.updated_ts_ms = ts;
            ApplyOutcome::Applied
        }),
        Event::Collected { run_id, collected } => with_run(runs, *run_id, |r| {
            r.collected = Some((**collected).clone());
            r.updated_ts_ms = ts;
            ApplyOutcome::Applied
        }),
        Event::Question { run_id, question } => with_run(runs, *run_id, |r| {
            // Idempotent re-ask while already parked updates the text;
            // otherwise the transition must be legal.
            if r.state != RunState::WaitingOnBoss
                && !r.state.can_transition_to(RunState::WaitingOnBoss)
            {
                return ApplyOutcome::IllegalTransition {
                    run_id: *run_id,
                    from: r.state,
                    to: RunState::WaitingOnBoss,
                };
            }
            r.state = RunState::WaitingOnBoss;
            r.question = Some(question.clone());
            // A fresh question invalidates any previous answer: the
            // broker must never read a stale reply.
            r.answer = None;
            r.updated_ts_ms = ts;
            ApplyOutcome::Applied
        }),
        Event::Answer { run_id, answer, .. } => with_run(runs, *run_id, |r| {
            if !r.state.can_transition_to(RunState::Running) {
                return ApplyOutcome::IllegalTransition {
                    run_id: *run_id,
                    from: r.state,
                    to: RunState::Running,
                };
            }
            r.state = RunState::Running;
            r.question = None;
            r.answer = Some(answer.clone());
            r.updated_ts_ms = ts;
            ApplyOutcome::Applied
        }),
        Event::Completed { run_id, exit_code } => with_run(runs, *run_id, |r| {
            let target = match exit_code {
                Some(0) => RunState::Completed,
                _ => RunState::Failed,
            };
            if !r.state.can_transition_to(target) {
                return ApplyOutcome::IllegalTransition {
                    run_id: *run_id,
                    from: r.state,
                    to: target,
                };
            }
            r.state = target;
            r.exit_code = *exit_code;
            if target == RunState::Failed {
                r.failure = Some(match exit_code {
                    Some(code) => format!("worker exited with code {code}"),
                    None => "worker terminated without an exit code".to_owned(),
                });
            }
            r.updated_ts_ms = ts;
            ApplyOutcome::Applied
        }),
        Event::Killed { run_id, reason } => with_run(runs, *run_id, |r| {
            if !r.state.can_transition_to(RunState::Killed) {
                return ApplyOutcome::IllegalTransition {
                    run_id: *run_id,
                    from: r.state,
                    to: RunState::Killed,
                };
            }
            r.state = RunState::Killed;
            r.kill_reason = Some(*reason);
            r.updated_ts_ms = ts;
            ApplyOutcome::Applied
        }),
        Event::Escalated {
            run_id,
            reason,
            needs,
        } => with_run(runs, *run_id, |r| {
            if !r.state.can_transition_to(RunState::Escalated) {
                return ApplyOutcome::IllegalTransition {
                    run_id: *run_id,
                    from: r.state,
                    to: RunState::Escalated,
                };
            }
            r.state = RunState::Escalated;
            r.escalation = Some(EscalationRecord {
                reason: reason.clone(),
                needs: needs.clone(),
            });
            r.updated_ts_ms = ts;
            ApplyOutcome::Applied
        }),
        Event::Error {
            run_id,
            message,
            terminal,
        } => with_run(runs, *run_id, |r| {
            if *terminal {
                if !r.state.can_transition_to(RunState::Failed) {
                    return ApplyOutcome::IllegalTransition {
                        run_id: *run_id,
                        from: r.state,
                        to: RunState::Failed,
                    };
                }
                r.state = RunState::Failed;
            }
            r.failure = Some(message.clone());
            r.updated_ts_ms = ts;
            ApplyOutcome::Applied
        }),
    }
}

fn with_run(
    runs: &mut BTreeMap<RunId, RunRecord>,
    run_id: RunId,
    f: impl FnOnce(&mut RunRecord) -> ApplyOutcome,
) -> ApplyOutcome {
    match runs.get_mut(&run_id) {
        Some(r) => f(r),
        None => ApplyOutcome::UnknownRun(run_id),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packet::WorkspaceMode;
    use crate::run::UsageTotals;

    fn fixed_run(id: &str) -> RunRecord {
        RunRecord {
            run_id: id.parse().expect("fixed ulid"),
            title: "t".into(),
            state: RunState::Queued,
            profile: "glm".into(),
            model: "big".into(),
            workspace_mode: WorkspaceMode::Dir,
            parent: None,
            origin_session: None,
            depth: 0,
            spawn_requested: false,
            verifier: false,
            budget: crate::packet::BudgetSpec::default(),
            node_id: "node-1".into(),
            run_dir: "runs/x".into(),
            created_ts_ms: 1,
            updated_ts_ms: 1,
            started_ts_ms: None,
            pod: None,
            worker_pid: None,
            exit_code: None,
            failure: None,
            kill_reason: None,
            usage: UsageTotals::default(),
            collected: None,
            question: None,
            answer: None,
            escalation: None,
        }
    }

    fn env(ts_ms: u64, event: Event) -> Envelope {
        Envelope { ts_ms, event }
    }

    const RUN: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAV";
    const POD: &str = "01BX5ZZKBKACTAV9WEVGEMMVRY";

    #[test]
    fn happy_path_fold_reaches_completed() {
        let run_id: RunId = RUN.parse().expect("ulid");
        let mut runs = BTreeMap::new();
        let events = [
            env(
                1,
                Event::Registered {
                    run: Box::new(fixed_run(RUN)),
                },
            ),
            env(
                2,
                Event::PodAssigned {
                    run_id,
                    pod_id: POD.parse().expect("ulid"),
                    pod_pid: 4242,
                },
            ),
            env(
                3,
                Event::State {
                    run_id,
                    state: RunState::Starting,
                    detail: None,
                },
            ),
            env(
                4,
                Event::Spawned {
                    run_id,
                    worker_pid: 5555,
                },
            ),
            env(
                5,
                Event::State {
                    run_id,
                    state: RunState::Running,
                    detail: None,
                },
            ),
            env(
                6,
                Event::Usage {
                    run_id,
                    usage: UsageTotals {
                        input_tokens: 10,
                        output_tokens: 20,
                        ..Default::default()
                    },
                },
            ),
            env(
                7,
                Event::Completed {
                    run_id,
                    exit_code: Some(0),
                },
            ),
        ];
        for e in &events {
            assert_eq!(apply(&mut runs, e), ApplyOutcome::Applied, "event {e:?}");
        }
        let r = &runs[&run_id];
        assert_eq!(r.state, RunState::Completed);
        assert_eq!(r.exit_code, Some(0));
        assert_eq!(r.worker_pid, Some(5555));
        assert_eq!(r.usage.output_tokens, 20);
        assert_eq!(r.updated_ts_ms, 7, "fold uses envelope time, not the clock");
    }

    #[test]
    fn nonzero_exit_fails_the_run_with_a_cause() {
        let run_id: RunId = RUN.parse().expect("ulid");
        let mut runs = BTreeMap::new();
        apply(
            &mut runs,
            &env(
                1,
                Event::Registered {
                    run: Box::new(fixed_run(RUN)),
                },
            ),
        );
        apply(
            &mut runs,
            &env(
                2,
                Event::State {
                    run_id,
                    state: RunState::Starting,
                    detail: None,
                },
            ),
        );
        apply(
            &mut runs,
            &env(
                3,
                Event::Completed {
                    run_id,
                    exit_code: Some(3),
                },
            ),
        );
        let r = &runs[&run_id];
        assert_eq!(r.state, RunState::Failed);
        assert_eq!(r.exit_code, Some(3));
        assert_eq!(r.failure.as_deref(), Some("worker exited with code 3"));
    }

    #[test]
    fn escalation_is_a_terminal_outcome_carrying_reason_and_needs() {
        let run_id: RunId = RUN.parse().expect("ulid");
        let mut runs = BTreeMap::new();
        for e in [
            env(
                1,
                Event::Registered {
                    run: Box::new(fixed_run(RUN)),
                },
            ),
            env(
                2,
                Event::State {
                    run_id,
                    state: RunState::Starting,
                    detail: None,
                },
            ),
            env(
                3,
                Event::State {
                    run_id,
                    state: RunState::Running,
                    detail: None,
                },
            ),
            env(
                4,
                Event::Escalated {
                    run_id,
                    reason: "cross-chunk reasoning — a split loses it".into(),
                    needs: "route to the largest-window profile".into(),
                },
            ),
        ] {
            assert_eq!(apply(&mut runs, &e), ApplyOutcome::Applied, "event {e:?}");
        }
        let r = &runs[&run_id];
        assert_eq!(r.state, RunState::Escalated);
        assert!(r.state.is_terminal(), "escalated is terminal");
        let esc = r.escalation.as_ref().expect("escalation record set");
        assert_eq!(esc.reason, "cross-chunk reasoning — a split loses it");
        assert_eq!(esc.needs, "route to the largest-window profile");
        assert_eq!(r.updated_ts_ms, 4, "fold uses envelope time");

        // Terminal: any further transition is refused without mutation.
        let outcome = apply(
            &mut runs,
            &env(
                5,
                Event::Completed {
                    run_id,
                    exit_code: Some(0),
                },
            ),
        );
        assert_eq!(
            outcome,
            ApplyOutcome::IllegalTransition {
                run_id,
                from: RunState::Escalated,
                to: RunState::Completed,
            }
        );
    }

    #[test]
    fn illegal_transition_is_refused_without_mutation() {
        let run_id: RunId = RUN.parse().expect("ulid");
        let mut runs = BTreeMap::new();
        apply(
            &mut runs,
            &env(
                1,
                Event::Registered {
                    run: Box::new(fixed_run(RUN)),
                },
            ),
        );
        // queued -> waiting_on_boss is not a legal edge.
        let outcome = apply(
            &mut runs,
            &env(
                2,
                Event::State {
                    run_id,
                    state: RunState::WaitingOnBoss,
                    detail: None,
                },
            ),
        );
        assert_eq!(
            outcome,
            ApplyOutcome::IllegalTransition {
                run_id,
                from: RunState::Queued,
                to: RunState::WaitingOnBoss,
            }
        );
        assert_eq!(runs[&run_id].state, RunState::Queued, "no mutation");
        assert_eq!(runs[&run_id].updated_ts_ms, 1, "no touch");
    }

    #[test]
    fn events_for_unknown_runs_are_reported_not_applied() {
        let mut runs = BTreeMap::new();
        let run_id: RunId = RUN.parse().expect("ulid");
        let outcome = apply(
            &mut runs,
            &env(
                1,
                Event::Spawned {
                    run_id,
                    worker_pid: 1,
                },
            ),
        );
        assert_eq!(outcome, ApplyOutcome::UnknownRun(run_id));
    }
}
