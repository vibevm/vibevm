use super::*;

use std::collections::VecDeque;
use std::sync::{Arc, Mutex, mpsc};
use std::thread;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::extract::State;
use axum::http::{Request, Response, StatusCode};
use axum::routing::any;
use serde_json::{Value, json};
use tokio::net::TcpListener;
use vibe_core::cancellation::CancellationToken;

#[test]
fn push_url_is_ssh_and_never_carries_the_token() {
    let token = Token::from_explicit("test-token-please-redact");
    let creator = GitverseRepoCreator::new(token, "vibespecs").unwrap();
    let org = creator.validate_scope("vibespecs").unwrap();
    let url = creator.push_url(&org, "flow-wal");
    assert_eq!(url, "git@gitverse.ru:vibespecs/flow-wal.git");
    assert!(!url.contains("test-token-please-redact"));
}

#[test]
fn host_name_is_gitverse_ru_by_default() {
    let token = Token::from_explicit("ignored");
    let creator = GitverseRepoCreator::new(token, "vibespecs").unwrap();
    assert_eq!(creator.host_name(), "gitverse.ru");
    assert_eq!(creator.expected_org(), Some("vibespecs"));
}

/// One recorded request: enough to pin the endpoint, the two required
/// GitVerse headers and the JSON body shape without a network call.
struct RecordedRequest {
    method: String,
    path: String,
    authorization: Option<String>,
    accept: Option<String>,
    body: Value,
}

#[derive(Default)]
struct MockStateInner {
    responses: VecDeque<(StatusCode, Value)>,
    requests: Vec<RecordedRequest>,
}

#[derive(Clone)]
struct MockState(Arc<Mutex<MockStateInner>>);

fn header(request: &Request<Body>, name: &str) -> Option<String> {
    request
        .headers()
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string)
}

async fn mock_handler(State(state): State<MockState>, request: Request<Body>) -> Response<Body> {
    let method = request.method().to_string();
    let path = request.uri().path().to_string();
    let authorization = header(&request, "authorization");
    let accept = header(&request, "accept");
    let body = to_bytes(request.into_body(), usize::MAX).await.unwrap();
    let body = serde_json::from_slice(&body).unwrap_or(Value::Null);
    let mut state = state.0.lock().unwrap();
    state.requests.push(RecordedRequest {
        method,
        path,
        authorization,
        accept,
        body,
    });
    let (status, response) = state.responses.pop_front().unwrap_or((
        StatusCode::INTERNAL_SERVER_ERROR,
        json!({"message": "mock response queue exhausted"}),
    ));
    Response::builder()
        .status(status)
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&response).unwrap()))
        .unwrap()
}

struct MockGitverse {
    base: String,
    state: Arc<Mutex<MockStateInner>>,
    _thread: thread::JoinHandle<()>,
}

impl MockGitverse {
    fn spawn() -> Self {
        let state = Arc::new(Mutex::new(MockStateInner::default()));
        let server_state = state.clone();
        let (sender, receiver) = mpsc::channel();
        let thread = thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            runtime.block_on(async move {
                let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
                sender
                    .send(format!("http://{}", listener.local_addr().unwrap()))
                    .unwrap();
                let app = Router::new()
                    .fallback(any(mock_handler))
                    .with_state(MockState(server_state));
                axum::serve(listener, app).await.unwrap();
            });
        });
        Self {
            base: receiver.recv().unwrap(),
            state,
            _thread: thread,
        }
    }

    fn respond(&self, status: StatusCode, response: Value) {
        self.state
            .lock()
            .unwrap()
            .responses
            .push_back((status, response));
    }

    fn creator(&self, token: &str) -> GitverseRepoCreator {
        GitverseRepoCreator::with_endpoint(
            Token::from_explicit(token),
            "vibevm",
            &self.base,
            "gitverse.test",
        )
        .unwrap()
    }

    fn request_count(&self) -> usize {
        self.state.lock().unwrap().requests.len()
    }
}

fn opts() -> CreateOpts {
    CreateOpts {
        description: Some("WAL discipline flow".to_string()),
        default_branch: Some("main".to_string()),
        homepage: None,
    }
}

#[test]
fn repo_exists_with_control_reports_200_as_present() {
    let mock = MockGitverse::spawn();
    mock.respond(StatusCode::OK, json!({"name": "org.vibevm.wal"}));
    let creator = mock.creator("presence-token");
    let org = creator.validate_scope("vibevm").unwrap();

    let found = creator
        .repo_exists_controlled(&org, "org.vibevm.wal", &CancellationToken::new())
        .unwrap();

    assert!(found);
    let state = mock.state.lock().unwrap();
    let request = &state.requests[0];
    assert_eq!(request.method, "GET");
    assert_eq!(request.path, "/repos/vibevm/org.vibevm.wal");
    assert_eq!(
        request.authorization.as_deref(),
        Some("Bearer presence-token")
    );
    assert_eq!(request.accept.as_deref(), Some(GITVERSE_ACCEPT));
}

#[test]
fn repo_exists_with_control_reports_404_as_absent() {
    let mock = MockGitverse::spawn();
    mock.respond(StatusCode::NOT_FOUND, json!({"message": "Not Found"}));
    let creator = mock.creator("presence-token");
    let org = creator.validate_scope("vibevm").unwrap();

    let found = creator
        .repo_exists_controlled(&org, "org.vibevm.missing", &CancellationToken::new())
        .unwrap();

    assert!(!found);
    assert_eq!(mock.request_count(), 1);
}

#[test]
fn create_repo_with_control_201_returns_the_clone_url() {
    let mock = MockGitverse::spawn();
    mock.respond(
        StatusCode::CREATED,
        json!({
            "clone_url": "https://gitverse.test/vibevm/org.vibevm.wal.git",
            "html_url": "https://gitverse.test/vibevm/org.vibevm.wal"
        }),
    );
    let creator = mock.creator("create-token");
    let org = creator.validate_scope("vibevm").unwrap();

    let info = creator
        .create_repo_controlled(&org, "org.vibevm.wal", &opts(), &CancellationToken::new())
        .unwrap();

    assert_eq!(
        info.clone_url,
        "https://gitverse.test/vibevm/org.vibevm.wal.git"
    );
    assert_eq!(info.html_url, "https://gitverse.test/vibevm/org.vibevm.wal");
    let state = mock.state.lock().unwrap();
    let request = &state.requests[0];
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, "/orgs/vibevm/repos");
    assert_eq!(request.accept.as_deref(), Some(GITVERSE_ACCEPT));
    assert_eq!(request.body["name"], "org.vibevm.wal");
    assert_eq!(request.body["private"], false);
    assert_eq!(request.body["default_branch"], "main");
    assert_eq!(request.body["auto_init"], false);
}

#[test]
fn create_repo_with_control_403_is_an_auth_refusal() {
    let mock = MockGitverse::spawn();
    mock.respond(StatusCode::FORBIDDEN, json!({"message": "Forbidden"}));
    let creator = mock.creator("weak-token");
    let org = creator.validate_scope("vibevm").unwrap();

    let error = creator
        .create_repo_controlled(&org, "org.vibevm.wal", &opts(), &CancellationToken::new())
        .unwrap_err();

    match error {
        PublishError::AuthForbidden { host, org } => {
            assert_eq!(host, "gitverse.test");
            assert_eq!(org, "vibevm");
        }
        other => panic!("expected AuthForbidden, got {other:?}"),
    }
}

#[test]
fn repo_exists_with_control_403_is_an_auth_refusal() {
    let mock = MockGitverse::spawn();
    mock.respond(StatusCode::FORBIDDEN, json!({"message": "Forbidden"}));
    let creator = mock.creator("weak-token");
    let org = creator.validate_scope("vibevm").unwrap();

    let error = creator
        .repo_exists_controlled(&org, "org.vibevm.wal", &CancellationToken::new())
        .unwrap_err();

    assert!(matches!(error, PublishError::AuthForbidden { .. }));
}

/// A token already cancelled when the adapter is entered must stop at the
/// checkpoint the managed client takes *before* contact — the mock sees no
/// request at all, for presence and for creation alike.
#[test]
fn a_cancelled_token_starts_no_request() {
    let mock = MockGitverse::spawn();
    let creator = mock.creator("never-sent");
    let org = creator.validate_scope("vibevm").unwrap();
    let cancelled = CancellationToken::new();
    cancelled.cancel();

    let presence = creator
        .repo_exists_controlled(&org, "org.vibevm.wal", &cancelled)
        .unwrap_err();
    let creation = creator
        .create_repo_controlled(&org, "org.vibevm.wal", &opts(), &cancelled)
        .unwrap_err();

    assert_eq!(mock.request_count(), 0, "no request may leave the adapter");
    for error in [&presence, &creation] {
        match error {
            PublishError::HttpFailed { host, message } => {
                assert_eq!(host, "gitverse.test");
                assert!(
                    message.contains("cancelled"),
                    "unexpected message: {message}"
                );
            }
            other => panic!("expected HttpFailed, got {other:?}"),
        }
    }
}

/// The GitHub adapter is the reference implementation of cooperative
/// control; a stop must read identically on both hosts, so the refusal
/// carried out of the managed client is compared message-for-message.
#[test]
fn a_cancelled_token_refuses_exactly_as_the_github_adapter_does() {
    fn message(error: &PublishError) -> String {
        match error {
            PublishError::HttpFailed { message, .. } => message.clone(),
            other => panic!("expected HttpFailed, got {other:?}"),
        }
    }

    let cancelled = CancellationToken::new();
    cancelled.cancel();

    // Nothing may be contacted here: a port-1 loopback endpoint would fail
    // as `HostUnreachable`, so a refusal that is not the cooperative stop
    // fails this test rather than passing vacuously.
    let unreachable = "http://127.0.0.1:1";
    let gitverse = GitverseRepoCreator::with_endpoint(
        Token::from_explicit("x"),
        "vibevm",
        unreachable,
        "gitverse.test",
    )
    .unwrap();
    let github = crate::GithubRepoCreator::with_endpoint(
        Token::from_explicit("x"),
        "vibevm",
        unreachable,
        "github.test",
    )
    .unwrap();
    let org = gitverse.validate_scope("vibevm").unwrap();

    assert_eq!(
        message(
            &gitverse
                .repo_exists_controlled(&org, "repo", &cancelled)
                .unwrap_err()
        ),
        message(
            &github
                .repo_exists_controlled(&org, "repo", &cancelled)
                .unwrap_err()
        ),
    );
    assert_eq!(
        message(
            &gitverse
                .create_repo_controlled(&org, "repo", &opts(), &cancelled)
                .unwrap_err()
        ),
        message(
            &github
                .create_repo_controlled(&org, "repo", &opts(), &cancelled)
                .unwrap_err()
        ),
    );
}
