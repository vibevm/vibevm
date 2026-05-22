//! End-to-end coverage of `vibe search` against a mock axum index
//! server. Exercises the multi-registry walk, dedup, env-var
//! attribution, and the JSON envelope shape — without any live
//! internet or a `vibe-index` binary dependency.

use std::sync::{Arc, Mutex, mpsc};
use std::thread;

use assert_cmd::Command;
use axum::Router;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use serde::Deserialize;
use tokio::net::TcpListener;

fn vibe() -> Command {
    Command::cargo_bin("vibe").expect("vibe binary built")
}

/// Cache-isolating wrapper. Every test gets its own tempdir for the
/// search-cache so successive runs do not poison each other through
/// `~/.vibe/search-cache/`. Tests that explicitly need a shared
/// cache (the cache hit/miss e2e) pass an explicit dir instead.
fn vibe_isolated_cache(cache_dir: &std::path::Path) -> Command {
    let mut c = vibe();
    c.env("VIBEVM_SEARCH_CACHE_DIR", cache_dir);
    c
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
    /// Successful PURL-lookup response. `None` => respond with status 503.
    purl_response: Option<serde_json::Value>,
    last_purl: Option<String>,
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

async fn purls_handler(
    State(state): State<MockState>,
    Path(purl): Path<String>,
) -> axum::response::Response {
    let mut canned = state.canned.lock().unwrap();
    canned.last_purl = Some(purl);
    let Some(body) = canned.purl_response.clone() else {
        return (StatusCode::SERVICE_UNAVAILABLE, "").into_response();
    };
    (StatusCode::OK, axum::Json(body)).into_response()
}

#[derive(Clone, Default)]
struct GitHubCanned {
    /// Map from "owner/repo" → vibe.toml content (raw TOML, not
    /// yet base64-encoded). `None` => respond 404 (not a vibevm package).
    package_files: std::collections::HashMap<String, Option<String>>,
    /// Map from org → list of repos (each just a name; non-fork,
    /// non-archived). One page only — the test only exercises one
    /// page of pagination.
    org_repos: std::collections::HashMap<String, Vec<String>>,
    last_org: Option<String>,
}

#[derive(Clone)]
struct GitHubMockState {
    canned: Arc<Mutex<GitHubCanned>>,
}

#[derive(Debug, Deserialize)]
struct OrgReposQuery {
    #[allow(dead_code)]
    per_page: Option<u32>,
    #[allow(dead_code)]
    page: Option<u32>,
}

async fn org_repos_handler(
    State(state): State<GitHubMockState>,
    Path(org): Path<String>,
    Query(_q): Query<OrgReposQuery>,
) -> axum::response::Response {
    let mut canned = state.canned.lock().unwrap();
    canned.last_org = Some(org.clone());
    let repos = canned.org_repos.get(&org).cloned().unwrap_or_default();
    let body: Vec<serde_json::Value> = repos
        .into_iter()
        .map(|name| {
            serde_json::json!({
                "name": name,
                "fork": false,
                "archived": false
            })
        })
        .collect();
    (StatusCode::OK, axum::Json(body)).into_response()
}

async fn contents_handler(
    State(state): State<GitHubMockState>,
    Path((owner, repo)): Path<(String, String)>,
) -> axum::response::Response {
    let canned = state.canned.lock().unwrap();
    let key = format!("{owner}/{repo}");
    match canned.package_files.get(&key) {
        Some(Some(toml_str)) => {
            let encoded = b64_encode_test(toml_str.as_bytes());
            (
                StatusCode::OK,
                axum::Json(serde_json::json!({
                    "encoding": "base64",
                    "content": encoded
                })),
            )
                .into_response()
        }
        Some(None) | None => StatusCode::NOT_FOUND.into_response(),
    }
}

fn b64_encode_test(input: &[u8]) -> String {
    // Mirror of search_full_scan::encode_base64 — keeping the test
    // self-contained.
    const TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    let mut i = 0;
    while i < input.len() {
        let b0 = input[i];
        let b1 = if i + 1 < input.len() { input[i + 1] } else { 0 };
        let b2 = if i + 2 < input.len() { input[i + 2] } else { 0 };
        let n = ((b0 as u32) << 16) | ((b1 as u32) << 8) | (b2 as u32);
        out.push(TABLE[((n >> 18) & 0x3f) as usize] as char);
        out.push(TABLE[((n >> 12) & 0x3f) as usize] as char);
        if i + 1 < input.len() {
            out.push(TABLE[((n >> 6) & 0x3f) as usize] as char);
        } else {
            out.push('=');
        }
        if i + 2 < input.len() {
            out.push(TABLE[(n & 0x3f) as usize] as char);
        } else {
            out.push('=');
        }
        i += 3;
    }
    out
}

struct GitHubMock {
    base_url: String,
    #[allow(dead_code)]
    canned: Arc<Mutex<GitHubCanned>>,
    _thread: thread::JoinHandle<()>,
}

fn spawn_github_mock(canned: GitHubCanned) -> GitHubMock {
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
            let state = GitHubMockState {
                canned: canned_for_thread,
            };
            let app = Router::new()
                .route("/orgs/{org}/repos", get(org_repos_handler))
                .route(
                    "/repos/{owner}/{repo}/contents/vibe.toml",
                    get(contents_handler),
                )
                .with_state(state);
            tx.send(format!("http://{addr}")).unwrap();
            axum::serve(listener, app).await.unwrap();
        });
    });
    GitHubMock {
        base_url: rx.recv().unwrap(),
        canned,
        _thread: handle,
    }
}

fn write_github_only_manifest(project_root: &std::path::Path) {
    let manifest = r#"[project]
name = "test-search"
version = "0.0.1"

[[registry]]
name = "vibespecs"
url = "https://github.com/vibespecs"
"#;
    std::fs::write(project_root.join("vibe.toml"), manifest).unwrap();
}

fn write_gitverse_only_manifest(project_root: &std::path::Path) {
    let manifest = r#"[project]
name = "test-search"
version = "0.0.1"

[[registry]]
name = "vibespecs-gitverse"
url = "https://gitverse.ru/vibespecs"
"#;
    std::fs::write(project_root.join("vibe.toml"), manifest).unwrap();
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
                .route("/v1/purls/{purl}", get(purls_handler))
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
    let cache_dir = tempfile::tempdir().unwrap();

    // Only `primary` has an index URL set; `secondary` lands in
    // `registries_unconfigured`.
    let out = vibe_isolated_cache(cache_dir.path())
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
    let cache_dir = tempfile::tempdir().unwrap();

    let out = vibe_isolated_cache(cache_dir.path())
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
    let cache_dir = tempfile::tempdir().unwrap();

    let out = vibe_isolated_cache(cache_dir.path())
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
    let cache_dir = tempfile::tempdir().unwrap();

    let out = vibe_isolated_cache(cache_dir.path())
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
    let cache_dir = tempfile::tempdir().unwrap();

    let out = vibe_isolated_cache(cache_dir.path())
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
fn search_full_scan_finds_matching_packages_in_github_org() {
    // vibespecs org carries 3 repos: flow-wal (matches), flow-other
    // (no match), feat-without-manifest (404 on contents). full-scan
    // returns one hit, attributed to source="full-scan".
    let mut canned = GitHubCanned::default();
    canned.org_repos.insert(
        "vibespecs".into(),
        vec![
            "flow-wal".into(),
            "flow-other".into(),
            "feat-without-manifest".into(),
        ],
    );
    canned.package_files.insert(
        "vibespecs/flow-wal".into(),
        Some(
            r#"[package]
kind = "flow"
name = "wal"
version = "0.1.0"
description = "Write-ahead log discipline for spec-driven projects."
"#
            .into(),
        ),
    );
    canned.package_files.insert(
        "vibespecs/flow-other".into(),
        Some(
            r#"[package]
kind = "flow"
name = "other"
version = "0.2.0"
description = "Atomic-commits checks."
"#
            .into(),
        ),
    );
    // feat-without-manifest: no package_files entry → 404
    let github = spawn_github_mock(canned);

    let project = tempfile::tempdir().unwrap();
    init_project(project.path());
    write_github_only_manifest(project.path());
    let cache_dir = tempfile::tempdir().unwrap();

    let out = vibe_isolated_cache(cache_dir.path())
        .env("VIBEVM_GITHUB_API_BASE", &github.base_url)
        .env_remove("VIBEVM_INDEX_URL_VIBESPECS")
        .env_remove("VIBEVM_PUBLISH_TOKEN_GITHUB")
        .env_remove("VIBEVM_PUBLISH_TOKEN")
        .arg("--json")
        .arg("search")
        .arg("wal")
        .arg("--full-scan")
        .arg("--path")
        .arg(project.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["command"], "search");
    let scanned = v["registries_full_scanned"].as_array().unwrap();
    assert_eq!(scanned.len(), 1);
    assert_eq!(scanned[0], "vibespecs");
    assert!(
        v["registries_unconfigured"]
            .as_array()
            .map(|a| a.is_empty())
            .unwrap_or(true),
        "vibespecs should have moved out of `unconfigured` once full-scan picked it up"
    );
    assert_eq!(v["hit_count"], 1);
    let hits = v["hits"].as_array().unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0]["kind"], "flow");
    assert_eq!(hits[0]["name"], "wal");
    assert_eq!(hits[0]["latest_stable"], "0.1.0");
    assert_eq!(hits[0]["registry"], "vibespecs");
    assert_eq!(hits[0]["source"], "full-scan");
    assert!(
        hits[0]["score"].as_u64().unwrap() >= 1,
        "wal token should hit name + description; score >= 1"
    );
}

#[test]
fn search_full_scan_unsupported_for_non_github_host() {
    let project = tempfile::tempdir().unwrap();
    init_project(project.path());
    write_gitverse_only_manifest(project.path());
    let cache_dir = tempfile::tempdir().unwrap();

    let out = vibe_isolated_cache(cache_dir.path())
        .env_remove("VIBEVM_INDEX_URL_VIBESPECS_GITVERSE")
        .arg("--json")
        .arg("search")
        .arg("wal")
        .arg("--full-scan")
        .arg("--path")
        .arg(project.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["hit_count"], 0);
    let unsupported = v["registries_full_scan_unsupported"].as_array().unwrap();
    assert_eq!(unsupported.len(), 1);
    assert_eq!(unsupported[0]["name"], "vibespecs-gitverse");
    assert!(
        unsupported[0]["reason"]
            .as_str()
            .unwrap()
            .contains("github.com"),
        "reason should mention the github.com restriction"
    );
}

#[test]
fn search_without_full_scan_keeps_unconfigured_status() {
    let project = tempfile::tempdir().unwrap();
    init_project(project.path());
    write_github_only_manifest(project.path());
    let cache_dir = tempfile::tempdir().unwrap();

    let out = vibe_isolated_cache(cache_dir.path())
        .env_remove("VIBEVM_INDEX_URL_VIBESPECS")
        .arg("--json")
        .arg("search")
        .arg("wal")
        .arg("--path")
        .arg(project.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["hit_count"], 0);
    let unconfigured = v["registries_unconfigured"].as_array().unwrap();
    assert_eq!(unconfigured.len(), 1);
    assert_eq!(unconfigured[0], "vibespecs");
    // No full-scan happened, so the `registries_full_scanned` field is
    // skipped entirely from the JSON envelope (skip_serializing_if).
    assert!(
        v.get("registries_full_scanned").is_none()
            || v["registries_full_scanned"]
                .as_array()
                .map(|a| a.is_empty())
                .unwrap_or(true)
    );
}

/// `vibe search` in a project where no registry has an
/// `VIBEVM_INDEX_URL_<R>` env var configured prints a text-mode
/// hint pointing the operator at `vibe install <kind>:<name>`
/// directly. Regression guard surfaced by the opencode glm-flash
/// walk: the prior message ("see docs/commands/search.md") was
/// terse enough that a small model interpreted empty search as
/// "registries broken" and started mutating the project's
/// registry list. The new hint names install as the way around
/// the missing index.
#[test]
fn search_text_hint_directs_agent_to_install_when_no_index_configured() {
    let project = tempfile::tempdir().unwrap();
    init_project(project.path());
    write_github_only_manifest(project.path());
    let cache_dir = tempfile::tempdir().unwrap();

    let out = vibe_isolated_cache(cache_dir.path())
        .env_remove("VIBEVM_INDEX_URL_VIBESPECS")
        .arg("search")
        .arg("wal")
        .arg("--path")
        .arg(project.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    // Headline still names the env-var so the agent recognises it.
    assert!(
        stdout.contains("VIBEVM_INDEX_URL_<R>"),
        "expected the env-var name in stdout:\n{stdout}"
    );
    // New hint: install bypasses the index. This is the load-bearing
    // line that prevents the panic-loop the opencode walk surfaced.
    assert!(
        stdout.contains("vibe install <kind>:<name>"),
        "expected an explicit install-as-workaround hint:\n{stdout}"
    );
    // Phrase wraps across a newline ("does not need an\nindex.") so
    // assert on the prefix that survives word-wrapping.
    assert!(
        stdout.contains("does not need an"),
        "expected the hint to state install bypasses the index:\n{stdout}"
    );
}

#[test]
fn search_caches_results_and_serves_subsequent_runs_from_disk() {
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
                    "description": "Pre-cache version."
                }
            ]
        })),
        ..Canned::default()
    });

    let project = tempfile::tempdir().unwrap();
    init_project(project.path());
    write_two_registry_manifest(project.path());
    let cache_dir = tempfile::tempdir().unwrap();

    // Run 1: cold cache → fetch from mock, write entry to disk.
    let out = vibe()
        .env("VIBEVM_INDEX_URL_PRIMARY", &mock.base_url)
        .env_remove("VIBEVM_INDEX_URL_SECONDARY")
        .env("VIBEVM_SEARCH_CACHE_DIR", cache_dir.path())
        .arg("--json")
        .arg("search")
        .arg("wal")
        .arg("--path")
        .arg(project.path())
        .output()
        .unwrap();
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let cold = &v["hits"][0];
    assert_eq!(cold["description"], "Pre-cache version.");

    // The cache directory now has a file under primary/.
    let primary_cache_dir = cache_dir.path().join("primary");
    assert!(primary_cache_dir.is_dir(), "expected per-registry cache dir");
    let entries: Vec<_> = std::fs::read_dir(&primary_cache_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|x| x == "json"))
        .collect();
    assert_eq!(entries.len(), 1, "exactly one cache entry written");

    // Mutate the mock so a network refetch would return different bytes.
    {
        let mut canned = mock.canned.lock().unwrap();
        canned.search_response = Some(serde_json::json!({
            "command": "search",
            "query": "wal",
            "hit_count": 1,
            "hits": [
                {
                    "kind": "flow",
                    "name": "wal",
                    "latest_stable": "0.2.0",
                    "score": 99,
                    "matched_tokens": ["wal"],
                    "description": "Post-cache (would be fresh)."
                }
            ]
        }));
    }

    // Run 2: warm cache → server change is INVISIBLE because we read from disk.
    let out = vibe()
        .env("VIBEVM_INDEX_URL_PRIMARY", &mock.base_url)
        .env_remove("VIBEVM_INDEX_URL_SECONDARY")
        .env("VIBEVM_SEARCH_CACHE_DIR", cache_dir.path())
        .arg("--json")
        .arg("search")
        .arg("wal")
        .arg("--path")
        .arg(project.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let cached_hit = &v["hits"][0];
    assert_eq!(
        cached_hit["description"], "Pre-cache version.",
        "cache hit should preserve the original payload regardless of server changes"
    );
    assert_eq!(cached_hit["latest_stable"], "0.1.0");

    // Run 3: --no-cache bypasses both read and write — server's mutated
    // response surfaces.
    let out = vibe()
        .env("VIBEVM_INDEX_URL_PRIMARY", &mock.base_url)
        .env_remove("VIBEVM_INDEX_URL_SECONDARY")
        .env("VIBEVM_SEARCH_CACHE_DIR", cache_dir.path())
        .arg("--json")
        .arg("search")
        .arg("wal")
        .arg("--no-cache")
        .arg("--path")
        .arg(project.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let bypass_hit = &v["hits"][0];
    assert_eq!(
        bypass_hit["description"], "Post-cache (would be fresh).",
        "--no-cache should fetch live and ignore the disk entry"
    );
    assert_eq!(bypass_hit["latest_stable"], "0.2.0");
}

#[test]
fn search_cache_ttl_zero_forces_refetch() {
    // --cache-ttl 0 means "any entry older than 0 seconds is stale",
    // so even a freshly-written entry on disk is treated as expired
    // on the very next call. The semantic is "always re-fetch but
    // also write the new result" — distinct from --no-cache (which
    // disables both read and write).
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
                    "description": "first."
                }
            ]
        })),
        ..Canned::default()
    });

    let project = tempfile::tempdir().unwrap();
    init_project(project.path());
    write_two_registry_manifest(project.path());
    let cache_dir = tempfile::tempdir().unwrap();

    let out = vibe()
        .env("VIBEVM_INDEX_URL_PRIMARY", &mock.base_url)
        .env_remove("VIBEVM_INDEX_URL_SECONDARY")
        .env("VIBEVM_SEARCH_CACHE_DIR", cache_dir.path())
        .arg("--json")
        .arg("search")
        .arg("wal")
        .arg("--path")
        .arg(project.path())
        .output()
        .unwrap();
    assert!(out.status.success());

    // Mutate response.
    {
        let mut canned = mock.canned.lock().unwrap();
        canned.search_response = Some(serde_json::json!({
            "command": "search",
            "query": "wal",
            "hit_count": 1,
            "hits": [
                {
                    "kind": "flow",
                    "name": "wal",
                    "latest_stable": "0.2.0",
                    "score": 1,
                    "matched_tokens": ["wal"],
                    "description": "second."
                }
            ]
        }));
    }

    // Sleep 1 second so the recorded timestamp gate (now - mtime > ttl)
    // fires deterministically. Without a sleep both readings could land
    // in the same UNIX-second and the gate would underflow to 0 vs
    // ttl=0 => not stale.
    std::thread::sleep(std::time::Duration::from_secs(1));

    let out = vibe()
        .env("VIBEVM_INDEX_URL_PRIMARY", &mock.base_url)
        .env_remove("VIBEVM_INDEX_URL_SECONDARY")
        .env("VIBEVM_SEARCH_CACHE_DIR", cache_dir.path())
        .arg("--json")
        .arg("search")
        .arg("wal")
        .arg("--cache-ttl")
        .arg("0")
        .arg("--path")
        .arg(project.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(
        v["hits"][0]["description"], "second.",
        "ttl=0 must invalidate the entry once 1 second has passed"
    );
}

#[test]
fn search_purl_lookup_returns_binding_site_and_dedups_across_registries() {
    let canned1 = Canned {
        purl_response: Some(serde_json::json!({
            "command": "purls",
            "purl": "pkg:cargo/sqlx@0.8.0",
            "hit_count": 1,
            "hits": [
                {
                    "kind": "flow",
                    "name": "sqlx-skin",
                    "version": "0.1.0",
                    "binding_site": "package"
                }
            ]
        })),
        ..Canned::default()
    };
    let canned2 = Canned {
        purl_response: Some(serde_json::json!({
            "command": "purls",
            "purl": "pkg:cargo/sqlx@0.8.0",
            "hit_count": 2,
            "hits": [
                {
                    "kind": "flow",
                    "name": "sqlx-skin",
                    "version": "0.1.0",
                    "binding_site": "package"
                },
                {
                    "kind": "stack",
                    "name": "rust",
                    "version": "0.2.0",
                    "binding_site": "subskill"
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
        .arg("--purl")
        .arg("pkg:cargo/sqlx@0.8.0")
        .arg("--path")
        .arg(project.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["command"], "search:purl");
    assert_eq!(v["purl"], "pkg:cargo/sqlx@0.8.0");
    // 2 unique (kind, name, version) tuples — sqlx-skin@0.1.0 dedupes
    // across registries; rust@0.2.0 only appears in secondary.
    assert_eq!(v["hit_count"], 2);
    let hits = v["hits"].as_array().unwrap();
    assert_eq!(hits.len(), 2);
    let sqlx_hit = hits
        .iter()
        .find(|h| h["name"] == "sqlx-skin")
        .expect("sqlx-skin hit present");
    assert_eq!(sqlx_hit["kind"], "flow");
    assert_eq!(sqlx_hit["binding_site"], "package");
    assert_eq!(
        sqlx_hit["registry"], "primary",
        "duplicate kept from earliest registry in vibe.toml"
    );
    let rust_hit = hits
        .iter()
        .find(|h| h["name"] == "rust")
        .expect("rust subskill hit");
    assert_eq!(rust_hit["binding_site"], "subskill");
    assert_eq!(rust_hit["registry"], "secondary");

    // Mock observed our PURL — proves the path-segment encoding round-trip.
    let canned = mock_primary.canned.lock().unwrap();
    assert_eq!(
        canned.last_purl.as_deref(),
        Some("pkg:cargo/sqlx@0.8.0"),
        "axum's Path<String> URL-decodes; if reqwest skipped encoding, the captured \
         segment would be the literal `:`-/-`@` already, so we'd still see this — but \
         decoded value matching means the round-trip is consistent"
    );
}

#[test]
fn search_purl_rejects_non_pkg_scheme() {
    let project = tempfile::tempdir().unwrap();
    init_project(project.path());
    write_two_registry_manifest(project.path());

    let out = vibe()
        .arg("search")
        .arg("--purl")
        .arg("npm:fastapi@0.116.1") // missing `pkg:` scheme
        .arg("--path")
        .arg(project.path())
        .output()
        .unwrap();
    assert!(!out.status.success(), "expected validation failure");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("pkg:"),
        "stderr should mention the missing `pkg:` scheme; got:\n{stderr}"
    );
}

#[test]
fn search_purl_and_query_are_mutually_exclusive() {
    let project = tempfile::tempdir().unwrap();
    init_project(project.path());
    write_two_registry_manifest(project.path());

    let out = vibe()
        .arg("search")
        .arg("wal")
        .arg("--purl")
        .arg("pkg:cargo/x")
        .arg("--path")
        .arg(project.path())
        .output()
        .unwrap();
    assert!(!out.status.success(), "clap group should reject both");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.to_lowercase().contains("cannot be used")
            || stderr.to_lowercase().contains("conflicts"),
        "stderr should mention mutual exclusion; got:\n{stderr}"
    );
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
