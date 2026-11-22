//! The sweeper: pod-loss detection, budget watchdogs, kill-delivery
//! escalation, and admission self-heal (plan D9, R6, Phase 4).
//!
//! Pods prove liveness by heartbeating; the journal only knows which pod
//! *was* bound to a run. The sweep closes the gap: a non-terminal run
//! whose pod is silent past the staleness window — or unknown after a
//! daemon restart past the adoption grace — gets its recorded pod pid
//! probed, and a dead pod reaps the run as `killed(pod_lost)`. A live
//! pid is always given more time: slow is not dead, and the OS-level
//! kill guarantee (Job Objects, F5) means a dead pod cannot have leaked
//! its worker.
//!
//! Phase 4 adds three passes on the same grid: the wall-clock budget
//! (`killed(budget)` past the packet's `wall_secs`; the token cap has a
//! fast path on usage arrival and this backstop), kill-delivery
//! escalation (a pending kill no heartbeat claimed within the window
//! goes to the OS fallback), and an admission tick (queued runs launch
//! whenever slots free — even when the freeing event was lost).

use std::sync::Arc;

use fractality_core::journal::Event;
use fractality_core::run::{KillReason, RunState};
use fractality_core::time::now_ms;

use crate::state::AppState;

specmark::scope!("spec://fractality/PROP-001#architecture");

/// Heartbeats older than this mark a pod stale.
const STALE_MS: u64 = 15_000;
/// After a daemon restart, pods get this long to re-register before
/// their pids are probed. Twice the heartbeat-staleness window: a pod
/// discovers a new generation on its next failed heartbeat (≤2 s) plus
/// one reconnect round — 20 s leaves real margin over the 5 s sweep grid.
const ADOPTION_GRACE_MS: u64 = 20_000;
/// A launched (`starting`) run whose pod never registered gets this long
/// before it is failed — pod.log in the run dir is the autopsy surface.
const POD_REGISTER_GRACE_MS: u64 = 60_000;
/// A pending kill unclaimed past this window escalates to the fallback
/// (several heartbeat intervals: the pod had its chances).
const KILL_CLAIM_MS: u64 = 10_000;
/// Sweep cadence.
const SWEEP_EVERY: std::time::Duration = std::time::Duration::from_secs(5);

/// Background loop; ends on daemon shutdown.
pub async fn reaper_loop(state: Arc<AppState>) {
    let mut shutdown_rx = state.shutdown.subscribe();
    loop {
        tokio::select! {
            _ = shutdown_rx.wait_for(|v| *v) => break,
            _ = tokio::time::sleep(SWEEP_EVERY) => {
                sweep(&state);
                sweep_budgets(&state);
                sweep_kill_delivery(&state);
                sweep_podless_starting(&state);
                crate::admission::tick(&state);
            }
        }
    }
}

/// The wall-clock budget backstop: a running run past its `wall_secs`
/// is killed with `killed(budget)`; the token cap re-checks here too in
/// case the fast path's snapshot was the one that got lost.
fn sweep_budgets(state: &AppState) {
    let now = now_ms();
    let over_budget: Vec<_> = state
        .list_runs(None, None)
        .into_iter()
        .filter(|r| !r.state.is_terminal() && r.state != RunState::Queued)
        .filter(|r| {
            let wall_hit = r.budget.wall_secs > 0
                && r.started_ts_ms
                    .is_some_and(|s| now.saturating_sub(s) > r.budget.wall_secs * 1000);
            let tokens_hit = r.budget.max_output_tokens > 0
                && r.usage.output_tokens > r.budget.max_output_tokens;
            wall_hit || tokens_hit
        })
        .collect();
    for run in over_budget {
        tracing::warn!(
            run_id = %run.run_id,
            wall_secs = run.budget.wall_secs,
            output_tokens = run.usage.output_tokens,
            "budget exceeded; killing"
        );
        crate::kill::kill_one(state, run.run_id, KillReason::Budget);
    }
}

/// Kill-delivery escalation: a `killed` run whose pod runtime still has
/// an unclaimed pending kill past the window — or whose pod went stale —
/// gets the OS fallback; at-least-once delivery, idempotent kill.
fn sweep_kill_delivery(state: &AppState) {
    let now = now_ms();
    let stale_kills: Vec<(u32, fractality_core::ids::PodId)> = {
        let inner = state.lock_inner();
        inner
            .pods
            .iter()
            .filter_map(|(pod_id, rt)| {
                let (_, armed_ts) = rt.pending_kill?;
                let unclaimed_long = now.saturating_sub(armed_ts) > KILL_CLAIM_MS;
                let pod_stale = now.saturating_sub(rt.last_heartbeat_ms) > STALE_MS;
                (unclaimed_long || pod_stale).then_some((rt.pod_pid, *pod_id))
            })
            .collect()
    };
    for (pod_pid, pod_id) in stale_kills {
        tracing::warn!(pod_pid, %pod_id, "pending kill unclaimed; OS fallback");
        crate::kill::fallback_kill(pod_pid);
        state.remove_pod(pod_id);
    }
}

/// A `starting` run with no pod binding past the registration grace:
/// the pod launch failed before it ever spoke — fail the run loudly and
/// name the autopsy surface. (Queued runs are healthy queue members and
/// are never touched here.)
fn sweep_podless_starting(state: &AppState) {
    let now = now_ms();
    let strays: Vec<_> = state
        .list_runs(Some(RunState::Starting), None)
        .into_iter()
        .filter(|r| r.pod.is_none())
        .filter(|r| now.saturating_sub(r.updated_ts_ms) > POD_REGISTER_GRACE_MS)
        .collect();
    for run in strays {
        tracing::error!(run_id = %run.run_id, "pod never registered; failing the run");
        let _ = state.record(Event::Error {
            run_id: run.run_id,
            message: format!(
                "pod never registered within {}s of launch; see `{}/pod.log`",
                POD_REGISTER_GRACE_MS / 1000,
                run.run_dir
            ),
            terminal: true,
        });
    }
}

fn sweep(state: &AppState) {
    let now = now_ms();
    // Snapshot under the lock; probe and record outside it.
    let (candidates, pods): (Vec<_>, std::collections::HashMap<_, _>) = {
        let inner = state.lock_inner();
        (
            inner
                .runs
                .values()
                .filter(|r| !r.state.is_terminal())
                .filter_map(|r| r.pod.map(|p| (r.run_id, p, r.updated_ts_ms)))
                .collect(),
            inner.pods.clone(),
        )
    };

    for (run_id, binding, updated_ts_ms) in candidates {
        let verdict = match pods.get(&binding.pod_id) {
            Some(rt) => {
                if now.saturating_sub(rt.last_heartbeat_ms) <= STALE_MS {
                    continue; // fresh heartbeat — healthy
                }
                probe_verdict(binding.pod_pid)
            }
            None => {
                // Unknown pod: either the daemon restarted (pod will
                // re-register) or the pod died while we were down.
                let quiet_for = now
                    .saturating_sub(state.started_ts_ms)
                    .min(now.saturating_sub(updated_ts_ms));
                if quiet_for <= ADOPTION_GRACE_MS {
                    continue; // still inside the adoption window
                }
                probe_verdict(binding.pod_pid)
            }
        };
        if verdict == PidVerdict::Dead {
            tracing::warn!(%run_id, pod_pid = binding.pod_pid, "pod lost; reaping run");
            if let Err(e) = state.record(Event::Killed {
                run_id,
                reason: KillReason::PodLost,
            }) {
                tracing::error!(%run_id, error = %e, "reaping failed");
            }
            state.remove_pod(binding.pod_id);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PidVerdict {
    Alive,
    Dead,
}

fn probe_verdict(pid: u32) -> PidVerdict {
    let mut system = sysinfo::System::new();
    let target = sysinfo::Pid::from_u32(pid);
    system.refresh_processes(sysinfo::ProcessesToUpdate::Some(&[target]), true);
    if system.process(target).is_some() {
        PidVerdict::Alive
    } else {
        PidVerdict::Dead
    }
}
