//! The HTTP bus (plan D10): versioned localhost API, bearer on every
//! call, uniform JSON errors carrying a hint (Class F diagnostics).

use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::middleware::{Next, from_fn_with_state};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use fractality_core::api::{
    Ack, ErrorResponse, HealthResponse, KillRequest, KillResponse, MetricsResponse, NodeResponse,
    PodCommand, PodEvent, PodEventRequest, PodHeartbeat, PodHeartbeatResponse, PodRegisterRequest,
    PodRegisterResponse, RegisterRunRequest, RunListResponse, ShutdownResponse, TreeNode,
};
use fractality_core::ids::{PodId, RunId};
use fractality_core::journal::Event;
use fractality_core::run::{Collected, KillReason, RunRecord, RunState, UsageTotals};
use fractality_core::time::now_ms;
use serde::Deserialize;

use crate::http_sessions as hs;
use crate::state::{AppState, PodRuntime, RecordError};

specmark::scope!("spec://fractality/PROP-001#architecture");

// 1 s (was 2 s): the heartbeat answer is also the kill-delivery channel
// (Phase 4) — P5 wants a tree dead in under two seconds, and worst-case
// delivery is one full interval. Localhost chatter is a non-cost.
const HEARTBEAT_INTERVAL_MS: u64 = 1_000;

/// Builds the versioned router with the bearer gate in front.
pub fn router(state: Arc<AppState>) -> Router {
    let api = Router::new()
        .route("/health", get(health))
        .route("/node", get(node))
        .route("/runs", post(register_run).get(list_runs))
        .route("/runs/{id}", get(get_run))
        .route("/runs/{id}/tree", get(get_tree))
        .route("/runs/{id}/kill", post(kill_run_endpoint))
        .route(
            "/runs/{id}/question",
            post(crate::http_questions::post_question),
        )
        .route(
            "/runs/{id}/answer",
            post(crate::http_questions::post_answer),
        )
        .route("/metrics", get(metrics))
        .route(
            "/decisions",
            post(crate::http_decisions::post_decision).get(crate::http_decisions::list_decisions),
        )
        .route("/sessions", post(hs::session_begin).get(hs::session_list))
        .route("/sessions/{id}", get(hs::session_get))
        .route("/sessions/{id}/events", post(hs::session_event))
        .route("/sessions/{id}/end", post(hs::session_end))
        .route("/sessions/{id}/metrics", get(hs::session_metrics))
        .route("/shutdown", post(shutdown))
        .route("/pods/register", post(pod_register))
        .route("/pods/{id}/heartbeat", post(pod_heartbeat))
        .route("/pods/{id}/event", post(pod_event))
        .layer(from_fn_with_state(state.clone(), auth))
        .with_state(state);
    Router::new().nest(fractality_core::api::API_PREFIX, api)
}

/// Uniform API error: status + message + fix-surface hint (shared with
/// the session leg, hence crate-visible).
pub(crate) struct ApiError {
    status: StatusCode,
    error: String,
    hint: Option<String>,
}

impl ApiError {
    pub(crate) fn new(status: StatusCode, error: impl Into<String>) -> Self {
        Self {
            status,
            error: error.into(),
            hint: None,
        }
    }

    pub(crate) fn hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(ErrorResponse {
                error: self.error,
                hint: self.hint,
            }),
        )
            .into_response()
    }
}

impl From<RecordError> for ApiError {
    fn from(e: RecordError) -> Self {
        match &e {
            RecordError::UnknownRun(_) => ApiError::new(StatusCode::NOT_FOUND, e.to_string())
                .hint("list known runs with GET /v0/runs"),
            RecordError::IllegalTransition { .. } => {
                ApiError::new(StatusCode::CONFLICT, e.to_string())
            }
            RecordError::Journal(_) => {
                ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
            }
        }
    }
}

async fn auth(
    State(state): State<Arc<AppState>>,
    req: axum::extract::Request,
    next: Next,
) -> Response {
    let expected = format!("Bearer {}", state.bearer);
    let presented = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok());
    if presented != Some(expected.as_str()) {
        return ApiError::new(
            StatusCode::UNAUTHORIZED,
            "missing or wrong bearer (D10: every call authenticates)",
        )
        .hint("read the current bearer from <home>/mc.lock")
        .into_response();
    }
    next.run(req).await
}

pub(crate) fn parse_run_id(raw: &str) -> Result<RunId, ApiError> {
    raw.parse().map_err(|_| {
        ApiError::new(StatusCode::BAD_REQUEST, format!("`{raw}` is not a run id"))
            .hint("run ids are 26-char ULIDs; see `fractality ps`")
    })
}

async fn health(State(state): State<Arc<AppState>>) -> Json<HealthResponse> {
    let (runs_total, runs_open) = state.counts();
    Json(HealthResponse {
        status: "ok".to_owned(),
        version: env!("CARGO_PKG_VERSION").to_owned(),
        pid: std::process::id(),
        started_ts_ms: state.started_ts_ms,
        node_id: state.node.node_id.clone(),
        runs_total,
        runs_open,
    })
}

async fn node(State(state): State<Arc<AppState>>) -> Json<NodeResponse> {
    Json(NodeResponse {
        node: state.node.clone(),
        scopes: state.scopes.clone(),
    })
}

async fn register_run(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RegisterRunRequest>,
) -> Result<(StatusCode, Json<RunRecord>), ApiError> {
    req.packet
        .validate()
        .map_err(|e| ApiError::new(StatusCode::BAD_REQUEST, e.to_string()))?;
    let parent_record = match req.parent {
        None => None,
        Some(parent) => match state.get_run(parent) {
            None => {
                return Err(ApiError::new(
                    StatusCode::BAD_REQUEST,
                    format!("parent run {parent} is unknown"),
                )
                .hint("register the parent first, or drop the parent field"));
            }
            Some(p) if p.state.is_terminal() => {
                return Err(ApiError::new(
                    StatusCode::BAD_REQUEST,
                    format!("parent run {parent} is already {}", p.state),
                )
                .hint("a terminal run cannot adopt children; drop the parent field"));
            }
            Some(p) => Some(p),
        },
    };
    let depth = parent_record.as_ref().map_or(0, |p| p.depth + 1);

    // The depth guard (D-C3-3): a spawn may not nest past the cap for the
    // parent's capability class — refused here at the door, before any pod
    // is provisioned, and independent of the need-gate's advisory
    // fold-at-cap so a caller that bypassed the gate still cannot open an
    // unbounded tree. The compiled-in routing policy is the v1 table (its
    // authored form in delegation-rules mirrors it; a home-relative
    // override is a later slice).
    if req.spawn
        && let Some(parent) = parent_record.as_ref()
    {
        let parent_class = crate::admission::parent_capability_class(&state, &parent.profile);
        if let Err(message) = crate::admission::check_spawn_depth(
            parent_class,
            parent.budget.max_depth,
            &fractality_core::RoutingPolicy::default(),
            depth,
        ) {
            return Err(ApiError::new(StatusCode::BAD_REQUEST, message)
                .hint("route this task, or fold it into the caller, instead of spawning deeper"));
        }
    }

    // Refuse a near-duplicate child (D-C3-4/5): a spawn whose task matches
    // an active sibling's is orchestration collapse (logic in admission).
    if req.spawn
        && let Some(parent) = req.parent
        && let Err(message) = crate::admission::check_not_duplicate(&state, &req.packet, parent)
    {
        return Err(ApiError::new(StatusCode::CONFLICT, message)
            .hint("await or reuse the sibling's result via context_from, or vary the task"));
    }

    // The product path validates at the door (D14/F16: the 400 names the
    // exact fix surface) — admission re-checks cheaply at launch time.
    if req.spawn
        && let Err(message) = crate::admission::preflight(&state, &req.packet)
    {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, message));
    }

    let run_id = RunId::generate();
    let run_dir = state.cfg.runs_root().join(run_id.to_string());
    std::fs::create_dir_all(run_dir.as_std_path()).map_err(|e| {
        ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("creating run dir `{run_dir}`: {e}"),
        )
    })?;
    // The packet lands in the run dir at registration (D4: the run dir is
    // the persistence plane from the first event on).
    let packet_toml = req.packet.to_toml_string().map_err(|e| {
        ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("rendering packet: {e}"),
        )
    })?;
    std::fs::write(run_dir.join("packet.toml").as_std_path(), packet_toml).map_err(|e| {
        ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("writing packet.toml: {e}"),
        )
    })?;

    let now = now_ms();
    let record = RunRecord {
        run_id,
        title: req.packet.task.title.clone(),
        state: RunState::Queued,
        profile: req.packet.routing.profile.clone(),
        model: req.packet.routing.model.clone(),
        workspace_mode: req.packet.workspace.mode,
        parent: req.parent,
        origin_session: req.origin_session,
        depth,
        spawn_requested: req.spawn,
        budget: req.packet.budget,
        node_id: state.node.node_id.clone(),
        run_dir,
        created_ts_ms: now,
        updated_ts_ms: now,
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
    };
    let stored = state.record(Event::Registered {
        run: Box::new(record),
    })?;

    // Attribution note (Campaign 2 D2): best-effort by design — an
    // unknown session id never fails the registration.
    state.note_delegation_best_effort(req.origin_session, stored.run_id);

    // Admission decides whether the run launches now or queues for a
    // slot (Phase 4). Either way registration succeeded — the caller
    // reads the fresh state (queued | starting | failed-at-launch).
    if req.spawn {
        crate::admission::tick(&state);
    }
    let fresh = state.get_run(stored.run_id).unwrap_or(stored);
    Ok((StatusCode::CREATED, Json(fresh)))
}

/// `POST /v0/runs/:id/kill` — the operator kill (D13), recursive over
/// the call tree when asked (Phase 4).
async fn kill_run_endpoint(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<KillRequest>,
) -> Result<Json<KillResponse>, ApiError> {
    let id = parse_run_id(&id)?;
    if state.get_run(id).is_none() {
        return Err(
            ApiError::new(StatusCode::NOT_FOUND, format!("run {id} is not registered"))
                .hint("list known runs with GET /v0/runs"),
        );
    }
    let results = crate::kill::kill(&state, id, KillReason::Manual, req.recursive);
    // Freed slots admit the next queued runs immediately.
    crate::admission::tick(&state);
    Ok(Json(KillResponse { results }))
}

/// `GET /v0/metrics` — the scoreboard aggregates (I3/D16).
async fn metrics(State(state): State<Arc<AppState>>) -> Json<MetricsResponse> {
    Json(crate::metrics::compute(&state.list_runs(None, None)))
}

#[derive(Debug, Deserialize)]
struct RunsQuery {
    state: Option<String>,
    limit: Option<usize>,
}

async fn list_runs(
    State(state): State<Arc<AppState>>,
    Query(q): Query<RunsQuery>,
) -> Result<Json<RunListResponse>, ApiError> {
    let state_filter = match q.state.as_deref() {
        None => None,
        Some(raw) => Some(
            raw.parse::<RunState>()
                .map_err(|e| ApiError::new(StatusCode::BAD_REQUEST, e))?,
        ),
    };
    Ok(Json(RunListResponse {
        runs: state.list_runs(state_filter, q.limit),
    }))
}

async fn get_run(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<RunRecord>, ApiError> {
    let id = parse_run_id(&id)?;
    state.get_run(id).map(Json).ok_or_else(|| {
        ApiError::new(StatusCode::NOT_FOUND, format!("run {id} is not registered"))
            .hint("list known runs with GET /v0/runs")
    })
}

async fn get_tree(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<TreeNode>, ApiError> {
    let id = parse_run_id(&id)?;
    state.tree(id).map(Json).ok_or_else(|| {
        ApiError::new(StatusCode::NOT_FOUND, format!("run {id} is not registered"))
            .hint("list known runs with GET /v0/runs")
    })
}

async fn shutdown(State(state): State<Arc<AppState>>) -> Json<ShutdownResponse> {
    tracing::info!("shutdown requested over the bus");
    let _ = state.shutdown.send_replace(true);
    Json(ShutdownResponse {
        shutting_down: true,
    })
}

async fn pod_register(
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

async fn pod_heartbeat(
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

async fn pod_event(
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
