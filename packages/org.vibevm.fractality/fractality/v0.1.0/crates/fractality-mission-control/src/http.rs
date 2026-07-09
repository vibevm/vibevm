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
    Ack, ErrorResponse, HealthResponse, NodeResponse, PodEvent, PodEventRequest, PodHeartbeat,
    PodHeartbeatResponse, PodRegisterRequest, PodRegisterResponse, RegisterRunRequest,
    RunListResponse, ShutdownResponse, TreeNode,
};
use fractality_core::ids::{PodId, RunId};
use fractality_core::journal::Event;
use fractality_core::run::{RunRecord, RunState, UsageTotals};
use fractality_core::time::now_ms;
use serde::Deserialize;

use crate::state::{AppState, PodRuntime, RecordError};

specmark::scope!("spec://fractality/PROP-001#architecture");

const HEARTBEAT_INTERVAL_MS: u64 = 2_000;

/// Builds the versioned router with the bearer gate in front.
pub fn router(state: Arc<AppState>) -> Router {
    let api = Router::new()
        .route("/health", get(health))
        .route("/node", get(node))
        .route("/runs", post(register_run).get(list_runs))
        .route("/runs/{id}", get(get_run))
        .route("/runs/{id}/tree", get(get_tree))
        .route("/shutdown", post(shutdown))
        .route("/pods/register", post(pod_register))
        .route("/pods/{id}/heartbeat", post(pod_heartbeat))
        .route("/pods/{id}/event", post(pod_event))
        .layer(from_fn_with_state(state.clone(), auth))
        .with_state(state);
    Router::new().nest(fractality_core::api::API_PREFIX, api)
}

/// Uniform API error: status + message + fix-surface hint.
struct ApiError {
    status: StatusCode,
    error: String,
    hint: Option<String>,
}

impl ApiError {
    fn new(status: StatusCode, error: impl Into<String>) -> Self {
        Self {
            status,
            error: error.into(),
            hint: None,
        }
    }

    fn hint(mut self, hint: impl Into<String>) -> Self {
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

fn parse_run_id(raw: &str) -> Result<RunId, ApiError> {
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
    if let Some(parent) = req.parent
        && state.get_run(parent).is_none()
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            format!("parent run {parent} is unknown"),
        )
        .hint("register the parent first, or drop the parent field"));
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
        node_id: state.node.node_id.clone(),
        run_dir,
        created_ts_ms: now,
        updated_ts_ms: now,
        pod: None,
        worker_pid: None,
        exit_code: None,
        failure: None,
        kill_reason: None,
        usage: UsageTotals::default(),
    };
    let stored = state.record(Event::Registered {
        run: Box::new(record),
    })?;
    Ok((StatusCode::CREATED, Json(stored)))
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
    Ok(Json(PodHeartbeatResponse {
        command: fractality_core::api::PodCommand::None,
    }))
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
    match req.event {
        PodEvent::Spawned { worker_pid } => {
            state.record(Event::Spawned { run_id, worker_pid })?;
            // A fast worker can exit before anyone observes `running`;
            // the transition is still Starting -> Running -> …, asserted
            // here where the spawn fact arrives.
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
        PodEvent::State { state: s, detail } => {
            state.record(Event::State {
                run_id,
                state: s,
                detail,
            })?;
        }
        PodEvent::Usage { usage } => {
            state.record(Event::Usage { run_id, usage })?;
        }
        PodEvent::Exit { exit_code } => {
            state.record(Event::Completed { run_id, exit_code })?;
        }
        PodEvent::Fault { message, terminal } => {
            state.record(Event::Error {
                run_id,
                message,
                terminal,
            })?;
        }
    }
    Ok(Json(Ack { ok: true }))
}
