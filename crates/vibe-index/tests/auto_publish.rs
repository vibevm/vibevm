//! `--auto-commit-push` end-to-end coverage. Each git-requiring test
//! builds its own working copy (a real `git init` of the data
//! directory, optionally wired to a bare remote) and drives the
//! mutating routes through axum's `oneshot` dispatcher. Preflight
//! (acceptance points 2 and 3) and the empty-diff success (point 6)
//! are covered as unit tests inside `src/publish.rs`; this file owns
//! the server-level behaviour: the commit message names the change
//! (point 4), a push failure keeps the request alive and moves the
//! counter (point 5), and the flag-off server never runs git (point 7).

use std::path::{Path, PathBuf};
use std::process::Command;

use axum::body::{Body, to_bytes};
use axum::http::{Method, Request, StatusCode, header};
use chrono::{DateTime, Utc};
use tower::util::ServiceExt;

use vibe_index::index::Index;
use vibe_index::index::memory::WriteCtx;
use vibe_index::server::{AppState, FileTokenStore, build_app};
use vibe_index::types::{
    BootSnippetEntry, NamingConvention, PackageKind, ProvidesEntry, VersionEntry,
};

const TOKEN: &str = "topsecret";

fn git_available() -> bool {
    Command::new("git").arg("--version").output().is_ok()
}

fn git_at(dir: &Path, args: &[&str]) {
    let s = Command::new("git")
        .current_dir(dir)
        .args(args)
        .status()
        .unwrap_or_else(|e| panic!("git {:?} in {}: {e}", args, dir.display()));
    assert!(s.success(), "git {:?} failed in {}", args, dir.display());
}

fn git_out(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .current_dir(dir)
        .args(args)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {:?} failed in {}: {}",
        args,
        dir.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn now() -> DateTime<Utc> {
    DateTime::parse_from_rfc3339("2026-05-06T12:00:00Z")
        .unwrap()
        .with_timezone(&Utc)
}

fn entry(kind: PackageKind, name: &str, version: &str) -> VersionEntry {
    VersionEntry {
        schema_version: VersionEntry::SCHEMA_VERSION,
        kind,
        group: "org.vibevm".parse().unwrap(),
        name: name.into(),
        version: version.parse().unwrap(),
        content_hash: format!("sha256:{name}{version}"),
        source_url: format!("https://example.invalid/{name}.git"),
        source_ref: format!("v{version}"),
        resolved_commit: None,
        registry: "vibespecs".into(),
        workspace_origin: None,
        license: Some("EULA".into()),
        authors: vec![],
        description: Some(format!("{name} package")),
        homepage: None,
        keywords: vec![name.into()],
        describes: None,
        compatibility: Default::default(),
        provides: ProvidesEntry::default(),
        requires: Default::default(),
        requires_any: vec![],
        obsoletes: Default::default(),
        conflicts: Default::default(),
        features: Default::default(),
        subskills: vec![],
        i18n: Default::default(),
        boot_snippet: Some(BootSnippetEntry {
            source: format!("boot/{name}.md"),
            category: None,
        }),
        files_count: 1,
        must_understand: vec![],
        yanked: false,
        frozen: false,
        indexed_at: now(),
        indexed_by: "vibe-index 0.1.0-dev".into(),
    }
}

/// Scratch dir holding `data` (a git working copy initialised as an
/// index) and, when `with_remote`, a bare `remote.git` wired as origin
/// with `main` tracking it. The returned `TempDir` owns both.
fn setup(with_remote: bool) -> (tempfile::TempDir, PathBuf, Option<PathBuf>) {
    let scratch = tempfile::tempdir().unwrap();
    let data = scratch.path().join("data");
    std::fs::create_dir_all(&data).unwrap();

    // A minimal valid index on disk so AppState can load it.
    let idx = Index::new(
        "vibespecs",
        "https://example.invalid/vibespecs",
        NamingConvention::Fqdn,
        now(),
    );
    idx.write_to(&data, &WriteCtx { at: now() }).unwrap();
    std::fs::write(data.join(".gitignore"), "/state/\n").unwrap();
    let state_dir = data.join("state");
    std::fs::create_dir_all(&state_dir).unwrap();
    std::fs::write(state_dir.join("admin.tokens"), format!("{TOKEN}\n")).unwrap();

    git_at(&data, &["init", "--quiet", "-b", "main"]);
    git_at(&data, &["config", "user.email", "index@test.invalid"]);
    git_at(&data, &["config", "user.name", "vibe-index test"]);
    git_at(&data, &["add", "-A"]);
    git_at(&data, &["commit", "--quiet", "-m", "initial"]);

    let remote = if with_remote {
        let r = scratch.path().join("remote.git");
        std::fs::create_dir_all(&r).unwrap();
        let s = Command::new("git")
            .args([
                "init",
                "--bare",
                "--quiet",
                "-b",
                "main",
                r.to_str().unwrap(),
            ])
            .status()
            .unwrap();
        assert!(s.success(), "git init --bare failed");
        git_at(&data, &["remote", "add", "origin", r.to_str().unwrap()]);
        git_at(&data, &["push", "--quiet", "-u", "origin", "main"]);
        Some(r)
    } else {
        None
    };

    (scratch, data, remote)
}

fn build(data: &Path, auto: bool) -> axum::Router {
    let idx = Index::load_from(data).unwrap();
    let tokens = FileTokenStore::load(data).unwrap();
    let state =
        AppState::with_tokens(data.to_path_buf(), false, idx, tokens).with_auto_commit_push(auto);
    build_app(state)
}

fn req_post(uri: &str, body: serde_json::Value) -> Request<Body> {
    Request::builder()
        .method(Method::POST)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {TOKEN}"))
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap()
}

fn req_delete(uri: &str) -> Request<Body> {
    Request::builder()
        .method(Method::DELETE)
        .uri(uri)
        .header(header::AUTHORIZATION, format!("Bearer {TOKEN}"))
        .body(Body::empty())
        .unwrap()
}

fn req_get(uri: &str) -> Request<Body> {
    Request::builder()
        .method(Method::GET)
        .uri(uri)
        .body(Body::empty())
        .unwrap()
}

async fn drain(resp: axum::response::Response) {
    let _ = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
}

async fn metrics_body(app: axum::Router) -> String {
    let resp = app.oneshot(req_get("/metrics")).await.unwrap();
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    String::from_utf8(bytes.to_vec()).unwrap()
}

/// Acceptance point 4: a successful upsert publishes, the commit
/// message names the change, and the bare remote receives it (Р1).
#[tokio::test]
async fn upsert_publishes_with_named_commit_and_pushes_to_remote() {
    if !git_available() {
        return;
    }
    let (_scratch, data, remote) = setup(true);
    let app = build(&data, true);
    let resp = app
        .oneshot(req_post(
            "/v1/packages",
            serde_json::to_value(entry(PackageKind::Flow, "wal", "0.1.0")).unwrap(),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    drain(resp).await;

    let expect = "index: upsert org.vibevm/wal@0.1.0";
    assert_eq!(git_out(&data, &["log", "--format=%s", "-n", "1"]), expect);
    assert_eq!(
        git_out(remote.as_ref().unwrap(), &["log", "--format=%s", "-n", "1"]),
        expect,
        "remote did not receive the published commit"
    );
}

/// Acceptance point 4 (deletes): remove-version and remove-package each
/// publish with a message that names the change.
#[tokio::test]
async fn delete_routes_publish_remove_messages() {
    if !git_available() {
        return;
    }
    let (_scratch, data, _remote) = setup(true);
    let app = build(&data, true);
    for v in ["0.1.0", "0.2.0"] {
        let r = app
            .clone()
            .oneshot(req_post(
                "/v1/packages",
                serde_json::to_value(entry(PackageKind::Flow, "wal", v)).unwrap(),
            ))
            .await
            .unwrap();
        assert_eq!(r.status(), StatusCode::CREATED);
        drain(r).await;
    }
    let r = app
        .clone()
        .oneshot(req_delete("/v1/packages/org.vibevm/wal/0.1.0"))
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    drain(r).await;
    assert_eq!(
        git_out(&data, &["log", "--format=%s", "-n", "1"]),
        "index: remove org.vibevm/wal@0.1.0"
    );

    let r = app
        .oneshot(req_delete("/v1/packages/org.vibevm/wal"))
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    drain(r).await;
    assert_eq!(
        git_out(&data, &["log", "--format=%s", "-n", "1"]),
        "index: remove org.vibevm/wal"
    );
}

/// F2-3: an identical repeat upsert creates no second commit. One
/// real change ⇒ one commit; the catalog's history records events
/// that actually happened, and the repeat still answers success —
/// the resource is already in the requested state, which is exactly
/// what idempotency means over HTTP.
#[tokio::test]
async fn identical_repeat_upsert_publishes_exactly_one_commit() {
    if !git_available() {
        return;
    }
    let (_scratch, data, _remote) = setup(true);
    let app = build(&data, true);
    let body = serde_json::to_value(entry(PackageKind::Flow, "wal", "0.1.0")).unwrap();
    // First POST creates; the identical repeat re-asserts the same
    // state and must still succeed (200, `created: false`).
    for expected in [StatusCode::CREATED, StatusCode::OK] {
        let resp = app
            .clone()
            .oneshot(req_post("/v1/packages", body.clone()))
            .await
            .unwrap();
        assert_eq!(resp.status(), expected);
        drain(resp).await;
    }
    // Exactly ONE commit beyond the initial one, naming the change.
    assert_eq!(
        git_out(&data, &["rev-list", "--count", "HEAD"]),
        "2",
        "the identical repeat must not add a commit"
    );
    let upsert_commits = git_out(&data, &["log", "--format=%s"])
        .lines()
        .filter(|l| *l == "index: upsert org.vibevm/wal@0.1.0")
        .count();
    assert_eq!(upsert_commits, 1, "one real change, one commit");
}

/// Acceptance point 5 / Р4: a push failure (no upstream configured)
/// does not drop the request, the local commit still stands, and the
/// failure counter moves.
#[tokio::test]
async fn push_failure_keeps_request_alive_and_counts() {
    if !git_available() {
        return;
    }
    let (_scratch, data, _remote) = setup(false); // no remote ⇒ push fails
    let app = build(&data, true);
    let resp = app
        .clone()
        .oneshot(req_post(
            "/v1/packages",
            serde_json::to_value(entry(PackageKind::Flow, "wal", "0.1.0")).unwrap(),
        ))
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::CREATED,
        "push failure must not 500"
    );
    drain(resp).await;
    // The mutation + local commit stand even though the push failed.
    assert_eq!(
        git_out(&data, &["log", "--format=%s", "-n", "1"]),
        "index: upsert org.vibevm/wal@0.1.0"
    );
    let metrics = metrics_body(app).await;
    assert!(
        metrics.contains("vibe_index_publish_failures_total 1"),
        "expected the failure counter at 1, got: {metrics}"
    );
}

/// Acceptance point 7: with the flag off, a successful mutation runs no
/// git — the working copy's HEAD is unchanged.
#[tokio::test]
async fn flag_off_runs_no_git() {
    if !git_available() {
        return;
    }
    let (_scratch, data, _remote) = setup(true);
    let app = build(&data, false);
    let resp = app
        .oneshot(req_post(
            "/v1/packages",
            serde_json::to_value(entry(PackageKind::Flow, "wal", "0.1.0")).unwrap(),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    drain(resp).await;
    // No commit was made — HEAD is still the initial commit.
    assert_eq!(
        git_out(&data, &["log", "--format=%s", "-n", "1"]),
        "initial"
    );
}
