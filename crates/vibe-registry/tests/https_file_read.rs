//! End-to-end coverage of the HTTPS single-file read that
//! `ShellGit::fetch_file_at_ref` tries before `git archive` (PROP-002
//! §2.12). One mock axum server stands in for both hosts the read knows:
//! it serves GitHub's raw bytes and GitVerse's base64 `contents` JSON on
//! the same root, since the two are told apart by the shape of the
//! request, not by the address of the server.
//!
//! Every test here binds the backend to a `git` binary that does not
//! exist. That is the proof, and it is an exact one: a read that returns
//! anything other than [`GitError::NotInstalled`] cannot have reached
//! git, and a read that returns `NotInstalled` can only have got there by
//! falling through to the git path deliberately.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, mpsc};
use std::thread;

use axum::Router;
use axum::extract::State;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use specmark::verifies;
use tokio::net::TcpListener;

use vibe_registry::git_backend::GitBackend;
use vibe_registry::{GitError, ShellGit};

/// A `git` that is not on anyone's PATH.
const NO_GIT: &str = "definitely-not-git-xyz";

/// The token used wherever a test drives the credentialed URL shape.
const TOKEN: &str = "ghp_secret_value";

#[derive(Clone)]
struct Answer {
    status: u16,
    body: Vec<u8>,
}

impl Answer {
    fn ok(body: &[u8]) -> Self {
        Answer {
            status: 200,
            body: body.to_vec(),
        }
    }
    fn status(status: u16) -> Self {
        Answer {
            status,
            body: Vec::new(),
        }
    }
    fn json(status: u16, body: &str) -> Self {
        Answer {
            status,
            body: body.as_bytes().to_vec(),
        }
    }
}

/// What the mock was asked for, and with what credentials.
#[derive(Clone)]
struct SeenRequest {
    target: String,
    authorization: Option<String>,
}

#[derive(Clone)]
struct MockState {
    seen: Arc<Mutex<Vec<SeenRequest>>>,
    answers: Arc<Mutex<HashMap<String, Answer>>>,
    fallback: Arc<Mutex<Answer>>,
}

async fn handler(State(state): State<MockState>, req: axum::extract::Request) -> Response {
    let target = req
        .uri()
        .path_and_query()
        .map(|pq| pq.to_string())
        .unwrap_or_default();
    let authorization = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);
    state.seen.lock().unwrap().push(SeenRequest {
        target: target.clone(),
        authorization,
    });
    let answer = state
        .answers
        .lock()
        .unwrap()
        .get(&target)
        .cloned()
        .unwrap_or_else(|| state.fallback.lock().unwrap().clone());
    (
        StatusCode::from_u16(answer.status).expect("test status code"),
        answer.body,
    )
        .into_response()
}

struct Mock {
    base_url: String,
    state: MockState,
    _thread: thread::JoinHandle<()>,
}

impl Mock {
    /// Start a mock whose every unscripted request gets `fallback`.
    fn start(fallback: Answer) -> Mock {
        let state = MockState {
            seen: Arc::new(Mutex::new(Vec::new())),
            answers: Arc::new(Mutex::new(HashMap::new())),
            fallback: Arc::new(Mutex::new(fallback)),
        };
        let state_for_thread = state.clone();
        let (tx, rx) = mpsc::channel();
        let handle = thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(async move {
                let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
                let addr = listener.local_addr().unwrap();
                let app = Router::new().fallback(handler).with_state(state_for_thread);
                tx.send(format!("http://{addr}")).unwrap();
                axum::serve(listener, app).await.unwrap();
            });
        });
        Mock {
            base_url: rx.recv().unwrap(),
            state,
            _thread: handle,
        }
    }

    fn answer(&self, target: &str, answer: Answer) -> &Self {
        self.state
            .answers
            .lock()
            .unwrap()
            .insert(target.to_string(), answer);
        self
    }

    fn seen(&self) -> Vec<SeenRequest> {
        self.state.seen.lock().unwrap().clone()
    }

    /// A backend that reads from this mock and cannot spawn git.
    fn backend(&self) -> ShellGit {
        ShellGit::new()
            .with_binary(NO_GIT)
            .with_raw_base(&self.base_url)
    }
}

/// The mock's answer for a GitVerse file read.
fn gitverse_file(base64: &str) -> Answer {
    Answer::json(
        200,
        &format!(r#"{{"encoding":"base64","content":"{base64}","size":10,"path":"vibe.toml"}}"#),
    )
}

/// GitVerse's miss, which travels inside a 400.
fn gitverse_missing_file() -> Answer {
    Answer::json(
        400,
        r#"{"code":4305,"message":"File: vibe.toml not found"}"#,
    )
}

const MANIFEST: &[u8] = b"[package]\ngroup = \"org.vibevm\"\nname = \"wal\"\n";
/// `MANIFEST`, base64-encoded, wrapped the way the route wraps it.
const MANIFEST_BASE64: &str = "W3BhY2thZ2Vd\\nCmdyb3VwID0gIm9yZy52aWJldm0iCm5hbWUgPSAid2FsIgo=";

#[test]
#[verifies(
    "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#perf",
    r = 1
)]
fn github_manifest_is_read_over_https_without_git() {
    let mock = Mock::start(Answer::status(404));
    mock.answer(
        "/vibespecs/org.vibevm.wal/v1.0.0/vibe.toml",
        Answer::ok(MANIFEST),
    );

    let bytes = mock
        .backend()
        .fetch_file_at_ref(
            "https://github.com/vibespecs/org.vibevm.wal.git",
            "v1.0.0",
            "vibe.toml",
        )
        .expect("the raw host served the manifest");
    assert_eq!(bytes, MANIFEST);

    // One request, at the address the mapping promises — and no git, or
    // the bogus binary would have surfaced instead of these bytes.
    let seen = mock.seen();
    assert_eq!(seen.len(), 1, "expected exactly one read");
    assert_eq!(seen[0].target, "/vibespecs/org.vibevm.wal/v1.0.0/vibe.toml");
    assert!(
        seen[0].authorization.is_none(),
        "a public read sends no token"
    );
}

#[test]
#[verifies(
    "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#perf",
    r = 1
)]
fn github_absent_marker_at_a_tag_is_answered_without_git() {
    // The ordinary redirect probe: no marker, at a release tag. This is
    // the case that used to cost a refused archive plus a full clone,
    // for every package in the graph.
    let mock = Mock::start(Answer::status(404));

    let err = mock
        .backend()
        .fetch_file_at_ref(
            "https://github.com/vibespecs/org.vibevm.wal.git",
            "v1.0.0",
            "vibe-redirect.toml",
        )
        .expect_err("a 404 on the marker is an absent marker");
    match &err {
        GitError::FileNotFoundInRef { url, refname, path } => {
            assert_eq!(url, "https://github.com/vibespecs/org.vibevm.wal.git");
            assert_eq!(refname, "v1.0.0");
            assert_eq!(path, "vibe-redirect.toml");
        }
        other => panic!("expected FileNotFoundInRef without git, got: {other:?}"),
    }
    assert_eq!(mock.seen().len(), 1);
}

#[test]
#[verifies(
    "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#failure-discriminator",
    r = 1
)]
fn github_absent_manifest_falls_back_to_git() {
    // A 404 on `vibe.toml` may be a missing file, a missing ref, a
    // missing repository or an invisible one. Only git says which, so
    // the read must go there — proved here by the bogus binary
    // answering, which it can only do from inside the git path.
    let mock = Mock::start(Answer::status(404));

    let err = mock
        .backend()
        .fetch_file_at_ref(
            "https://github.com/vibespecs/org.vibevm.wal.git",
            "v1.0.0",
            "vibe.toml",
        )
        .expect_err("a manifest 404 is not an answer");
    assert!(
        matches!(err, GitError::NotInstalled),
        "the manifest read must reach the git path, got: {err:?}"
    );
    assert_eq!(mock.seen().len(), 1, "the https read was still attempted");
}

#[test]
#[verifies(
    "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#failure-discriminator",
    r = 1
)]
fn github_absent_marker_on_a_branch_falls_back_to_git() {
    // A branch is a moving tip, and both `[requires.packages]` git
    // sources and `vibe install --git … --branch <b>` read one. A miss
    // there says nothing trustworthy about the file, so it goes to git
    // as well.
    let mock = Mock::start(Answer::status(404));

    let err = mock
        .backend()
        .fetch_file_at_ref(
            "https://github.com/vibespecs/org.vibevm.wal.git",
            "main",
            "vibe-redirect.toml",
        )
        .expect_err("a miss on a moving ref is not an answer");
    assert!(
        matches!(err, GitError::NotInstalled),
        "a branch miss must reach the git path, got: {err:?}"
    );
    assert_eq!(
        mock.seen()[0].target,
        "/vibespecs/org.vibevm.wal/main/vibe-redirect.toml"
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-000#token-secrecy")]
fn a_token_rides_the_header_and_appears_in_no_error() {
    // A 500 is the "we cannot tell" case: the read falls back to git,
    // and nothing along the way may carry the secret anywhere but the
    // Authorization header.
    let mock = Mock::start(Answer::status(500));

    let err = mock
        .backend()
        .fetch_file_at_ref(
            &format!("https://x-access-token:{TOKEN}@github.com/vibespecs/org.vibevm.wal.git"),
            "v1.0.0",
            "vibe.toml",
        )
        .expect_err("a 500 falls back to git, which is not installed here");

    let seen = mock.seen();
    assert_eq!(seen.len(), 1);
    assert_eq!(
        seen[0].authorization.as_deref(),
        Some(format!("Bearer {TOKEN}").as_str()),
        "the token must travel as a bearer header"
    );
    assert!(
        !seen[0].target.contains(TOKEN),
        "the token must not reach the request URL: {}",
        seen[0].target
    );
    assert!(
        !err.to_string().contains(TOKEN),
        "the token must not reach an error message: {err}"
    );
}

#[test]
#[verifies(
    "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#perf",
    r = 1
)]
fn gitverse_manifest_is_read_over_https_without_git() {
    let mock = Mock::start(gitverse_missing_file());
    mock.answer(
        "/api/repos/vibespecs/org.vibevm.wal/contents/vibe.toml?ref=v1.0.0",
        gitverse_file(MANIFEST_BASE64),
    );

    let bytes = mock
        .backend()
        .fetch_file_at_ref(
            "https://gitverse.ru/vibespecs/org.vibevm.wal.git",
            "v1.0.0",
            "vibe.toml",
        )
        .expect("the contents route served the manifest");
    assert_eq!(bytes, MANIFEST);
    assert_eq!(mock.seen().len(), 1);
}

#[test]
#[verifies(
    "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#perf",
    r = 1
)]
fn gitverse_reports_an_absent_marker_but_only_for_its_own_miss_code() {
    // 4305 — the file is not there. Answerable at a tag, without git.
    let mock = Mock::start(gitverse_missing_file());
    let err = mock
        .backend()
        .fetch_file_at_ref(
            "https://gitverse.ru/vibespecs/org.vibevm.wal.git",
            "v1.0.0",
            "vibe-redirect.toml",
        )
        .expect_err("4305 is an absent marker");
    assert!(
        matches!(err, GitError::FileNotFoundInRef { .. }),
        "expected FileNotFoundInRef without git, got: {err:?}"
    );

    // 4004 — a complaint about the repository, not about the file. That
    // is git's to diagnose.
    let other = Mock::start(Answer::json(
        400,
        r#"{"code":4004,"message":"doesn't have repository"}"#,
    ));
    let err = other
        .backend()
        .fetch_file_at_ref(
            "https://gitverse.ru/vibespecs/org.vibevm.wal.git",
            "v1.0.0",
            "vibe-redirect.toml",
        )
        .expect_err("4004 is not an answer about the file");
    assert!(
        matches!(err, GitError::NotInstalled),
        "a non-miss 400 must reach the git path, got: {err:?}"
    );
}

#[test]
fn hosts_and_shapes_outside_the_fast_path_never_reach_the_read() {
    let mock = Mock::start(Answer::ok(MANIFEST));
    for url in [
        // ssh spellings of the very hosts the read knows.
        "git@github.com:vibespecs/org.vibevm.wal.git",
        "ssh://git@gitverse.ru/vibespecs/org.vibevm.wal.git",
        // a host with no row in the table.
        "https://gitlab.example/vibespecs/org.vibevm.wal.git",
        // a local registry.
        "file:///tmp/registry/org.vibevm.wal",
    ] {
        let err = mock
            .backend()
            .fetch_file_at_ref(url, "v1.0.0", "vibe.toml")
            .expect_err("these all belong to git, which is not installed here");
        assert!(
            matches!(err, GitError::NotInstalled),
            "{url} must go straight to git, got: {err:?}"
        );
    }
    assert!(
        mock.seen().is_empty(),
        "no read may be attempted for these: {:?}",
        mock.seen().iter().map(|r| &r.target).collect::<Vec<_>>()
    );
}
