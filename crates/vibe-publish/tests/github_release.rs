use std::collections::VecDeque;
use std::sync::{Arc, Mutex, mpsc};
use std::thread;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::extract::State;
use axum::http::{HeaderMap, Request, Response, StatusCode};
use axum::routing::any;
use serde_json::{Value, json};
use tokio::net::TcpListener;
use vibe_publish::{
    CreateGithubRelease, GithubReleaseClient, GithubReleaseError, Token, UpdateGithubRelease,
    sha256_digest,
};

struct PlannedResponse {
    status: StatusCode,
    body: Vec<u8>,
    content_type: &'static str,
}

#[derive(Debug)]
struct CapturedRequest {
    method: String,
    uri: String,
    headers: HeaderMap,
    body: Vec<u8>,
}

#[derive(Default)]
struct MockStateInner {
    responses: VecDeque<PlannedResponse>,
    requests: Vec<CapturedRequest>,
}

#[derive(Clone)]
struct MockState(Arc<Mutex<MockStateInner>>);

async fn handle(State(state): State<MockState>, request: Request<Body>) -> Response<Body> {
    let (parts, body) = request.into_parts();
    let body = to_bytes(body, usize::MAX).await.unwrap().to_vec();
    let mut state = state.0.lock().unwrap();
    state.requests.push(CapturedRequest {
        method: parts.method.to_string(),
        uri: parts.uri.to_string(),
        headers: parts.headers,
        body,
    });
    let planned = state.responses.pop_front().unwrap_or(PlannedResponse {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        body: br#"{"message":"mock response queue exhausted"}"#.to_vec(),
        content_type: "application/json",
    });
    Response::builder()
        .status(planned.status)
        .header("content-type", planned.content_type)
        .body(Body::from(planned.body))
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
        let handle = thread::spawn(move || {
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
                    .fallback(any(handle))
                    .with_state(MockState(server_state));
                axum::serve(listener, app).await.unwrap();
            });
        });
        Self {
            base: receiver.recv().unwrap(),
            state,
            _thread: handle,
        }
    }

    fn json(&self, status: StatusCode, body: Value) {
        self.state
            .lock()
            .unwrap()
            .responses
            .push_back(PlannedResponse {
                status,
                body: serde_json::to_vec(&body).unwrap(),
                content_type: "application/json",
            });
    }

    fn bytes(&self, status: StatusCode, body: &[u8]) {
        self.state
            .lock()
            .unwrap()
            .responses
            .push_back(PlannedResponse {
                status,
                body: body.to_vec(),
                content_type: "application/octet-stream",
            });
    }

    fn client(&self, token: &str) -> GithubReleaseClient {
        GithubReleaseClient::with_endpoints(
            Token::from_explicit(token),
            "vibevm",
            "vibe",
            &format!("{}/api", self.base),
            &format!("{}/uploads", self.base),
        )
        .unwrap()
    }

    fn anonymous_client(&self) -> GithubReleaseClient {
        GithubReleaseClient::anonymous_with_endpoints(
            "vibevm",
            "vibe",
            &format!("{}/api", self.base),
            &format!("{}/uploads", self.base),
        )
        .unwrap()
    }
}

fn release(id: u64, tag: &str) -> Value {
    json!({
        "id": id,
        "tag_name": tag,
        "target_commitish": "abc123",
        "name": tag,
        "body": "",
        "draft": false,
        "prerelease": false,
        "html_url": format!("https://example.test/releases/{tag}"),
        "upload_url": "https://uploads.example.test/template{?name,label}"
    })
}

fn asset(id: u64, name: &str, bytes: &[u8], download_url: &str) -> Value {
    json!({
        "id": id,
        "name": name,
        "size": bytes.len(),
        "digest": sha256_digest(bytes),
        "browser_download_url": download_url,
        "content_type": "application/octet-stream"
    })
}

#[test]
fn create_find_list_update_release_and_force_move_tag() {
    let mock = MockGithub::spawn();
    let client = mock.client("release-token");
    mock.json(StatusCode::OK, release(7, "v1.2.3"));
    mock.json(StatusCode::CREATED, release(8, "v1.2.3"));
    mock.json(
        StatusCode::OK,
        json!([asset(
            10,
            "vibe.zip",
            b"zip",
            "https://example.test/vibe.zip"
        )]),
    );
    mock.json(StatusCode::OK, release(8, "v1.2.3"));
    mock.json(
        StatusCode::OK,
        json!({
            "ref": "refs/tags/v1.2.3",
            "object": { "sha": "def456", "type": "commit", "url": "https://example.test/object" }
        }),
    );

    assert_eq!(client.find_release("v1.2.3").unwrap().unwrap().id, 7);
    let request = CreateGithubRelease::for_version(&"1.2.3".parse().unwrap(), "abc123");
    assert_eq!(client.create_release(&request).unwrap().id, 8);
    assert_eq!(client.list_assets(8).unwrap().len(), 1);
    let update = UpdateGithubRelease {
        body: Some("replacement".to_string()),
        target_commitish: Some("def456".to_string()),
        ..UpdateGithubRelease::default()
    };
    assert_eq!(client.update_release(8, &update).unwrap().id, 8);
    assert_eq!(
        client
            .force_move_tag("v1.2.3", "def456")
            .unwrap()
            .object
            .sha,
        "def456"
    );

    let state = mock.state.lock().unwrap();
    let requests = &state.requests;
    assert_eq!(requests.len(), 5);
    assert_eq!(requests[0].method, "GET");
    assert_eq!(
        requests[0].uri,
        "/api/repos/vibevm/vibe/releases/tags/v1.2.3"
    );
    assert_eq!(requests[1].method, "POST");
    assert_eq!(requests[1].uri, "/api/repos/vibevm/vibe/releases");
    let create_body: Value = serde_json::from_slice(&requests[1].body).unwrap();
    assert_eq!(create_body["tag_name"], "v1.2.3");
    assert_eq!(create_body["target_commitish"], "abc123");
    assert_eq!(
        requests[2].uri,
        "/api/repos/vibevm/vibe/releases/8/assets?per_page=100"
    );
    assert_eq!(requests[3].method, "PATCH");
    let move_body: Value = serde_json::from_slice(&requests[4].body).unwrap();
    assert_eq!(
        requests[4].uri,
        "/api/repos/vibevm/vibe/git/refs/tags/v1.2.3"
    );
    assert_eq!(move_body, json!({"sha": "def456", "force": true}));
    for (index, request) in requests.iter().enumerate() {
        if matches!(index, 0 | 2) {
            assert!(request.headers.get("authorization").is_none());
        } else {
            assert_eq!(request.headers["authorization"], "Bearer release-token");
        }
        assert_eq!(request.headers["x-github-api-version"], "2022-11-28");
    }
}

#[test]
fn upload_public_download_rename_and_delete_use_the_right_auth_boundaries() {
    let mock = MockGithub::spawn();
    let client = mock.client("release-token");
    let bytes = b"bundle bytes";
    let public_url = format!("{}/downloads/bundle", mock.base);
    mock.json(
        StatusCode::CREATED,
        asset(20, "temporary.zip", bytes, &public_url),
    );
    mock.bytes(StatusCode::OK, bytes);
    mock.json(StatusCode::OK, asset(20, "vibe.zip", bytes, &public_url));
    mock.bytes(StatusCode::NO_CONTENT, b"");

    let uploaded = client
        .upload_asset(3, "temporary.zip", "application/zip", bytes)
        .unwrap();
    assert_eq!(client.download_asset(uploaded.id).unwrap(), bytes);
    assert_eq!(
        client.rename_asset(20, "vibe.zip").unwrap().name,
        "vibe.zip"
    );
    client.delete_asset(20).unwrap();

    let state = mock.state.lock().unwrap();
    assert_eq!(state.requests[0].method, "POST");
    assert_eq!(
        state.requests[0].uri,
        "/uploads/repos/vibevm/vibe/releases/3/assets?name=temporary.zip"
    );
    assert_eq!(state.requests[0].body, bytes);
    assert_eq!(state.requests[0].headers["content-type"], "application/zip");
    assert_eq!(
        state.requests[1].uri,
        "/api/repos/vibevm/vibe/releases/assets/20"
    );
    assert!(state.requests[1].headers.get("authorization").is_none());
    assert_eq!(state.requests[2].method, "PATCH");
    assert_eq!(state.requests[3].method, "DELETE");
}

#[test]
fn upsert_release_updates_an_existing_mutable_release() {
    let mock = MockGithub::spawn();
    let client = mock.client("release-token");
    mock.json(StatusCode::OK, release(40, "v2.0.0"));
    let mut updated = release(40, "v2.0.0");
    updated["target_commitish"] = json!("replacement-commit");
    mock.json(StatusCode::OK, updated);

    let request = CreateGithubRelease::for_version(&"2.0.0".parse().unwrap(), "replacement-commit");
    let outcome = client.upsert_release(&request).unwrap();
    assert_eq!(outcome.id, 40);
    assert_eq!(outcome.target_commitish, "replacement-commit");

    let state = mock.state.lock().unwrap();
    assert_eq!(state.requests.len(), 2);
    assert_eq!(state.requests[0].method, "GET");
    assert_eq!(state.requests[1].method, "PATCH");
    assert_eq!(state.requests[1].uri, "/api/repos/vibevm/vibe/releases/40");
}

#[test]
fn replace_uploads_verifies_deletes_old_then_renames() {
    let mock = MockGithub::spawn();
    let client = mock.client("release-token");
    let old = b"old";
    let new = b"new bundle";
    let public_url = format!("{}/downloads/vibe.zip", mock.base);
    mock.json(
        StatusCode::OK,
        json!([asset(21, "vibe.zip", old, &public_url)]),
    );
    mock.json(
        StatusCode::CREATED,
        asset(22, "temporary", new, &public_url),
    );
    mock.bytes(StatusCode::NO_CONTENT, b"");
    mock.json(StatusCode::OK, asset(22, "vibe.zip", new, &public_url));

    let replaced = client
        .publish_asset(4, "vibe.zip", "application/zip", new)
        .unwrap();
    assert_eq!(replaced.id, 22);
    assert_eq!(replaced.name, "vibe.zip");

    let state = mock.state.lock().unwrap();
    assert_eq!(state.requests.len(), 4);
    assert_eq!(state.requests[0].method, "GET");
    assert_eq!(state.requests[1].method, "POST");
    assert!(state.requests[1].uri.contains("name=.vibe-upload-"));
    assert_eq!(
        state.requests[2].uri,
        "/api/repos/vibevm/vibe/releases/assets/21"
    );
    assert_eq!(state.requests[2].method, "DELETE");
    assert_eq!(state.requests[3].method, "PATCH");
    let rename_body: Value = serde_json::from_slice(&state.requests[3].body).unwrap();
    assert_eq!(rename_body, json!({"name": "vibe.zip"}));
}

#[test]
fn replacement_verification_failure_keeps_the_old_asset() {
    let mock = MockGithub::spawn();
    let client = mock.client("release-token");
    let public_url = format!("{}/downloads/vibe.zip", mock.base);
    mock.json(
        StatusCode::OK,
        json!([asset(30, "vibe.zip", b"old", &public_url)]),
    );
    let mut wrong = asset(31, "temporary", b"new", &public_url);
    wrong["digest"] = json!(sha256_digest(b"different"));
    mock.json(StatusCode::CREATED, wrong);

    let error = client
        .replace_asset(4, "vibe.zip", "application/zip", b"new")
        .unwrap_err();
    assert!(matches!(
        error,
        GithubReleaseError::AssetVerification { .. }
    ));
    let state = mock.state.lock().unwrap();
    assert_eq!(state.requests.len(), 2, "old asset must not be deleted");
}

#[test]
fn not_found_and_unprocessable_are_typed_and_redacted() {
    let mock = MockGithub::spawn();
    let token = "secret-release-token";
    let client = mock.client(token);
    mock.json(StatusCode::NOT_FOUND, json!({"message": "Not Found"}));
    assert!(client.find_release("v9.9.9").unwrap().is_none());

    mock.json(StatusCode::NOT_FOUND, json!({"message": "gone"}));
    assert!(matches!(
        client.delete_asset(404),
        Err(GithubReleaseError::NotFound { .. })
    ));

    mock.json(
        StatusCode::UNPROCESSABLE_ENTITY,
        json!({
            "message": format!("bad {token} at https://publisher:password@example.test/release")
        }),
    );
    let request = CreateGithubRelease::for_version(&"1.0.0".parse().unwrap(), "abc123");
    let rendered = client.create_release(&request).unwrap_err().to_string();
    assert!(rendered.contains("422"));
    assert!(!rendered.contains(token));
    assert!(!rendered.contains("password"));
    assert!(rendered.contains("https://***@example.test/release"));
}

#[test]
fn anonymous_client_discovers_lists_and_downloads_by_exact_asset_id() {
    let mock = MockGithub::spawn();
    let client = mock.anonymous_client();
    mock.json(StatusCode::OK, release(50, "v3.0.0"));
    mock.json(
        StatusCode::OK,
        json!([asset(51, "vibe.zip", b"bundle", "ignored")]),
    );
    mock.bytes(StatusCode::OK, b"bundle");

    assert_eq!(client.find_release("v3.0.0").unwrap().unwrap().id, 50);
    assert_eq!(client.list_assets(50).unwrap()[0].id, 51);
    assert_eq!(client.download_asset(51).unwrap(), b"bundle");

    let state = mock.state.lock().unwrap();
    assert_eq!(state.requests.len(), 3);
    assert_eq!(
        state.requests[2].uri,
        "/api/repos/vibevm/vibe/releases/assets/51"
    );
    assert_eq!(
        state.requests[2].headers["accept"],
        "application/octet-stream"
    );
    for request in &state.requests {
        assert!(request.headers.get("authorization").is_none());
    }
}

#[test]
fn authenticated_read_variants_discover_list_and_download_draft_assets() {
    let mock = MockGithub::spawn();
    let client = mock.client("draft-token");
    let mut draft = release(60, "v4.0.0");
    draft["draft"] = json!(true);
    mock.json(StatusCode::OK, draft);
    mock.json(
        StatusCode::OK,
        json!([asset(61, "vibe.zip", b"draft bundle", "ignored")]),
    );
    mock.bytes(StatusCode::OK, b"draft bundle");

    assert!(
        client
            .find_release_authenticated("v4.0.0")
            .unwrap()
            .unwrap()
            .draft
    );
    assert_eq!(client.list_assets_authenticated(60).unwrap()[0].id, 61);
    assert_eq!(
        client.download_asset_authenticated(61).unwrap(),
        b"draft bundle"
    );

    let state = mock.state.lock().unwrap();
    assert_eq!(state.requests.len(), 3);
    assert_eq!(
        state.requests[0].uri,
        "/api/repos/vibevm/vibe/releases/tags/v4.0.0"
    );
    assert_eq!(
        state.requests[1].uri,
        "/api/repos/vibevm/vibe/releases/60/assets?per_page=100"
    );
    assert_eq!(
        state.requests[2].uri,
        "/api/repos/vibevm/vibe/releases/assets/61"
    );
    assert_eq!(
        state.requests[2].headers["accept"],
        "application/octet-stream"
    );
    for request in &state.requests {
        assert_eq!(request.headers["authorization"], "Bearer draft-token");
        assert_eq!(request.headers["x-github-api-version"], "2022-11-28");
    }
}

#[test]
fn anonymous_write_refuses_before_sending_a_request() {
    let mock = MockGithub::spawn();
    let client = mock.anonymous_client();
    let request = CreateGithubRelease::for_version(&"1.0.0".parse().unwrap(), "abc123");
    let errors = [
        client.find_release_authenticated("v1.0.0").unwrap_err(),
        client.list_assets_authenticated(1).unwrap_err(),
        client.download_asset_authenticated(1).unwrap_err(),
        client.create_release(&request).unwrap_err(),
        client.upsert_release(&request).unwrap_err(),
        client
            .update_release(1, &UpdateGithubRelease::default())
            .unwrap_err(),
        client
            .upload_asset(1, "vibe.zip", "application/zip", b"bytes")
            .unwrap_err(),
        client.delete_asset(1).unwrap_err(),
        client.rename_asset(1, "vibe.zip").unwrap_err(),
        client
            .publish_asset(1, "vibe.zip", "application/zip", b"bytes")
            .unwrap_err(),
        client.force_move_tag("v1.0.0", "abc123").unwrap_err(),
    ];
    for error in errors {
        assert!(matches!(
            &error,
            GithubReleaseError::AuthenticationRequired { .. }
        ));
        let rendered = error.to_string();
        assert!(rendered.contains("requires a publish token"));
        assert!(rendered.contains("load_token_for_host"));
    }
    assert!(mock.state.lock().unwrap().requests.is_empty());
}

#[test]
fn configured_endpoints_reject_and_redact_user_info() {
    let rendered = GithubReleaseClient::with_endpoints(
        Token::from_explicit("password"),
        "vibevm",
        "vibe",
        "https://publisher:password@example.test/api",
        "https://uploads.example.test",
    )
    .err()
    .expect("credentialed API base must be refused")
    .to_string();
    assert!(!rendered.contains("password"));
    assert!(rendered.contains("https://***@example.test/api"));
}
