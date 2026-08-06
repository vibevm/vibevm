//! A2-INDEXAUTH — index-client authentication coverage. A mock axum
//! server that **captures the `Authorization` header** stands in for a
//! vibe-index host, so the tests can prove which request sites carry a
//! bearer token (and which must not). Covers:
//!
//! - acceptance 1 — a `Bearer` plan attaches `Authorization: Bearer …`
//!   to every request site (probe, `by-name` file route, `/v1` server
//!   route);
//! - acceptance 2 / 3 — `None` plan (and thus any `http://` index,
//!   which `for_registry` maps to `None`) sends no header;
//! - acceptance 4 — `ssh` / `credential-helper` send no header and a
//!   401/403 surfaces `IndexError::AuthIncapable` with guidance;
//! - acceptance 5 — a refused probe (401/403) is distinguishable from
//!   an absent index.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, mpsc};
use std::thread;

use axum::Router;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::get;
use tokio::net::TcpListener;

use vibe_registry::{BearerToken, IndexAuth, IndexClient, IndexError, ProbeOutcome};

#[derive(Default)]
struct Canned {
    /// 0 ⇒ 200 for `/repomd.json`.
    repomd_status: u16,
    /// 0 ⇒ 200 for `/v1/packages`.
    search_status: u16,
    search_body: Option<serde_json::Value>,
    by_name: HashMap<String, serde_json::Value>,
}

#[derive(Clone)]
struct MockState {
    files: Arc<Mutex<Canned>>,
    /// Most recent `Authorization` header value observed on ANY route,
    /// or `None` once a headerless request has arrived. Reset is not
    /// needed: each test spawns its own mock.
    last_auth: Arc<Mutex<Option<String>>>,
}

impl MockState {
    fn capture(&self, headers: &HeaderMap) {
        let value = headers
            .get("authorization")
            .and_then(|h| h.to_str().ok())
            .map(str::to_owned);
        *self.last_auth.lock().unwrap() = value;
    }
}

fn status_of(n: u16) -> StatusCode {
    match n {
        0 => StatusCode::OK,
        n => StatusCode::from_u16(n).unwrap_or(StatusCode::OK),
    }
}

async fn repomd_handler(
    State(state): State<MockState>,
    headers: HeaderMap,
) -> axum::response::Response {
    state.capture(&headers);
    let s = state.files.lock().unwrap().repomd_status;
    (status_of(s), "").into_response()
}

async fn search_handler(
    State(state): State<MockState>,
    headers: HeaderMap,
) -> axum::response::Response {
    state.capture(&headers);
    let files = state.files.lock().unwrap();
    if let Some(body) = files.search_body.clone() {
        return (StatusCode::OK, axum::Json(body)).into_response();
    }
    (status_of(files.search_status), "").into_response()
}

async fn by_name_handler(
    State(state): State<MockState>,
    headers: HeaderMap,
    Path(name_with_ext): Path<String>,
) -> axum::response::Response {
    state.capture(&headers);
    let Some(name) = name_with_ext.strip_suffix(".json") else {
        return (StatusCode::NOT_FOUND, "expected .json").into_response();
    };
    match state.files.lock().unwrap().by_name.get(name).cloned() {
        Some(body) => (StatusCode::OK, axum::Json(body)).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

struct Mock {
    base_url: String,
    last_auth: Arc<Mutex<Option<String>>>,
    _thread: thread::JoinHandle<()>,
}

fn spawn_mock(canned: Canned) -> Mock {
    let last_auth = Arc::new(Mutex::new(None));
    let files = Arc::new(Mutex::new(canned));
    let last_auth_for_thread = last_auth.clone();
    let files_for_thread = files.clone();
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
                last_auth: last_auth_for_thread,
            };
            let app = Router::new()
                .route("/repomd.json", get(repomd_handler))
                .route("/v1/packages", get(search_handler))
                .route("/by-name/{name}", get(by_name_handler))
                .with_state(state);
            tx.send(format!("http://{addr}")).unwrap();
            axum::serve(listener, app).await.unwrap();
        });
    });
    Mock {
        base_url: rx.recv().unwrap(),
        last_auth,
        _thread: handle,
    }
}

fn bearer(token: &str) -> IndexAuth {
    IndexAuth::Bearer(BearerToken::new(token.into()))
}

fn ok_search_body() -> serde_json::Value {
    serde_json::json!({ "query": "wal", "hit_count": 0, "hits": [] })
}

// ---- Р3 layer 2: the attachment itself refuses plaintext ----
//
// The mocks here are `http://`, which is exactly the point: even a
// `Bearer` plan must NOT send the header over a plaintext base. This is
// the road that was holey before the scheme gate moved into the
// attachment (`build_client` checks the base). The positive case — a
// header IS produced for a `Bearer` plan — is covered at the unit level
// (`auth.rs` `header_map_attaches_bearer_authorization`); an https
// end-to-end round-trip would need TLS, which this crate does not
// depend on. The single `build_client` chokepoint carries the produced
// header to all five request sites, so the unit proof + the gate below
// cover "every request carries the token over https".

#[test]
fn bearer_plan_over_http_base_sends_no_authorization_header() {
    // The exact road the review flagged: a public constructor with an
    // `http://` base and a `Bearer` plan. The attachment must refuse —
    // no `Authorization` header reaches the server.
    let canned = Canned {
        search_body: Some(ok_search_body()),
        ..Canned::default()
    };
    let mock = spawn_mock(canned);
    let client = IndexClient::at_with_auth(&mock.base_url, bearer("test-bearer"));
    client.search("wal", None, None).expect("search ok");
    assert!(
        mock.last_auth.lock().unwrap().is_none(),
        "an http:// base must not send Authorization even under a Bearer plan"
    );
}

#[test]
fn bearer_plan_over_http_base_sends_no_header_on_probe() {
    // Same gate, on the probe path: a `Bearer` plan probing an `http://`
    // index attaches nothing.
    let mock = spawn_mock(Canned::default());
    let outcome = IndexClient::probe(&mock.base_url, bearer("probe-tok"));
    assert!(matches!(outcome, ProbeOutcome::Found(_)));
    assert!(
        mock.last_auth.lock().unwrap().is_none(),
        "an http:// probe must not send Authorization even under a Bearer plan"
    );
}

// ---- acceptance 2 / 3: None plan (and http) sends no header ----

#[test]
fn none_plan_sends_no_authorization_header() {
    // `at()` builds a None plan — byte-identical to pre-A2 behaviour.
    // This is also what `for_registry` yields for an `http://` index
    // (a token never travels over plaintext) and for `auth = "none"`.
    let canned = Canned {
        search_body: Some(ok_search_body()),
        ..Canned::default()
    };
    let mock = spawn_mock(canned);
    let client = IndexClient::at(&mock.base_url);
    client.search("wal", None, None).expect("search ok");
    assert!(
        mock.last_auth.lock().unwrap().is_none(),
        "no Authorization header should be sent under a None plan"
    );
}

// ---- acceptance 4: ssh/credential-helper 401 ⇒ AuthIncapable ----

#[test]
fn ssh_plan_401_surfaces_auth_incapable_error() {
    // `ssh` cannot supply HTTP credentials; a 401 must name the regime
    // and the fix, not look like a generic status.
    let canned = Canned {
        search_status: 401,
        ..Canned::default()
    };
    let mock = spawn_mock(canned);
    let client = IndexClient::at_with_auth(&mock.base_url, IndexAuth::HttpIncapable("ssh"));
    let err = client.search("wal", None, None).unwrap_err();
    match err {
        IndexError::AuthIncapable { regime, status, .. } => {
            assert_eq!(regime, "ssh");
            assert_eq!(status, 401);
        }
        other => panic!("expected AuthIncapable, got {other:?}"),
    }
    // The rendered error names the regime and the fix.
    let msg = format!(
        "{}",
        IndexError::AuthIncapable {
            url: String::new(),
            regime: "ssh",
            status: 401,
        }
    );
    assert!(msg.contains("ssh"), "error should name the regime: {msg}");
    assert!(
        msg.contains("token-env"),
        "error should tell the operator to switch to token-env: {msg}"
    );
    // And no header was attached under this regime.
    assert!(
        mock.last_auth.lock().unwrap().is_none(),
        "ssh plan must attach no Authorization header"
    );
}

#[test]
fn ssh_plan_403_also_surfaces_auth_incapable() {
    let canned = Canned {
        search_status: 403,
        ..Canned::default()
    };
    let mock = spawn_mock(canned);
    let client = IndexClient::at_with_auth(
        &mock.base_url,
        IndexAuth::HttpIncapable("credential-helper"),
    );
    let err = client.search("wal", None, None).unwrap_err();
    assert!(matches!(
        err,
        IndexError::AuthIncapable {
            regime: "credential-helper",
            status: 403,
            ..
        }
    ));
}

#[test]
fn none_plan_401_surfaces_plain_status_not_auth_incapable() {
    // A 401 under a genuinely anonymous (None) registry is a plain
    // Status — the regime routing is what upgrades it to AuthIncapable.
    let canned = Canned {
        search_status: 401,
        ..Canned::default()
    };
    let mock = spawn_mock(canned);
    let client = IndexClient::at(&mock.base_url);
    let err = client.search("wal", None, None).unwrap_err();
    assert!(matches!(err, IndexError::Status { status: 401, .. }));
}

// ---- acceptance 5: refused probe ≠ absent probe ----

#[test]
fn probe_refused_401_is_distinguishable_from_absent() {
    // 401 on the probe ⇒ Refused (with guidance), never silent Absent.
    let mock = spawn_mock(Canned {
        repomd_status: 401,
        ..Canned::default()
    });
    let outcome = IndexClient::probe(&mock.base_url, IndexAuth::None);
    match outcome {
        ProbeOutcome::Refused { reason } => {
            assert!(
                reason.contains("token-env"),
                "refusal reason should guide the operator: {reason}"
            );
        }
        other => panic!("expected Refused, got {other:?}"),
    }
}

#[test]
fn probe_absent_on_404_is_not_refused() {
    // 404 on the probe ⇒ Absent (silent fall-through), as before.
    let mock = spawn_mock(Canned {
        repomd_status: 404,
        ..Canned::default()
    });
    let outcome = IndexClient::probe(&mock.base_url, IndexAuth::None);
    assert!(matches!(outcome, ProbeOutcome::Absent));
}
