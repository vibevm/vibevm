//! The `/v0/decisions` leg of the bus (D-C3-8): the need-gate decision
//! log. Split from `http.rs` along the cell budget; the router there is
//! the single registration point.

use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use fractality_core::api::{Ack, DecisionListResponse};

use crate::http::ApiError;
use crate::state::AppState;

specmark::scope!("spec://fractality/PROP-001#architecture");

/// `POST /v0/decisions` — append a need-gate decision to the decisions
/// journal (D-C3-8). Best-effort telemetry: the boss's `gate --record`
/// posts here; a write failure is a 500 the caller may ignore.
pub(crate) async fn post_decision(
    State(state): State<Arc<AppState>>,
    Json(record): Json<fractality_core::DecisionRecord>,
) -> Result<Json<Ack>, ApiError> {
    state.record_decision(record).map_err(|e| {
        ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, e)
            .hint("ensure the journal directory is writable and has space")
    })?;
    Ok(Json(Ack { ok: true }))
}

/// `GET /v0/decisions` — the decision log, oldest first (the soft-label
/// table's raw rows, D-C3-8).
pub(crate) async fn list_decisions(
    State(state): State<Arc<AppState>>,
) -> Json<DecisionListResponse> {
    Json(DecisionListResponse {
        decisions: state.decisions(),
    })
}
