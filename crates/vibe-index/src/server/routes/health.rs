use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use serde::Serialize;

use crate::server::state::AppState;

#[derive(Serialize)]
pub struct Health {
    pub status: &'static str,
    pub registry: String,
}

pub async fn healthz(State(state): State<Arc<AppState>>) -> Json<Health> {
    state.stats.note_request();
    let registry = state.index.read().await.registry.clone();
    Json(Health {
        status: "ok",
        registry,
    })
}

pub async fn readyz(State(state): State<Arc<AppState>>) -> Json<Health> {
    state.stats.note_request();
    let registry = state.index.read().await.registry.clone();
    Json(Health {
        status: "ready",
        registry,
    })
}
