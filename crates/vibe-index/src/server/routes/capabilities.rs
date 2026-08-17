specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#root");
use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, State};
use semver::Version;
use serde::Serialize;
use vibe_core::Group;

use crate::index::quarantine::{self, Unavailable};
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
    /// Unusable versions that WOULD have matched the requested
    /// capability — named, not hidden (PROP-044 §4.5). Absent when
    /// there are none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unavailable: Vec<Unavailable>,
}

#[derive(Serialize)]
pub struct Hit {
    pub kind: PackageKind,
    pub group: Group,
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
    let cap_norm = capability.trim().to_string();
    let unavailable =
        quarantine::refused_where(&index, |v| search::provides_capability(v, &cap_norm));
    let hits = entries
        .iter()
        .map(|e| Hit {
            kind: e.kind.clone(),
            group: e.group.clone(),
            name: e.name.clone(),
            version: e.version.clone(),
            capability_advertised: e.provides.as_ref().and_then(|p| {
                p.capabilities
                    .iter()
                    .find(|c: &&String| {
                        c.starts_with(&capability) || capability.starts_with(c.as_str())
                    })
                    .cloned()
            }),
        })
        .collect::<Vec<_>>();
    Ok(Json(Response {
        command: "capabilities",
        capability,
        hit_count: hits.len(),
        hits,
        unavailable,
    }))
}
