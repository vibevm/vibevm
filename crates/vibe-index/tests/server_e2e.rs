//! End-to-end coverage of the read-only HTTP server. Drives every
//! documented route through axum's `oneshot` dispatcher — no actual
//! TCP listener bound — so the tests run hermetically and cheaply.

use std::path::PathBuf;

use axum::body::{Body, to_bytes};
use axum::http::{Method, Request, StatusCode, header};
use chrono::{DateTime, Utc};
use tower::util::ServiceExt;

use vibe_index::index::Index;
use vibe_index::server::{AppState, build_app};
use vibe_index::types::{
    BootSnippetEntry, Group, NamingConvention, PackageEntry, PackageKind, ProvidesEntry,
    VersionEntry,
};

fn now() -> DateTime<Utc> {
    DateTime::parse_from_rfc3339("2026-05-06T12:00:00Z")
        .unwrap()
        .with_timezone(&Utc)
}

fn entry(
    kind: PackageKind,
    name: &str,
    version: &str,
    description: Option<&str>,
    capabilities: &[&str],
    describes: Option<&str>,
) -> VersionEntry {
    VersionEntry {
        schema_version: VersionEntry::SCHEMA_VERSION,
        kind,
        group: Group::parse("org.vibevm").unwrap(),
        name: name.into(),
        version: version.parse().unwrap(),
        content_hash: format!("sha256:{name}{version}"),
        source_url: format!("https://example.invalid/{name}.git"),
        source_ref: format!("v{version}"),
        resolved_commit: Some("abc123".into()),
        registry: "vibespecs".into(),
        workspace_origin: None,
        license: Some("EULA".into()),
        authors: vec!["Tester".into()],
        description: description.map(|s| s.to_string()),
        homepage: None,
        keywords: vec![name.into()],
        describes: describes.map(|s| s.to_string()),
        compatibility: Default::default(),
        provides: ProvidesEntry {
            capabilities: capabilities.iter().map(|s| s.to_string()).collect(),
        },
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

fn populated_state() -> (tempfile::TempDir, AppState) {
    let tmp = tempfile::tempdir().unwrap();
    let mut idx = Index::new(
        "vibespecs",
        "https://example.invalid/vibespecs",
        NamingConvention::Fqdn,
    );
    idx.upsert(entry(
        PackageKind::Flow,
        "wal",
        "0.1.0",
        Some("WAL discipline"),
        &["interface:wal"],
        None,
    ));
    idx.upsert(entry(
        PackageKind::Flow,
        "wal",
        "0.2.0",
        Some("WAL discipline"),
        &["interface:wal"],
        None,
    ));
    idx.upsert(entry(
        PackageKind::Flow,
        "sqlx-skin",
        "0.1.0",
        Some("documentation for the sqlx 0.8.x lineage"),
        &[],
        Some("pkg:cargo/sqlx@0.8.0"),
    ));
    let mut entry_with_subskill = entry(
        PackageKind::Stack,
        "rust",
        "0.1.0",
        Some("rust toolchain"),
        &["interface:rust-stack"],
        None,
    );
    let pkg = idx
        .by_pkgref
        .entry((Group::parse("org.vibevm").unwrap(), "rust".to_string()))
        .or_insert_with(|| PackageEntry::new(Group::parse("org.vibevm").unwrap(), "rust", now()));
    entry_with_subskill.subskills = vec![vibe_index::types::SubskillEntry {
        path: "extras".into(),
        delivery: vibe_index::types::DeliveryMode::Eager,
        describes: Some("pkg:cargo/sqlx@0.8.0".into()),
        description: Some("rust extras".into()),
        channels: vec!["manual".into()],
    }];
    pkg.versions.push(entry_with_subskill);
    pkg.finalise();

    idx.write_to(tmp.path()).unwrap();
    let state = AppState::new(tmp.path().to_path_buf(), true, idx);
    (tmp, state)
}

async fn body_to_string(body: Body) -> String {
    let bytes = to_bytes(body, usize::MAX).await.unwrap();
    String::from_utf8(bytes.to_vec()).unwrap()
}

async fn body_to_json(body: Body) -> serde_json::Value {
    let s = body_to_string(body).await;
    serde_json::from_str(&s).unwrap_or_else(|e| panic!("body not json: {e} :: {s}"))
}

fn req(method: Method, uri: &str) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .body(Body::empty())
        .unwrap()
}

#[tokio::test]
async fn healthz_returns_ok() {
    let (_tmp, state) = populated_state();
    let app = build_app(state);
    let resp = app.oneshot(req(Method::GET, "/healthz")).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_to_json(resp.into_body()).await;
    assert_eq!(body["status"], "ok");
    assert_eq!(body["registry"], "vibespecs");
}

#[tokio::test]
async fn readyz_returns_ready() {
    let (_tmp, state) = populated_state();
    let app = build_app(state);
    let resp = app.oneshot(req(Method::GET, "/readyz")).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_to_json(resp.into_body()).await;
    assert_eq!(body["status"], "ready");
}

#[tokio::test]
async fn repomd_json_served_from_disk() {
    let (_tmp, state) = populated_state();
    let app = build_app(state);
    let resp = app
        .oneshot(req(Method::GET, "/v1/index/repomd.json"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let ct = resp.headers().get(header::CONTENT_TYPE).unwrap();
    assert_eq!(ct, "application/json");
    let body: serde_json::Value = body_to_json(resp.into_body()).await;
    assert_eq!(body["registry"], "vibespecs");
}

#[tokio::test]
async fn primary_jsonl_served_with_ndjson_content_type() {
    let (_tmp, state) = populated_state();
    let app = build_app(state);
    let resp = app
        .oneshot(req(Method::GET, "/v1/index/primary.jsonl"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let ct = resp.headers().get(header::CONTENT_TYPE).unwrap();
    assert_eq!(ct, "application/x-ndjson");
    let body = body_to_string(resp.into_body()).await;
    let lines: Vec<&str> = body.lines().filter(|l| !l.is_empty()).collect();
    assert!(lines.len() >= 4); // 4 versions across 3 packages
}

#[tokio::test]
async fn by_cap_jsonl_serves_inverted_capability_index() {
    let (_tmp, state) = populated_state();
    let app = build_app(state);
    // populated_state has flow:wal + stack:rust each providing
    // `interface:wal`. Slug is fs-safe encoding (`:` → `--`).
    let resp = app
        .oneshot(req(Method::GET, "/v1/index/by-cap/interface--wal.jsonl"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_to_string(resp.into_body()).await;
    assert!(body.contains("\"capability\":\"interface:wal\""));
    assert!(body.contains("\"name\":\"wal\""));
}

#[tokio::test]
async fn by_purl_jsonl_serves_inverted_describes_index() {
    let (_tmp, state) = populated_state();
    let app = build_app(state);
    // populated_state has flow:sqlx-skin describing pkg:cargo/sqlx@0.8.0
    // (package binding) and stack:rust subskill describing same purl.
    let resp = app
        .oneshot(req(
            Method::GET,
            "/v1/index/by-purl/pkg--cargo--sqlx--0.8.0.jsonl",
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_to_string(resp.into_body()).await;
    assert!(body.contains("\"binding_site\":\"package\""));
    assert!(body.contains("\"binding_site\":\"subskill\""));
}

#[tokio::test]
async fn by_cap_route_404s_for_missing_slug() {
    let (_tmp, state) = populated_state();
    let app = build_app(state);
    let resp = app
        .oneshot(req(
            Method::GET,
            "/v1/index/by-cap/definitely-not-a-real-cap.jsonl",
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn primary_jsonl_gz_served_with_gzip_encoding() {
    let (_tmp, state) = populated_state();
    let app = build_app(state);
    let resp = app
        .oneshot(req(Method::GET, "/v1/index/primary.jsonl.gz"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(
        resp.headers().get(header::CONTENT_TYPE).unwrap(),
        "application/x-ndjson"
    );
    assert_eq!(
        resp.headers().get(header::CONTENT_ENCODING).unwrap(),
        "gzip"
    );
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    // The gzip magic number is 0x1f 0x8b.
    assert_eq!(bytes[0], 0x1f);
    assert_eq!(bytes[1], 0x8b);
}

#[tokio::test]
async fn by_name_route_serves_candidate_set_file() {
    let (_tmp, state) = populated_state();
    let app = build_app(state);
    let resp = app
        .oneshot(req(Method::GET, "/v1/index/by-name/wal.json"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = body_to_json(resp.into_body()).await;
    assert_eq!(body["name"], "wal");
    // The by-name file is the candidate set: one package per group,
    // each carrying its versions (PROP-008 §2.8).
    let packages = body["packages"].as_array().unwrap();
    assert_eq!(packages.len(), 1);
    assert_eq!(packages[0]["versions"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn by_name_route_404s_for_missing() {
    let (_tmp, state) = populated_state();
    let app = build_app(state);
    let resp = app
        .oneshot(req(Method::GET, "/v1/index/by-name/nope.json"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn packages_list_returns_sorted_envelope() {
    let (_tmp, state) = populated_state();
    let app = build_app(state);
    let resp = app
        .oneshot(req(Method::GET, "/v1/packages?limit=10"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = body_to_json(resp.into_body()).await;
    assert_eq!(body["command"], "list");
    assert_eq!(body["package_count"], 3);
}

#[tokio::test]
async fn packages_search_via_query_param() {
    let (_tmp, state) = populated_state();
    let app = build_app(state);
    let resp = app
        .oneshot(req(Method::GET, "/v1/packages?q=rust"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = body_to_json(resp.into_body()).await;
    assert_eq!(body["command"], "search");
    let hits = body["hits"].as_array().unwrap();
    assert!(hits.iter().any(|h| h["name"] == "rust"));
}

#[tokio::test]
async fn package_versions_returns_full_entries() {
    let (_tmp, state) = populated_state();
    let app = build_app(state);
    let resp = app
        .oneshot(req(Method::GET, "/v1/packages/org.vibevm/wal"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = body_to_json(resp.into_body()).await;
    assert_eq!(body["name"], "wal");
    assert_eq!(body["versions"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn single_version_returns_entry() {
    let (_tmp, state) = populated_state();
    let app = build_app(state);
    let resp = app
        .oneshot(req(Method::GET, "/v1/packages/org.vibevm/wal/0.2.0"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = body_to_json(resp.into_body()).await;
    assert_eq!(body["version"], "0.2.0");
    assert_eq!(body["content_hash"], "sha256:wal0.2.0");
}

#[tokio::test]
async fn single_version_404_for_missing_version() {
    let (_tmp, state) = populated_state();
    let app = build_app(state);
    let resp = app
        .oneshot(req(Method::GET, "/v1/packages/org.vibevm/wal/9.9.9"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn capabilities_route_lists_advertisers() {
    let (_tmp, state) = populated_state();
    let app = build_app(state);
    let resp = app
        .oneshot(req(Method::GET, "/v1/capabilities/interface:wal"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = body_to_json(resp.into_body()).await;
    assert!(body["hit_count"].as_u64().unwrap() >= 1);
}

#[tokio::test]
async fn purls_route_lists_describing_packages() {
    let (_tmp, state) = populated_state();
    let app = build_app(state);
    let resp = app
        .oneshot(req(Method::GET, "/v1/purls/pkg:cargo%2Fsqlx@0.8.0"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = body_to_json(resp.into_body()).await;
    // Both flow:sqlx-skin (package-level describes) and stack:rust
    // (subskill-level describes) match.
    assert_eq!(body["hit_count"], 2);
}

#[tokio::test]
async fn admin_status_returns_counts() {
    let (_tmp, state) = populated_state();
    let app = build_app(state);
    let resp = app
        .oneshot(req(Method::GET, "/v1/admin/status"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = body_to_json(resp.into_body()).await;
    assert_eq!(body["registry"], "vibespecs");
    assert_eq!(body["package_count"], 3);
    assert_eq!(body["version_count"], 4);
    assert_eq!(body["read_only"], true);
}

#[tokio::test]
async fn metrics_route_emits_prometheus_lines() {
    let (_tmp, state) = populated_state();
    let app = build_app(state);
    let resp = app.oneshot(req(Method::GET, "/metrics")).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_to_string(resp.into_body()).await;
    assert!(body.contains("vibe_index_packages_total 3"));
    assert!(body.contains("vibe_index_versions_total 4"));
    assert!(body.contains("vibe_index_read_only 1"));
}

#[tokio::test]
async fn unknown_group_in_url_yields_not_found() {
    // A syntactically valid but unindexed group resolves to no package.
    let (_tmp, state) = populated_state();
    let app = build_app(state);
    let resp = app
        .oneshot(req(Method::GET, "/v1/packages/com.absent/wal"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

fn _silence_unused_pathbuf(_: PathBuf) {}
