//! HTTP write-surface coverage — POST /v1/packages, DELETE routes,
//! bearer-token auth.

use axum::body::{Body, to_bytes};
use axum::http::{Method, Request, StatusCode, header};
use chrono::{DateTime, Utc};
use tower::util::ServiceExt;

use vibe_index::index::Index;
use vibe_index::server::{AppState, TokenStore, build_app};
use vibe_index::types::{
    BootSnippetEntry, NamingConvention, PackageKind, ProvidesEntry, VersionEntry,
};

fn now() -> DateTime<Utc> {
    DateTime::parse_from_rfc3339("2026-05-06T12:00:00Z")
        .unwrap()
        .with_timezone(&Utc)
}

fn entry(kind: PackageKind, name: &str, version: &str) -> VersionEntry {
    VersionEntry {
        schema_version: VersionEntry::SCHEMA_VERSION,
        kind,
        name: name.into(),
        version: version.parse().unwrap(),
        content_hash: format!("sha256:{name}{version}"),
        source_url: format!("https://example.invalid/{name}.git"),
        source_ref: format!("v{version}"),
        resolved_commit: None,
        registry: "vibespecs".into(),
        license: Some("EULA".into()),
        authors: vec![],
        description: Some(format!("{name} package")),
        homepage: None,
        keywords: vec![name.into()],
        describes: None,
        compatibility: Default::default(),
        provides: ProvidesEntry::default(),
        requires: Default::default(),
        requires_any: vec![],
        obsoletes: Default::default(),
        conflicts: Default::default(),
        features: Default::default(),
        subskills: vec![],
        i18n: Default::default(),
        boot_snippet: Some(BootSnippetEntry {
            source: format!("boot/{name}.md"),
            category: None,
        }),
        files_count: 1,
        indexed_at: now(),
        indexed_by: "vibe-index 0.1.0-dev".into(),
    }
}

fn fresh_state(read_only: bool, with_token: Option<&str>) -> (tempfile::TempDir, AppState) {
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
    // Rebuild the index from disk so AppState owns a fresh copy.
    let idx2 = Index::load_from(tmp.path()).unwrap();
    let state = AppState::with_tokens(tmp.path().to_path_buf(), read_only, idx2, tokens);
    (tmp, state)
}

async fn body_to_json(body: Body) -> serde_json::Value {
    let bytes = to_bytes(body, usize::MAX).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

fn req_post_json(uri: &str, token: Option<&str>, body: serde_json::Value) -> Request<Body> {
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

fn req_delete(uri: &str, token: Option<&str>) -> Request<Body> {
    let mut b = Request::builder().method(Method::DELETE).uri(uri);
    if let Some(t) = token {
        b = b.header(header::AUTHORIZATION, format!("Bearer {t}"));
    }
    b.body(Body::empty()).unwrap()
}

#[tokio::test]
async fn post_packages_inserts_entry() {
    let (_tmp, state) = fresh_state(false, Some("topsecret"));
    let app = build_app(state);
    let payload = serde_json::to_value(entry(PackageKind::Flow, "wal", "0.1.0")).unwrap();
    let resp = app
        .oneshot(req_post_json("/v1/packages", Some("topsecret"), payload))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = body_to_json(resp.into_body()).await;
    assert_eq!(body["created"], true);
    assert_eq!(body["name"], "wal");
}

#[tokio::test]
async fn post_packages_upsert_returns_200_for_existing_version() {
    let (_tmp, state) = fresh_state(false, Some("topsecret"));
    let app = build_app(state);
    let payload = serde_json::to_value(entry(PackageKind::Flow, "wal", "0.1.0")).unwrap();
    let resp1 = app
        .clone()
        .oneshot(req_post_json(
            "/v1/packages",
            Some("topsecret"),
            payload.clone(),
        ))
        .await
        .unwrap();
    assert_eq!(resp1.status(), StatusCode::CREATED);
    let resp2 = app
        .oneshot(req_post_json("/v1/packages", Some("topsecret"), payload))
        .await
        .unwrap();
    assert_eq!(resp2.status(), StatusCode::OK);
    let body = body_to_json(resp2.into_body()).await;
    assert_eq!(body["created"], false);
}

#[tokio::test]
async fn post_without_token_is_401() {
    let (_tmp, state) = fresh_state(false, Some("topsecret"));
    let app = build_app(state);
    let payload = serde_json::to_value(entry(PackageKind::Flow, "wal", "0.1.0")).unwrap();
    let resp = app
        .oneshot(req_post_json("/v1/packages", None, payload))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn post_with_wrong_token_is_401() {
    let (_tmp, state) = fresh_state(false, Some("topsecret"));
    let app = build_app(state);
    let payload = serde_json::to_value(entry(PackageKind::Flow, "wal", "0.1.0")).unwrap();
    let resp = app
        .oneshot(req_post_json(
            "/v1/packages",
            Some("not-the-token"),
            payload,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn post_when_read_only_is_403_even_with_token() {
    let (_tmp, state) = fresh_state(true, Some("topsecret"));
    let app = build_app(state);
    let payload = serde_json::to_value(entry(PackageKind::Flow, "wal", "0.1.0")).unwrap();
    let resp = app
        .oneshot(req_post_json("/v1/packages", Some("topsecret"), payload))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn post_when_no_tokens_loaded_is_403() {
    let (_tmp, state) = fresh_state(false, None);
    let app = build_app(state);
    let payload = serde_json::to_value(entry(PackageKind::Flow, "wal", "0.1.0")).unwrap();
    let resp = app
        .oneshot(req_post_json("/v1/packages", Some("anything"), payload))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn post_with_mismatched_registry_is_400() {
    let (_tmp, state) = fresh_state(false, Some("topsecret"));
    let app = build_app(state);
    let mut e = entry(PackageKind::Flow, "wal", "0.1.0");
    e.registry = "wrong-registry".into();
    let payload = serde_json::to_value(e).unwrap();
    let resp = app
        .oneshot(req_post_json("/v1/packages", Some("topsecret"), payload))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn delete_version_removes_existing() {
    let (_tmp, state) = fresh_state(false, Some("topsecret"));
    let app = build_app(state);
    let payload = serde_json::to_value(entry(PackageKind::Flow, "wal", "0.1.0")).unwrap();
    app.clone()
        .oneshot(req_post_json("/v1/packages", Some("topsecret"), payload))
        .await
        .unwrap();
    let resp = app
        .oneshot(req_delete("/v1/packages/flow/wal/0.1.0", Some("topsecret")))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_to_json(resp.into_body()).await;
    assert_eq!(body["removed"], true);
}

#[tokio::test]
async fn delete_version_unauthenticated_is_401() {
    let (_tmp, state) = fresh_state(false, Some("topsecret"));
    let app = build_app(state);
    let resp = app
        .oneshot(req_delete("/v1/packages/flow/wal/0.1.0", None))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn delete_package_drops_all_versions() {
    let (_tmp, state) = fresh_state(false, Some("topsecret"));
    let app = build_app(state);
    let v1 = serde_json::to_value(entry(PackageKind::Flow, "wal", "0.1.0")).unwrap();
    let v2 = serde_json::to_value(entry(PackageKind::Flow, "wal", "0.2.0")).unwrap();
    app.clone()
        .oneshot(req_post_json("/v1/packages", Some("topsecret"), v1))
        .await
        .unwrap();
    app.clone()
        .oneshot(req_post_json("/v1/packages", Some("topsecret"), v2))
        .await
        .unwrap();
    let resp = app
        .oneshot(req_delete("/v1/packages/flow/wal", Some("topsecret")))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_to_json(resp.into_body()).await;
    assert_eq!(body["removed"], true);
    assert_eq!(body["version"], serde_json::Value::Null);
}

#[tokio::test]
async fn delete_missing_returns_removed_false() {
    let (_tmp, state) = fresh_state(false, Some("topsecret"));
    let app = build_app(state);
    let resp = app
        .oneshot(req_delete(
            "/v1/packages/flow/ghost-package",
            Some("topsecret"),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_to_json(resp.into_body()).await;
    assert_eq!(body["removed"], false);
}
