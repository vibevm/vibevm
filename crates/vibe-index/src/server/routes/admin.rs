use std::sync::Arc;
use std::sync::atomic::Ordering;

use axum::Json;
use axum::extract::State;
use serde::Serialize;

use crate::server::state::AppState;

#[derive(Serialize)]
pub struct Status {
    pub command: &'static str,
    pub registry: String,
    pub registry_url: String,
    pub generator: String,
    pub uptime_seconds: u64,
    pub read_only: bool,
    pub package_count: u32,
    pub version_count: u32,
    pub requests_total: u64,
    pub mutations_total: u64,
}

pub async fn status(State(state): State<Arc<AppState>>) -> Json<Status> {
    state.stats.note_request();
    let index = state.index.read().await;
    Json(Status {
        command: "admin:status",
        registry: index.registry.clone(),
        registry_url: index.registry_url.clone(),
        generator: state.generator.clone(),
        uptime_seconds: state.started_at.elapsed().as_secs(),
        read_only: state.read_only,
        package_count: index.package_count(),
        version_count: index.version_count(),
        requests_total: state.stats.requests_total.load(Ordering::Relaxed),
        mutations_total: state.stats.mutations_total.load(Ordering::Relaxed),
    })
}
