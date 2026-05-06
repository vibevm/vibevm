//! HTTP server — boots an axum app over the in-RAM [`Index`] and
//! exposes the read-only routes documented in PROP-005 §2.10.
//! Slice 5 lands the read surface; slice 6 adds the write surface
//! (POST/DELETE) and bearer-token auth.

pub mod auth;
pub mod error;
pub mod lock;
pub mod metrics;
pub mod routes;
pub mod state;

pub use auth::TokenStore;
pub use error::ApiError;
pub use lock::ServerLock;
pub use state::AppState;

use std::sync::Arc;

use axum::Router;
use axum::routing::get;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

/// Build the axum router for read-only mode (slice 5). Slice 6
/// extends the same builder with mutating routes behind a bearer-token
/// guard.
pub fn build_app(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        // Liveness / readiness.
        .route("/healthz", get(routes::health::healthz))
        .route("/readyz", get(routes::health::readyz))
        // Static index files (raw — same shape as on disk).
        .route(
            "/v1/index/repomd.json",
            get(routes::index_files::repomd_json),
        )
        .route(
            "/v1/index/primary.jsonl",
            get(routes::index_files::primary_jsonl),
        )
        .route(
            "/v1/index/primary.jsonl.gz",
            get(routes::index_files::primary_jsonl_gz),
        )
        .route(
            "/v1/index/by-name/{kind}/{name}",
            get(routes::index_files::by_name_json),
        )
        // Structured query.
        .route(
            "/v1/packages",
            get(routes::packages::list_or_search).post(routes::packages::upsert),
        )
        .route(
            "/v1/packages/{kind}/{name}",
            get(routes::packages::package_versions).delete(routes::packages::delete_package),
        )
        .route(
            "/v1/packages/{kind}/{name}/{version}",
            get(routes::packages::single_version).delete(routes::packages::delete_version),
        )
        .route(
            "/v1/capabilities/{capability}",
            get(routes::capabilities::lookup),
        )
        .route("/v1/purls/{purl}", get(routes::purls::lookup))
        // Admin (read-only in slice 5; reindex POST lands in slice 6).
        .route("/v1/admin/status", get(routes::admin::status))
        // Observability.
        .route("/metrics", get(routes::metrics::prometheus))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(Arc::new(state))
}
