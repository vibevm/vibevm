//! End-to-end coverage for `vibe-index reindex --from-github`. A
//! mock GitHub REST API runs in a background thread on a random
//! port; the canned responses point at local bare repositories so
//! `git clone` resolves entirely against the filesystem. No network
//! access required.

use std::path::Path;
use std::process::Command;
use std::sync::mpsc;
use std::thread;

use assert_cmd::Command as AssertCommand;
use axum::Router;
use axum::extract::{Path as AxumPath, Query, State};
use axum::http::{HeaderMap, HeaderValue, header};
use axum::response::IntoResponse;
use axum::routing::get;
use serde::Deserialize;
use tokio::net::TcpListener;

fn cmd() -> AssertCommand {
    AssertCommand::cargo_bin("vibe-index").expect("vibe-index binary built")
}

fn git_available() -> bool {
    Command::new("git").arg("--version").output().is_ok()
}

#[derive(Debug, Clone)]
struct CannedRepo {
    name: String,
    clone_url: String,
}

#[derive(Clone)]
struct MockState {
    pages: Vec<Vec<CannedRepo>>,
    base_url: String,
}

#[derive(Deserialize)]
struct PageQ {
    #[serde(default = "one")]
    page: usize,
}

fn one() -> usize {
    1
}

async fn list_repos_handler(
    State(state): State<MockState>,
    AxumPath(_org): AxumPath<String>,
    Query(q): Query<PageQ>,
) -> impl IntoResponse {
    let pages = &state.pages;
    let idx = q.page.saturating_sub(1);
    let page = pages.get(idx).cloned().unwrap_or_default();
    let body: Vec<serde_json::Value> = page
        .iter()
        .map(|r| {
            serde_json::json!({
                "name": r.name,
                "clone_url": r.clone_url,
                "default_branch": "main",
                "fork": false,
            })
        })
        .collect();
    let mut headers = HeaderMap::new();
    if idx + 1 < pages.len() {
        let next = format!(
            "<{}/orgs/x/repos?page={}>; rel=\"next\"",
            state.base_url,
            idx + 2
        );
        headers.insert(header::LINK, HeaderValue::from_str(&next).unwrap());
    }
    (headers, axum::Json(body))
}

struct MockServer {
    base_url: String,
    _thread: thread::JoinHandle<()>,
}

fn spawn_mock(pages: Vec<Vec<CannedRepo>>) -> MockServer {
    let (tx, rx) = mpsc::channel();
    let handle = thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async move {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let addr = listener.local_addr().unwrap();
            let base_url = format!("http://{addr}");
            let state = MockState {
                pages,
                base_url: base_url.clone(),
            };
            let app = Router::new()
                .route("/orgs/{org}/repos", get(list_repos_handler))
                .with_state(state);
            tx.send(base_url).unwrap();
            axum::serve(listener, app).await.unwrap();
        });
    });
    let base_url = rx.recv().unwrap();
    MockServer {
        base_url,
        _thread: handle,
    }
}

fn make_local_repo(
    parent: &Path,
    dir_name: &str,
    manifests: &[(&str, &str)],
) -> std::path::PathBuf {
    let repo = parent.join(dir_name);
    std::fs::create_dir_all(&repo).unwrap();
    git(&repo, &["init", "--quiet", "-b", "main"]);
    git(&repo, &["config", "user.email", "test@test.invalid"]);
    git(&repo, &["config", "user.name", "Test"]);
    for (tag, body) in manifests {
        std::fs::write(repo.join("vibe.toml"), body).unwrap();
        std::fs::write(repo.join("README.md"), format!("# {tag}\n")).unwrap();
        git(&repo, &["add", "."]);
        git(&repo, &["commit", "--quiet", "-m", tag]);
        git(&repo, &["tag", tag]);
    }
    repo
}

fn git(repo: &Path, args: &[&str]) {
    let status = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .status()
        .expect("git invokable");
    assert!(status.success(), "git {args:?} failed");
}

fn manifest(name: &str, kind: &str, version: &str) -> String {
    format!(
        "[package]\ngroup = \"org.vibevm\"\nname = \"{name}\"\nkind = \"{kind}\"\nversion = \"{version}\"\nlicense = \"EULA\"\n"
    )
}

fn local_clone_url(p: &Path) -> String {
    p.to_string_lossy().replace('\\', "/")
}

#[test]
fn from_github_walks_mock_org_into_index() {
    if !git_available() {
        return;
    }
    let work = tempfile::tempdir().unwrap();
    let upstream = work.path().join("upstream");
    std::fs::create_dir_all(&upstream).unwrap();

    let wal = make_local_repo(
        &upstream,
        "flow-wal",
        &[("v0.1.0", &manifest("wal", "flow", "0.1.0"))],
    );
    let rust = make_local_repo(
        &upstream,
        "stack-rust",
        &[
            ("v0.1.0", &manifest("rust", "stack", "0.1.0")),
            ("v0.2.0", &manifest("rust", "stack", "0.2.0")),
        ],
    );

    let mock = spawn_mock(vec![vec![
        CannedRepo {
            name: "flow-wal".into(),
            clone_url: local_clone_url(&wal),
        },
        CannedRepo {
            name: "stack-rust".into(),
            clone_url: local_clone_url(&rust),
        },
    ]]);

    let data = work.path().join("data");
    cmd()
        .args([
            "init",
            data.to_str().unwrap(),
            "--registry",
            "vibespecs",
            "--registry-url",
            "https://example.invalid/vibespecs",
        ])
        .assert()
        .success();

    let cache = work.path().join("clones");
    let out = cmd()
        .args([
            "reindex",
            data.to_str().unwrap(),
            "--from-github",
            "vibespecs",
            "--api-base",
            &mock.base_url,
            "--clone-cache",
            cache.to_str().unwrap(),
            "--full",
            "--json",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    let summary: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(summary["source"], "github");
    assert_eq!(summary["package_count"], 2);
    assert_eq!(summary["version_count"], 3);

    assert!(data.join("by-name/wal.json").exists());
    let rust_json: serde_json::Value =
        serde_json::from_slice(&std::fs::read(data.join("by-name/rust.json")).unwrap()).unwrap();
    assert_eq!(rust_json["packages"][0]["latest_stable"], "0.2.0");

    // The clone cache survives so a second --from-github would reuse it.
    assert!(cache.join("flow-wal/.git").exists());
    assert!(cache.join("stack-rust/.git").exists());
}

#[test]
fn from_github_follows_link_pagination() {
    if !git_available() {
        return;
    }
    let work = tempfile::tempdir().unwrap();
    let upstream = work.path().join("upstream");
    std::fs::create_dir_all(&upstream).unwrap();

    // 3 repos split across 2 pages: first page has 2, second has 1.
    let r1 = make_local_repo(
        &upstream,
        "flow-a",
        &[("v0.1.0", &manifest("a", "flow", "0.1.0"))],
    );
    let r2 = make_local_repo(
        &upstream,
        "flow-b",
        &[("v0.1.0", &manifest("b", "flow", "0.1.0"))],
    );
    let r3 = make_local_repo(
        &upstream,
        "flow-c",
        &[("v0.1.0", &manifest("c", "flow", "0.1.0"))],
    );

    let mock = spawn_mock(vec![
        vec![
            CannedRepo {
                name: "flow-a".into(),
                clone_url: local_clone_url(&r1),
            },
            CannedRepo {
                name: "flow-b".into(),
                clone_url: local_clone_url(&r2),
            },
        ],
        vec![CannedRepo {
            name: "flow-c".into(),
            clone_url: local_clone_url(&r3),
        }],
    ]);

    let data = work.path().join("data");
    cmd()
        .args([
            "init",
            data.to_str().unwrap(),
            "--registry",
            "vibespecs",
            "--registry-url",
            "https://example.invalid/vibespecs",
        ])
        .assert()
        .success();

    let out = cmd()
        .args([
            "reindex",
            data.to_str().unwrap(),
            "--from-github",
            "vibespecs",
            "--api-base",
            &mock.base_url,
            "--full",
            "--json",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    let summary: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(summary["package_count"], 3);
}

#[test]
fn from_github_surfaces_api_errors() {
    if !git_available() {
        return;
    }
    let work = tempfile::tempdir().unwrap();
    // Mock with no repos at all and bind to a port; we point at a
    // bogus path so the mock returns 404 default-handled by axum.
    let mock = spawn_mock(vec![vec![]]);
    let bogus = format!("{}/bogus", mock.base_url);

    let data = work.path().join("data");
    cmd()
        .args([
            "init",
            data.to_str().unwrap(),
            "--registry",
            "vibespecs",
            "--registry-url",
            "https://example.invalid/vibespecs",
        ])
        .assert()
        .success();
    let out = cmd()
        .args([
            "reindex",
            data.to_str().unwrap(),
            "--from-github",
            "vibespecs",
            "--api-base",
            &bogus,
            "--full",
        ])
        .assert()
        .failure();
    let stderr = String::from_utf8(out.get_output().stderr.clone()).unwrap();
    assert!(
        stderr.contains("GitHub API"),
        "expected GitHub API error mention, got: {stderr}"
    );
}
