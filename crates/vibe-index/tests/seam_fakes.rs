//! Seam oracles — drive the HTTP surface through *injected* fakes for
//! the two swappable server dependencies. The e2e suites exercise the
//! production variants (a real `admin.tokens` file, the token-bucket
//! limiter); these prove the seams themselves: `AppState::with_seams`
//! accepts any `TokenStore` / `RateLimiter`, and a handler honours
//! whichever was injected — with no file and no clock in sight.

use axum::body::Body;
use axum::http::{Method, Request, StatusCode, header};
use tower::util::ServiceExt;

use vibe_index::index::Index;
use vibe_index::server::{
    AppState, RateDecision, RateLimitConfig, RateLimitKey, RateLimiter, TokenBucketRateLimiter,
    TokenStore, build_app,
};
use vibe_index::types::{Group, NamingConvention, PackageKind, VersionEntry};

/// A [`TokenStore`] with no file behind it — every check returns the
/// configured verdict. `has_any` is always true so `require_writeable`
/// proceeds to the token check rather than short-circuiting on the
/// "no tokens configured" path.
#[derive(Debug)]
struct FakeTokenStore {
    verdict: bool,
}

impl TokenStore for FakeTokenStore {
    fn has_any(&self) -> bool {
        true
    }
    fn check(&self, _supplied: &str) -> bool {
        self.verdict
    }
}

/// A [`RateLimiter`] that rejects every request — an enabled quota with
/// no bucket math, so the middleware's deny path is exercised directly.
#[derive(Debug)]
struct AlwaysDenyRateLimiter {
    config: RateLimitConfig,
}

impl AlwaysDenyRateLimiter {
    fn new() -> Self {
        AlwaysDenyRateLimiter {
            // Non-zero rpm → `enabled()` is true → the middleware
            // consults us instead of taking the disabled fast path.
            config: RateLimitConfig {
                per_token_rpm: 1,
                per_ip_rpm: 1,
                max_buckets: 1,
            },
        }
    }
}

impl RateLimiter for AlwaysDenyRateLimiter {
    fn config(&self) -> &RateLimitConfig {
        &self.config
    }
    fn check(&self, _key: RateLimitKey<'_>) -> RateDecision {
        RateDecision {
            allowed: false,
            limit: 1,
            remaining: 0,
            retry_after_seconds: 1.0,
        }
    }
}

/// A fresh server over an empty `vibespecs` index with both seams
/// injected directly. No `admin.tokens` file is ever written — auth is
/// whatever `tokens` decides.
fn state_with_seams(
    tokens: Box<dyn TokenStore>,
    rate_limiter: Box<dyn RateLimiter>,
) -> (tempfile::TempDir, AppState) {
    let tmp = tempfile::tempdir().unwrap();
    let idx = Index::new(
        "vibespecs",
        "https://example.invalid/vibespecs",
        NamingConvention::Fqdn,
    );
    idx.write_to(tmp.path()).unwrap();
    let idx2 = Index::load_from(tmp.path()).unwrap();
    let state = AppState::with_seams(tmp.path().to_path_buf(), false, idx2, tokens, rate_limiter);
    (tmp, state)
}

/// A disabled production limiter — the natural "rate limiting off"
/// object for the auth oracles, which are not about quotas.
fn no_rate_limit() -> Box<dyn RateLimiter> {
    Box::new(TokenBucketRateLimiter::new(RateLimitConfig::disabled()))
}

/// A minimal entry whose `registry` matches the server, so it clears the
/// upsert scope check and reaches the auth-gated insert.
fn sample_payload() -> serde_json::Value {
    let mut e = VersionEntry::minimal(
        PackageKind::Flow,
        Group::parse("org.vibevm").unwrap(),
        "wal",
        "0.1.0".parse().unwrap(),
    );
    e.registry = "vibespecs".into();
    serde_json::to_value(e).unwrap()
}

fn post(uri: &str, token: Option<&str>, body: serde_json::Value) -> Request<Body> {
    let mut b = Request::builder()
        .method(Method::POST)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json");
    if let Some(t) = token {
        b = b.header(header::AUTHORIZATION, format!("Bearer {t}"));
    }
    b.body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap()
}

#[tokio::test]
#[specmark::verifies("spec://vibevm/modules/vibe-index/PROP-005#server-mode", r = 1)]
async fn write_is_authorised_by_the_injected_token_store() {
    // The always-accept store authorises the write although no
    // admin.tokens file exists anywhere — proof the auth decision flows
    // through the seam, not a file the handler reads behind its back.
    let (_tmp, state) =
        state_with_seams(Box::new(FakeTokenStore { verdict: true }), no_rate_limit());
    let app = build_app(state);
    let resp = app
        .oneshot(post("/v1/packages", Some("any-token"), sample_payload()))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
}

#[tokio::test]
#[specmark::verifies("spec://vibevm/modules/vibe-index/PROP-005#server-mode", r = 1)]
async fn write_is_refused_by_the_injected_token_store() {
    // Same request, same valid-looking Bearer token — but the injected
    // store rejects, so the handler 401s. The verdict is the seam's.
    let (_tmp, state) =
        state_with_seams(Box::new(FakeTokenStore { verdict: false }), no_rate_limit());
    let app = build_app(state);
    let resp = app
        .oneshot(post("/v1/packages", Some("any-token"), sample_payload()))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
#[specmark::verifies("spec://vibevm/modules/vibe-index/PROP-005#http", r = 1)]
async fn the_injected_rate_limiter_blocks_through_the_middleware() {
    // A read needs no auth, so a 429 here can only be the injected
    // limiter's deny verdict reaching the response through the
    // middleware seam — no bucket ever filled or drained.
    let (_tmp, state) = state_with_seams(
        Box::new(FakeTokenStore { verdict: false }),
        Box::new(AlwaysDenyRateLimiter::new()),
    );
    let app = build_app(state);
    let req = Request::builder()
        .method(Method::GET)
        .uri("/v1/packages")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
}
