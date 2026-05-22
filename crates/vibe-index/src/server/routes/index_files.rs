//! GET handlers that serve the on-disk index files verbatim. The
//! server is the single writer; the on-disk shape stays consistent
//! with the in-RAM state at every batch update.

use std::sync::Arc;

use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};

use crate::server::error::ApiError;
use crate::server::state::AppState;

pub async fn repomd_json(State(state): State<Arc<AppState>>) -> Result<Response, ApiError> {
    state.stats.note_request();
    serve_file(&state.data_dir.join("repomd.json"), "application/json").await
}

pub async fn primary_jsonl(State(state): State<Arc<AppState>>) -> Result<Response, ApiError> {
    state.stats.note_request();
    serve_file(
        &state.data_dir.join("primary.jsonl"),
        "application/x-ndjson",
    )
    .await
}

pub async fn primary_jsonl_gz(State(state): State<Arc<AppState>>) -> Result<Response, ApiError> {
    state.stats.note_request();
    let path = state.data_dir.join("primary.jsonl.gz");
    let bytes = match tokio::fs::read(&path).await {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(ApiError::not_found(format!(
                "`{}` is not present in this index",
                path.display()
            )));
        }
        Err(e) => {
            return Err(ApiError::internal(format!(
                "could not read `{}`: {e}",
                path.display()
            )));
        }
    };
    let mut resp = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/x-ndjson")
        .header(header::CONTENT_ENCODING, "gzip")
        .body(Body::from(bytes))
        .map_err(|e| ApiError::internal(format!("response build: {e}")))?;
    resp.headers_mut()
        .insert(header::CACHE_CONTROL, "no-cache".parse().unwrap());
    Ok(resp.into_response())
}

pub async fn by_cap_jsonl(
    State(state): State<Arc<AppState>>,
    Path(slug_with_ext): Path<String>,
) -> Result<Response, ApiError> {
    state.stats.note_request();
    let slug = slug_with_ext.strip_suffix(".jsonl").ok_or_else(|| {
        ApiError::not_found(format!(
            "expected `<slug>.jsonl` path segment, got `{slug_with_ext}`"
        ))
    })?;
    let path = state.data_dir.join("by-cap").join(format!("{slug}.jsonl"));
    serve_file(&path, "application/x-ndjson").await
}

pub async fn by_purl_jsonl(
    State(state): State<Arc<AppState>>,
    Path(slug_with_ext): Path<String>,
) -> Result<Response, ApiError> {
    state.stats.note_request();
    let slug = slug_with_ext.strip_suffix(".jsonl").ok_or_else(|| {
        ApiError::not_found(format!(
            "expected `<slug>.jsonl` path segment, got `{slug_with_ext}`"
        ))
    })?;
    let path = state.data_dir.join("by-purl").join(format!("{slug}.jsonl"));
    serve_file(&path, "application/x-ndjson").await
}

pub async fn by_name_json(
    State(state): State<Arc<AppState>>,
    Path(name_with_ext): Path<String>,
) -> Result<Response, ApiError> {
    state.stats.note_request();
    let name = name_with_ext.strip_suffix(".json").ok_or_else(|| {
        ApiError::not_found(format!(
            "expected `<name>.json` path segment, got `{name_with_ext}`"
        ))
    })?;
    let path = state.data_dir.join("by-name").join(format!("{name}.json"));
    serve_file(&path, "application/json").await
}

async fn serve_file(path: &std::path::Path, content_type: &str) -> Result<Response, ApiError> {
    let bytes = match tokio::fs::read(path).await {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(ApiError::not_found(format!(
                "`{}` is not present in this index",
                path.display()
            )));
        }
        Err(e) => {
            return Err(ApiError::internal(format!(
                "could not read `{}`: {e}",
                path.display()
            )));
        }
    };
    let mut resp = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .body(Body::from(bytes))
        .map_err(|e| ApiError::internal(format!("response build: {e}")))?;
    resp.headers_mut()
        .insert(header::CACHE_CONTROL, "no-cache".parse().unwrap());
    Ok(resp.into_response())
}
