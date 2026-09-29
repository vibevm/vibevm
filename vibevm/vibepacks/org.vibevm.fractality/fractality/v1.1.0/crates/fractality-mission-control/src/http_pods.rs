//! The pod leg of the HTTP bus (plan D3/D10): a per-run pod supervisor
//! registers, heartbeats (the kill-delivery channel rides the answer),
//! and reports worker lifecycle events that fold into the journal. Split
//! from `http.rs` along the client-leg/pod-leg seam the DTOs already draw
//! (`api.rs`); the router in `http.rs` is the single registration point.

use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use fractality_core::api::{
    Ack, PodCommand, PodEvent, PodEventRequest, PodHeartbeat, PodHeartbeatResponse,
    PodRegisterRequest, PodRegisterResponse,
};
use fractality_core::ids::PodId;
use fractality_core::journal::Event;
use fractality_core::run::{Collected, KillReason, RunState};
use fractality_core::time::now_ms;

use crate::http::ApiError;
use crate::state::{AppState, PodRuntime, RecordError};

// 1 s (was 2 s): the heartbeat answer is also the kill-delivery channel
// (Phase 4) — P5 wants a tree dead in under two seconds, and worst-case
// delivery is one full interval. Localhost chatter is a non-cost.
const HEARTBEAT_INTERVAL_MS: u64 = 1_000;

specmark::scope!("spec://fractality/PROP-001#architecture");

pub(crate) async fn pod_register(
    State(state): State<Arc<AppState>>,
    Json(req): Json<PodRegisterRequest>,
) -> Result<Json<PodRegisterResponse>, ApiError> {
    let run = state
        .get_run(req.run_id)
        .ok_or(RecordError::UnknownRun(req.run_id))
        .map_err(ApiError::from)?;
    if run.state.is_terminal() {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            format!("run {} is already {}", req.run_id, run.state),
        )
        .hint("a terminal run cannot adopt a pod; the pod should exit"));
    }

    state.record(Event::PodAssigned {
        run_id: req.run_id,
        pod_id: req.pod_id,
        pod_pid: req.pod_pid,
    })?;
    // First registration moves the run out of the queue; re-registration
    // after a daemon restart must NOT rewind a running run.
    if run.state == RunState::Queued {
        state.record(Event::State {
            run_id: req.run_id,
            state: RunState::Starting,
            detail: None,
        })?;
    }
    state.upsert_pod(
        req.pod_id,
        PodRuntime {
            run_id: req.run_id,
            pod_pid: req.pod_pid,
            last_heartbeat_ms: now_ms(),
            pending_kill: None,
        },
    );
    tracing::info!(run_id = %req.run_id, pod_id = %req.pod_id, pod_pid = req.pod_pid, "pod registered");
    Ok(Json(PodRegisterResponse {
        adopted: true,
        heartbeat_interval_ms: HEARTBEAT_INTERVAL_MS,
    }))
}

pub(crate) async fn pod_heartbeat(
    State(state): State<Arc<AppState>>,
    Path(pod_id): Path<String>,
    Json(_hb): Json<PodHeartbeat>,
) -> Result<Json<PodHeartbeatResponse>, ApiError> {
    let pod_id: PodId = pod_id.parse().map_err(|_| {
        ApiError::new(
            StatusCode::BAD_REQUEST,
            format!("`{pod_id}` is not a pod id"),
        )
    })?;
    if !state.pod_seen(pod_id) {
        return Err(ApiError::new(
            StatusCode::NOT_FOUND,
            format!("pod {pod_id} is not registered on this mission-control"),
        )
        .hint("re-register with POST /v0/pods/register (expected after a daemon restart)"));
    }
    // The heartbeat answer is the pod's command channel (D3): an armed
    // kill rides back here. If this response is lost in flight, the
    // sweeper re-arms it — delivery is at-least-once, the pod's kill is
    // idempotent.
    let command = match state.take_pending_kill(pod_id) {
        Some(reason) => PodCommand::Kill { reason },
        None => PodCommand::None,
    };
    Ok(Json(PodHeartbeatResponse { command }))
}

pub(crate) async fn pod_event(
    State(state): State<Arc<AppState>>,
    Path(pod_id): Path<String>,
    Json(req): Json<PodEventRequest>,
) -> Result<Json<Ack>, ApiError> {
    let _pod_id: PodId = pod_id.parse().map_err(|_| {
        ApiError::new(
            StatusCode::BAD_REQUEST,
            format!("`{pod_id}` is not a pod id"),
        )
    })?;
    let run_id = req.run_id;
    let run = state
        .get_run(run_id)
        .ok_or(RecordError::UnknownRun(run_id))?;
    // A pod reporting on a run mission-control already closed (killed by
    // operator or budget) is the normal tail of a kill, not a protocol
    // violation: state-machine events become acks, usage and collection
    // still land as facts.
    let already_terminal = run.state.is_terminal();
    match req.event {
        PodEvent::Spawned { worker_pid } => {
            if !already_terminal {
                state.record(Event::Spawned { run_id, worker_pid })?;
                // A fast worker can exit before anyone observes `running`;
                // the transition is still Starting -> Running -> …,
                // asserted here where the spawn fact arrives.
                let run = state
                    .get_run(run_id)
                    .ok_or(RecordError::UnknownRun(run_id))?;
                if run.state == RunState::Starting {
                    state.record(Event::State {
                        run_id,
                        state: RunState::Running,
                        detail: None,
                    })?;
                }
            }
        }
        PodEvent::State { state: s, detail } => {
            if !already_terminal {
                state.record(Event::State {
                    run_id,
                    state: s,
                    detail,
                })?;
            }
        }
        PodEvent::Usage { usage } => {
            state.record(Event::Usage { run_id, usage })?;
            // Budget fast path (Phase 4): the token cap fires the moment
            // the snapshot crosses it — the sweeper is only the backstop.
            if !already_terminal
                && run.budget.max_output_tokens > 0
                && usage.output_tokens > run.budget.max_output_tokens
            {
                tracing::warn!(
                    %run_id,
                    output_tokens = usage.output_tokens,
                    cap = run.budget.max_output_tokens,
                    "output-token budget exceeded; killing"
                );
                crate::kill::kill_one(&state, run_id, KillReason::Budget);
            }
        }
        PodEvent::Collected {
            result_source,
            result_path,
            acceptance_passed,
            acceptance_total,
            acceptance_skipped,
        } => {
            // Terminal-tolerant by design: a killed worker's partial
            // collection is still a fact worth folding.
            let result = result_path
                .as_deref()
                .and_then(|p| mint_file_ref(&state, p));
            state.record(Event::Collected {
                run_id,
                collected: Box::new(Collected {
                    result_source,
                    result,
                    result_path,
                    acceptance_passed,
                    acceptance_total,
                    acceptance_skipped,
                }),
            })?;
        }
        PodEvent::Exit { exit_code } => {
            if already_terminal {
                tracing::info!(
                    %run_id,
                    ?exit_code,
                    "exit report on a closed run acknowledged (kill tail)"
                );
            } else {
                state.record(Event::Completed { run_id, exit_code })?;
                // A freed slot admits the next queued run.
                crate::admission::tick(&state);
            }
        }
        PodEvent::Fault { message, terminal } => {
            if already_terminal {
                tracing::warn!(%run_id, %message, "fault reported on a closed run");
            } else {
                state.record(Event::Error {
                    run_id,
                    message,
                    terminal,
                })?;
                if terminal {
                    crate::admission::tick(&state);
                }
            }
        }
    }
    Ok(Json(Ack { ok: true }))
}

/// Mints the claim-check reference for a collected result (D19): the
/// path must lie inside a known scope; the etag is the cheap size+mtime
/// fingerprint readers use as `If-Match`.
fn mint_file_ref(state: &AppState, path: &camino::Utf8Path) -> Option<fractality_core::FileRef> {
    let scope = state.scopes.first()?;
    let rel = path.strip_prefix(&scope.root).ok()?;
    let meta = std::fs::metadata(path.as_std_path()).ok()?;
    let mtime_ms = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    Some(fractality_core::FileRef {
        fs: scope.id.clone(),
        path: rel.as_str().replace('\\', "/"),
        range: fractality_core::fileref::RefRange::whole(),
        etag: Some(format!("{:x}-{:x}", meta.len(), mtime_ms)),
        sha256: None,
        grant: None,
    })
}
