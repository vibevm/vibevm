//! The kill path (Phase 4): journal the decision, deliver via the pod,
//! fall back to the OS when the pod cannot answer.
//!
//! Mission-control records `killed(reason)` at decision time — that is
//! the authoritative state (the run is dead the moment the operator or
//! the budget says so). Delivery is best-effort layered: a live pod gets
//! the command on its next heartbeat and closes its Job Object (the F5
//! guarantee); a stale or vanished pod gets the OS fallback
//! (`taskkill /T /F` on the pod pid — the pod dies, `KILL_ON_JOB_CLOSE`
//! reaps its worker tree). The sweeper re-arms undelivered kills.

use fractality_core::api::{KillOutcome, KillResult};
use fractality_core::ids::RunId;
use fractality_core::journal::Event;
use fractality_core::run::KillReason;
use fractality_core::time::now_ms;

use crate::state::AppState;

specmark::scope!("spec://fractality/PROP-001#architecture");

/// Heartbeats older than this mean "do not wait for the pod" — the
/// fallback fires immediately. Kept in step with the reaper's window.
const POD_FRESH_MS: u64 = 15_000;

/// Kills one run (optionally its subtree, root first). Terminal runs
/// answer `AlreadyTerminal`; everything else is journaled `killed` and
/// signaled. The root-first order stops a dying parent from racing new
/// children into the tree (registration refuses terminal parents).
pub fn kill(state: &AppState, root: RunId, reason: KillReason, recursive: bool) -> Vec<KillResult> {
    let targets = if recursive {
        state.subtree_ids(root)
    } else {
        vec![root]
    };
    targets
        .into_iter()
        .map(|run_id| KillResult {
            run_id,
            outcome: kill_one(state, run_id, reason),
        })
        .collect()
}

/// The single-run kill: journal → pend-or-fallback.
pub fn kill_one(state: &AppState, run_id: RunId, reason: KillReason) -> KillOutcome {
    let Some(run) = state.get_run(run_id) else {
        // Only reachable through subtree races; treat as already gone.
        return KillOutcome::AlreadyTerminal;
    };
    if run.state.is_terminal() {
        return KillOutcome::AlreadyTerminal;
    }
    if let Err(e) = state.record(Event::Killed { run_id, reason }) {
        // The one legal race: the run went terminal between the check
        // and the record. Anything else is a real fault worth a log.
        tracing::warn!(%run_id, error = %e, "kill record refused");
        return KillOutcome::AlreadyTerminal;
    }

    signal_pod(state, run_id, &run, reason);
    KillOutcome::Killed
}

/// Delivery: heartbeat command for a fresh pod, OS fallback otherwise.
fn signal_pod(
    state: &AppState,
    run_id: RunId,
    run: &fractality_core::run::RunRecord,
    reason: KillReason,
) {
    let pod_fresh = {
        let inner = state.lock_inner();
        run.pod
            .and_then(|binding| inner.pods.get(&binding.pod_id))
            .is_some_and(|rt| now_ms().saturating_sub(rt.last_heartbeat_ms) <= POD_FRESH_MS)
    };
    if pod_fresh && state.pend_kill(run_id, reason) {
        tracing::info!(%run_id, %reason, "kill armed on the pod's next heartbeat");
        return;
    }
    match run.pod {
        Some(binding) => {
            tracing::warn!(
                %run_id,
                pod_pid = binding.pod_pid,
                "pod not fresh; OS fallback kill"
            );
            fallback_kill(binding.pod_pid);
        }
        None => {
            // Queued or starting-without-a-pod: nothing to signal — the
            // journaled `killed` state alone stops admission/adoption
            // (a late pod registration against a terminal run is refused).
            tracing::info!(%run_id, "no pod bound; kill is journal-only");
        }
    }
}

/// The pod-loss fallback (F5's demoted path): terminate the pod's own
/// process tree — the dying pod's Job Object handle closes and
/// `KILL_ON_JOB_CLOSE` reaps the worker tree with it.
pub fn fallback_kill(pod_pid: u32) {
    #[cfg(windows)]
    let result = std::process::Command::new("taskkill")
        .args(["/PID", &pod_pid.to_string(), "/T", "/F"])
        .output();
    #[cfg(unix)]
    let result = std::process::Command::new("kill")
        .args(["-9", &pod_pid.to_string()])
        .output();
    match result {
        Ok(out) if out.status.success() => {
            tracing::info!(pod_pid, "fallback kill delivered");
        }
        Ok(out) => {
            // Exit 128 on Windows = "not found": the pod died on its own
            // — the goal state, not an error.
            tracing::warn!(
                pod_pid,
                code = out.status.code().unwrap_or(-1),
                stderr = %String::from_utf8_lossy(&out.stderr).trim(),
                "fallback kill did not confirm"
            );
        }
        Err(e) => tracing::error!(pod_pid, error = %e, "fallback kill failed to spawn"),
    }
}
