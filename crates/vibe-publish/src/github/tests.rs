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

#[test]
fn push_url_embeds_token_for_https() {
    let token = Token::from_explicit("test-token-please-redact");
    let creator = GithubRepoCreator::new(token, "vibespecs").unwrap();
    let org = creator.validate_scope("vibespecs").unwrap();
    let url = creator.push_url(&org, "flow-wal");
    assert_eq!(
        url,
        "https://x-access-token:test-token-please-redact@github.com/vibespecs/flow-wal.git"
    );
}

#[test]
fn push_url_does_not_appear_in_creator_debug() {
    let token = Token::from_explicit("super-secret-do-not-leak");
    let dbg = format!("{token:?}");
    assert!(!dbg.contains("super-secret-do-not-leak"));
    assert!(dbg.contains("***"));
}

#[test]
fn expected_org_is_set_at_construction() {
    let token = Token::from_explicit("ignored");
    let creator = GithubRepoCreator::new(token, "my-org").unwrap();
    assert_eq!(creator.expected_org(), Some("my-org"));
}

#[test]
fn validate_scope_passes_for_matching_org() {
    let token = Token::from_explicit("ignored");
    let creator = GithubRepoCreator::new(token, "vibespecs").unwrap();
    assert!(creator.validate_scope("vibespecs").is_ok());
}

#[test]
fn validate_scope_blocks_user_namespace() {
    let token = Token::from_explicit("ignored");
    let creator = GithubRepoCreator::new(token, "vibespecs").unwrap();
    let err = creator
        .validate_scope("some-other-user")
        .expect_err("scope guard must fire");
    assert!(matches!(err, PublishError::ScopeViolation { .. }));
}

#[test]
fn host_name_is_github_com_by_default() {
    let token = Token::from_explicit("ignored");
    let creator = GithubRepoCreator::new(token, "vibespecs").unwrap();
    assert_eq!(creator.host_name(), "github.com");
}

#[derive(Default)]
struct MockStateInner {
    responses: VecDeque<(StatusCode, Value)>,
    requests: Vec<Value>,
}

#[derive(Clone)]
struct MockState(Arc<Mutex<MockStateInner>>);

async fn mock_handler(State(state): State<MockState>, request: Request<Body>) -> Response<Body> {
    let body = to_bytes(request.into_body(), usize::MAX).await.unwrap();
    let request = serde_json::from_slice(&body).unwrap_or(Value::Null);
    let mut state = state.0.lock().unwrap();
    state.requests.push(request);
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

struct MockGithub {
    base: String,
    state: Arc<Mutex<MockStateInner>>,
    _thread: thread::JoinHandle<()>,
}

impl MockGithub {
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

    fn success(&self, name: &str) {
        self.respond(
            StatusCode::CREATED,
            json!({
                "clone_url": format!("https://github.test/vibevm/{name}.git"),
                "html_url": format!("https://github.test/vibevm/{name}")
            }),
        );
    }

    fn creator(&self, token: &str) -> GithubRepoCreator {
        GithubRepoCreator::with_endpoint(
            Token::from_explicit(token),
            "vibevm",
            &self.base,
            "github.test",
        )
        .unwrap()
    }
}

fn create(
    creator: &GithubRepoCreator,
    name: &str,
    description: Option<String>,
    homepage: Option<String>,
) -> Result<RepoInfo, PublishError> {
    let org = creator.validate_scope("vibevm").unwrap();
    creator.create_repo(
        &org,
        name,
        &CreateOpts {
            description,
            homepage,
            default_branch: Some("main".to_string()),
        },
    )
}

#[test]
fn create_repo_bounds_multibyte_description_and_preserves_ordinary_text() {
    let mock = MockGithub::spawn();
    mock.success("long");
    mock.success("ordinary");
    let creator = mock.creator("test-token");

    let long = "🦀".repeat(400);
    create(&creator, "long", Some(long), None).unwrap();
    let ordinary = "A short repository description.";
    create(&creator, "ordinary", Some(ordinary.to_string()), None).unwrap();

    let state = mock.state.lock().unwrap();
    let shortened = state.requests[0]["description"].as_str().unwrap();
    assert_eq!(shortened.chars().count(), GITHUB_DESCRIPTION_MAX_CHARS);
    assert!(shortened.ends_with('…'));
    assert_eq!(
        shortened
            .chars()
            .filter(|character| *character == '🦀')
            .count(),
        349
    );
    assert_eq!(state.requests[1]["description"], ordinary);
}

#[test]
fn create_repo_422_already_exists_is_an_actionable_race() {
    let mock = MockGithub::spawn();
    mock.respond(
        StatusCode::UNPROCESSABLE_ENTITY,
        json!({
            "message": "Validation Failed",
            "errors": [{
                "resource": "Repository",
                "field": "name",
                "code": "custom",
                "message": "name already exists on this account"
            }]
        }),
    );
    let creator = mock.creator("race-token");
    let error = create(&creator, "existing", None, None)
        .unwrap_err()
        .to_string();
    assert!(error.contains("created concurrently"));
    assert!(error.contains("Re-run `vibe registry publish`"));
    assert!(!error.contains("race-token"));
}

#[test]
fn create_repo_422_other_validation_errors_are_not_called_races_and_are_redacted() {
    let mock = MockGithub::spawn();
    let token = "validation-secret";
    for (field, code, message) in [
        ("description", "too_long", "description is too long"),
        (
            "homepage",
            "invalid",
            "invalid https://publisher:password@example.test URL",
        ),
        ("name", "invalid", "validation-secret is not a valid name"),
    ] {
        mock.respond(
            StatusCode::UNPROCESSABLE_ENTITY,
            json!({
                "message": "Validation Failed",
                "errors": [{
                    "resource": "Repository",
                    "field": field,
                    "code": code,
                    "message": message
                }]
            }),
        );
    }
    let creator = mock.creator(token);
    for field in ["description", "homepage", "name"] {
        let rendered = create(&creator, "invalid", None, None)
            .unwrap_err()
            .to_string();
        assert!(rendered.contains(&format!("field={field}")));
        assert!(rendered.contains("Check the repository name"));
        assert!(!rendered.contains("created concurrently"));
        assert!(!rendered.contains(token));
        assert!(!rendered.contains("password"));
    }
}
