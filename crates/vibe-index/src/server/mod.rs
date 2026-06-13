//! HTTP server — boots an axum app over the in-RAM [`Index`] and
//! exposes the read-only routes documented in PROP-005 §2.10.
//! Slice 5 lands the read surface; slice 6 adds the write surface
//! (POST/DELETE) and bearer-token auth.

specmark::scope!("spec://vibevm/modules/vibe-index/PROP-005#http");

pub mod auth;
pub mod error;
pub mod metrics;
pub mod rate_limit;
pub mod routes;
pub mod state;

pub use auth::TokenStore;
pub use error::ApiError;
pub use rate_limit::{RateDecision, RateLimitConfig, RateLimitKey, RateLimiter};
pub use state::AppState;

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::extract::{ConnectInfo, Request, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use specmark::spec;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

/// Build the axum router for read-only mode (slice 5). Slice 6
/// extends the same builder with mutating routes behind a bearer-token
/// guard. Rate-limit middleware runs first when the configured
/// quotas are non-zero (slice 23, PROP-005 §9 Q10).
#[spec(
    implements = "spec://vibevm/modules/vibe-index/PROP-005#server-mode",
    r = 1
)]
pub fn build_app(state: AppState) -> Router {
    let state = Arc::new(state);
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let rate_limit_layer =
        middleware::from_fn_with_state(Arc::clone(&state), rate_limit_middleware);

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
            "/v1/index/by-name/{name}",
            get(routes::index_files::by_name_json),
        )
        .route(
            "/v1/index/by-cap/{slug}",
            get(routes::index_files::by_cap_jsonl),
        )
        .route(
            "/v1/index/by-purl/{slug}",
            get(routes::index_files::by_purl_jsonl),
        )
        // Structured query.
        .route(
            "/v1/packages",
            get(routes::packages::list_or_search).post(routes::packages::upsert),
        )
        .route(
            "/v1/packages/{group}/{name}",
            get(routes::packages::package_versions).delete(routes::packages::delete_package),
        )
        .route(
            "/v1/packages/{group}/{name}/{version}",
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
        .layer(rate_limit_layer)
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

/// Rate-limit middleware. PROP-005 §9 Q10. Routes
/// `/healthz` / `/readyz` / `/metrics` are exempt; everything else
/// consults the appropriate token-bucket. When no quota is
/// configured (disabled), the middleware is a no-op fast path.
async fn rate_limit_middleware(
    State(state): State<Arc<AppState>>,
    req: Request<Body>,
    next: Next,
) -> Response {
    let cfg = state.rate_limiter.config();
    if !cfg.enabled() {
        return next.run(req).await;
    }
    let path = req.uri().path();
    if matches!(path, "/healthz" | "/readyz" | "/metrics") {
        return next.run(req).await;
    }

    // Bearer present → per-token bucket; otherwise per-IP.
    let bearer = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(|s| s.to_string());

    let decision = if let Some(token) = &bearer {
        state
            .rate_limiter
            .check(rate_limit::RateLimitKey::Token(token))
    } else {
        let ip = req
            .extensions()
            .get::<ConnectInfo<SocketAddr>>()
            .map(|ci| ci.0.ip())
            .unwrap_or(IpAddr::V4(Ipv4Addr::LOCALHOST));
        state.rate_limiter.check(rate_limit::RateLimitKey::Ip(ip))
    };

    if !decision.allowed {
        return rate_limited_response(&decision);
    }

    let mut resp = next.run(req).await;
    decorate_with_rate_headers(resp.headers_mut(), &decision);
    resp
}

fn rate_limited_response(decision: &rate_limit::RateDecision) -> Response {
    let body = serde_json::json!({
        "type":   "vibe-index/error/rate-limited",
        "title":  "rate limit exceeded",
        "status": StatusCode::TOO_MANY_REQUESTS.as_u16(),
        "detail": format!(
            "this client's request rate has exceeded its bucket of {} per minute; \
             retry in ~{:.1} second(s)",
            decision.limit, decision.retry_after_seconds
        ),
    });
    let mut resp = (StatusCode::TOO_MANY_REQUESTS, axum::Json(body)).into_response();
    decorate_with_rate_headers(resp.headers_mut(), decision);
    let retry_after = decision.retry_after_seconds.ceil().max(1.0) as u64;
    if let Ok(v) = HeaderValue::from_str(&retry_after.to_string()) {
        resp.headers_mut().insert(header::RETRY_AFTER, v);
    }
    resp
}

fn decorate_with_rate_headers(headers: &mut HeaderMap, decision: &rate_limit::RateDecision) {
    if decision.limit == 0 {
        return;
    }
    if let Ok(v) = HeaderValue::from_str(&decision.limit.to_string()) {
        headers.insert("x-ratelimit-limit", v);
    }
    if let Ok(v) = HeaderValue::from_str(&decision.remaining.to_string()) {
        headers.insert("x-ratelimit-remaining", v);
    }
}
