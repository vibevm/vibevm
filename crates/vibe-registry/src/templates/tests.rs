//! The template layer against a local fixture registry and a transport
//! that serves documents out of a directory — no socket, ever.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tempfile::TempDir;
use vibe_core::Group;

use super::*;

/// A transport over a `BTreeMap<url, body>`, counting every read — which
/// is how «nothing touches the network twice» becomes an assertion.
struct Served {
    documents: BTreeMap<String, String>,
    reads: Mutex<Vec<String>>,
}

impl Served {
    fn new(documents: BTreeMap<String, String>) -> Self {
        Served {
            documents,
            reads: Mutex::new(Vec::new()),
        }
    }
}

impl TemplateTransport for Arc<Served> {
    fn read(&self, document: Document, package: &str, url: &str) -> Result<Fetched, TemplateError> {
        self.reads.lock().unwrap().push(url.to_string());
        match self.documents.get(url) {
            Some(body) => Ok(Fetched {
                chosen_url: url.to_string(),
                body: body.clone(),
            }),
            None => Err(TemplateError::Fetch {
                package: package.to_string(),
                what: document.label().to_string(),
                url: url.to_string(),
                reason: "the fixture serves no such document".to_string(),
            }),
        }
    }
}

const INDEX_URL: &str = "https://example.invalid/dist/index.json";
const DIGEST_URL_TEMPLATE: &str = "https://example.invalid/dist/v{version}/SHASUMS256.txt";

fn index_document() -> String {
    r#"[
        {"version":"v2.1.0","lts":"Argon","date":"2026-01-01"},
        {"version":"v2.0.0","lts":false,"date":"2025-06-01"},
        {"version":"v1.4.0","lts":"Boron","date":"2024-06-01"},
        {"version":"nonsense","lts":false}
    ]"#
    .to_string()
}

fn digest_document(version: &str) -> String {
    format!("{}  tool-{version}-linux-x86_64.tar.gz\n", "a".repeat(64))
}

fn template_toml() -> String {
    format!(
        concat!(
            "[package]\n",
            "group = \"org.example\"\n",
            "name = \"tool\"\n",
            "kind = \"app\"\n",
            "version = \"0.3.0\"\n",
            "template = true\n",
            "description = \"tool {{version}}\"\n",
            "\n[application]\n",
            "id = \"tool\"\n",
            "\n[template.versions]\n",
            "kind = \"json\"\n",
            "url = \"{index}\"\n",
            "select = \"\"\n",
            "version_field = \"version\"\n",
            "strip_prefix = \"v\"\n",
            "\n[template.digests]\n",
            "kind = \"sha256sums\"\n",
            "url = \"{digests}\"\n",
            "\n[[template.channel]]\n",
            "rule = \"truthy\"\n",
            "name = \"lts\"\n",
            "from = \"lts\"\n",
            "\n[[template.channel]]\n",
            "rule = \"value\"\n",
            "prefix = \"lts\"\n",
            "from = \"lts\"\n",
            "\n[[installer_options]]\n",
            "id = \"linux-x64\"\n",
            "name = \"tool {{version}} for Linux\"\n",
            "platform = \"linux\"\n",
            "platform_name = \"Linux\"\n",
            "arch = \"x86_64\"\n",
            "path_dirs = [\"bin\"]\n",
            "\n[[installer_options.steps]]\n",
            "id = \"unpack\"\n",
            "title = \"Unpack tool {{version}}\"\n",
            "kind = \"archive\"\n",
            "format = \"tar_gz\"\n",
            "source = \"https://example.invalid/dist/v{{version}}/tool-{{version}}-{{platform}}-{{arch}}.tar.gz\"\n",
            "sha256 = \"{{sha256:tool-{{version}}-{{platform}}-{{arch}}.tar.gz}}\"\n",
        ),
        index = INDEX_URL,
        digests = DIGEST_URL_TEMPLATE,
    )
}

/// A fixture registry laid out `<root>/<group>/<name>/v<version>/`, with
/// one template revision and whatever concrete versions are asked for.
fn fixture_registry(concrete: &[&str]) -> (TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().to_path_buf();
    let package = root.join("org.example/tool");
    let revision = package.join("v0.3.0");
    std::fs::create_dir_all(&revision).unwrap();
    std::fs::write(revision.join("vibe.toml"), template_toml()).unwrap();
    std::fs::write(revision.join("README.md"), "# tool\n").unwrap();
    for version in concrete {
        let published = package.join(format!("v{version}"));
        std::fs::create_dir_all(&published).unwrap();
        std::fs::write(
            published.join("vibe.toml"),
            format!(
                "[package]\ngroup = \"org.example\"\nname = \"tool\"\nkind = \"app\"\n\
                 version = \"{version}\"\n\n[application]\nid = \"tool\"\n"
            ),
        )
        .unwrap();
    }
    (dir, root)
}

fn served() -> Arc<Served> {
    let mut documents = BTreeMap::new();
    documents.insert(INDEX_URL.to_string(), index_document());
    for version in ["2.1.0", "2.0.0", "1.4.0"] {
        documents.insert(
            DIGEST_URL_TEMPLATE.replace("{version}", version),
            digest_document(version),
        );
    }
    Arc::new(Served::new(documents))
}

fn layer(settings: &Path, posture: Posture, served: &Arc<Served>) -> TemplateLayer {
    TemplateLayer::with_transport(
        settings,
        Duration::from_secs(900),
        posture,
        Box::new(Arc::clone(served)),
    )
}

fn group() -> Group {
    Group::parse("org.example").unwrap()
}

#[test]
fn discovery_splits_revisions_from_concrete_versions() {
    let (_guard, root) = fixture_registry(&["2.0.0"]);
    let settings = tempfile::tempdir().unwrap();
    let documents = served();
    let layer = layer(settings.path(), Posture::Cached, &documents);
    let template = layer
        .discover_in_directory(&root, &group(), "tool")
        .unwrap()
        .expect("the fixture publishes one template revision");
    assert_eq!(template.revision.to_string(), "0.3.0");
    assert_eq!(template.revisions.len(), 1);
    assert_eq!(
        template
            .published
            .iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>(),
        vec!["2.0.0"]
    );
}

#[test]
fn a_package_without_a_template_is_not_one() {
    let dir = tempfile::tempdir().unwrap();
    let plain = dir.path().join("org.example/plain/v1.0.0");
    std::fs::create_dir_all(&plain).unwrap();
    std::fs::write(
        plain.join("vibe.toml"),
        "[package]\ngroup = \"org.example\"\nname = \"plain\"\nkind = \"flow\"\nversion = \"1.0.0\"\n",
    )
    .unwrap();
    let settings = tempfile::tempdir().unwrap();
    let documents = served();
    let layer = layer(settings.path(), Posture::Cached, &documents);
    assert!(
        layer
            .discover_in_directory(dir.path(), &group(), "plain")
            .unwrap()
            .is_none()
    );
}

#[test]
fn the_domain_unions_both_sets_and_concrete_wins() {
    let (_guard, root) = fixture_registry(&["2.0.0"]);
    let settings = tempfile::tempdir().unwrap();
    let documents = served();
    let layer = layer(settings.path(), Posture::Cached, &documents);
    let template = layer
        .discover_in_directory(&root, &group(), "tool")
        .unwrap()
        .unwrap();
    let domain = layer.domain(&template).unwrap();
    assert_eq!(
        domain
            .all()
            .iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>(),
        vec!["1.4.0", "2.0.0", "2.1.0"]
    );
    assert_eq!(
        domain.origin(&"2.1.0".parse().unwrap()),
        Some(Origin::Template)
    );
    assert_eq!(
        domain.origin(&"2.0.0".parse().unwrap()),
        Some(Origin::Overrides),
        "published and produced: the publication wins"
    );
    assert!(!domain.needs_render(&"2.0.0".parse().unwrap()));
    assert!(domain.needs_render(&"2.1.0".parse().unwrap()));
    // The template revision is not a produced version.
    assert!(domain.origin(&"0.3.0".parse().unwrap()).is_none());
    // The unparseable upstream record is skipped with a reason, not an
    // error.
    assert_eq!(domain.skipped().len(), 1);
}

#[test]
fn the_channel_rules_produce_the_two_node_shaped_channels() {
    let (_guard, root) = fixture_registry(&[]);
    let settings = tempfile::tempdir().unwrap();
    let documents = served();
    let layer = layer(settings.path(), Posture::Cached, &documents);
    let template = layer
        .discover_in_directory(&root, &group(), "tool")
        .unwrap()
        .unwrap();
    let domain = layer.domain(&template).unwrap();
    let channels = domain.channels();
    assert_eq!(
        channels["lts"]
            .iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>(),
        vec!["1.4.0", "2.1.0"],
        "`rule = \"truthy\"` admits every version whose field is truthy"
    );
    assert_eq!(
        channels["lts/argon"]
            .iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>(),
        vec!["2.1.0"]
    );
    assert!(channels.contains_key("lts/boron"));
    assert!(
        !channels.keys().any(|name| name == "lts/false"),
        "a falsy field names no channel"
    );
}

#[test]
fn lowering_a_channel_goes_through_the_layer() {
    use vibe_core::channel::lower_channel;

    let (_guard, root) = fixture_registry(&[]);
    let settings = tempfile::tempdir().unwrap();
    let documents = served();
    let layer = layer(settings.path(), Posture::Cached, &documents);
    let template = layer
        .discover_in_directory(&root, &group(), "tool")
        .unwrap()
        .unwrap();
    let channels = crate::templates::domain::TemplateChannels::new(&layer, &template);
    let mut reference = vibe_core::PackageRef::parse("org.example/tool@lts").unwrap();
    let via = lower_channel(&mut reference, &channels).unwrap();
    assert_eq!(via.unwrap().as_str(), "lts");
    assert_eq!(reference.version.to_string(), "=2.1.0");
}

#[test]
fn enumerating_costs_one_request_and_a_second_run_costs_none() {
    let (_guard, root) = fixture_registry(&[]);
    let settings = tempfile::tempdir().unwrap();
    let documents = served();
    {
        let layer = layer(settings.path(), Posture::Cached, &documents);
        let template = layer
            .discover_in_directory(&root, &group(), "tool")
            .unwrap()
            .unwrap();
        layer.domain(&template).unwrap();
        // Resolving against the domain needs no further request.
        layer.domain(&template).unwrap();
    }
    assert_eq!(
        documents.reads.lock().unwrap().len(),
        1,
        "the version list is fetched once and then read from the cache"
    );
    {
        let layer = layer(settings.path(), Posture::Cached, &documents);
        let template = layer
            .discover_in_directory(&root, &group(), "tool")
            .unwrap()
            .unwrap();
        layer.domain(&template).unwrap();
    }
    assert_eq!(
        documents.reads.lock().unwrap().len(),
        1,
        "a second invocation inside the TTL makes zero requests"
    );
}

#[test]
fn installing_one_version_costs_exactly_one_more_request() {
    let (_guard, root) = fixture_registry(&[]);
    let settings = tempfile::tempdir().unwrap();
    let documents = served();
    let layer = layer(settings.path(), Posture::Cached, &documents);
    let template = layer
        .discover_in_directory(&root, &group(), "tool")
        .unwrap()
        .unwrap();
    let domain = layer.domain(&template).unwrap();
    let version: semver::Version = "2.1.0".parse().unwrap();
    let record = domain.record(&version).unwrap();
    // The skeleton path costs nothing.
    let skeleton = layer
        .skeleton_manifest(&template, &version, record)
        .unwrap();
    assert_eq!(
        skeleton.package.as_ref().unwrap().version.to_string(),
        "2.1.0"
    );
    assert_eq!(documents.reads.lock().unwrap().len(), 1);
    // The full render costs one digest request, and only one.
    let full = layer.full(&template, &version, record).unwrap();
    assert_eq!(documents.reads.lock().unwrap().len(), 2);
    layer.full(&template, &version, record).unwrap();
    assert_eq!(
        documents.reads.lock().unwrap().len(),
        2,
        "a digest document is immutable and is never revalidated"
    );
    assert!(full.toml.contains(&"a".repeat(64)));
    assert!(
        !full.toml.contains("template"),
        "no template vocabulary survives"
    );
}

#[test]
fn materialising_writes_an_ordinary_package_directory() {
    let (_guard, root) = fixture_registry(&[]);
    let settings = tempfile::tempdir().unwrap();
    let documents = served();
    let layer = layer(settings.path(), Posture::Cached, &documents);
    let template = layer
        .discover_in_directory(&root, &group(), "tool")
        .unwrap()
        .unwrap();
    let domain = layer.domain(&template).unwrap();
    let version: semver::Version = "2.1.0".parse().unwrap();
    let render = layer
        .full(&template, &version, domain.record(&version).unwrap())
        .unwrap();
    let resolved = layer.materialise(&template, &render).unwrap();
    assert_eq!(resolved.version, version);
    // The template's whole tree came along, with the manifest replaced.
    assert!(resolved.source_dir.join("README.md").is_file());
    let manifest = vibe_core::manifest::Manifest::read(resolved.source_dir.join("vibe.toml"))
        .expect("a materialised render is an ordinary manifest");
    assert_eq!(manifest.package.unwrap().version.to_string(), "2.1.0");
}

#[test]
fn offline_with_a_cold_cache_refuses_by_name_and_by_path() {
    let (_guard, root) = fixture_registry(&[]);
    let settings = tempfile::tempdir().unwrap();
    let documents = served();
    let layer = layer(settings.path(), Posture::Offline, &documents);
    let template = layer
        .discover_in_directory(&root, &group(), "tool")
        .unwrap()
        .unwrap();
    let error = layer.domain(&template).unwrap_err();
    assert_eq!(error.code(), "template_cache_missing");
    let message = error.to_string();
    assert!(message.contains("versions.json"), "{message}");
    assert!(message.contains("org.example/tool"), "{message}");
    assert!(
        documents.reads.lock().unwrap().is_empty(),
        "offline fetches nothing"
    );
}

#[test]
fn offline_with_a_warm_cache_reads_it_whatever_its_age() {
    let (_guard, root) = fixture_registry(&[]);
    let settings = tempfile::tempdir().unwrap();
    let documents = served();
    {
        let layer = layer(settings.path(), Posture::Cached, &documents);
        let template = layer
            .discover_in_directory(&root, &group(), "tool")
            .unwrap()
            .unwrap();
        let domain = layer.domain(&template).unwrap();
        let version: semver::Version = "2.1.0".parse().unwrap();
        layer
            .full(&template, &version, domain.record(&version).unwrap())
            .unwrap();
    }
    let before = documents.reads.lock().unwrap().len();
    // A year later, offline: the cache answers both documents.
    let layer = layer(settings.path(), Posture::Offline, &documents).at_time(2_000_000_000);
    let template = layer
        .discover_in_directory(&root, &group(), "tool")
        .unwrap()
        .unwrap();
    let domain = layer.domain(&template).unwrap();
    let version: semver::Version = "2.1.0".parse().unwrap();
    layer
        .full(&template, &version, domain.record(&version).unwrap())
        .unwrap();
    assert_eq!(documents.reads.lock().unwrap().len(), before);
}

#[test]
fn refresh_refetches_the_version_list_inside_the_ttl() {
    let (_guard, root) = fixture_registry(&[]);
    let settings = tempfile::tempdir().unwrap();
    let documents = served();
    {
        let layer = layer(settings.path(), Posture::Cached, &documents);
        let template = layer
            .discover_in_directory(&root, &group(), "tool")
            .unwrap()
            .unwrap();
        layer.domain(&template).unwrap();
    }
    assert_eq!(documents.reads.lock().unwrap().len(), 1);
    let layer = layer(settings.path(), Posture::Refresh, &documents);
    let template = layer
        .discover_in_directory(&root, &group(), "tool")
        .unwrap()
        .unwrap();
    layer.domain(&template).unwrap();
    assert_eq!(documents.reads.lock().unwrap().len(), 2);
}

#[test]
fn a_changed_digest_document_is_the_alarm_naming_the_differing_field() {
    let (_guard, root) = fixture_registry(&[]);
    let settings = tempfile::tempdir().unwrap();
    let documents = served();
    let version: semver::Version = "2.1.0".parse().unwrap();

    let first = {
        let layer = layer(settings.path(), Posture::Cached, &documents);
        let template = layer
            .discover_in_directory(&root, &group(), "tool")
            .unwrap()
            .unwrap();
        let domain = layer.domain(&template).unwrap();
        let render = layer
            .full(&template, &version, domain.record(&version).unwrap())
            .unwrap();
        layer.materialise(&template, &render).unwrap();
        lock_record(&template, &render, Some(INDEX_URL))
    };

    // Upstream rewrites the digest document and the cached copy is
    // dropped — exactly the state the alarm exists for.
    let mut mutated = Served::new(BTreeMap::new());
    mutated.documents = served().documents.clone();
    mutated.documents.insert(
        DIGEST_URL_TEMPLATE.replace("{version}", "2.1.0"),
        format!("{}  tool-2.1.0-linux-x86_64.tar.gz\n", "b".repeat(64)),
    );
    let cache = TemplateCache::at(&settings.path().join(CACHE_SUBDIR), &group(), "tool");
    std::fs::remove_file(cache.digest_path(&version)).unwrap();

    let mutated = Arc::new(mutated);
    let layer = layer(settings.path(), Posture::Cached, &mutated);
    let template = layer
        .discover_in_directory(&root, &group(), "tool")
        .unwrap()
        .unwrap();
    let domain = layer.domain(&template).unwrap();
    let error = layer
        .verify_recorded(
            &template,
            &version,
            domain.record(&version).unwrap(),
            &first,
        )
        .unwrap_err();
    assert_eq!(error.code(), "template_render_changed");
    let message = error.to_string();
    assert!(message.contains("sha256"), "{message}");
    assert!(message.contains("no --trust-template") || message.contains("--trust-template"));
}

#[test]
fn an_unchanged_render_verifies_and_a_new_revision_alone_is_no_alarm() {
    let (_guard, root) = fixture_registry(&[]);
    let settings = tempfile::tempdir().unwrap();
    let documents = served();
    let version: semver::Version = "2.1.0".parse().unwrap();
    let layer = layer(settings.path(), Posture::Cached, &documents);
    let template = layer
        .discover_in_directory(&root, &group(), "tool")
        .unwrap()
        .unwrap();
    let domain = layer.domain(&template).unwrap();
    let record = domain.record(&version).unwrap();
    let render = layer.full(&template, &version, record).unwrap();
    let mut recorded = lock_record(&template, &render, Some(INDEX_URL));
    layer
        .verify_recorded(&template, &version, record, &recorded)
        .expect("the same digests render the same bytes");
    // The hash binds, not the revision: an older recorded revision with
    // the same produced version and the same digests is not an alarm.
    recorded.revision = "0.1.0".parse().unwrap();
    layer
        .verify_recorded(&template, &version, record, &recorded)
        .expect("a template revision alone is not an alarm");
}
