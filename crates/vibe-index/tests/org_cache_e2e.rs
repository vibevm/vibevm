//! Acceptance coverage for the org-image cache and `rescan-org`
//! (slice A3). A mock GitHub REST API runs in a background thread on
//! a random port; it emits an `ETag` on every 200, answers `304 Not
//! Modified` when the request's `If-None-Match` matches, counts every
//! list request, and logs `(page, had_if_none_match, status)` so a
//! test can prove the host was — or was not — re-walked. Canned
//! `clone_url`s point at local git repos so `git clone` resolves
//! against the filesystem; no network access required.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;

use assert_cmd::Command as AssertCommand;
use axum::Router;
use axum::extract::{Path as AxumPath, Query, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use serde::Deserialize;
use tokio::net::TcpListener;

fn cmd() -> AssertCommand {
    vibe_test_support::cargo_bin("vibe-index")
}

fn git_available() -> bool {
    Command::new("git").arg("--version").output().is_ok()
}

#[derive(Debug, Clone)]
struct CannedRepo {
    name: String,
    clone_url: String,
}

/// One "version" of the org the mock serves. Swapping it between runs
/// simulates an org change (different repos and/or a new ETag).
#[derive(Clone)]
struct OrgVersion {
    etag: String,
    /// When false the mock omits `ETag` on 200s — modelling a host
    /// that gives no validator (ИЗМЕРЬ-2 / acceptance #4).
    emit_etag: bool,
    pages: Vec<Vec<CannedRepo>>,
}

#[derive(Clone, Copy)]
struct ReqRec {
    page: usize,
    had_inm: bool,
    status: u16,
}

#[derive(Clone)]
struct MockState {
    version: Arc<Mutex<OrgVersion>>,
    list_count: Arc<AtomicUsize>,
    log: Arc<Mutex<Vec<ReqRec>>>,
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
    headers: HeaderMap,
) -> Response {
    state.list_count.fetch_add(1, Ordering::SeqCst);
    let page = q.page;
    let idx = page.saturating_sub(1);
    let v = state.version.lock().unwrap().clone();
    let page_repos = v.pages.get(idx).cloned().unwrap_or_default();
    let inm = headers
        .get(header::IF_NONE_MATCH)
        .and_then(|h| h.to_str().ok())
        .map(str::to_owned);
    let had_inm = inm.is_some();
    // The conditional probe is sent on page 1 only.
    if idx == 0 && inm.as_deref() == Some(v.etag.as_str()) {
        push(
            &state.log,
            ReqRec {
                page,
                had_inm,
                status: 304,
            },
        );
        // 304 — no body, no validator re-emitted (GitHub's behaviour).
        return Response::builder()
            .status(StatusCode::NOT_MODIFIED)
            .body(axum::body::Body::empty())
            .unwrap();
    }
    let body: Vec<serde_json::Value> = page_repos
        .iter()
        .map(|r| {
            serde_json::json!({ "name": r.name, "clone_url": r.clone_url, "default_branch": "main", "fork": false })
        })
        .collect();
    let mut resp = axum::Json(body).into_response();
    if v.emit_etag {
        resp.headers_mut()
            .insert(header::ETAG, HeaderValue::from_str(&v.etag).unwrap());
    }
    if idx + 1 < v.pages.len() {
        let next = format!(
            "<{}/orgs/x/repos?page={}>; rel=\"next\"",
            state.base_url,
            idx + 2
        );
        resp.headers_mut()
            .insert(header::LINK, HeaderValue::from_str(&next).unwrap());
    }
    push(
        &state.log,
        ReqRec {
            page,
            had_inm,
            status: 200,
        },
    );
    resp
}

fn push(log: &Arc<Mutex<Vec<ReqRec>>>, rec: ReqRec) {
    log.lock().unwrap().push(rec);
}

struct MockServer {
    base_url: String,
    version: Arc<Mutex<OrgVersion>>,
    list_count: Arc<AtomicUsize>,
    log: Arc<Mutex<Vec<ReqRec>>>,
    _thread: thread::JoinHandle<()>,
}

impl MockServer {
    fn count(&self) -> usize {
        self.list_count.load(Ordering::SeqCst)
    }
    /// Requests logged since the `count` snapshot `since`. The log
    /// grows one entry per request, so a count snapshot is a valid
    /// log index.
    fn log_since(&self, since: usize) -> Vec<ReqRec> {
        self.log.lock().unwrap()[since..].to_vec()
    }
    fn set_version(&self, version: OrgVersion) {
        *self.version.lock().unwrap() = version;
    }
}

fn spawn_mock(version: OrgVersion) -> MockServer {
    let (tx, rx) = mpsc::channel();
    let version = Arc::new(Mutex::new(version));
    let list_count = Arc::new(AtomicUsize::new(0));
    let log: Arc<Mutex<Vec<ReqRec>>> = Arc::new(Mutex::new(Vec::new()));
    let (v_t, c_t, l_t) = (version.clone(), list_count.clone(), log.clone());
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
                version: v_t,
                list_count: c_t,
                log: l_t,
                base_url: base_url.clone(),
            };
            let app = Router::new()
                .route("/orgs/{org}/repos", get(list_repos_handler))
                .with_state(state);
            tx.send(base_url).unwrap();
            axum::serve(listener, app).await.unwrap();
        });
    });
    MockServer {
        base_url: rx.recv().unwrap(),
        version,
        list_count,
        log,
        _thread: handle,
    }
}

// --- helpers -----------------------------------------------------

fn make_local_repo(parent: &Path, dir_name: &str, manifest_body: &str) -> PathBuf {
    let repo = parent.join(dir_name);
    std::fs::create_dir_all(&repo).unwrap();
    git(&repo, &["init", "--quiet", "-b", "main"]);
    git(&repo, &["config", "user.email", "t@t.invalid"]);
    git(&repo, &["config", "user.name", "T"]);
    std::fs::write(repo.join("vibe.toml"), manifest_body).unwrap();
    std::fs::write(repo.join("README.md"), "# pkg\n").unwrap();
    git(&repo, &["add", "."]);
    git(&repo, &["commit", "--quiet", "-m", "init"]);
    git(&repo, &["tag", "v0.1.0"]);
    repo
}

fn git(repo: &Path, args: &[&str]) {
    let s = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .status()
        .expect("git invokable");
    assert!(s.success(), "git {args:?} failed");
}

fn manifest(name: &str) -> String {
    format!(
        "[package]\ngroup = \"org.vibevm\"\nname = \"{name}\"\nkind = \"flow\"\nversion = \"0.1.0\"\nlicense = \"EULA\"\n"
    )
}

fn local_clone_url(p: &Path) -> String {
    p.to_string_lossy().replace('\\', "/")
}

/// Build a workdir + mock + initialised data dir for the given pages.
/// Each `(dir, pkg)` becomes a local repo served on its page.
fn fixture(
    pages: &[Vec<(&str, &str)>],
    etag: &str,
    emit_etag: bool,
) -> (tempfile::TempDir, MockServer, PathBuf) {
    let work = tempfile::tempdir().unwrap();
    let upstream = work.path().join("upstream");
    std::fs::create_dir_all(&upstream).unwrap();
    let mock_pages: Vec<Vec<CannedRepo>> = pages
        .iter()
        .map(|page| {
            page.iter()
                .map(|(dir, pkg)| {
                    let repo = make_local_repo(&upstream, dir, &manifest(pkg));
                    CannedRepo {
                        name: (*dir).into(),
                        clone_url: local_clone_url(&repo),
                    }
                })
                .collect()
        })
        .collect();
    let mock = spawn_mock(OrgVersion {
        etag: etag.into(),
        emit_etag,
        pages: mock_pages,
    });
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
    (work, mock, data)
}

fn gh_args(verb: &str, data: &Path, api_base: &str, org: &str) -> Vec<String> {
    vec![
        verb.into(),
        data.to_str().unwrap().into(),
        "--from-github".into(),
        org.into(),
        "--api-base".into(),
        api_base.into(),
        "--json".into(),
    ]
}

fn run_summary(args: &[String]) -> serde_json::Value {
    let out = cmd().args(args).assert().success();
    serde_json::from_str(&String::from_utf8(out.get_output().stdout.clone()).unwrap()).unwrap()
}

const ETAG_V1: &str = "\"etag-v1\"";
const ETAG_V2: &str = "\"etag-v2\"";

/// Acceptance #2 / #6 / #8 — a second consecutive run with no org
/// change does NOT re-enumerate: page 1 is probed once, the host
/// answers 304, the cached list is reused. The hit is visible in
/// `--json` (`org_cache: "hit"`); the first run shows `"miss"`.
#[test]
fn second_run_hits_cache_without_rewalking() {
    if !git_available() {
        return;
    }
    let (_work, mock, data) = fixture(
        &[vec![("flow-wal", "wal")], vec![("stack-rust", "rust")]],
        ETAG_V1,
        true,
    );

    let before1 = mock.count();
    let s1 = run_summary(&gh_args("reindex", &data, &mock.base_url, "vibespecs"));
    assert_eq!(s1["org_cache"], "miss", "first run must be a miss");
    assert_eq!(s1["package_count"], 2);
    assert_eq!(mock.count() - before1, 2, "first run walks both pages");
    assert!(data.join("state/org-cache.json").exists()); // #8: image persisted

    let before2 = mock.count();
    let s2 = run_summary(&gh_args("reindex", &data, &mock.base_url, "vibespecs"));
    assert_eq!(s2["org_cache"], "hit", "second run must be a hit");
    assert_eq!(s2["package_count"], 2);
    assert_eq!(mock.count() - before2, 1, "second run probes once");
    let r2 = mock.log_since(before2);
    assert_eq!(r2.len(), 1);
    assert!(r2[0].had_inm, "the probe must carry If-None-Match");
    assert_eq!(r2[0].status, 304);
    assert_eq!(r2[0].page, 1);
}

/// Acceptance #3 — an org change between runs is detected: the stored
/// validator no longer matches, the host answers 200, image refreshed.
#[test]
fn org_change_between_runs_is_detected() {
    if !git_available() {
        return;
    }
    let (_work, mock, data) = fixture(&[vec![("flow-wal", "wal")]], ETAG_V1, true);

    let s1 = run_summary(&gh_args("reindex", &data, &mock.base_url, "vibespecs"));
    assert_eq!(s1["org_cache"], "miss");
    assert_eq!(s1["package_count"], 1);

    // Org changes: a repo is added AND the ETag rotates.
    mock.set_version(OrgVersion {
        etag: ETAG_V2.into(),
        emit_etag: true,
        pages: vec![vec![
            canned("flow-wal", "wal", &_work),
            canned("stack-rust", "rust", &_work),
        ]],
    });

    let before = mock.count();
    let s2 = run_summary(&gh_args("reindex", &data, &mock.base_url, "vibespecs"));
    assert_eq!(s2["org_cache"], "miss", "changed org must re-enumerate");
    assert_eq!(s2["package_count"], 2, "the new repo must be indexed");
    let r2 = mock.log_since(before);
    assert!(
        r2[0].had_inm,
        "run 2 should still probe with the stale validator"
    );
    assert_eq!(r2[0].status, 200, "validator mismatch ⇒ 200, not 304");
}

/// Helper: a CannedRepo whose clone_url points at an upstream repo
/// under `work/upstream/<dir>`. Used when a test mutates the served
/// set after `fixture`: the repo is created only if absent (a repo
/// `fixture` already made is reused — re-running `make_local_repo`
/// would collide on the existing `v0.1.0` tag).
fn canned(dir: &str, pkg: &str, work: &tempfile::TempDir) -> CannedRepo {
    let upstream = work.path().join("upstream");
    let repo = upstream.join(dir);
    if !repo.join(".git").exists() {
        make_local_repo(&upstream, dir, &manifest(pkg));
    }
    CannedRepo {
        name: dir.into(),
        clone_url: local_clone_url(&repo),
    }
}

/// Acceptance #4 — when the host supplies no validator, the org cannot
/// be probed and is re-enumerated (never silently trusted).
#[test]
fn no_validator_means_reenumerate_not_trust() {
    if !git_available() {
        return;
    }
    let (_work, mock, data) = fixture(
        &[vec![("flow-wal", "wal")]],
        ETAG_V1,
        false, /* no ETag */
    );
    let args = gh_args("reindex", &data, &mock.base_url, "vibespecs");

    let s1 = run_summary(&args);
    assert_eq!(s1["org_cache"], "miss");
    let cache: serde_json::Value =
        serde_json::from_slice(&std::fs::read(data.join("state/org-cache.json")).unwrap()).unwrap();
    assert_eq!(cache["etag"], serde_json::Value::Null);

    let before = mock.count();
    let s2 = run_summary(&args);
    assert_eq!(s2["org_cache"], "miss", "no validator ⇒ must re-enumerate");
    let r2 = mock.log_since(before);
    assert!(!r2[0].had_inm, "no validator ⇒ no If-None-Match sent");
    assert_eq!(r2[0].status, 200);
}

/// Acceptance #5 — `rescan-org` enumerates unconditionally even when
/// the cache is fresh: it never sends If-None-Match and walks every
/// page.
#[test]
fn rescan_org_is_unconditional_even_when_cache_fresh() {
    if !git_available() {
        return;
    }
    let (_work, mock, data) = fixture(
        &[vec![("flow-wal", "wal")], vec![("stack-rust", "rust")]],
        ETAG_V1,
        true,
    );

    // Prime a fresh cache (etag v1, both repos).
    let s1 = run_summary(&gh_args("reindex", &data, &mock.base_url, "vibespecs"));
    assert_eq!(s1["org_cache"], "miss");

    // rescan-org even though the cache is fresh (etag would match).
    let before = mock.count();
    let s2 = run_summary(&gh_args("rescan-org", &data, &mock.base_url, "vibespecs"));
    assert_eq!(s2["org_cache"], "miss", "rescan must re-enumerate");
    assert_eq!(s2["package_count"], 2);
    assert_eq!(mock.count() - before, 2, "rescan walks every page");
    for rec in mock.log_since(before) {
        assert!(!rec.had_inm, "rescan must not send If-None-Match");
        assert_eq!(rec.status, 200);
    }
    assert!(data.join("state/org-cache.json").exists()); // Р4: image refreshed
}

/// Acceptance #7 — an image taken for one org is never served for
/// another. Same ETag, different repos, different org: the org
/// mismatch forces a re-enumeration and overwrites the image.
#[test]
fn cache_not_reused_for_different_org() {
    if !git_available() {
        return;
    }
    let (_work, mock, data) = fixture(&[vec![("flow-wal", "wal")]], ETAG_V1, true);

    // Run for org A — caches image with org = "org-a".
    let sa = run_summary(&gh_args("reindex", &data, &mock.base_url, "org-a"));
    assert_eq!(sa["org_cache"], "miss");
    assert_eq!(sa["package_count"], 1);

    // Swap served repos (same ETag) then run for org B.
    mock.set_version(OrgVersion {
        etag: ETAG_V1.into(),
        emit_etag: true,
        pages: vec![vec![canned("stack-rust", "rust", &_work)]],
    });

    let before = mock.count();
    let sb = run_summary(&gh_args("reindex", &data, &mock.base_url, "org-b"));
    assert_eq!(
        sb["org_cache"], "miss",
        "cross-org cache must not be trusted"
    );
    assert_eq!(
        sb["package_count"], 1,
        "B enumerated the current org, not org-a's cached repos"
    );
    let rb = mock.log_since(before);
    assert!(!rb[0].had_inm, "org mismatch ⇒ cache ignored ⇒ no probe");

    let cache: serde_json::Value =
        serde_json::from_slice(&std::fs::read(data.join("state/org-cache.json")).unwrap()).unwrap();
    assert_eq!(cache["org"], "org-b"); // overwritten, not org-a's
}

/// Acceptance #8 + Р1 opt-out — a first run with no cache behaves as
/// before (enumerates, writes the image), and `--no-cache-org`
/// disables the cache entirely (no read, no write, no `org_cache`).
#[test]
fn first_run_and_no_cache_org_behaviour() {
    if !git_available() {
        return;
    }

    // --- `--no-cache-org`: cache fully off -------------------------
    let (_work_a, mock_a, data_a) = fixture(&[vec![("flow-wal", "wal")]], ETAG_V1, true);
    let mut args = gh_args("reindex", &data_a, &mock_a.base_url, "vibespecs");
    args.push("--no-cache-org".into());
    let sa = run_summary(&args);
    assert!(
        sa.get("org_cache").is_none(),
        "`--no-cache-org` must omit org_cache"
    );
    assert_eq!(
        sa["package_count"], 1,
        "still enumerates and builds the index"
    );
    assert!(
        !data_a.join("state/org-cache.json").exists(),
        "`--no-cache-org` must not write the image"
    );

    // --- First run with cache (default): behaves as before ----------
    let (_work_b, mock_b, data_b) = fixture(&[vec![("flow-wal", "wal")]], ETAG_V1, true);
    let out = cmd()
        .args(gh_args("reindex", &data_b, &mock_b.base_url, "vibespecs"))
        .assert()
        .success();
    let sb: serde_json::Value =
        serde_json::from_str(&String::from_utf8(out.get_output().stdout.clone()).unwrap()).unwrap();
    assert_eq!(sb["org_cache"], "miss");
    assert_eq!(sb["package_count"], 1, "enumerates and indexes like before");
    assert!(
        data_b.join("state/org-cache.json").exists(),
        "image persisted"
    );
    assert!(data_b.join("by-name/wal.json").exists());
}

/// Р5 — hit and miss are visible in the human-readable output too.
#[test]
fn hit_and_miss_visible_in_text_output() {
    if !git_available() {
        return;
    }
    let (_work, mock, data) = fixture(&[vec![("flow-wal", "wal")]], ETAG_V1, true);
    let text = |data: &Path, base: &str| -> String {
        let out = cmd()
            .args([
                "reindex",
                data.to_str().unwrap(),
                "--from-github",
                "vibespecs",
                "--api-base",
                base,
            ])
            .assert()
            .success();
        String::from_utf8(out.get_output().stdout.clone()).unwrap()
    };
    let t1 = text(&data, &mock.base_url);
    assert!(
        t1.contains("cache     : miss"),
        "first run text must show the miss, got:\n{t1}"
    );
    let t2 = text(&data, &mock.base_url);
    assert!(
        t2.contains("cache     : hit"),
        "second run text must show the hit, got:\n{t2}"
    );
}
