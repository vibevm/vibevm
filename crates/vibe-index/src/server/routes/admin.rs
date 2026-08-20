specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#root");
use std::sync::Arc;
use std::sync::atomic::Ordering;

use axum::Json;
use axum::extract::State;
use serde::Serialize;

use crate::server::ApiError;
use crate::server::state::AppState;
use crate::wire_count::checked_u32;

#[derive(Serialize)]
pub struct Status {
    pub command: &'static str,
    pub registry: String,
    pub registry_url: String,
    pub generator: String,
    #[serde(with = "crate::types::wire_decimal")]
    pub uptime_seconds: u64,
    pub read_only: bool,
    pub package_count: u32,
    pub version_count: u32,
    #[serde(with = "crate::types::wire_decimal")]
    pub requests_total: u64,
    #[serde(with = "crate::types::wire_decimal")]
    pub mutations_total: u64,
}

/// The counts are the WRITER's — everything the index HOLDS, including
/// quarantined versions (R55.5): telemetry about contents, not an
/// answer about a package — do not "fix" them to `usable_version_count`.
pub async fn status(State(state): State<Arc<AppState>>) -> Result<Json<Status>, ApiError> {
    state.stats.note_request();
    let index = state.index.read().await;
    let package_count = checked_u32("package_count", index.by_pkgref.len())
        .map_err(|error| ApiError::internal(error.to_string()))?;
    let version_count = index
        .by_pkgref
        .values()
        .try_fold(0usize, |count, package| {
            count.checked_add(package.versions.len())
        })
        .ok_or_else(|| {
            ApiError::internal(
                "wire field `version_count` overflowed usize while counting the in-memory catalog \
                 (violates spec://org.vibevm.core/vibevm/common/PROP-044#machinery; \
                 fix: reduce the catalog or widen the field's schema and writer together)",
            )
        })?;
    let version_count = checked_u32("version_count", version_count)
        .map_err(|error| ApiError::internal(error.to_string()))?;
    Ok(Json(Status {
        command: "admin:status",
        registry: index.registry.clone(),
        registry_url: index.registry_url.clone(),
        generator: state.generator.clone(),
        uptime_seconds: state.started_at.elapsed().as_secs(),
        read_only: state.read_only,
        package_count,
        version_count,
        requests_total: state.stats.requests_total.load(Ordering::Relaxed),
        mutations_total: state.stats.mutations_total.load(Ordering::Relaxed),
    }))
}
