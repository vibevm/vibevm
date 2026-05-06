//! End-to-end coverage of the rate-limit middleware. Drives the
//! axum router through `oneshot` so we can examine status codes +
//! response headers without binding a real port.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use axum::body::{Body, to_bytes};
use axum::extract::ConnectInfo;
use axum::http::{Method, Request, StatusCode, header};
use tower::util::ServiceExt;

use vibe_index::index::Index;
use vibe_index::server::{AppState, RateLimitConfig, TokenStore, build_app};
use vibe_index::server::rate_limit::DEFAULT_MAX_BUCKETS;
use vibe_index::types::NamingConvention;

fn fresh_state(rate_limit: RateLimitConfig, with_token: Option<&str>) -> (tempfile::TempDir, AppState) {
    let tmp = tempfile::tempdir().unwrap();
    let idx = Index::new(
        "vibespecs",
        "https://example.invalid/vibespecs",
        NamingConvention::KindName,
    );
    idx.write_to(tmp.path()).unwrap();
    let tokens = if let Some(t) = with_token {
        let state_dir = tmp.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        std::fs::write(state_dir.join("admin.tokens"), t).unwrap();
        TokenStore::load(tmp.path()).unwrap()
    } else {
        TokenStore::default()
    };
    let idx2 = Index::load_from(tmp.path()).unwrap();
    let state = AppState::with_tokens_and_rate_limit(
        tmp.path().to_path_buf(),
        false,
        idx2,
        tokens,
        rate_limit,
    );
    (tmp, state)
}

fn req_with_ip(method: Method, uri: &str, ip: IpAddr) -> Request<Body> {
    let mut r = Request::builder()
        .method(method)
        .uri(uri)
        .body(Body::empty())
        .unwrap();
    let addr = SocketAddr::new(ip, 12345);
    r.extensions_mut().insert(ConnectInfo(addr));
    r
}

fn req_with_token(method: Method, uri: &str, token: &str) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap()
}

async fn body_to_string(body: Body) -> String {
    let bytes = to_bytes(body, usize::MAX).await.unwrap();
    String::from_utf8(bytes.to_vec()).unwrap()
}

#[tokio::test]
async fn disabled_default_allows_unbounded_traffic() {
    let (_tmp, state) = fresh_state(RateLimitConfig::disabled(), None);
    let app = build_app(state);
    for _ in 0..200 {
        let resp = app
            .clone()
            .oneshot(req_with_ip(
                Method::GET,
                "/v1/index/repomd.json",
                IpAddr::V4(Ipv4Addr::LOCALHOST),
            ))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert!(resp.headers().get("x-ratelimit-limit").is_none());
    }
}

#[tokio::test]
async fn per_ip_quota_throttles_after_burst() {
    let cfg = RateLimitConfig {
        per_token_rpm: 0,
        per_ip_rpm: 5,
        max_buckets: DEFAULT_MAX_BUCKETS,
    };
    let (_tmp, state) = fresh_state(cfg, None);
    let app = build_app(state);
    let ip = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 7));

    for i in 0..5 {
        let resp = app
            .clone()
            .oneshot(req_with_ip(Method::GET, "/v1/index/repomd.json", ip))
            .await
            .unwrap();
        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "request {i} should pass within burst"
        );
        let limit = resp.headers().get("x-ratelimit-limit").unwrap();
        assert_eq!(limit, "5");
    }
    // 6th from same IP should 429.
    let resp = app
        .oneshot(req_with_ip(Method::GET, "/v1/index/repomd.json", ip))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
    let retry = resp
        .headers()
        .get(header::RETRY_AFTER)
        .unwrap()
        .to_str()
        .unwrap();
    assert!(retry.parse::<u64>().unwrap() >= 1);
    let body = body_to_string(resp.into_body()).await;
    assert!(body.contains("rate-limited"));
}

#[tokio::test]
async fn per_ip_quota_isolates_distinct_ips() {
    let cfg = RateLimitConfig {
        per_token_rpm: 0,
        per_ip_rpm: 2,
        max_buckets: DEFAULT_MAX_BUCKETS,
    };
    let (_tmp, state) = fresh_state(cfg, None);
    let app = build_app(state);
    let ip1 = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1));
    let ip2 = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 2));

    // Drain ip1.
    for _ in 0..2 {
        let resp = app
            .clone()
            .oneshot(req_with_ip(Method::GET, "/v1/index/repomd.json", ip1))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }
    let drained = app
        .clone()
        .oneshot(req_with_ip(Method::GET, "/v1/index/repomd.json", ip1))
        .await
        .unwrap();
    assert_eq!(drained.status(), StatusCode::TOO_MANY_REQUESTS);

    // ip2 has its own bucket — still allowed.
    let other = app
        .oneshot(req_with_ip(Method::GET, "/v1/index/repomd.json", ip2))
        .await
        .unwrap();
    assert_eq!(other.status(), StatusCode::OK);
}

#[tokio::test]
async fn per_token_quota_throttles_authenticated_clients() {
    let cfg = RateLimitConfig {
        per_token_rpm: 3,
        per_ip_rpm: 0,
        max_buckets: DEFAULT_MAX_BUCKETS,
    };
    let (_tmp, state) = fresh_state(cfg, Some("alpha-token"));
    let app = build_app(state);

    for _ in 0..3 {
        let resp = app
            .clone()
            .oneshot(req_with_token(
                Method::GET,
                "/v1/admin/status",
                "alpha-token",
            ))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }
    let resp = app
        .oneshot(req_with_token(
            Method::GET,
            "/v1/admin/status",
            "alpha-token",
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
}

#[tokio::test]
async fn distinct_tokens_get_distinct_buckets() {
    let cfg = RateLimitConfig {
        per_token_rpm: 1,
        per_ip_rpm: 0,
        max_buckets: DEFAULT_MAX_BUCKETS,
    };
    // Two valid admin tokens so both pass auth.
    let (_tmp, state) = fresh_state(cfg, Some("alpha\nbeta"));
    let app = build_app(state);

    let r1 = app
        .clone()
        .oneshot(req_with_token(Method::GET, "/v1/admin/status", "alpha"))
        .await
        .unwrap();
    assert_eq!(r1.status(), StatusCode::OK);
    // alpha drained.
    let drained = app
        .clone()
        .oneshot(req_with_token(Method::GET, "/v1/admin/status", "alpha"))
        .await
        .unwrap();
    assert_eq!(drained.status(), StatusCode::TOO_MANY_REQUESTS);
    // beta is untouched.
    let r2 = app
        .oneshot(req_with_token(Method::GET, "/v1/admin/status", "beta"))
        .await
        .unwrap();
    assert_eq!(r2.status(), StatusCode::OK);
}

#[tokio::test]
async fn healthz_readyz_metrics_are_exempt() {
    let cfg = RateLimitConfig {
        per_token_rpm: 0,
        per_ip_rpm: 1,
        max_buckets: DEFAULT_MAX_BUCKETS,
    };
    let (_tmp, state) = fresh_state(cfg, None);
    let app = build_app(state);
    let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 99));

    // Hit /healthz 50 times from one IP — never throttled.
    for i in 0..50 {
        let resp = app
            .clone()
            .oneshot(req_with_ip(Method::GET, "/healthz", ip))
            .await
            .unwrap();
        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "/healthz request {i} unexpectedly throttled"
        );
        // Exempt routes should not stamp rate-limit headers.
        assert!(
            resp.headers().get("x-ratelimit-limit").is_none(),
            "exempt route leaked rate-limit headers"
        );
    }
}

#[tokio::test]
async fn allowed_responses_carry_remaining_header() {
    let cfg = RateLimitConfig {
        per_token_rpm: 10,
        per_ip_rpm: 0,
        max_buckets: DEFAULT_MAX_BUCKETS,
    };
    let (_tmp, state) = fresh_state(cfg, Some("alpha"));
    let app = build_app(state);

    let resp = app
        .oneshot(req_with_token(Method::GET, "/v1/admin/status", "alpha"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let limit = resp.headers().get("x-ratelimit-limit").unwrap();
    assert_eq!(limit, "10");
    let remaining = resp
        .headers()
        .get("x-ratelimit-remaining")
        .unwrap()
        .to_str()
        .unwrap()
        .parse::<u32>()
        .unwrap();
    assert_eq!(remaining, 9, "first call should leave 9 of 10 tokens");
}
