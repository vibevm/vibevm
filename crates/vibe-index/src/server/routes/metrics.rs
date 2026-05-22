use std::sync::Arc;

use axum::extract::State;
use axum::http::header;
use axum::response::{IntoResponse, Response};

use crate::server::metrics as renderer;
use crate::server::state::AppState;

pub async fn prometheus(State(state): State<Arc<AppState>>) -> Response {
    state.stats.note_request();
    let index = state.index.read().await;
    let body = renderer::render(
        &state,
        index.package_count() as u64,
        index.version_count() as u64,
    );
    let mut resp = body.into_response();
    resp.headers_mut().insert(
        header::CONTENT_TYPE,
        "text/plain; version=0.0.4".parse().unwrap(),
    );
    resp
}
