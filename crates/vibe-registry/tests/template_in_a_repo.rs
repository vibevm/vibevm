//! A template package in an **indexed per-package** registry — the
//! registry shape the public `vibespecs` host has, and the one that
//! until now could see a template's revisions and none of the versions
//! it produces (PROP-075 `#TEMPLATE-IN-A-REPO`, `BACKLOG.md#b-298`).
//!
//! The mock here is the honest one: an index that serves a `by-name`
//! record whose rows carry `template`, and a git backend that serves one
//! file at one tag and **refuses every clone**. If the discovery path
//! needed a clone, or needed one read per tag, these tests would fail —
//! which is the property `#TEMPLATE-DISCOVERY` says the index exists to
//! buy.

use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::Duration;

use axum::Router;
use axum::extract::{Path as AxumPath, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use specmark::verifies;
use tempfile::tempdir;
use tokio::net::TcpListener;

use vibe_core::manifest::NamingConvention;
use vibe_core::{Group, PackageKind, PackageRef};
use vibe_registry::git_backend::GitBackend;
use vibe_registry::templates::{Posture, TemplateLayer};
use vibe_registry::{GitError, GitPerPackageRegistry, IndexClient};

/// The template document the revision's tag serves. `kind = "list"`, so
/// the version domain needs no fetched document at all, and no
/// `[template.digests]`, so no digest document is ever named
/// (PROP-075 `#TEMPLATE-NO-DIGEST`).
const TEMPLATE_TOML: &str = "[package]\n\
     group = \"org.example\"\n\
     name = \"tool\"\n\
     kind = \"app\"\n\
     version = \"1.1.0\"\n\
     template = true\n\
     description = \"tool {version}\"\n\
     \n[application]\n\
     id = \"tool\"\n\
     \n[template.versions]\n\
     kind = \"list\"\n\
     [[template.versions.item]]\n\
     version = \"0.41.0\"\n\
     meta = { lts = \"Iron\" }\n\
     [[template.versions.item]]\n\
     version = \"0.42.0\"\n\
     \n[[template.channel]]\n\
     rule = \"truthy\"\n\
     name = \"lts\"\n\
     from = \"lts\"\n";

/// Install one template layer for this test binary. Write-once, so every
/// test in the process shares it; the layer holds no per-test state.
fn layer_installed() {
    static ROOT: std::sync::OnceLock<tempfile::TempDir> = std::sync::OnceLock::new();
    let root = ROOT.get_or_init(|| tempdir().expect("a settings root"));
    let layer = TemplateLayer::with_transport(
        root.path(),
        Duration::from_secs(900),
        // Offline: nothing in these tests has a document to fetch, and
        // a posture that could reach the network would hide it if one
        // crept in.
        Posture::Offline,
        Box::new(NoTransport),
    );
    vibe_registry::templates::install(Arc::new(layer));
}

/// A transport that refuses everything: no template document in these
/// tests is fetched, and this proves it rather than promising it.
struct NoTransport;

impl vibe_registry::templates::TemplateTransport for NoTransport {
    fn read(
        &self,
        document: vibe_registry::templates::Document,
        package: &str,
        url: &str,
    ) -> Result<vibe_registry::templates::Fetched, vibe_registry::templates::TemplateError> {
        Err(vibe_registry::templates::TemplateError::Fetch {
            package: package.to_string(),
            what: document.label().to_string(),
            url: url.to_string(),
            reason: "this test fetches no template document".to_string(),
        })
    }
}

// ----------------------------------------------------------- the mock --

#[derive(Default)]
struct Canned {
    by_name: HashMap<String, serde_json::Value>,
}

#[derive(Clone)]
struct MockState {
    files: Arc<Mutex<Canned>>,
}

async fn repomd_handler() -> impl IntoResponse {
    (
        StatusCode::OK,
        axum::Json(serde_json::json!({
            "schema_version": 1,
            "registry": "vibespecs",
            "registry_url": "https://example.invalid",
            "naming": "fqdn",
            "generated_at": "2026-11-11T12:00:00Z",
            "generator": "mock",
            "package_count": 1,
            "version_count": 1,
            "files": {}
        })),
    )
        .into_response()
}

async fn by_name_handler(
    State(state): State<MockState>,
    AxumPath(file): AxumPath<String>,
) -> axum::response::Response {
    let Some(name) = file.strip_suffix(".json") else {
        return StatusCode::NOT_FOUND.into_response();
    };
    match state.files.lock().unwrap().by_name.get(name).cloned() {
        Some(value) => (StatusCode::OK, axum::Json(value)).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

struct Mock {
    base_url: String,
    _thread: thread::JoinHandle<()>,
}

fn spawn_mock(canned: Canned) -> Mock {
    let files = Arc::new(Mutex::new(canned));
    let (tx, rx) = mpsc::channel();
    let handle = thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async move {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let port = listener.local_addr().unwrap().port();
            tx.send(port).unwrap();
            let app = Router::new()
                .route("/repomd.json", get(repomd_handler))
                .route("/by-name/{file}", get(by_name_handler))
                .with_state(MockState { files });
            axum::serve(listener, app).await.unwrap();
        });
    });
    let port = rx.recv().unwrap();
    Mock {
        base_url: format!("http://127.0.0.1:{port}"),
        _thread: handle,
    }
}

/// A `by-name` record for `org.example/tool`: two template revisions and
/// one concrete publication, which is the real `org.vibevm.node/node`
/// shape in miniature.
fn by_name_record(versions: &[(&str, bool)]) -> serde_json::Value {
    let rows: Vec<serde_json::Value> = versions
        .iter()
        .map(|(version, template)| {
            serde_json::json!({
                "schema_version": 1,
                "kind": PackageKind::App,
                "group": "org.example",
                "name": "tool",
                "version": version,
                "content_hash": "sha256:0000",
                "source_url": "https://example.invalid/x.git",
                "source_ref": format!("v{version}"),
                "registry": "vibespecs",
                "files_count": 1,
                "indexed_at": "2026-11-11T12:00:00Z",
                "indexed_by": "mock",
                "template": template,
            })
        })
        .collect();
    serde_json::json!({
        "name": "tool",
        "indexed_at": "2026-11-11T12:00:00Z",
        "packages": [{
            "group": "org.example",
            "name": "tool",
            "indexed_at": "2026-11-11T12:00:00Z",
            "versions": rows,
        }],
    })
}

/// A backend that serves exactly one file at exactly one ref and refuses
/// every clone — so a path that needed a working tree, or that read one
/// file per tag, fails here instead of passing quietly.
#[derive(Default)]
struct ServesOneFile {
    reads: AtomicUsize,
    clones: AtomicUsize,
}

impl GitBackend for ServesOneFile {
    fn list_tags(&self, url: &str) -> Result<Vec<String>, GitError> {
        // The index answers; the tag list must never be needed.
        Err(GitError::RepoNotFound {
            url: url.to_string(),
        })
    }
    fn fetch_file_at_ref(&self, url: &str, refname: &str, path: &str) -> Result<Vec<u8>, GitError> {
        self.reads.fetch_add(1, Ordering::SeqCst);
        if refname == "v1.1.0" && path == "vibe.toml" {
            return Ok(TEMPLATE_TOML.as_bytes().to_vec());
        }
        Err(GitError::RepoNotFound {
            url: format!("{url}@{refname}:{path}"),
        })
    }
    /// The one checkout this path is allowed: the revision's tree,
    /// and only when a version is actually rendered.
    fn bootstrap(&self, _url: &str, refname: &str, dest: &Path) -> Result<(), GitError> {
        self.clones.fetch_add(1, Ordering::SeqCst);
        assert_eq!(refname, "v1.1.0", "only the active revision is checked out");
        std::fs::create_dir_all(dest).unwrap();
        std::fs::write(dest.join("vibe.toml"), TEMPLATE_TOML).unwrap();
        // A file beside the manifest, so the test can see that a
        // rendered version is a copy of the revision's whole tree.
        std::fs::write(dest.join("README.md"), "# tool\n").unwrap();
        Ok(())
    }
    fn update(&self, _dest: &Path, refname: &str) -> Result<(), GitError> {
        assert_eq!(refname, "v1.1.0");
        self.clones.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

fn registry(
    mock: &Mock,
    backend: Arc<ServesOneFile>,
) -> (GitPerPackageRegistry, tempfile::TempDir) {
    let cache = tempdir().unwrap();
    let registry = GitPerPackageRegistry::open_with_mirrors(
        "vibespecs",
        "https://example.invalid/vibespecs",
        "main",
        NamingConvention::Fqdn,
        Vec::new(),
        cache.path(),
        backend,
        3600,
    )
    .unwrap()
    .with_index_client(IndexClient::at(&mock.base_url));
    (registry, cache)
}

fn fixture() -> (Mock, Arc<ServesOneFile>) {
    layer_installed();
    let mut canned = Canned::default();
    canned.by_name.insert(
        "tool".into(),
        by_name_record(&[("1.0.0", true), ("1.1.0", true), ("0.41.0", false)]),
    );
    (spawn_mock(canned), Arc::new(ServesOneFile::default()))
}

// ---------------------------------------------------------- the tests --

/// The version domain is the template's, unioned with the concrete
/// publication — and the two revisions are in neither.
#[test]
#[verifies(
    "spec://org.vibevm.core/vibevm/common/PROP-075#TEMPLATE-IN-A-REPO",
    r = 1
)]
fn the_domain_is_the_templates_versions_and_not_its_revisions() {
    let (mock, backend) = fixture();
    let (registry, _cache) = registry(&mock, Arc::clone(&backend));
    let group = Group::parse("org.example").unwrap();

    let versions = registry.list_versions(&group, "tool").unwrap();
    assert_eq!(
        versions.iter().map(ToString::to_string).collect::<Vec<_>>(),
        // 0.41.0 is in both sets (`#TEMPLATE-CONCRETE-WINS`), 0.42.0 is
        // the template's alone, and 1.0.0 / 1.1.0 are the revisions.
        vec!["0.41.0".to_string(), "0.42.0".to_string()],
    );

    // Exactly one file read, at the newest revision's tag — not one per
    // tag, which is the cost the index exists to avoid.
    assert_eq!(
        backend.reads.load(Ordering::SeqCst),
        1,
        "discovery costs one read at the active revision's tag"
    );
}

/// `@latest` picks the newest version the template produces, which no
/// tag of the repository names — the case that could not work at all
/// before, because the registry only ever saw its own tags.
#[test]
#[verifies(
    "spec://org.vibevm.core/vibevm/common/PROP-075#TEMPLATE-IN-A-REPO",
    r = 1
)]
fn latest_selects_a_version_only_the_template_produces() {
    let (mock, backend) = fixture();
    let (registry, _cache) = registry(&mock, backend);
    let pkgref = PackageRef::parse("org.example/tool").unwrap();
    let resolved = registry.resolve(&pkgref).unwrap();
    assert_eq!(resolved.version.to_string(), "0.42.0");
    // The resolved tree is the layer's **rendered** directory, not this
    // registry's clone directory: nothing below the seam learns that a
    // manifest was rendered.
    assert!(
        vibe_registry::templates::is_rendered_tree(&resolved.source_dir),
        "a produced version is materialised: {}",
        resolved.source_dir.display()
    );
    // The whole revision tree came along, which is why rendering is the
    // one thing that pays for a checkout.
    assert!(resolved.source_dir.join("README.md").is_file());
    let rendered = std::fs::read_to_string(resolved.source_dir.join("vibe.toml")).unwrap();
    assert!(rendered.contains("tool 0.42.0"), "{rendered}");
    assert!(
        !rendered.contains("[template"),
        "the vocabulary is erased: {rendered}"
    );
}

/// A channel lowers against the template's own rules, which is what
/// `vibe install -g <package>@lts` needs and what this registry shape
/// answered `no version matches` to before.
#[test]
#[verifies(
    "spec://org.vibevm.core/vibevm/common/PROP-075#TEMPLATE-IN-A-REPO",
    r = 1
)]
fn a_channel_lowers_against_the_templates_rules() {
    let (mock, backend) = fixture();
    let (registry, _cache) = registry(&mock, backend);
    let pkgref = PackageRef::parse("org.example/tool@lts").unwrap();
    let resolved = registry.resolve(&pkgref).unwrap();
    // `lts` has one member: the row whose `meta.lts` is truthy.
    assert_eq!(resolved.version.to_string(), "0.41.0");
}

/// Naming a revision exactly still refuses, exactly as it does on a
/// directory registry: a revision is the version of the template, never
/// a version of the thing it produces
/// (PROP-075 `#TEMPLATE-REVISION-NOT-A-VERSION`).
#[test]
#[verifies(
    "spec://org.vibevm.core/vibevm/common/PROP-075#TEMPLATE-REVISION-NOT-A-VERSION",
    r = 1
)]
fn a_revision_is_still_not_selectable_here() {
    let (mock, backend) = fixture();
    let (registry, _cache) = registry(&mock, backend);
    let pkgref = PackageRef::parse("org.example/tool@1.1.0").unwrap();
    let error = registry.resolve(&pkgref).unwrap_err();
    assert!(
        matches!(
            error,
            vibe_registry::RegistryError::NoMatchingVersion { .. }
        ),
        "unexpected: {error:?}"
    );
}

/// A package with no template row is untouched: no file read at all, and
/// the versions are the index's own rows. This is every package in a
/// registry, and it must cost nothing new.
#[test]
#[verifies(
    "spec://org.vibevm.core/vibevm/common/PROP-075#TEMPLATE-IN-A-REPO",
    r = 1
)]
fn a_package_with_no_template_costs_no_extra_read() {
    layer_installed();
    let mut canned = Canned::default();
    canned.by_name.insert(
        "tool".into(),
        by_name_record(&[("1.0.0", false), ("2.0.0", false)]),
    );
    let mock = spawn_mock(canned);
    let backend = Arc::new(ServesOneFile::default());
    let (registry, _cache) = registry(&mock, Arc::clone(&backend));
    let group = Group::parse("org.example").unwrap();

    let versions = registry.list_versions(&group, "tool").unwrap();
    assert_eq!(
        versions.iter().map(ToString::to_string).collect::<Vec<_>>(),
        vec!["1.0.0".to_string(), "2.0.0".to_string()]
    );
    assert_eq!(
        backend.reads.load(Ordering::SeqCst),
        0,
        "no template, no revision to read"
    );
}
