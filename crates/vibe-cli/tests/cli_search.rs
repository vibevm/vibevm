//! End-to-end coverage of `vibe search` against a mock axum index
//! server. Exercises the multi-registry walk, dedup, env-var
//! attribution, and the JSON envelope shape — without any live
//! internet or `services/vibe-index` binary dependency.

use std::sync::{Arc, Mutex, mpsc};
use std::thread;

use assert_cmd::Command;
use axum::Router;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use serde::Deserialize;
use tokio::net::TcpListener;

fn vibe() -> Command {
    Command::cargo_bin("vibe").expect("vibe binary built")
}

#[derive(Clone)]
struct MockState {
    canned: Arc<Mutex<Canned>>,
}

#[derive(Default, Clone)]
struct Canned {
    /// Successful search response. `None` => respond with status 503.
    search_response: Option<serde_json::Value>,
    last_query: Option<String>,
    last_kind: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SearchQuery {
    q: Option<String>,
    kind: Option<String>,
    #[allow(dead_code)]
    limit: Option<usize>,
}

async fn repomd_handler() -> impl IntoResponse {
    (
        StatusCode::OK,
        axum::Json(serde_json::json!({
            "schema_version": 1,
            "registry": "vibespecs",
            "registry_url": "https://example.invalid/vibespecs",
            "naming": "kind-name",
            "generated_at": "2026-05-07T00:00:00Z",
            "generator": "mock",
            "package_count": 0,
            "version_count": 0,
            "files": {}
        })),
    )
        .into_response()
}

async fn search_handler(
    State(state): State<MockState>,
    Query(q): Query<SearchQuery>,
) -> axum::response::Response {
    let mut canned = state.canned.lock().unwrap();
    canned.last_query = q.q.clone();
    canned.last_kind = q.kind.clone();
    let Some(body) = canned.search_response.clone() else {
        return (StatusCode::SERVICE_UNAVAILABLE, "").into_response();
    };
    (StatusCode::OK, axum::Json(body)).into_response()
}

struct Mock {
    base_url: String,
    canned: Arc<Mutex<Canned>>,
    _thread: thread::JoinHandle<()>,
}

fn spawn_mock(canned: Canned) -> Mock {
    let canned = Arc::new(Mutex::new(canned));
    let canned_for_thread = canned.clone();
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
                canned: canned_for_thread,
            };
            let app = Router::new()
                .route("/repomd.json", get(repomd_handler))
                .route("/v1/packages", get(search_handler))
                .with_state(state);
            tx.send(format!("http://{addr}")).unwrap();
            axum::serve(listener, app).await.unwrap();
        });
    });
    Mock {
        base_url: rx.recv().unwrap(),
        canned,
        _thread: handle,
    }
}

fn init_project(dir: &std::path::Path) {
    vibe()
        .arg("init")
        .arg("--path")
        .arg(dir)
        .arg("--no-registry")
        .assert()
        .success();
}

fn write_two_registry_manifest(project_root: &std::path::Path) {
    let manifest = r#"[project]
name = "test-search"
version = "0.0.1"

[[registry]]
name = "primary"
url = "https://example.invalid/primary"

[[registry]]
name = "secondary"
url = "https://example.invalid/secondary"
"#;
    std::fs::write(project_root.join("vibe.toml"), manifest).unwrap();
}

#[test]
fn search_aggregates_hits_from_configured_registries() {
    let mock = spawn_mock(Canned {
        search_response: Some(serde_json::json!({
            "command": "search",
            "query": "wal",
            "hit_count": 1,
            "hits": [
                {
                    "kind": "flow",
                    "name": "wal",
                    "latest_stable": "0.1.0",
                    "score": 3,
                    "matched_tokens": ["wal"],
                    "description": "Write-ahead log."
                }
            ]
        })),
        ..Canned::default()
    });

    let project = tempfile::tempdir().unwrap();
    init_project(project.path());
    write_two_registry_manifest(project.path());

    // Only `primary` has an index URL set; `secondary` lands in
    // `registries_unconfigured`.
    let out = vibe()
        .env("VIBEVM_INDEX_URL_PRIMARY", &mock.base_url)
        .env_remove("VIBEVM_INDEX_URL_SECONDARY")
        .arg("--json")
        .arg("search")
        .arg("wal")
        .arg("--path")
        .arg(project.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("stdout must be JSON");
    assert_eq!(v["command"], "search");
    assert_eq!(v["query"], "wal");
    assert_eq!(v["hit_count"], 1);
    let searched = v["registries_searched"].as_array().unwrap();
    assert_eq!(searched.len(), 1);
    assert_eq!(searched[0], "primary");
    let unconfigured = v["registries_unconfigured"].as_array().unwrap();
    assert_eq!(unconfigured.len(), 1);
    assert_eq!(unconfigured[0], "secondary");
    let unreachable = v["registries_unreachable"].as_array().unwrap();
    assert!(unreachable.is_empty());
    let hits = v["hits"].as_array().unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0]["kind"], "flow");
    assert_eq!(hits[0]["name"], "wal");
    assert_eq!(hits[0]["registry"], "primary");
    assert_eq!(hits[0]["score"], 3);
    assert_eq!(hits[0]["latest_stable"], "0.1.0");

    // The mock observed our query.
    let canned = mock.canned.lock().unwrap();
    assert_eq!(canned.last_query.as_deref(), Some("wal"));
    assert!(canned.last_kind.is_none());
}

#[test]
fn search_dedup_keeps_highest_score_across_registries() {
    let canned1 = Canned {
        search_response: Some(serde_json::json!({
            "command": "search",
            "query": "wal",
            "hit_count": 1,
            "hits": [
                {
                    "kind": "flow",
                    "name": "wal",
                    "latest_stable": "0.1.0",
                    "score": 1,
                    "matched_tokens": ["wal"],
                    "description": "Stale copy on primary."
                }
            ]
        })),
        ..Canned::default()
    };
    let canned2 = Canned {
        search_response: Some(serde_json::json!({
            "command": "search",
            "query": "wal",
            "hit_count": 1,
            "hits": [
                {
                    "kind": "flow",
                    "name": "wal",
                    "latest_stable": "0.2.0",
                    "score": 5,
                    "matched_tokens": ["wal", "log", "atomic"],
                    "description": "Fresh entry on secondary."
                }
            ]
        })),
        ..Canned::default()
    };
    let mock_primary = spawn_mock(canned1);
    let mock_secondary = spawn_mock(canned2);

    let project = tempfile::tempdir().unwrap();
    init_project(project.path());
    write_two_registry_manifest(project.path());

    let out = vibe()
        .env("VIBEVM_INDEX_URL_PRIMARY", &mock_primary.base_url)
        .env("VIBEVM_INDEX_URL_SECONDARY", &mock_secondary.base_url)
        .arg("--json")
        .arg("search")
        .arg("wal")
        .arg("--path")
        .arg(project.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();

    assert_eq!(v["hit_count"], 1);
    let hits = v["hits"].as_array().unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0]["score"], 5);
    assert_eq!(hits[0]["registry"], "secondary");
    assert_eq!(hits[0]["latest_stable"], "0.2.0");

    let searched = v["registries_searched"].as_array().unwrap();
    assert_eq!(searched.len(), 2);
}

#[test]
fn search_reports_unreachable_registry_without_aborting() {
    // `primary` returns a real hit. `secondary` 503s on every search;
    // its repomd.json probe still succeeds, so the failure surfaces
    // as a per-registry `registries_unreachable` entry, not a hard
    // command error.
    let mock_primary = spawn_mock(Canned {
        search_response: Some(serde_json::json!({
            "command": "search",
            "query": "wal",
            "hit_count": 1,
            "hits": [
                {
                    "kind": "flow",
                    "name": "wal",
                    "latest_stable": "0.1.0",
                    "score": 2,
                    "matched_tokens": ["wal"],
                    "description": "ok."
                }
            ]
        })),
        ..Canned::default()
    });
    let mock_secondary = spawn_mock(Canned {
        search_response: None, // => 503 from search_handler
        ..Canned::default()
    });

    let project = tempfile::tempdir().unwrap();
    init_project(project.path());
    write_two_registry_manifest(project.path());

    let out = vibe()
        .env("VIBEVM_INDEX_URL_PRIMARY", &mock_primary.base_url)
        .env("VIBEVM_INDEX_URL_SECONDARY", &mock_secondary.base_url)
        .arg("--json")
        .arg("search")
        .arg("wal")
        .arg("--path")
        .arg(project.path())
        .output()
        .unwrap();
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();

    assert_eq!(v["hit_count"], 1);
    let searched = v["registries_searched"].as_array().unwrap();
    assert_eq!(searched.len(), 1);
    assert_eq!(searched[0], "primary");
    let unreachable = v["registries_unreachable"].as_array().unwrap();
    assert_eq!(unreachable.len(), 1);
    assert_eq!(unreachable[0]["name"], "secondary");
}

#[test]
fn search_filters_to_one_registry_via_flag() {
    let mock = spawn_mock(Canned {
        search_response: Some(serde_json::json!({
            "command": "search",
            "query": "wal",
            "hit_count": 1,
            "hits": [
                {
                    "kind": "flow",
                    "name": "wal",
                    "latest_stable": "0.1.0",
                    "score": 1,
                    "matched_tokens": ["wal"],
                    "description": null
                }
            ]
        })),
        ..Canned::default()
    });

    let project = tempfile::tempdir().unwrap();
    init_project(project.path());
    write_two_registry_manifest(project.path());

    let out = vibe()
        .env("VIBEVM_INDEX_URL_PRIMARY", &mock.base_url)
        .env("VIBEVM_INDEX_URL_SECONDARY", &mock.base_url)
        .arg("--json")
        .arg("search")
        .arg("wal")
        .arg("--registry")
        .arg("secondary")
        .arg("--path")
        .arg(project.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();

    let searched = v["registries_searched"].as_array().unwrap();
    assert_eq!(searched.len(), 1, "only the named registry should be walked");
    assert_eq!(searched[0], "secondary");
    let unconfigured = v["registries_unconfigured"].as_array().unwrap();
    assert!(unconfigured.is_empty(), "primary was filtered out — not reported");
}

#[test]
fn search_kind_flag_is_propagated_to_index() {
    let mock = spawn_mock(Canned {
        search_response: Some(serde_json::json!({
            "command": "search",
            "query": "wal",
            "hit_count": 0,
            "hits": []
        })),
        ..Canned::default()
    });

    let project = tempfile::tempdir().unwrap();
    init_project(project.path());
    write_two_registry_manifest(project.path());

    let out = vibe()
        .env("VIBEVM_INDEX_URL_PRIMARY", &mock.base_url)
        .env_remove("VIBEVM_INDEX_URL_SECONDARY")
        .arg("--json")
        .arg("search")
        .arg("wal")
        .arg("--kind")
        .arg("feat")
        .arg("--path")
        .arg(project.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    let canned = mock.canned.lock().unwrap();
    assert_eq!(canned.last_kind.as_deref(), Some("feat"));
}

#[test]
fn search_errors_when_registry_name_unknown() {
    let project = tempfile::tempdir().unwrap();
    init_project(project.path());
    write_two_registry_manifest(project.path());

    let out = vibe()
        .arg("search")
        .arg("wal")
        .arg("--registry")
        .arg("ghost")
        .arg("--path")
        .arg(project.path())
        .output()
        .unwrap();
    assert!(!out.status.success(), "should fail when --registry unknown");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("ghost"),
        "stderr should mention the unknown registry name; got:\n{stderr}"
    );
}
