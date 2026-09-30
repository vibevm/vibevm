//! `IndexClient::search` against a static mirror — the shape of every
//! raw GitHub index, the default `vibespecs` one included: `hello.json`,
//! `repomd.json` and `primary.jsonl` served as files, and no
//! `/v1/packages` route at all. The mock answers 404 to everything it
//! does not serve, exactly as a raw-file host does, and logs every path
//! it was asked for so the test can say what the client did and did not
//! fetch.

use std::sync::{Arc, Mutex, mpsc};
use std::thread;

use axum::Router;
use axum::extract::State;
use axum::http::{StatusCode, Uri};
use axum::response::IntoResponse;
use axum::routing::get;
use specmark::verifies;
use tokio::net::TcpListener;

use vibe_core::PackageKind;
use vibe_registry::{IndexAuth, IndexClient, IndexError, ProbeOutcome};

#[derive(Clone)]
struct MockState {
    /// The catalog to serve; `None` means the mirror has no `primary.jsonl`.
    primary: Option<String>,
    log: Arc<Mutex<Vec<String>>>,
}

impl MockState {
    fn record(&self, path: &str) {
        self.log.lock().unwrap().push(path.to_string());
    }
}

async fn hello_handler(State(state): State<MockState>) -> impl IntoResponse {
    state.record("/hello.json");
    (
        StatusCode::OK,
        axum::Json(serde_json::json!({
            "vibe": "hello/1",
            "worlds": [{"epoch": 1, "path": "."}]
        })),
    )
        .into_response()
}

async fn repomd_handler(State(state): State<MockState>) -> impl IntoResponse {
    state.record("/repomd.json");
    (
        StatusCode::OK,
        axum::Json(serde_json::json!({
            "schema_version": 1,
            "registry": "vibespecs",
            "registry_url": "https://example.invalid",
            "naming": "fqdn",
            "generated_at": "2026-09-30T00:00:00Z",
            "generator": "mock",
            "package_count": 4,
            "version_count": 5,
            "files": {}
        })),
    )
        .into_response()
}

async fn primary_handler(State(state): State<MockState>) -> impl IntoResponse {
    state.record("/primary.jsonl");
    match &state.primary {
        Some(body) => (StatusCode::OK, body.clone()).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn fallback_handler(State(state): State<MockState>, uri: Uri) -> impl IntoResponse {
    state.record(uri.path());
    StatusCode::NOT_FOUND
}

struct Mock {
    base_url: String,
    log: Arc<Mutex<Vec<String>>>,
    _thread: thread::JoinHandle<()>,
}

fn spawn_mirror(primary: Option<String>) -> Mock {
    let log = Arc::new(Mutex::new(Vec::new()));
    let state = MockState {
        primary,
        log: log.clone(),
    };
    let (tx, rx) = mpsc::channel();
    let handle = thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async move {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let addr = listener.local_addr().unwrap();
            let app = Router::new()
                .route("/hello.json", get(hello_handler))
                .route("/repomd.json", get(repomd_handler))
                .route("/primary.jsonl", get(primary_handler))
                .fallback(fallback_handler)
                .with_state(state);
            tx.send(format!("http://{addr}")).unwrap();
            axum::serve(listener, app).await.unwrap();
        });
    });
    Mock {
        base_url: rx.recv().unwrap(),
        log,
        _thread: handle,
    }
}

/// One catalog line with the fields every record carries, plus the
/// ones search reads. `extra` is spliced in verbatim (`"key":value,…`).
fn line(kind: &str, group: &str, name: &str, version: &str, extra: &str) -> String {
    format!(
        r#"{{"schema_version":1,"kind":"{kind}","group":"{group}","name":"{name}","version":"{version}",{extra}"content_hash":"sha256:0","source_url":"https://example.invalid/{group}.{name}","source_ref":"v{version}","registry":"vibespecs","files_count":1,"indexed_at":"2026-09-30T00:00:00Z","indexed_by":"mock"}}"#
    )
}

fn catalog() -> String {
    [
        line(
            "flow",
            "org.example",
            "wal",
            "0.1.0",
            r#""description":"Write-ahead log.","#,
        ),
        line(
            "flow",
            "org.example",
            "wal",
            "0.2.0-beta.1",
            r#""description":"Write-ahead log, second edition.","#,
        ),
        line(
            "feat",
            "org.example",
            "audit-log",
            "0.2.0",
            r#""description":"Append-only audit trail.","keywords":["log"],"#,
        ),
        line(
            "tool",
            "org.example",
            "wal-tools",
            "1.0.0",
            r#""description":"wal helpers","must_understand":["b080-test-capability"],"#,
        ),
        line(
            "flow",
            "org.other",
            "wal",
            "3.0.0",
            r#""description":"Another write-ahead log.","#,
        ),
    ]
    .join("\n")
        + "\n"
}

fn probe(mock: &Mock) -> IndexClient {
    match IndexClient::probe(&mock.base_url, IndexAuth::None) {
        ProbeOutcome::Found(client) => client,
        other => panic!("the mirror must be found through its handshake, got {other:?}"),
    }
}

#[test]
#[verifies(
    "spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#integration",
    r = 1
)]
fn a_static_mirror_is_searched_through_its_catalog() {
    let mock = spawn_mirror(Some(catalog()));
    let client = probe(&mock);

    let results = client
        .search("wal log", None, None)
        .expect("a static mirror answers a search from its catalog");

    assert_eq!(results.query, "wal log");
    assert_eq!(results.hit_count, 3);
    let names: Vec<&str> = results.hits.iter().map(|h| h.name.as_str()).collect();
    assert_eq!(
        names,
        ["wal", "wal", "audit-log"],
        "score, then (group, name)"
    );
    let first = &results.hits[0];
    assert_eq!(first.kind, PackageKind::Flow);
    assert_eq!(first.score, 2);
    assert_eq!(first.matched_tokens, ["log", "wal"]);
    assert_eq!(first.description.as_deref(), Some("Write-ahead log."));
    assert_eq!(
        first.latest_stable.as_ref().unwrap().to_string(),
        "0.1.0",
        "the prerelease is neither scored nor the latest stable"
    );
    assert_eq!(
        results.hits[1].description.as_deref(),
        Some("Another write-ahead log.")
    );
    assert_eq!(results.hits[2].score, 1);
    assert!(
        !results.hits.iter().any(|h| h.name == "wal-tools"),
        "a version this build cannot act on is not searched"
    );

    let log = mock.log.lock().unwrap();
    assert!(
        log.iter().any(|p| p == "/v1/packages"),
        "the route is asked first, and the mirror's 404 is what says «no server»: {log:?}"
    );
    assert!(log.iter().any(|p| p == "/primary.jsonl"), "{log:?}");
}

#[test]
fn kind_and_limit_shape_the_catalog_answer_as_the_route_would() {
    let mock = spawn_mirror(Some(catalog()));
    let client = probe(&mock);

    let feats = client
        .search("wal log", Some(PackageKind::Feat), None)
        .unwrap();
    assert_eq!(feats.hit_count, 1);
    assert_eq!(feats.hits[0].name, "audit-log");

    let one = client.search("wal log", None, Some(1)).unwrap();
    assert_eq!(one.hit_count, 1);
    assert_eq!(one.hits.len(), 1);
    assert_eq!(one.hits[0].description.as_deref(), Some("Write-ahead log."));
}

#[test]
fn a_base_with_neither_route_nor_catalog_keeps_the_route_s_404() {
    let mock = spawn_mirror(None);
    let client = probe(&mock);

    let err = client.search("wal", None, None).unwrap_err();
    match err {
        IndexError::Status { url, status } => {
            assert_eq!(status, 404);
            assert!(url.ends_with("/v1/packages"), "{url}");
        }
        other => panic!("unexpected error variant: {other:?}"),
    }
    let log = mock.log.lock().unwrap();
    assert!(log.iter().any(|p| p == "/primary.jsonl"), "{log:?}");
}
