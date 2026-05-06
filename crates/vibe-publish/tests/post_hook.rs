//! End-to-end coverage of the post-publish index hook against an
//! axum mock server. The hook is exercised via `fire_index_hook`
//! after a fixture package is staged on disk; the mock asserts the
//! POST shape and returns a recorded status.

use std::path::Path;
use std::sync::{Arc, Mutex, mpsc};
use std::thread;

use axum::Router;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode, header};
use axum::routing::post;
use serde::Deserialize;
use tokio::net::TcpListener;
use vibe_core::PackageKind;
use vibe_publish::{PublishOutcome, fire_index_hook};

#[derive(Default)]
struct Captured {
    /// Last POST body received.
    body: Option<serde_json::Value>,
    /// Last Authorization header received.
    auth: Option<String>,
    /// Status code the next POST should return.
    return_status: u16,
}

#[derive(Clone)]
struct MockState {
    captured: Arc<Mutex<Captured>>,
}

async fn capture_post(
    State(state): State<MockState>,
    headers: HeaderMap,
    axum::Json(body): axum::Json<serde_json::Value>,
) -> impl axum::response::IntoResponse {
    let mut c = state.captured.lock().unwrap();
    c.body = Some(body);
    c.auth = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    let status = match c.return_status {
        0 => StatusCode::CREATED,
        s => StatusCode::from_u16(s).unwrap_or(StatusCode::CREATED),
    };
    (status, axum::Json(serde_json::json!({"created": true})))
}

struct Mock {
    base_url: String,
    captured: Arc<Mutex<Captured>>,
    _thread: thread::JoinHandle<()>,
}

fn spawn_mock() -> Mock {
    let captured = Arc::new(Mutex::new(Captured {
        body: None,
        auth: None,
        return_status: 0,
    }));
    let captured_for_thread = captured.clone();
    let (tx, rx) = mpsc::channel();
    let handle = thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async move {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let addr = listener.local_addr().unwrap();
            let state = MockState {
                captured: captured_for_thread,
            };
            let app = Router::new()
                .route("/v1/packages", post(capture_post))
                .with_state(state);
            tx.send(format!("http://{addr}")).unwrap();
            axum::serve(listener, app).await.unwrap();
        });
    });
    Mock {
        base_url: rx.recv().unwrap(),
        captured,
        _thread: handle,
    }
}

fn write_fixture_package(dir: &Path) {
    std::fs::write(
        dir.join("vibe-package.toml"),
        r#"[package]
name = "wal"
kind = "flow"
version = "0.1.0"
authors = ["Test"]
license = "EULA"
description = "test"
keywords = ["wal"]

[provides]
capabilities = ["interface:wal"]
"#,
    )
    .unwrap();
    std::fs::write(dir.join("README.md"), "# wal\n").unwrap();
}

fn outcome() -> PublishOutcome {
    PublishOutcome {
        kind: PackageKind::Flow,
        name: "wal".into(),
        version: "0.1.0".parse().unwrap(),
        repo_name: "flow-wal".into(),
        repo_url: "https://github.com/vibespecs/flow-wal.git".into(),
        tag: "v0.1.0".into(),
        created_repo: true,
        host: "github.com".into(),
        dry_run: false,
    }
}

fn temp_set(key: &str, val: &str) -> EnvGuard {
    unsafe { std::env::set_var(key, val) };
    EnvGuard {
        key: key.to_string(),
    }
}

struct EnvGuard {
    key: String,
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        unsafe { std::env::remove_var(&self.key) };
    }
}

#[test]
fn fires_when_env_configured_and_index_returns_201() {
    let mock = spawn_mock();
    let _u = temp_set("VIBEVM_INDEX_URL_HOOKTESTA", &mock.base_url);
    let _t = temp_set("VIBEVM_INDEX_TOKEN_HOOKTESTA", "alpha-token");
    let work = tempfile::tempdir().unwrap();
    let pkg = work.path().join("pkg");
    std::fs::create_dir_all(&pkg).unwrap();
    write_fixture_package(&pkg);

    let report = fire_index_hook(&outcome(), &pkg, "hooktesta");
    assert!(report.fired, "expected fired=true, got {report:?}");
    assert_eq!(report.status, Some(201));

    let captured = mock.captured.lock().unwrap();
    assert_eq!(captured.auth.as_deref(), Some("Bearer alpha-token"));
    let body = captured.body.as_ref().unwrap();
    assert_eq!(body["kind"], "flow");
    assert_eq!(body["name"], "wal");
    assert_eq!(body["version"], "0.1.0");
    assert_eq!(body["registry"], "hooktesta");
    assert_eq!(body["source_url"], "https://github.com/vibespecs/flow-wal.git");
    assert_eq!(body["source_ref"], "v0.1.0");
    assert!(body["content_hash"]
        .as_str()
        .unwrap()
        .starts_with("sha256:"));
    assert_eq!(body["files_count"], 2);
    assert_eq!(body["i18n"]["default"], "en");
    assert!(body["indexed_at"].is_string());
}

#[test]
fn dormant_when_url_missing() {
    let _t = temp_set("VIBEVM_INDEX_TOKEN_HOOKTESTB", "tok");
    let work = tempfile::tempdir().unwrap();
    let pkg = work.path().join("pkg");
    std::fs::create_dir_all(&pkg).unwrap();
    write_fixture_package(&pkg);
    let report = fire_index_hook(&outcome(), &pkg, "hooktestb");
    assert!(!report.fired);
    assert_eq!(report.error, None);
}

#[test]
fn dormant_when_token_missing() {
    let _u = temp_set("VIBEVM_INDEX_URL_HOOKTESTC", "https://example.invalid");
    let work = tempfile::tempdir().unwrap();
    let pkg = work.path().join("pkg");
    std::fs::create_dir_all(&pkg).unwrap();
    write_fixture_package(&pkg);
    let report = fire_index_hook(&outcome(), &pkg, "hooktestc");
    assert!(!report.fired);
}

#[test]
fn surfaces_error_on_unexpected_status() {
    let mock = spawn_mock();
    {
        let mut c = mock.captured.lock().unwrap();
        c.return_status = 500;
    }
    let _u = temp_set("VIBEVM_INDEX_URL_HOOKTESTD", &mock.base_url);
    let _t = temp_set("VIBEVM_INDEX_TOKEN_HOOKTESTD", "tok");
    let work = tempfile::tempdir().unwrap();
    let pkg = work.path().join("pkg");
    std::fs::create_dir_all(&pkg).unwrap();
    write_fixture_package(&pkg);
    let report = fire_index_hook(&outcome(), &pkg, "hooktestd");
    assert!(!report.fired);
    let err = report.error.expect("expected error");
    assert!(err.contains("500"), "expected 500 in error message: {err}");
}

#[test]
fn surfaces_error_when_manifest_missing() {
    let mock = spawn_mock();
    let _u = temp_set("VIBEVM_INDEX_URL_HOOKTESTE", &mock.base_url);
    let _t = temp_set("VIBEVM_INDEX_TOKEN_HOOKTESTE", "tok");
    let work = tempfile::tempdir().unwrap();
    // No vibe-package.toml in work.path() — the read should fail.
    let report = fire_index_hook(&outcome(), work.path(), "hookteste");
    assert!(!report.fired);
    assert!(report.error.is_some());
}

// Suppress unused-import lint when `Deserialize` is needed only for
// future test extensions.
#[allow(dead_code)]
fn _silence_deserialize<T: for<'a> Deserialize<'a>>(_: &T) {}
