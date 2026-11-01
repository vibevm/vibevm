//! B-227: the index client checks a `by-name/<name>.json` it receives
//! against the `sha256` `repomd.json` lists for it, and a copy the
//! manifest does not vouch for sends the selector to the package's tags.
//!
//! The fixture reproduces the 2026-10-31 shape: a cache still serves the
//! `by-name/vim.json` that knew only `1.1.1`, while `repomd.json` already
//! names the file written for `1.1.2`. A loopback server stands in for the
//! static index; the git side is a canned tag list, so no test here leaves
//! the machine.

use std::path::Path;
use std::sync::{Arc, mpsc};
use std::thread;

use axum::Router;
use axum::extract::State;
use axum::routing::get;
use sha2::{Digest, Sha256};
use tempfile::tempdir;
use tokio::net::TcpListener;

use vibe_core::Group;
use vibe_core::manifest::NamingConvention;
use vibe_registry::git_backend::GitBackend;
use vibe_registry::{GitError, GitPerPackageRegistry, IndexClient};

/// The two bodies the static index serves.
#[derive(Clone)]
struct Served {
    repomd: Arc<String>,
    by_name: Arc<String>,
}

fn spawn_index(served: Served) -> String {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async move {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            tx.send(format!("http://{}", listener.local_addr().unwrap()))
                .unwrap();
            let app = Router::new()
                .route(
                    "/repomd.json",
                    get(|State(s): State<Served>| async move { s.repomd.to_string() }),
                )
                .route(
                    "/by-name/vim.json",
                    get(|State(s): State<Served>| async move { s.by_name.to_string() }),
                )
                .with_state(served);
            axum::serve(listener, app).await.unwrap();
        });
    });
    rx.recv().unwrap()
}

/// `by-name/vim.json` listing `versions` of `org.example/vim`.
fn by_name(versions: &[&str]) -> String {
    let entries: Vec<serde_json::Value> = versions
        .iter()
        .map(|v| {
            serde_json::json!({
                "schema_version": 1, "kind": "app", "group": "org.example", "name": "vim",
                "version": v, "content_hash": "sha256:0000",
                "source_url": "https://example.invalid/org.example.vim.git",
                "source_ref": format!("v{v}"), "registry": "vibespecs", "files_count": 1,
                "indexed_at": "2026-10-31T12:00:00Z", "indexed_by": "fixture",
            })
        })
        .collect();
    serde_json::to_string_pretty(&serde_json::json!({
        "name": "vim",
        "indexed_at": "2026-10-31T12:00:00Z",
        "packages": [{
            "group": "org.example", "name": "vim",
            "indexed_at": "2026-10-31T12:00:00Z", "versions": entries,
        }],
    }))
    .unwrap()
}

/// `repomd.json` whose `files` map lists `by-name/vim.json` with the size
/// and digest of `vouched` — or lists no `by-name` file at all, the shape
/// of a writer older than the per-file entries.
fn repomd(vouched: Option<&str>) -> String {
    let mut files = serde_json::json!({ "by-name": { "kind": "directory", "entries": 1 } });
    if let Some(body) = vouched {
        files["by-name/vim.json"] = serde_json::json!({
            "kind": "file",
            "size": body.len().to_string(),
            "sha256": format!("sha256:{:x}", Sha256::digest(body.as_bytes())),
        });
    }
    serde_json::json!({
        "schema_version": 1, "registry": "vibespecs",
        "registry_url": "https://example.invalid", "naming": "fqdn",
        "generated_at": "2026-10-31T12:10:00Z", "generator": "fixture",
        "package_count": 1, "version_count": 2, "files": files,
    })
    .to_string()
}

/// The package repository's tags: the truth the fallback reads.
struct Tags;

impl GitBackend for Tags {
    fn list_tags(&self, _url: &str) -> Result<Vec<String>, GitError> {
        Ok(vec!["v1.1.1".into(), "v1.1.2".into()])
    }
    fn fetch_file_at_ref(&self, url: &str, _r: &str, _p: &str) -> Result<Vec<u8>, GitError> {
        Err(GitError::RepoNotFound { url: url.into() })
    }
    fn bootstrap(&self, url: &str, _r: &str, _d: &Path) -> Result<(), GitError> {
        Err(GitError::RepoNotFound { url: url.into() })
    }
    fn update(&self, _d: &Path, _r: &str) -> Result<(), GitError> {
        Ok(())
    }
}

fn versions_seen(served: Served) -> Vec<String> {
    let base = spawn_index(served);
    let cache = tempdir().unwrap();
    let registry = GitPerPackageRegistry::open_with_mirrors(
        "vibespecs",
        "https://example.invalid/vibespecs",
        "main",
        NamingConvention::Fqdn,
        Vec::new(),
        cache.path(),
        Arc::new(Tags),
        3600,
    )
    .unwrap()
    .with_index_client(IndexClient::at(&base));
    let group = Group::parse("org.example").unwrap();
    registry
        .list_versions(&group, "vim")
        .unwrap()
        .iter()
        .map(ToString::to_string)
        .collect()
}

#[test]
fn a_by_name_file_older_than_repomd_falls_back_to_the_tags() {
    let stale = by_name(&["1.1.1"]);
    let fresh = by_name(&["1.1.1", "1.1.2"]);
    let seen = versions_seen(Served {
        repomd: Arc::new(repomd(Some(&fresh))),
        by_name: Arc::new(stale),
    });
    assert_eq!(
        seen,
        ["1.1.1", "1.1.2"],
        "the stale copy knew only 1.1.1; the tags must have been read instead"
    );
}

#[test]
fn a_by_name_file_repomd_vouches_for_is_served_from_the_index() {
    // The control: the index's answer (one version) differs from the tags
    // (two), so the result says which source was used.
    let current = by_name(&["1.1.1"]);
    let seen = versions_seen(Served {
        repomd: Arc::new(repomd(Some(&current))),
        by_name: Arc::new(current),
    });
    assert_eq!(seen, ["1.1.1"]);
}

#[test]
fn a_repomd_without_per_file_entries_leaves_the_by_name_file_trusted() {
    let seen = versions_seen(Served {
        repomd: Arc::new(repomd(None)),
        by_name: Arc::new(by_name(&["1.1.1"])),
    });
    assert_eq!(
        seen,
        ["1.1.1"],
        "nothing to check against: trusted as before"
    );
}
