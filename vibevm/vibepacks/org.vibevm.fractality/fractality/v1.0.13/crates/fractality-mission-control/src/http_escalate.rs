//! The D-C3-6 escalate leg of the bus: a worker (via its broker) hands its
//! whole task UP the tree. Terminal — the run ends `escalated` and the
//! record climbs the `parent` edges to the human at the top; there is no
//! resume (the ascent generalization of the D18 question leg in
//! `http_questions`). Split from `http.rs`; the router there registers it.

use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use fractality_core::journal::Event;
use fractality_core::run::RunRecord;

use crate::http::{ApiError, parse_run_id};
use crate::state::AppState;

specmark::scope!("spec://fractality/PROP-001#architecture");

/// `POST /v0/runs/:id/escalate` — the worker hands its task up (D-C3-6):
/// `running | waiting_on_boss -> escalated`, a terminal outcome (a wrong
/// state answers 409 via the record validator). The reason/needs persist
/// on the plane (escalation.md, I2: the bus carries it, the file records
/// it) and ride the record so a parent can act as the escalation climbs.
pub(crate) async fn post_escalate(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<fractality_core::api::EscalateRequest>,
) -> Result<Json<RunRecord>, ApiError> {
    let id = parse_run_id(&id)?;
    if req.reason.trim().is_empty() {
        return Err(
            ApiError::new(StatusCode::BAD_REQUEST, "an escalation needs a reason")
                .hint("state why the task cannot be finished here and what would unblock it"),
        );
    }
    let stored = state.record(Event::Escalated {
        run_id: id,
        reason: req.reason.clone(),
        needs: req.needs.clone(),
    })?;
    let path = stored.run_dir.join("escalation.md");
    let body = format!(
        "# escalation\n\n## reason\n\n{}\n\n## needs\n\n{}\n",
        req.reason, req.needs
    );
    if let Err(e) = std::fs::write(path.as_std_path(), &body) {
        tracing::warn!(%path, error = %e, "escalation.md write failed (bus record stands)");
    }
    Ok(Json(stored))
}
