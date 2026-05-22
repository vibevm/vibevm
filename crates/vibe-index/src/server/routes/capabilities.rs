use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, State};
use semver::Version;
use serde::Serialize;

use crate::index::search;
use crate::server::error::ApiError;
use crate::server::state::AppState;
use crate::types::PackageKind;

#[derive(Serialize)]
pub struct Response {
    pub command: &'static str,
    pub capability: String,
    pub hit_count: usize,
    pub hits: Vec<Hit>,
}

#[derive(Serialize)]
pub struct Hit {
    pub kind: PackageKind,
    pub name: String,
    pub version: Version,
    pub capability_advertised: Option<String>,
}

pub async fn lookup(
    State(state): State<Arc<AppState>>,
    Path(capability): Path<String>,
) -> Result<Json<Response>, ApiError> {
    state.stats.note_request();
    let index = state.index.read().await;
    let entries = search::lookup_capability(&index, &capability);
    let hits = entries
        .iter()
        .map(|e| Hit {
            kind: e.kind,
            name: e.name.clone(),
            version: e.version.clone(),
            capability_advertised: e
                .provides
                .capabilities
                .iter()
                .find(|c: &&String| {
                    c.starts_with(&capability) || capability.starts_with(c.as_str())
                })
                .cloned(),
        })
        .collect::<Vec<_>>();
    Ok(Json(Response {
        command: "capabilities",
        capability,
        hit_count: hits.len(),
        hits,
    }))
}
