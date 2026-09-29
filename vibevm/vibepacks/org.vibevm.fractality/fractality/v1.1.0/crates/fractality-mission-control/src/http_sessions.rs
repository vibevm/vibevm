//! The session leg of the bus (Campaign 2 D2/D3): begin/resume, the
//! initiative-facts channel, end, and the session-scoped scoreboard
//! reads. Split from `http.rs` along the cell budget; the router there
//! is the single registration point.

use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use fractality_core::api::{
    SessionBeginRequest, SessionBeginResponse, SessionEventRequest, SessionListResponse,
    SessionMetricsResponse,
};
use fractality_core::ids::SessionId;
use serde::Deserialize;

use crate::http::ApiError;
use crate::sessions::SessionError;
use crate::state::AppState;

specmark::scope!("spec://fractality/PROP-001#sessions");

impl From<SessionError> for ApiError {
    fn from(e: SessionError) -> Self {
        match &e {
            SessionError::UnknownSession(_) => ApiError::new(StatusCode::NOT_FOUND, e.to_string())
                .hint("list known sessions with GET /v0/sessions"),
            SessionError::Journal(_) => {
                ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
            }
        }
    }
}

fn parse_session_id(raw: &str) -> Result<SessionId, ApiError> {
    raw.parse().map_err(|_| {
        ApiError::new(
            StatusCode::BAD_REQUEST,
            format!("`{raw}` is not a session id"),
        )
        .hint("session ids are 26-char ULIDs; see GET /v0/sessions")
    })
}

/// `POST /v0/sessions` — begin (or resume) a boss session (D2/D3).
pub(crate) async fn session_begin(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SessionBeginRequest>,
) -> Result<(StatusCode, Json<SessionBeginResponse>), ApiError> {
    if req.harness.trim().is_empty() || req.external_id.trim().is_empty() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "harness and external_id must be non-empty",
        )
        .hint("the adapter passes its harness label and the harness's session id"));
    }
    let (session, resumed) = state.session_begin(&req.harness, &req.external_id, req.cwd)?;
    let status = if resumed {
        StatusCode::OK
    } else {
        StatusCode::CREATED
    };
    Ok((status, Json(SessionBeginResponse { session, resumed })))
}

#[derive(Debug, Deserialize)]
pub(crate) struct SessionsQuery {
    #[serde(default)]
    open: bool,
}

/// `GET /v0/sessions[?open=true]` — newest last (D17).
pub(crate) async fn session_list(
    State(state): State<Arc<AppState>>,
    Query(q): Query<SessionsQuery>,
) -> Json<SessionListResponse> {
    Json(SessionListResponse {
        sessions: state.list_sessions(q.open),
    })
}

pub(crate) async fn session_get(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<fractality_core::SessionRecord>, ApiError> {
    let id = parse_session_id(&id)?;
    state.get_session(id).map(Json).ok_or_else(|| {
        ApiError::new(StatusCode::NOT_FOUND, format!("session {id} is unknown"))
            .hint("list known sessions with GET /v0/sessions")
    })
}

/// `POST /v0/sessions/:id/events` — record one initiative fact.
pub(crate) async fn session_event(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<SessionEventRequest>,
) -> Result<Json<fractality_core::SessionRecord>, ApiError> {
    let id = parse_session_id(&id)?;
    Ok(Json(state.session_note(id, req.note)?))
}

/// `POST /v0/sessions/:id/end` — the harness reported the session end.
pub(crate) async fn session_end(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<fractality_core::SessionRecord>, ApiError> {
    let id = parse_session_id(&id)?;
    Ok(Json(state.session_end(id)?))
}

/// `GET /v0/sessions/:id/metrics` — the session-scoped scoreboard
/// facts (record + runs bucket + parked questions).
pub(crate) async fn session_metrics(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<SessionMetricsResponse>, ApiError> {
    let id = parse_session_id(&id)?;
    let (session, runs, parked) = state.session_metrics(id).ok_or_else(|| {
        ApiError::new(StatusCode::NOT_FOUND, format!("session {id} is unknown"))
            .hint("list known sessions with GET /v0/sessions")
    })?;
    Ok(Json(SessionMetricsResponse {
        session,
        runs,
        parked,
    }))
}
