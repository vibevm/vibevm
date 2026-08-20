//! End-to-end coverage of the index-location ladder (PROP-005 §2.2
//! `#form-factor`, B-083): env `VIBEVM_INDEX_URL_<REGISTRY>` > the
//! `[[registry]].index_url` manifest key > the default
//! `<registry-url>/index`, with `none` on either explicit step
//! switching the index off.
//!
//! Each test builds a `MultiRegistryResolver` through
//! `from_manifest` — the same path production takes — against a mock
//! axum index and a git backend that always says "repo missing". Any
//! version list that comes back can therefore only have come through
//! the ladder-attached index client, never from `git ls-remote`.
//!
//! The mock serves the index surface at BOTH its root and its
//! `/index/` prefix, so one server stands in for an explicitly-pinned
//! index (`index_url = "<mock>"`) and for the default location of a
//! registry whose url IS the mock (`<registry-url>/index`).
//!
//! No env mutation here, deliberately: libtest runs bodies on many
//! threads, and a runtime `set_var` is the exact UB that made it
//! `unsafe` in edition 2024 — the discipline's unsafe-gate refuses it
//! outside the audit crate, and the audit crate's own doctrine allows
//! mutation only in a pre-`main` constructor. The env rung's
//! precedence is pinned by the pure-core unit tests
//! (`index_client/locate.rs`, `resolve_index_url_with` with the env
//! step passed in); what this binary adds is the rungs that need a
//! real server — key, default, and `none`. Registry names here are
//! unique to this file, so no ambient `VIBEVM_INDEX_URL_<R>` variable
//! can exist for them and the env rung is vacuously empty.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex, mpsc};
use std::thread;

use axum::Router;
use axum::extract::{Path as AxumPath, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use specmark::verifies;
use tempfile::tempdir;
use tokio::net::TcpListener;

use vibe_core::Group;
use vibe_core::manifest::{AuthKind, NamingConvention, RegistrySection};
use vibe_registry::git_backend::GitBackend;
use vibe_registry::{GitError, MultiRegistryResolver};

#[derive(Default)]
struct CannedFiles {
    /// Keyed by bare `name` — the `by-name/` candidate-set layer
    /// (PROP-008 §2.8). `None` serves a 404.
    by_name: HashMap<String, Option<serde_json::Value>>,
}

#[derive(Clone)]
struct MockState {
    files: Arc<Mutex<CannedFiles>>,
}

async fn repomd_handler() -> impl IntoResponse {
    (
        StatusCode::OK,
        axum::Json(serde_json::json!({
            "schema_version": 1,
            "registry": "ladder",
            "registry_url": "https://example.invalid",
            "naming": "kind-name",
            "generated_at": "2026-08-20T12:00:00Z",
            "generator": "mock",
            "package_count": 1,
            "version_count": 1,
            "files": {}
        })),
    )
}

async fn by_name_handler(
    State(state): State<MockState>,
    AxumPath(name_with_ext): AxumPath<String>,
) -> axum::response::Response {
    let Some(name) = name_with_ext.strip_suffix(".json") else {
        return (StatusCode::NOT_FOUND, "expected .json").into_response();
    };
    let payload = state.files.lock().unwrap().by_name.get(name).cloned();
    match payload {
        Some(Some(v)) => (StatusCode::OK, axum::Json(v)).into_response(),
        Some(None) => StatusCode::NOT_FOUND.into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

struct Mock {
    base_url: String,
    _thread: thread::JoinHandle<()>,
}

fn spawn_mock(files: CannedFiles) -> Mock {
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
                files: Arc::new(Mutex::new(files)),
            };
            let app = Router::new()
                .route("/repomd.json", get(repomd_handler))
                .route("/by-name/{name}", get(by_name_handler))
                .route("/index/repomd.json", get(repomd_handler))
                .route("/index/by-name/{name}", get(by_name_handler))
                .with_state(state);
            tx.send(format!("http://{addr}")).unwrap();
            axum::serve(listener, app).await.unwrap();
        });
    });
    Mock {
        base_url: rx.recv().unwrap(),
        _thread: handle,
    }
}

/// A `by-name/<name>.json` candidate set carrying one `(group, name)`
/// package — the PROP-008 §2.8 layout the index serves.
fn name_entry_json(group: &str, name: &str, versions: &[&str]) -> serde_json::Value {
    let entries: Vec<serde_json::Value> = versions
        .iter()
        .map(|v| {
            serde_json::json!({
                "schema_version": 1,
                "kind": "flow",
                "group": group,
                "name": name,
                "version": v,
                "content_hash": "sha256:0000",
                "source_url": "https://example.invalid/x.git",
                "source_ref": format!("v{v}"),
                "registry": "ladder",
                "files_count": 1,
                "indexed_at": "2026-08-20T12:00:00Z",
                "indexed_by": "mock",
            })
        })
        .collect();
    serde_json::json!({
        "name": name,
        "indexed_at": "2026-08-20T12:00:00Z",
        "packages": [
            {
                "group": group,
                "name": name,
                "indexed_at": "2026-08-20T12:00:00Z",
                "latest_stable": versions.last(),
                "versions": entries,
            }
        ],
    })
}

/// A backend that always says "repo missing" so any version list the
/// resolver returns can only have come from the attached index client.
#[derive(Default)]
struct AlwaysMissing;

impl GitBackend for AlwaysMissing {
    fn list_tags(&self, url: &str) -> Result<Vec<String>, GitError> {
        Err(GitError::RepoNotFound {
            url: url.to_string(),
        })
    }
    fn fetch_file_at_ref(
        &self,
        url: &str,
        _refname: &str,
        _path: &str,
    ) -> Result<Vec<u8>, GitError> {
        Err(GitError::RepoNotFound {
            url: url.to_string(),
        })
    }
    fn bootstrap(&self, url: &str, _refname: &str, _dest: &Path) -> Result<(), GitError> {
        Err(GitError::RepoNotFound {
            url: url.to_string(),
        })
    }
    fn update(&self, _dest: &Path, _refname: &str) -> Result<(), GitError> {
        Ok(())
    }
}

fn section(name: &str, url: &str, index_url: Option<&str>) -> RegistrySection {
    RegistrySection {
        name: name.to_string(),
        url: url.to_string(),
        r#ref: "main".to_string(),
        naming: NamingConvention::Fqdn,
        auth: AuthKind::None,
        token_env: None,
        enabled: true,
        index_url: index_url.map(|s| s.to_string()),
    }
}

/// Build the resolver the way production does, keeping the cache
/// tempdir alive for the resolver's lifetime.
fn resolver(reg: RegistrySection) -> (MultiRegistryResolver, tempfile::TempDir) {
    let cache = tempdir().unwrap();
    let r = MultiRegistryResolver::from_manifest(
        &[reg],
        &[],
        &[],
        cache.path().to_path_buf(),
        Arc::new(AlwaysMissing),
        3600,
    )
    .unwrap();
    (r, cache)
}

fn versions_of(r: &MultiRegistryResolver, name: &str) -> Vec<String> {
    r.list_versions(&Group::parse("org.vibevm").unwrap(), name)
        .unwrap()
        .into_iter()
        .map(|v| v.to_string())
        .collect()
}

/// The manifest key attaches an index with no env var in sight — the
/// B-083 headline: what the spec's example promised now works.
#[test]
#[verifies(
    "spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#form-factor",
    r = 1
)]
fn manifest_key_attaches_index_without_env() {
    let mut canned = CannedFiles::default();
    canned.by_name.insert(
        "wal".into(),
        Some(name_entry_json("org.vibevm", "wal", &["0.1.0", "0.2.0"])),
    );
    let mock = spawn_mock(canned);
    let (r, _cache) = resolver(section(
        "ladderkey",
        "https://example.invalid/vibespecs",
        Some(&mock.base_url),
    ));
    assert_eq!(versions_of(&r, "wal"), vec!["0.1.0", "0.2.0"]);
}

/// With neither env nor key, the resolver tries the default
/// `<registry-url>/index` — here that location is the mock itself.
#[test]
#[verifies(
    "spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#form-factor",
    r = 1
)]
fn default_location_is_probed_when_no_key_is_set() {
    let mut canned = CannedFiles::default();
    canned.by_name.insert(
        "wal".into(),
        Some(name_entry_json("org.vibevm", "wal", &["0.3.0"])),
    );
    let mock = spawn_mock(canned);
    // Registry url IS the mock base, so the default rung resolves to
    // `<mock>/index` — a path the mock serves.
    let (r, _cache) = resolver(section("ladderdefault", &mock.base_url, None));
    assert_eq!(versions_of(&r, "wal"), vec!["0.3.0"]);
}

/// `none` switches the index off even where the default location
/// would answer: same mock serving `<registry-url>/index` as above,
/// but the key says `none`, so the only possible answer is the git
/// path's UnknownPackage.
#[test]
#[verifies(
    "spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#form-factor",
    r = 1
)]
fn none_switches_the_index_off_even_where_a_default_would_answer() {
    let mut canned = CannedFiles::default();
    canned.by_name.insert(
        "wal".into(),
        Some(name_entry_json("org.vibevm", "wal", &["0.3.0"])),
    );
    let mock = spawn_mock(canned);
    let (r, _cache) = resolver(section("laddernone", &mock.base_url, Some("none")));
    let err = r
        .list_versions(&Group::parse("org.vibevm").unwrap(), "wal")
        .expect_err("the disabled index must leave the git path's error");
    // The resolver's exhausted-walk error, not an index answer: the
    // attempt summary shows the registry consulted through the git
    // path and finding nothing there — the mock that serves
    // `<registry-url>/index` was never asked.
    match err {
        vibe_registry::RegistryError::PackageNotFoundEverywhere { summary, .. } => {
            assert!(summary.contains("not found"), "summary: {summary}");
        }
        other => panic!("unexpected error variant: {other:?}"),
    }
}
