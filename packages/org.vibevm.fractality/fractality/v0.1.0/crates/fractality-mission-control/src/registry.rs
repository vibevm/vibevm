//! The reaper: pod-loss detection and run reconciliation (plan D9, R6).
//!
//! Pods prove liveness by heartbeating; the journal only knows which pod
//! *was* bound to a run. The sweep closes the gap: a non-terminal run
//! whose pod is silent past the staleness window — or unknown after a
//! daemon restart past the adoption grace — gets its recorded pod pid
//! probed, and a dead pod reaps the run as `killed(pod_lost)`. A live
//! pid is always given more time: slow is not dead, and the OS-level
//! kill guarantee (Job Objects, F5) means a dead pod cannot have leaked
//! its worker.

use std::sync::Arc;

use fractality_core::journal::Event;
use fractality_core::run::KillReason;
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
/// Sweep cadence.
const SWEEP_EVERY: std::time::Duration = std::time::Duration::from_secs(5);

/// Background loop; ends on daemon shutdown.
pub async fn reaper_loop(state: Arc<AppState>) {
    let mut shutdown_rx = state.shutdown.subscribe();
    loop {
        tokio::select! {
            _ = shutdown_rx.wait_for(|v| *v) => break,
            _ = tokio::time::sleep(SWEEP_EVERY) => sweep(&state),
        }
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
