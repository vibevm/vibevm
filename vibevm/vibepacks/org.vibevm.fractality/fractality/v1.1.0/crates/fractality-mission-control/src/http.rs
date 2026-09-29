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
    ErrorResponse, HealthResponse, KillRequest, KillResponse, MetricsResponse, NodeResponse,
    RegisterRunRequest, RunListResponse, ShutdownResponse, TreeNode,
};
use fractality_core::ids::RunId;
use fractality_core::journal::Event;
use fractality_core::run::{KillReason, RunRecord, RunState, UsageTotals};
use fractality_core::time::now_ms;
use serde::Deserialize;

use crate::http_sessions as hs;
use crate::state::{AppState, RecordError};

specmark::scope!("spec://fractality/PROP-001#architecture");

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
        .route(
            "/runs/{id}/escalate",
            post(crate::http_escalate::post_escalate),
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
        .route("/pods/register", post(crate::http_pods::pod_register))
        .route(
            "/pods/{id}/heartbeat",
            post(crate::http_pods::pod_heartbeat),
        )
        .route("/pods/{id}/event", post(crate::http_pods::pod_event))
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
    // Cold-verifier suppression (Ф5, FD-9): an acceptance/verifier packet
    // must have real work to check — refused at the door when its
    // `context_from` names no run that produced a result.
    if let Err(message) = crate::admission::check_verifier_has_work(&state, &req.packet) {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, message).hint(
            "run the work first, then verify its results via context_from; \
             a verifier over an empty tree is refused",
        ));
    }
    // The advisor bar (PP-003, D-C3-7): an advice call is refused when its
    // caller (parent) is below the `advisor_enabled` capability bar (RD-10:
    // advice makes a weak caller worse).
    if let Err(message) = crate::admission::check_advisor_caller_class(
        &state,
        &req.packet,
        req.parent,
        &fractality_core::RoutingPolicy::default(),
    ) {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, message)
            .hint("only a caller of capability class >= medium may consult an advisor"));
    }
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

    // Sibling invariants (D-C3-4/5): no near-duplicate, one merge node max.
    if req.spawn
        && let Some(parent) = req.parent
        && let Err(message) =
            crate::admission::check_sibling_invariants(&state, &req.packet, parent)
    {
        return Err(ApiError::new(StatusCode::CONFLICT, message)
            .hint("vary the task, or keep at most one output.merge child per parent"));
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
        verifier: req.packet.output.verifier,
        advice: req.packet.output.advice,
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
        escalation: None,
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
