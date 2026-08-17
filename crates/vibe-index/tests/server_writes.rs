//! HTTP write-surface coverage — POST /v1/packages, DELETE routes,
//! bearer-token auth.

use axum::body::{Body, to_bytes};
use axum::http::{Method, Request, StatusCode, header};
use chrono::{DateTime, Utc};
use specmark::verifies;
use tower::util::ServiceExt;

use vibe_index::cli::serve;
use vibe_index::index::Index;
use vibe_index::index::memory::{WriteCtx, default_generator};
use vibe_index::journal::{Event, JournalRecord, append, default_dir, project, replay};
use vibe_index::server::{AppState, FileTokenStore, build_app};
use vibe_index::types::{BootSnippetEntry, Group, NamingConvention, PackageKind, VersionEntry};

fn now() -> DateTime<Utc> {
    DateTime::parse_from_rfc3339("2026-05-06T12:00:00Z")
        .unwrap()
        .with_timezone(&Utc)
}

fn entry(kind: PackageKind, name: &str, version: &str) -> VersionEntry {
    VersionEntry {
        schema_version: VersionEntry::SCHEMA_VERSION,
        kind,
        group: Group::parse("org.vibevm").unwrap(),
        name: name.into(),
        version: version.parse().unwrap(),
        content_hash: format!("sha256:{name}{version}"),
        source_url: format!("https://example.invalid/{name}.git"),
        source_ref: format!("v{version}"),
        resolved_commit: None,
        registry: "vibespecs".into(),
        workspace_origin: None,
        license: Some("EULA".into()),
        authors: vec![],
        description: Some(format!("{name} package")),
        homepage: None,
        keywords: vec![name.into()],
        describes: None,
        compatibility: Default::default(),
        provides: None,
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
        must_understand: vec![],
        yanked: false,
        frozen: false,
        indexed_at: now(),
        indexed_by: "vibe-index 0.1.0-dev".into(),
    }
}

/// Seed a data-dir the way the server's own boot reads it (Ф3.2c2): a
/// journal carrying `Initialised` and nothing else. The in-memory
/// state is that journal's projection — the catalog is never an input
/// anywhere in the server, so the seed does not write one. The
/// resulting AppState is byte-for-byte what it was under the old
/// catalog seed (an empty `vibespecs` index), which is why no test's
/// assertions move.
fn fresh_state(read_only: bool, with_token: Option<&str>) -> (tempfile::TempDir, AppState) {
    let tmp = tempfile::tempdir().unwrap();
    let tokens = if let Some(t) = with_token {
        let state_dir = tmp.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        std::fs::write(state_dir.join("admin.tokens"), t).unwrap();
        FileTokenStore::load(tmp.path()).unwrap()
    } else {
        FileTokenStore::default()
    };
    seed_initialised(tmp.path());
    let idx = project(replay(&default_dir(tmp.path())).unwrap()).unwrap();
    let state = AppState::with_tokens(tmp.path().to_path_buf(), read_only, idx, tokens);
    (tmp, state)
}

/// Append the one record an `init`-ed data-dir's journal starts with.
fn seed_initialised(data_dir: &std::path::Path) {
    append(
        &default_dir(data_dir),
        &JournalRecord {
            at: now(),
            actor: default_generator(),
            event: Event::Initialised {
                registry: "vibespecs".into(),
                registry_url: "https://example.invalid/vibespecs".into(),
                naming: NamingConvention::Fqdn,
            },
        },
    )
    .unwrap();
}

/// The data-dir's journal, replayed — the truth the catalog projects
/// from, read the way only a test may read it (through the store).
fn journal_of(data_dir: &std::path::Path) -> Vec<JournalRecord> {
    replay(&default_dir(data_dir)).unwrap()
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

fn req_get(uri: &str) -> Request<Body> {
    Request::builder()
        .method(Method::GET)
        .uri(uri)
        .body(Body::empty())
        .unwrap()
}

#[tokio::test]
#[verifies(
    "spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#server-mode",
    r = 1
)]
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
        .oneshot(req_delete(
            "/v1/packages/org.vibevm/wal/0.1.0",
            Some("topsecret"),
        ))
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
        .oneshot(req_delete("/v1/packages/org.vibevm/wal/0.1.0", None))
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
        .oneshot(req_delete("/v1/packages/org.vibevm/wal", Some("topsecret")))
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
            "/v1/packages/org.vibevm/ghost-package",
            Some("topsecret"),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_to_json(resp.into_body()).await;
    assert_eq!(body["removed"], false);
}

/// Ф3.2c2 acceptance 1 (upsert): a successful POST lands a
/// `Published` fact in the journal — the truth the catalog is
/// projected from.
#[tokio::test]
async fn post_writes_published_record_to_journal() {
    let (tmp, state) = fresh_state(false, Some("topsecret"));
    let app = build_app(state);
    let payload = serde_json::to_value(entry(PackageKind::Flow, "wal", "0.1.0")).unwrap();
    let resp = app
        .oneshot(req_post_json("/v1/packages", Some("topsecret"), payload))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let _ = body_to_json(resp.into_body()).await;
    let records = journal_of(tmp.path());
    let published = records
        .iter()
        .filter(|r| matches!(r.event, Event::Published { .. }))
        .count();
    assert_eq!(published, 1, "one POST, one Published fact");
    assert!(
        records.iter().any(|r| match &r.event {
            Event::Published { entry } =>
                entry.name == "wal" && entry.version.to_string() == "0.1.0",
            _ => false,
        }),
        "the Published fact must name wal@0.1.0"
    );
}

/// Ф3.2c2 acceptance 2 (F2-3 under the journal): an identical repeat
/// POST appends NO record — the journal carries no fact that asserts
/// nothing, which is also what keeps the auto-commit path at one
/// commit per real change.
#[tokio::test]
async fn identical_repeat_post_appends_no_journal_record() {
    let (tmp, state) = fresh_state(false, Some("topsecret"));
    let app = build_app(state);
    let payload = serde_json::to_value(entry(PackageKind::Flow, "wal", "0.1.0")).unwrap();
    for expected in [StatusCode::CREATED, StatusCode::OK] {
        let resp = app
            .clone()
            .oneshot(req_post_json(
                "/v1/packages",
                Some("topsecret"),
                payload.clone(),
            ))
            .await
            .unwrap();
        assert_eq!(resp.status(), expected);
        let _ = body_to_json(resp.into_body()).await;
    }
    let records = journal_of(tmp.path());
    assert_eq!(
        records.len(),
        2,
        "Initialised + exactly one Published; the identical repeat must not grow the journal"
    );
}

/// Ф3.2c2 acceptance 1 (deletes): each DELETE lands a `Removed` fact
/// with the expected version shape — `Some(v)` for the version route,
/// `None` for the package route.
#[tokio::test]
async fn deletes_write_removed_records_to_journal() {
    let (tmp, state) = fresh_state(false, Some("topsecret"));
    let app = build_app(state);
    for v in ["0.1.0", "0.2.0"] {
        let resp = app
            .clone()
            .oneshot(req_post_json(
                "/v1/packages",
                Some("topsecret"),
                serde_json::to_value(entry(PackageKind::Flow, "wal", v)).unwrap(),
            ))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
        let _ = body_to_json(resp.into_body()).await;
    }
    let resp = app
        .clone()
        .oneshot(req_delete(
            "/v1/packages/org.vibevm/wal/0.1.0",
            Some("topsecret"),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let _ = body_to_json(resp.into_body()).await;
    let resp = app
        .oneshot(req_delete("/v1/packages/org.vibevm/wal", Some("topsecret")))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let _ = body_to_json(resp.into_body()).await;

    let records = journal_of(tmp.path());
    let removed: Vec<&Event> = records
        .iter()
        .filter_map(|r| match &r.event {
            e @ Event::Removed { .. } => Some(e),
            _ => None,
        })
        .collect();
    assert_eq!(removed.len(), 2, "two DELETEs, two Removed facts");
    assert!(
        matches!(
            &removed[..],
            [
                Event::Removed {
                    version: Some(v), ..
                },
                Event::Removed { version: None, .. }
            ] if v.to_string() == "0.1.0"
        ),
        "the version route must record Removed@Some(0.1.0) and the \
         package route Removed@None, in that order"
    );
}

/// Ф3.2c2 acceptance 3: a data-dir seeded ONLY with a journal (no
/// catalog was ever written) boots the server through the same
/// `boot_index` the CLI's `serve` uses, and a read route serves the
/// folded package.
#[tokio::test]
async fn server_boots_from_journal_and_serves_reads() {
    let tmp = tempfile::tempdir().unwrap();
    let journal = default_dir(tmp.path());
    seed_initialised(tmp.path());
    append(
        &journal,
        &JournalRecord {
            at: now(),
            actor: default_generator(),
            event: Event::Published {
                entry: Box::new(entry(PackageKind::Flow, "wal", "0.1.0")),
            },
        },
    )
    .unwrap();
    assert!(
        !tmp.path().join("repomd.json").exists(),
        "the seed wrote a journal and nothing else — no catalog exists yet"
    );
    let index = serve::boot_index(tmp.path()).unwrap();
    let state = AppState::new(tmp.path().to_path_buf(), true, index);
    let app = build_app(state);
    let resp = app
        .oneshot(req_get("/v1/packages/org.vibevm/wal"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_to_json(resp.into_body()).await;
    assert_eq!(body["name"], "wal");
    assert_eq!(body["versions"].as_array().unwrap().len(), 1);
    assert_eq!(body["versions"][0]["version"], "0.1.0");
}

/// Ф3.2c2 acceptance 4: a data-dir with a catalog but no journal
/// (every pre-journal data-dir's shape) is REFUSED at boot, with a
/// refusal that names the truth layer and the recreate recipe.
#[test]
fn boot_refuses_catalog_without_journal() {
    let tmp = tempfile::tempdir().unwrap();
    let idx = Index::new(
        "vibespecs",
        "https://example.invalid/vibespecs",
        NamingConvention::Fqdn,
        now(),
    );
    idx.write_to(tmp.path(), &WriteCtx { at: now() }).unwrap();
    assert!(
        tmp.path().join("repomd.json").exists(),
        "precondition: the catalog is there, the journal is not"
    );
    let err = serve::boot_index(tmp.path()).unwrap_err().to_string();
    assert!(
        err.contains("cannot be served from its journal"),
        "the refusal must say what cannot serve: {err}"
    );
    assert!(
        err.contains("vibe-index init"),
        "the refusal must carry the recreate recipe: {err}"
    );
}
