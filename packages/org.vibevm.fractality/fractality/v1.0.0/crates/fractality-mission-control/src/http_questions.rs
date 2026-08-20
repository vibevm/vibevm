//! The D18 question/answer leg of the bus: a worker (via its broker)
//! parks on a question; the boss — or a Ф5 profile rule — answers and
//! the run resumes. Split from `http.rs` along the cell budget; the
//! router there is the single registration point.

use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use fractality_core::journal::Event;
use fractality_core::run::RunRecord;

use crate::http::{ApiError, parse_run_id};
use crate::state::AppState;

specmark::scope!("spec://fractality/PROP-001#architecture");

/// `POST /v0/runs/:id/question` — the worker parks on a question
/// (D18): `running -> waiting_on_boss`, question.md persists the text
/// on the plane (I2: the bus carries it, the file records it).
pub(crate) async fn post_question(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<fractality_core::api::QuestionRequest>,
) -> Result<Json<RunRecord>, ApiError> {
    let id = parse_run_id(&id)?;
    if req.question.trim().is_empty() {
        return Err(
            ApiError::new(StatusCode::BAD_REQUEST, "an empty question cannot be asked")
                .hint("send a precise, answerable question"),
        );
    }
    let stored = state.record(Event::Question {
        run_id: id,
        question: req.question.clone(),
    })?;
    let path = stored.run_dir.join("question.md");
    if let Err(e) = std::fs::write(path.as_std_path(), &req.question) {
        tracing::warn!(%path, error = %e, "question.md write failed (bus record stands)");
    }
    // Ф5 (the D18 layer-2 slice): consult the profile's auto-answer
    // rules. A hit answers immediately — the run passes through
    // waiting_on_boss in one journaled breath (both facts on record),
    // and the broker's first poll already sees the reply. Best-effort:
    // unreadable profiles simply mean "no rules" (the escalation parks
    // for the boss exactly as before).
    if let Ok(profiles) = fractality_core::profile::ProfilesFile::load(&state.cfg.home)
        && let Ok(profile) = profiles.get(&stored.profile)
        && let Some(rule) = profile.permissions.auto_answer(&req.question)
    {
        tracing::info!(run = %id, rule = %rule.name, "question auto-answered by profile rule");
        let answered = state.record(Event::Answer {
            run_id: id,
            answer: rule.answer.clone(),
            auto_rule: Some(rule.name.clone()),
        })?;
        let path = answered.run_dir.join("answer.md");
        if let Err(e) = std::fs::write(path.as_std_path(), &rule.answer) {
            tracing::warn!(%path, error = %e, "answer.md write failed (bus record stands)");
        }
        return Ok(Json(answered));
    }
    Ok(Json(stored))
}

/// `POST /v0/runs/:id/answer` — the boss answers; the run resumes and
/// the broker returns the text as the worker's tool result.
pub(crate) async fn post_answer(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<fractality_core::api::AnswerRequest>,
) -> Result<Json<RunRecord>, ApiError> {
    let id = parse_run_id(&id)?;
    let stored = state.record(Event::Answer {
        run_id: id,
        answer: req.answer.clone(),
        auto_rule: None,
    })?;
    let path = stored.run_dir.join("answer.md");
    if let Err(e) = std::fs::write(path.as_std_path(), &req.answer) {
        tracing::warn!(%path, error = %e, "answer.md write failed (bus record stands)");
    }
    Ok(Json(stored))
}
