//! End-to-end coverage of the client half of the eternal handshake
//! (PROP-044 `##ONE-ETERNAL-FILE`): `IndexClient::probe` asks
//! `hello.json` FIRST at both probe candidates, selects the world of
//! the epoch this build reads, refuses loudly — with the offered
//! epochs, its own epoch, and a recipe — when the index offers no
//! such world, and keeps today's `repomd.json` path byte for byte
//! when no handshake answers.
//!
//! Every request the mock serves is recorded, so the tests assert
//! not only outcomes but the ORDER the client asks in, and that a
//! named `successor` address is never requested (the device: the
//! successor points at a path the mock does not know — had the
//! client followed it, it would have gotten a 404 and the outcome
//! would have collapsed to `Absent`, failing the refusal assert).

use std::collections::HashMap;
use std::sync::{Arc, Mutex, mpsc};
use std::thread;

use axum::Router;
use axum::extract::{Path as AxumPath, State};
use axum::http::{StatusCode, Uri};
use axum::response::IntoResponse;
use axum::routing::get;
use tokio::net::TcpListener;

use vibe_core::{Group, PackageKind};
use vibe_registry::{IndexAuth, IndexClient, ProbeOutcome};
use vibe_wire::generated::format_id::FormatId;

#[derive(Default)]
struct CannedFiles {
    repomd_status: u16,
    /// `Some(body)` ⇒ `/hello.json` answers 200 with this JSON.
    hello: Option<serde_json::Value>,
    /// Keyed by bare `name` — the `by-name/` layer is the candidate
    /// set `by-name/<name>.json` (PROP-008 §2.8).
    by_name: HashMap<String, Option<serde_json::Value>>,
}

#[derive(Clone)]
struct MockState {
    files: Arc<Mutex<CannedFiles>>,
    /// Every request path the mock has served, in order.
    log: Arc<Mutex<Vec<String>>>,
}

impl MockState {
    fn record(&self, path: &str) {
        self.log.lock().unwrap().push(path.to_string());
    }
}

async fn repomd_handler(State(state): State<MockState>) -> impl IntoResponse {
    state.record("/repomd.json");
    let s = state.files.lock().unwrap().repomd_status;
    let status = match s {
        0 => StatusCode::OK,
        n => StatusCode::from_u16(n).unwrap_or(StatusCode::OK),
    };
    (
        status,
        axum::Json(serde_json::json!({
            "schema_version": 1,
            "registry": "vibespecs",
            "registry_url": "https://example.invalid",
            "naming": "kind-name",
            "generated_at": "2026-05-06T12:00:00Z",
            "generator": "mock",
            "package_count": 1,
            "version_count": 1,
            "files": {}
        })),
    )
        .into_response()
}

async fn hello_handler(State(state): State<MockState>) -> impl IntoResponse {
    state.record("/hello.json");
    match state.files.lock().unwrap().hello.clone() {
        Some(body) => (StatusCode::OK, axum::Json(body)).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn by_name_handler(
    State(state): State<MockState>,
    AxumPath(name_with_ext): AxumPath<String>,
    uri: Uri,
) -> axum::response::Response {
    let name = match name_with_ext.strip_suffix(".json") {
        Some(n) => n,
        None => return (StatusCode::NOT_FOUND, "expected .json").into_response(),
    };
    // The actual path, not a rebuilt one: the route is mounted at
    // the root AND under a world's prefix, and the log must show
    // which one a request took.
    state.record(uri.path());
    let payload = state.files.lock().unwrap().by_name.get(name).cloned();
    match payload {
        Some(Some(v)) => (StatusCode::OK, axum::Json(v)).into_response(),
        Some(None) => StatusCode::NOT_FOUND.into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

/// Records any path the mock does not route — the successor
/// no-follow proof reads this log, and the un-routed probe
/// candidates (`/v1/index/…`) land here as 404s.
async fn fallback_handler(State(state): State<MockState>, uri: Uri) -> impl IntoResponse {
    state.record(uri.path());
    (StatusCode::NOT_FOUND, "no such route").into_response()
}

struct Mock {
    base_url: String,
    files: Arc<Mutex<CannedFiles>>,
    log: Arc<Mutex<Vec<String>>>,
    _thread: thread::JoinHandle<()>,
}

fn spawn_mock(files: CannedFiles) -> Mock {
    let files = Arc::new(Mutex::new(files));
    let log = Arc::new(Mutex::new(Vec::new()));
    let files_for_thread = files.clone();
    let log_for_thread = log.clone();
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
                files: files_for_thread,
                log: log_for_thread,
            };
            let app = Router::new()
                .route("/repomd.json", get(repomd_handler))
                .route("/hello.json", get(hello_handler))
                .route("/by-name/{name}", get(by_name_handler))
                // A world with `path != "."` serves its whole file
                // tree under its prefix — the refinement test reads
                // the by-name layer from there.
                .route("/e2/by-name/{name}", get(by_name_handler))
                .fallback(fallback_handler)
                .with_state(state);
            tx.send(format!("http://{addr}")).unwrap();
            axum::serve(listener, app).await.unwrap();
        });
    });
    Mock {
        base_url: rx.recv().unwrap(),
        files,
        log,
        _thread: handle,
    }
}

impl Mock {
    fn served_paths(&self) -> Vec<String> {
        self.log.lock().unwrap().clone()
    }
}

/// The epoch of the catalog world this build reads — taken from the
/// generated registry exactly as the client takes it, so a registry
/// renumbering moves test and code together.
fn own_epoch() -> u32 {
    FormatId::IndexRepomd.epoch()
}

/// The handshake-format string this build writes and reads.
fn expected_vibe() -> String {
    format!("hello/{}", FormatId::Handshake.epoch())
}

/// A `by-name/<name>.json` candidate set carrying one `(group, name)`
/// package — the PROP-008 §2.8 layout the index serves.
fn name_entry_json(
    group: &str,
    kind: PackageKind,
    name: &str,
    versions: &[&str],
) -> serde_json::Value {
    let entries: Vec<serde_json::Value> = versions
        .iter()
        .map(|v| {
            serde_json::json!({
                "schema_version": 1,
                "kind": kind,
                "group": group,
                "name": name,
                "version": v,
                "content_hash": "sha256:0000",
                "source_url": "https://example.invalid/x.git",
                "source_ref": format!("v{v}"),
                "registry": "vibespecs",
                "files_count": 1,
                "indexed_at": "2026-05-06T12:00:00Z",
                "indexed_by": "mock",
            })
        })
        .collect();
    serde_json::json!({
        "name": name,
        "indexed_at": "2026-05-06T12:00:00Z",
        "packages": [
            {
                "group": group,
                "name": name,
                "indexed_at": "2026-05-06T12:00:00Z",
                "latest_stable": versions.last(),
                "versions": entries,
            }
        ],
    })
}

// ---- case 1: handshake present, epoch matches, path "." ----

#[test]
fn probe_reads_handshake_first_and_selects_the_world_of_its_epoch() {
    let mut canned = CannedFiles {
        repomd_status: 200,
        by_name: HashMap::new(),
        hello: Some(serde_json::json!({
            "vibe": expected_vibe(),
            "worlds": [{"epoch": own_epoch(), "path": "."}],
        })),
    };
    canned.by_name.insert(
        "wal".into(),
        Some(name_entry_json(
            "org.vibevm",
            PackageKind::Flow,
            "wal",
            &["0.1.0", "0.2.0"],
        )),
    );
    let mock = spawn_mock(canned);
    let outcome = IndexClient::probe(&mock.base_url, IndexAuth::None);
    let client = match outcome {
        ProbeOutcome::Found(c) => c,
        other => panic!("expected Found, got {other:?}"),
    };
    // `path == "."` must NOT grow a `/.` tail on the base.
    let expected_base = mock.base_url.trim_end_matches('/').to_string();
    assert_eq!(client.file_base(), expected_base);
    // The handshake was asked first at BOTH candidates, and no
    // `repomd.json` probe ever ran — the handshake already answered.
    assert_eq!(
        mock.served_paths(),
        vec![
            "/v1/index/hello.json".to_string(),
            "/hello.json".to_string()
        ]
    );
    // The subsequent by-name request rides the world-refined base.
    let org = Group::parse("org.vibevm").unwrap();
    let versions = client.list_versions(&org, "wal").unwrap().unwrap();
    assert_eq!(
        versions.iter().map(|v| v.to_string()).collect::<Vec<_>>(),
        vec!["0.1.0".to_string(), "0.2.0".to_string()]
    );
}

// ---- case 2 (the compatibility test): no handshake ⇒ today's path ----

#[test]
fn probe_without_handshake_keeps_todays_repomd_path() {
    let canned = CannedFiles {
        repomd_status: 200,
        by_name: HashMap::new(),
        hello: None,
    };
    let mock = spawn_mock(canned);
    let outcome = IndexClient::probe(&mock.base_url, IndexAuth::None);
    let client = match outcome {
        ProbeOutcome::Found(c) => c,
        other => panic!("expected Found, got {other:?}"),
    };
    assert_eq!(client.file_base(), mock.base_url.trim_end_matches('/'));
    // Both hello candidates were tried FIRST, then today's repomd
    // candidates in today's order — the old surface, unchanged, just
    // preceded by the two handshake GETs.
    assert_eq!(
        mock.served_paths(),
        vec![
            "/v1/index/hello.json".to_string(),
            "/hello.json".to_string(),
            "/v1/index/repomd.json".to_string(),
            "/repomd.json".to_string(),
        ]
    );
}

// ---- case 3: handshake present, own epoch absent ----

#[test]
fn probe_refuses_when_the_index_offers_no_world_of_this_epoch() {
    let own = own_epoch();
    let canned = CannedFiles {
        repomd_status: 200,
        by_name: HashMap::new(),
        hello: Some(serde_json::json!({
            "vibe": expected_vibe(),
            "worlds": [
                {"epoch": own + 1, "path": "next"},
                {"epoch": own + 2, "path": "later"},
            ],
            "min_client": "9.9.9",
            "notice": "epoch 1 has moved; take the next world",
        })),
    };
    let mock = spawn_mock(canned);
    let outcome = IndexClient::probe(&mock.base_url, IndexAuth::None);
    let reason = match outcome {
        ProbeOutcome::Refused { reason } => reason,
        other => panic!("expected Refused, got {other:?}"),
    };
    // Substrings, not the whole sentence: the text may move, the
    // requirements may not. The offered epochs, this build's epoch,
    // the recipe, and the optional explanations.
    let offered = format!("[{}, {}]", own + 1, own + 2);
    assert!(
        reason.contains(&offered),
        "refusal should list the offered epochs {offered}: {reason}"
    );
    assert!(
        reason.contains(&format!("epoch {own}")),
        "refusal should name this build's epoch {own}: {reason}"
    );
    assert!(
        reason.contains("update vibe"),
        "refusal should carry a recipe: {reason}"
    );
    assert!(
        reason.contains("9.9.9"),
        "refusal should explain min_client: {reason}"
    );
    assert!(
        reason.contains("epoch 1 has moved"),
        "refusal should carry the notice: {reason}"
    );
}

// ---- case 4: successor named, never followed ----

#[test]
fn probe_names_successor_but_never_requests_it() {
    let own = own_epoch();
    let canned = CannedFiles {
        repomd_status: 200,
        by_name: HashMap::new(),
        hello: Some(serde_json::json!({
            "vibe": expected_vibe(),
            "worlds": [{"epoch": own + 1, "path": "next"}],
        })),
    };
    let mock = spawn_mock(canned);
    // The successor points INSIDE this same mock, at a path it does
    // not route: had the client followed the pointer, the request
    // would land in the fallback log and the outcome would collapse
    // to Absent (404 everywhere under that base) instead of the
    // refusal asserted below. The address embeds the mock's own
    // ephemeral port, so it is written after the spawn.
    let successor = format!("{}/e2/hello.json", mock.base_url.trim_end_matches('/'));
    mock.files.lock().unwrap().hello = Some(serde_json::json!({
        "vibe": expected_vibe(),
        "worlds": [{"epoch": own + 1, "path": "next"}],
        "successor": successor,
    }));
    let outcome = IndexClient::probe(&mock.base_url, IndexAuth::None);
    let reason = match outcome {
        ProbeOutcome::Refused { reason } => reason,
        other => panic!("expected Refused, got {other:?}"),
    };
    assert!(
        reason.contains(&successor),
        "refusal should name the successor address: {reason}"
    );
    assert!(
        reason.contains("not followed automatically"),
        "refusal should say the successor is not followed: {reason}"
    );
    let requested = mock.served_paths();
    assert!(
        !requested.iter().any(|p| p.starts_with("/e2")),
        "no request may go to the successor address, got {requested:?}"
    );
}

// ---- case 5: a 200 body that does not parse is a refusal ----

#[test]
fn probe_refuses_on_a_handshake_body_that_does_not_parse() {
    let canned = CannedFiles {
        repomd_status: 200,
        by_name: HashMap::new(),
        hello: Some(serde_json::json!({"greeting": "not a handshake"})),
    };
    let mock = spawn_mock(canned);
    let outcome = IndexClient::probe(&mock.base_url, IndexAuth::None);
    let reason = match outcome {
        ProbeOutcome::Refused { reason } => reason,
        other => panic!("expected Refused, got {other:?}"),
    };
    assert!(
        reason.contains("does not parse"),
        "refusal should say the body does not parse: {reason}"
    );
    let url = format!("{}/hello.json", mock.base_url.trim_end_matches('/'));
    assert!(
        reason.contains(&url),
        "refusal should name where the broken handshake lives: {reason}"
    );
    // No repomd fallback: a broken handshake is loud, not hidden
    // behind "compatibility".
    assert!(
        !mock.served_paths().iter().any(|p| p.contains("repomd")),
        "a broken handshake must not fall back to repomd: {:?}",
        mock.served_paths()
    );
}

// ---- case 6: a handshake format newer than this build ----

#[test]
fn probe_refuses_on_an_unknown_handshake_format() {
    let canned = CannedFiles {
        repomd_status: 200,
        by_name: HashMap::new(),
        hello: Some(serde_json::json!({
            "vibe": "hello/99",
            "worlds": [{"epoch": own_epoch(), "path": "."}],
        })),
    };
    let mock = spawn_mock(canned);
    let outcome = IndexClient::probe(&mock.base_url, IndexAuth::None);
    let reason = match outcome {
        ProbeOutcome::Refused { reason } => reason,
        other => panic!("expected Refused, got {other:?}"),
    };
    assert!(
        reason.contains("hello/99"),
        "refusal should name the offered handshake format: {reason}"
    );
    assert!(
        reason.contains(&expected_vibe()),
        "refusal should name the format this build reads: {reason}"
    );
}

// ---- the cheap extra: a world path other than "." refines the base ----

#[test]
fn probe_refines_the_base_by_a_non_dot_world_path() {
    let mut canned = CannedFiles {
        repomd_status: 200,
        by_name: HashMap::new(),
        hello: Some(serde_json::json!({
            "vibe": expected_vibe(),
            "worlds": [{"epoch": own_epoch(), "path": "e2"}],
        })),
    };
    canned.by_name.insert(
        "wal".into(),
        Some(name_entry_json(
            "org.vibevm",
            PackageKind::Flow,
            "wal",
            &["1.0.0"],
        )),
    );
    let mock = spawn_mock(canned);
    let outcome = IndexClient::probe(&mock.base_url, IndexAuth::None);
    let client = match outcome {
        ProbeOutcome::Found(c) => c,
        other => panic!("expected Found, got {other:?}"),
    };
    let refined = format!("{}/e2", mock.base_url.trim_end_matches('/'));
    assert_eq!(client.file_base(), refined);
    let org = Group::parse("org.vibevm").unwrap();
    let versions = client.list_versions(&org, "wal").unwrap().unwrap();
    assert_eq!(
        versions.iter().map(|v| v.to_string()).collect::<Vec<_>>(),
        vec!["1.0.0".to_string()]
    );
    assert!(
        mock.served_paths()
            .iter()
            .any(|p| p == "/e2/by-name/wal.json")
    );
}
