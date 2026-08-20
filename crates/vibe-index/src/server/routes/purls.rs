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
use crate::wire_count::checked_u32;

#[derive(Serialize)]
pub struct Response {
    pub command: &'static str,
    pub purl: String,
    pub hit_count: u32,
    pub hits: Vec<Hit>,
    /// Unusable versions that WOULD have matched the requested PURL —
    /// named, not hidden (PROP-044 §4.5). Absent when there are none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unavailable: Vec<Unavailable>,
}

#[derive(Serialize)]
pub struct Hit {
    pub kind: PackageKind,
    pub group: Group,
    pub name: String,
    pub version: Version,
    pub binding_site: &'static str,
}

pub async fn lookup(
    State(state): State<Arc<AppState>>,
    Path(purl): Path<String>,
) -> Result<Json<Response>, ApiError> {
    state.stats.note_request();
    let index = state.index.read().await;
    let purl_norm = purl.trim().to_string();
    let entries = search::lookup_purl(&index, &purl_norm);
    let unavailable = quarantine::refused_where(&index, |v| search::describes_purl(v, &purl_norm));
    let hits = entries
        .iter()
        .map(|e| Hit {
            kind: e.kind.clone(),
            group: e.group.clone(),
            name: e.name.clone(),
            version: e.version.clone(),
            binding_site: if e.describes.as_deref() == Some(purl_norm.as_str()) {
                "package"
            } else {
                "subskill"
            },
        })
        .collect::<Vec<_>>();
    let hit_count = checked_u32("hit_count", hits.len())
        .map_err(|error| ApiError::internal(error.to_string()))?;
    Ok(Json(Response {
        command: "purls",
        purl,
        hit_count,
        hits,
        unavailable,
    }))
}
