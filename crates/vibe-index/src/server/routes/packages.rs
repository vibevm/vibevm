//! Structured query routes for `/v1/packages*`.
//!
//! A package is addressed by its `(group, name)` identity (PROP-008
//! §2.2): the path shape is `/v1/packages/{group}/{name}[/{version}]`.
//! `kind` is metadata and never keys a route.

specmark::scope!("spec://vibevm/modules/vibe-registry/PROP-008#index-ext");

use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode, header};
use semver::Version;
use serde::{Deserialize, Serialize};
use vibe_core::Group;

use crate::index::search;
use crate::server::error::ApiError;
use crate::server::state::AppState;
use crate::types::{PackageKind, VersionEntry};

#[derive(Debug, Deserialize, Default)]
pub struct ListSearchQuery {
    pub kind: Option<PackageKind>,
    pub q: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct ListResponse {
    pub command: &'static str,
    pub registry: String,
    pub package_count: u32,
    pub returned: usize,
    pub offset: usize,
    pub limit: usize,
    pub packages: Vec<PackageRow>,
}

#[derive(Debug, Serialize)]
pub struct PackageRow {
    /// `kind` is metadata (PROP-008 §2.3) — read from the package's
    /// versions; `None` only for a zero-version package row.
    pub kind: Option<PackageKind>,
    pub group: Group,
    pub name: String,
    pub latest_stable: Option<Version>,
    pub versions: Vec<Version>,
    pub description: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SearchResponse {
    pub command: &'static str,
    pub query: String,
    pub hit_count: usize,
    pub hits: Vec<SearchHit>,
}

#[derive(Debug, Serialize)]
pub struct SearchHit {
    pub kind: PackageKind,
    pub group: Group,
    pub name: String,
    pub latest_stable: Option<Version>,
    pub score: u32,
    pub matched_tokens: Vec<String>,
    pub description: Option<String>,
}

/// Parse a `{group}` path segment into a validated [`Group`]. A
/// malformed segment is a 400 — the URL itself is syntactically wrong.
fn parse_group(s: &str) -> Result<Group, ApiError> {
    Group::parse(s).map_err(|e| ApiError::bad_request(format!("invalid group `{s}`: {e}")))
}

/// The package's `kind` — metadata carried per version (PROP-008 §2.3).
/// `None` only for the rare zero-version package row.
fn package_kind(pkg: &crate::types::PackageEntry) -> Option<PackageKind> {
    pkg.versions.first().map(|v| v.kind)
}

pub async fn list_or_search(
    State(state): State<Arc<AppState>>,
    Query(q): Query<ListSearchQuery>,
) -> Result<axum::response::Response, ApiError> {
    state.stats.note_request();
    let limit = q.limit.unwrap_or(50);
    let offset = q.offset.unwrap_or(0);
    let index = state.index.read().await;

    if let Some(query) = &q.q {
        let hits = search::search(&index, query, q.kind);
        let hits: Vec<SearchHit> = hits
            .into_iter()
            .skip(offset)
            .take(limit)
            .map(|h| SearchHit {
                kind: h.kind,
                group: h.group,
                name: h.name,
                latest_stable: h.latest_stable,
                score: h.score,
                matched_tokens: h.matched_tokens,
                description: h.description,
            })
            .collect();
        let body = SearchResponse {
            command: "search",
            query: query.clone(),
            hit_count: hits.len(),
            hits,
        };
        return Ok(Json(body).into_response());
    }

    // List mode. `kind` is per-version metadata, so the `?kind=` filter
    // keeps a package iff any of its versions carries that kind.
    let mut rows: Vec<PackageRow> = index
        .by_pkgref
        .values()
        .filter(|p| {
            q.kind
                .is_none_or(|k| p.versions.iter().any(|v| v.kind == k))
        })
        .map(|p| PackageRow {
            kind: package_kind(p),
            group: p.group.clone(),
            name: p.name.clone(),
            latest_stable: p.latest_stable.clone(),
            versions: p.versions.iter().map(|v| v.version.clone()).collect(),
            description: p.versions.last().and_then(|v| v.description.clone()),
        })
        .collect();
    rows.sort_by(|a, b| a.group.cmp(&b.group).then(a.name.cmp(&b.name)));
    let package_count = rows.len() as u32;
    let returned: Vec<PackageRow> = rows.into_iter().skip(offset).take(limit).collect();
    let body = ListResponse {
        command: "list",
        registry: index.registry.clone(),
        package_count,
        returned: returned.len(),
        offset,
        limit,
        packages: returned,
    };
    Ok(Json(body).into_response())
}

pub async fn package_versions(
    State(state): State<Arc<AppState>>,
    Path((group_str, name)): Path<(String, String)>,
) -> Result<Json<PackageVersionsResponse>, ApiError> {
    state.stats.note_request();
    let group = parse_group(&group_str)?;
    let index = state.index.read().await;
    let pkg = index
        .get(&group, &name)
        .ok_or_else(|| ApiError::not_found(format!("`{group}/{name}` is not in the index")))?;
    Ok(Json(PackageVersionsResponse {
        command: "package",
        kind: package_kind(pkg),
        group: pkg.group.clone(),
        name: pkg.name.clone(),
        latest_stable: pkg.latest_stable.clone(),
        versions: pkg.versions.clone(),
    }))
}

#[derive(Serialize)]
pub struct PackageVersionsResponse {
    pub command: &'static str,
    pub kind: Option<PackageKind>,
    pub group: Group,
    pub name: String,
    pub latest_stable: Option<Version>,
    pub versions: Vec<VersionEntry>,
}

pub async fn single_version(
    State(state): State<Arc<AppState>>,
    Path((group_str, name, version_str)): Path<(String, String, String)>,
) -> Result<Json<VersionEntry>, ApiError> {
    state.stats.note_request();
    let group = parse_group(&group_str)?;
    let v: Version = version_str
        .parse()
        .map_err(|e| ApiError::bad_request(format!("`{version_str}` is not valid semver: {e}")))?;
    let index = state.index.read().await;
    let pkg = index
        .get(&group, &name)
        .ok_or_else(|| ApiError::not_found(format!("`{group}/{name}` is not in the index")))?;
    let entry = pkg
        .versions
        .iter()
        .find(|e| e.version == v)
        .ok_or_else(|| {
            ApiError::not_found(format!(
                "`{group}/{name}@{version_str}` is not in the index"
            ))
        })?
        .clone();
    Ok(Json(entry))
}

use axum::response::IntoResponse;

// ---------------------------------------------------------------------------
// Mutating endpoints (slice 6)
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct UpsertResponse {
    pub command: &'static str,
    pub kind: PackageKind,
    pub group: Group,
    pub name: String,
    pub version: Version,
    pub created: bool,
}

pub async fn upsert(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(entry): Json<VersionEntry>,
) -> Result<(StatusCode, Json<UpsertResponse>), ApiError> {
    require_writeable(&state, &headers)?;
    if entry.registry != state.index.read().await.registry {
        return Err(ApiError::bad_request(format!(
            "scope violation: entry.registry=`{}` differs from server registry=`{}`",
            entry.registry,
            state.index.read().await.registry
        )));
    }
    let kind = entry.kind;
    let group = entry.group.clone();
    let name = entry.name.clone();
    let version = entry.version.clone();

    let created = {
        let mut idx = state.index.write().await;
        let existed = idx
            .get(&group, &name)
            .map(|p| p.versions.iter().any(|v| v.version == version))
            .unwrap_or(false);
        idx.upsert(entry);
        idx.write_to(&state.data_dir)
            .map_err(|e| ApiError::internal(format!("could not persist index: {e}")))?;
        !existed
    };

    state.stats.note_mutation();
    let status = if created {
        StatusCode::CREATED
    } else {
        StatusCode::OK
    };
    Ok((
        status,
        Json(UpsertResponse {
            command: "upsert",
            kind,
            group,
            name,
            version,
            created,
        }),
    ))
}

#[derive(Debug, Serialize)]
pub struct DeleteResponse {
    pub command: &'static str,
    pub group: Group,
    pub name: String,
    pub version: Option<Version>,
    pub removed: bool,
}

pub async fn delete_version(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((group_str, name, version_str)): Path<(String, String, String)>,
) -> Result<Json<DeleteResponse>, ApiError> {
    require_writeable(&state, &headers)?;
    let group = parse_group(&group_str)?;
    let v: Version = version_str
        .parse()
        .map_err(|e| ApiError::bad_request(format!("`{version_str}` is not valid semver: {e}")))?;
    let removed = {
        let mut idx = state.index.write().await;
        let r = idx.remove_version(&group, &name, &v);
        if r {
            idx.write_to(&state.data_dir)
                .map_err(|e| ApiError::internal(format!("could not persist index: {e}")))?;
        }
        r
    };
    if removed {
        state.stats.note_mutation();
    }
    Ok(Json(DeleteResponse {
        command: "delete",
        group,
        name,
        version: Some(v),
        removed,
    }))
}

pub async fn delete_package(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((group_str, name)): Path<(String, String)>,
) -> Result<Json<DeleteResponse>, ApiError> {
    require_writeable(&state, &headers)?;
    let group = parse_group(&group_str)?;
    let removed = {
        let mut idx = state.index.write().await;
        let r = idx.remove_package(&group, &name);
        if r {
            idx.write_to(&state.data_dir)
                .map_err(|e| ApiError::internal(format!("could not persist index: {e}")))?;
        }
        r
    };
    if removed {
        state.stats.note_mutation();
    }
    Ok(Json(DeleteResponse {
        command: "delete",
        group,
        name,
        version: None,
        removed,
    }))
}

fn require_writeable(state: &AppState, headers: &HeaderMap) -> Result<(), ApiError> {
    if state.read_only {
        return Err(ApiError::forbidden("server is running in --read-only mode"));
    }
    if !state.tokens.has_any() {
        return Err(ApiError::forbidden(
            "server has no admin tokens configured (--auth-tokens-file required for writes)",
        ));
    }
    let supplied = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));
    let Some(token) = supplied else {
        return Err(ApiError::unauthorized());
    };
    if !state.tokens.check(token) {
        return Err(ApiError::unauthorized());
    }
    Ok(())
}
