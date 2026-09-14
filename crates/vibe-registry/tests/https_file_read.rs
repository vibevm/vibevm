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

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::Duration;

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
    /// `Retry-After`, verbatim, when this answer carries one.
    retry_after: Option<String>,
}

impl Answer {
    fn ok(body: &[u8]) -> Self {
        Answer {
            status: 200,
            body: body.to_vec(),
            retry_after: None,
        }
    }
    fn status(status: u16) -> Self {
        Answer {
            status,
            body: Vec::new(),
            retry_after: None,
        }
    }
    fn json(status: u16, body: &str) -> Self {
        Answer {
            status,
            body: body.as_bytes().to_vec(),
            retry_after: None,
        }
    }
    /// Send this answer with a `Retry-After` of `value`.
    fn after(mut self, value: &str) -> Self {
        self.retry_after = Some(value.to_string());
        self
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
    /// Answers for the first requests to a target, in order — how a host
    /// that refuses once and then serves is written down.
    scripted: Arc<Mutex<HashMap<String, VecDeque<Answer>>>>,
    answers: Arc<Mutex<HashMap<String, Answer>>>,
    fallback: Arc<Mutex<Answer>>,
    /// Every pause the read decided to take, in order, instead of taking
    /// it (see [`Mock::backend`]).
    pauses: Arc<Mutex<Vec<Duration>>>,
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
    let scripted = state
        .scripted
        .lock()
        .unwrap()
        .get_mut(&target)
        .and_then(VecDeque::pop_front);
    let answer = scripted.unwrap_or_else(|| {
        state
            .answers
            .lock()
            .unwrap()
            .get(&target)
            .cloned()
            .unwrap_or_else(|| state.fallback.lock().unwrap().clone())
    });
    let mut response = (
        StatusCode::from_u16(answer.status).expect("test status code"),
        answer.body,
    )
        .into_response();
    if let Some(value) = answer.retry_after {
        response
            .headers_mut()
            .insert(header::RETRY_AFTER, value.parse().expect("test header"));
    }
    response
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
            scripted: Arc::new(Mutex::new(HashMap::new())),
            answers: Arc::new(Mutex::new(HashMap::new())),
            fallback: Arc::new(Mutex::new(fallback)),
            pauses: Arc::new(Mutex::new(Vec::new())),
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

    /// Answer the first requests to `target` with `answers`, in order;
    /// once that script runs out, the ordinary answer serves.
    fn script(&self, target: &str, answers: Vec<Answer>) -> &Self {
        self.state
            .scripted
            .lock()
            .unwrap()
            .insert(target.to_string(), answers.into());
        self
    }

    fn seen(&self) -> Vec<SeenRequest> {
        self.state.seen.lock().unwrap().clone()
    }

    /// What the read decided to wait, in order. The schedule is asserted
    /// from here rather than from a stopwatch: no test in this file ever
    /// sleeps.
    fn pauses(&self) -> Vec<Duration> {
        self.state.pauses.lock().unwrap().clone()
    }

    /// A backend that reads from this mock, records its pauses instead
    /// of taking them, and cannot spawn git.
    fn backend(&self) -> ShellGit {
        let pauses = Arc::clone(&self.state.pauses);
        ShellGit::new()
            .with_binary(NO_GIT)
            .with_raw_base(&self.base_url)
            .with_raw_sleeper(move |waited| pauses.lock().unwrap().push(waited))
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
    r = 3
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
    r = 3
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
    // A 500 is the "we cannot tell" case: the read asks again, a bounded
    // few times, and then falls back to git. Every one of those asks is
    // a fresh request, so the secrecy rule has to hold for each — and
    // nothing along the way may carry the secret anywhere but the
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
    assert_eq!(seen.len(), 3, "a 5xx is asked again before git is");
    for request in &seen {
        assert_eq!(
            request.authorization.as_deref(),
            Some(format!("Bearer {TOKEN}").as_str()),
            "the token must travel as a bearer header"
        );
        assert!(
            !request.target.contains(TOKEN),
            "the token must not reach the request URL: {}",
            request.target
        );
    }
    assert!(
        !err.to_string().contains(TOKEN),
        "the token must not reach an error message: {err}"
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-000#token-secrecy")]
fn a_credentialed_read_that_misses_carries_no_token_into_its_error() {
    // A 404 on the manifest is the miss that falls through to git, so
    // this walks both halves of the read: the https attempt, which sends
    // the token as a header and nothing more, and the git path, which
    // builds the error. Nothing an operator would see may hold it.
    let mock = Mock::start(Answer::status(404));

    let err = mock
        .backend()
        .fetch_file_at_ref(
            &format!("https://x-access-token:{TOKEN}@github.com/vibespecs/org.vibevm.wal.git"),
            "v1.0.0",
            "vibe.toml",
        )
        .expect_err("a manifest 404 falls through to git, which is not installed here");
    assert!(
        !err.to_string().contains(TOKEN),
        "the token must not reach an error message: {err}"
    );

    let seen = mock.seen();
    assert_eq!(seen.len(), 1);
    assert_eq!(
        seen[0].authorization.as_deref(),
        Some(format!("Bearer {TOKEN}").as_str())
    );
    assert!(
        !seen[0].target.contains(TOKEN),
        "request URL: {}",
        seen[0].target
    );
}

#[test]
#[verifies(
    "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#perf",
    r = 3
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
    r = 3
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

/// The address the mock serves the manifest from, for the tests below.
const MANIFEST_TARGET: &str = "/vibespecs/org.vibevm.wal/v1.0.0/vibe.toml";

#[test]
#[verifies(
    "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#perf",
    r = 3
)]
fn a_rate_limited_read_asks_again_instead_of_paying_for_git() {
    // The measured case: one 429 in a resolve walk used to cost the
    // whole archive → clone → fetch → checkout ladder. A pause and a
    // second ask serve the file instead — and serve it without git,
    // which the bogus binary proves.
    let mock = Mock::start(Answer::status(404));
    mock.script(
        MANIFEST_TARGET,
        vec![Answer::status(429), Answer::ok(MANIFEST)],
    );

    let bytes = mock
        .backend()
        .fetch_file_at_ref(
            "https://github.com/vibespecs/org.vibevm.wal.git",
            "v1.0.0",
            "vibe.toml",
        )
        .expect("the second ask was served");
    assert_eq!(bytes, MANIFEST);
    assert_eq!(mock.seen().len(), 2, "one refusal, then one answer");
    assert_eq!(mock.pauses(), vec![Duration::from_millis(500)]);
}

#[test]
fn a_host_that_keeps_faulting_is_left_to_git() {
    // Three asks, two pauses, and then exactly the behaviour that was
    // there before any of this: the ladder, and its diagnosis.
    let mock = Mock::start(Answer::status(503));

    let err = mock
        .backend()
        .fetch_file_at_ref(
            "https://github.com/vibespecs/org.vibevm.wal.git",
            "v1.0.0",
            "vibe.toml",
        )
        .expect_err("a host that never answers falls through to git");
    assert!(
        matches!(err, GitError::NotInstalled),
        "the read must end in the git path, got: {err:?}"
    );
    assert_eq!(mock.seen().len(), 3, "the read is bounded at three asks");
    assert_eq!(
        mock.pauses(),
        vec![Duration::from_millis(500), Duration::from_secs(1)],
        "the pause doubles, and there is none after the last ask"
    );
}

#[test]
fn a_retry_after_sets_the_pause_the_host_asked_for() {
    // The host knows its own limit better than a constant does, so its
    // number replaces the schedule's — within the bound this path keeps.
    let mock = Mock::start(Answer::status(404));
    mock.script(
        MANIFEST_TARGET,
        vec![Answer::status(429).after("2"), Answer::ok(MANIFEST)],
    );

    let bytes = mock
        .backend()
        .fetch_file_at_ref(
            "https://github.com/vibespecs/org.vibevm.wal.git",
            "v1.0.0",
            "vibe.toml",
        )
        .expect("the second ask was served");
    assert_eq!(bytes, MANIFEST);
    assert_eq!(
        mock.pauses(),
        vec![Duration::from_secs(2)],
        "the header's two seconds, not the schedule's half"
    );
}

#[test]
fn an_answer_about_the_file_is_never_asked_for_twice() {
    // A 404 is an answer, not a refusal to answer. Whether it ends as a
    // reported absence (the marker, at a tag) or as a question for git
    // (the manifest), it is asked exactly once and waits for nothing.
    let marker = Mock::start(Answer::status(404));
    let err = marker
        .backend()
        .fetch_file_at_ref(
            "https://github.com/vibespecs/org.vibevm.wal.git",
            "v1.0.0",
            "vibe-redirect.toml",
        )
        .expect_err("a 404 on the marker is an absent marker");
    assert!(matches!(err, GitError::FileNotFoundInRef { .. }), "{err:?}");
    assert_eq!(marker.seen().len(), 1);
    assert!(marker.pauses().is_empty(), "a miss waits for nothing");

    let manifest = Mock::start(Answer::status(404));
    let err = manifest
        .backend()
        .fetch_file_at_ref(
            "https://github.com/vibespecs/org.vibevm.wal.git",
            "v1.0.0",
            "vibe.toml",
        )
        .expect_err("a manifest 404 is git's to diagnose");
    assert!(matches!(err, GitError::NotInstalled), "{err:?}");
    assert_eq!(manifest.seen().len(), 1);
    assert!(manifest.pauses().is_empty());
}

#[test]
#[verifies(
    "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#perf",
    r = 3
)]
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
