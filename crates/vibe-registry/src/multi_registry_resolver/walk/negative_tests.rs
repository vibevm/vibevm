//! B-199: a remote registry is asked about a package once per run, and the
//! declared order still decides which registry serves it.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-model");

use super::*;
use crate::multi_registry_resolver::test_support::*;
use std::sync::Mutex;
use vibe_core::progress::{Progress, ProgressEvent, ProgressEventKind, ProgressObserver};

#[derive(Default)]
struct Recording(Mutex<Vec<ProgressEvent>>);
impl ProgressObserver for Recording {
    fn observe(&self, event: ProgressEvent) {
        self.0.lock().unwrap().push(event);
    }
}

impl Recording {
    /// How many progress details carry `needle` — the round-trip count the
    /// walk reports for one source.
    fn details(&self, needle: &str) -> usize {
        self.0
            .lock()
            .unwrap()
            .iter()
            .filter(
                |e| matches!(&e.kind, ProgressEventKind::Detail { message } if message == needle),
            )
            .count()
    }
}

/// The canned backend, counting the tag listings each remote URL receives —
/// the request a remote registry without an index answers «not here» with.
#[derive(Default)]
struct Counting {
    fake: FakeBackend,
    tags: Mutex<Vec<String>>,
}
impl Counting {
    fn tag_calls(&self, url: &str) -> usize {
        self.tags
            .lock()
            .unwrap()
            .iter()
            .filter(|u| *u == url)
            .count()
    }
}
impl GitBackend for Counting {
    fn bootstrap(&self, url: &str, reference: &str, dest: &Path) -> Result<(), GitError> {
        self.fake.bootstrap(url, reference, dest)
    }
    fn update(&self, dest: &Path, reference: &str) -> Result<(), GitError> {
        self.fake.update(dest, reference)
    }
    fn list_tags(&self, url: &str) -> Result<Vec<String>, GitError> {
        self.tags.lock().unwrap().push(url.to_string());
        self.fake.list_tags(url)
    }
    fn fetch_file_at_ref(
        &self,
        url: &str,
        reference: &str,
        path: &str,
    ) -> Result<Vec<u8>, GitError> {
        self.fake.fetch_file_at_ref(url, reference, path)
    }
}

const ONE: &str = "git@host:one/org.vibevm.wal.git";
const TWO: &str = "git@host:two/org.vibevm.wal.git";

/// A local directory registry holding `org.vibevm/wal@1.0.0`, and its
/// `file://` url.
fn local_registry() -> (tempfile::TempDir, String) {
    let root = tempfile::tempdir().unwrap();
    let pkg = root.path().join("org.vibevm").join("wal").join("v1.0.0");
    std::fs::create_dir_all(&pkg).unwrap();
    std::fs::write(pkg.join("vibe.toml"), manifest_text("wal", "flow", "1.0.0")).unwrap();
    let path = root.path().display().to_string().replace('\\', "/");
    let url = if path.starts_with('/') {
        format!("file://{path}")
    } else {
        format!("file:///{path}")
    };
    (root, url)
}

/// Two remotes listed before the local registry, observed.
fn resolver(
    cache: &Path,
    local_url: &str,
    backend: Arc<Counting>,
    recording: Arc<Recording>,
) -> MultiRegistryResolver {
    MultiRegistryResolver::from_manifest(
        &[
            registry_section("one", "git@host:one"),
            registry_section("two", "git@host:two"),
            registry_section("local", local_url),
        ],
        &[],
        &[],
        cache.to_path_buf(),
        backend,
        DEFAULT_FRESHNESS_SECS,
    )
    .unwrap()
    .with_progress(Progress::new(recording))
}

/// Every walk one install makes for a package: the version listing, the
/// selection, the metadata read and the download-source lookup.
fn install_walks(r: &MultiRegistryResolver) -> MultiResolution {
    let version = semver::Version::new(1, 0, 0);
    assert_eq!(
        r.list_versions(&org(), "wal").unwrap(),
        vec![version.clone()]
    );
    let pkg = PackageRef::parse("org.vibevm/wal@^1.0").unwrap();
    r.resolve(&pkg).unwrap();
    r.fetch_manifest(&org(), "wal", &version).unwrap();
    r.resolve_for(&pkg, ResolutionPurpose::DownloadSource)
        .unwrap()
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-model"
)]
fn remotes_that_lack_a_local_package_are_asked_once_per_run() {
    let (_root, local_url) = local_registry();
    let cache = tempfile::tempdir().unwrap();
    let backend = Arc::new(Counting::default());
    let recording = Arc::new(Recording::default());
    let r = resolver(cache.path(), &local_url, backend.clone(), recording.clone());

    // Without the memory each of the four walks asked both remotes: four
    // listings apiece. A re-resolution in the same run asks nothing new.
    for _ in 0..2 {
        let served = install_walks(&r);
        assert_eq!(served.registry_name.as_deref(), Some("local"));
        assert_eq!(
            served.source_url,
            crate::local_registry::source_uri_for_local(&served.resolved.source_dir)
        );
        assert!(served.source_url.starts_with(&local_url));
    }
    assert_eq!(backend.tag_calls(ONE), 1);
    assert_eq!(backend.tag_calls(TWO), 1);
    assert_eq!(
        recording.details("checking remote registry one (git@host:one) for org.vibevm/wal"),
        1
    );
    assert_eq!(
        recording.details("checking remote registry two (git@host:two) for org.vibevm/wal"),
        1
    );
    assert_eq!(
        recording.details("remote registry one: not here (answered earlier in this run)"),
        7
    );
    assert_eq!(
        recording.details(&format!("checking local registry local ({local_url})")),
        8
    );
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-model"
)]
fn an_earlier_remote_that_holds_the_package_still_shadows_the_local_one() {
    let (_root, local_url) = local_registry();
    let cache = tempfile::tempdir().unwrap();
    let backend = Arc::new(Counting::default());
    backend.fake.seed_tags(TWO, vec!["v1.0.0".into()]);
    backend.fake.seed_file(
        TWO,
        "v1.0.0",
        "vibe.toml",
        manifest_text("wal", "flow", "1.0.0").into_bytes(),
    );
    let recording = Arc::new(Recording::default());
    let r = resolver(cache.path(), &local_url, backend.clone(), recording.clone());

    let served = install_walks(&r);
    assert_eq!(served.registry_name.as_deref(), Some("two"));
    assert_eq!(served.source_url, TWO);
    // The miss is remembered; the hit is asked every time, as before.
    assert_eq!(backend.tag_calls(ONE), 1);
    assert_eq!(backend.tag_calls(TWO), 4);
    assert_eq!(
        recording.details(&format!("checking local registry local ({local_url})")),
        0
    );
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#failure-discriminator"
)]
fn a_public_refusal_is_remembered_but_a_strict_auth_halt_is_not() {
    // A public refusal needs an `auth = "none"` registry. An SSH-form row with
    // omitted auth now lowers to `ssh` (PROP-002#AUTH-DECLARATION-PRESENCE),
    // so the refusing remote is the anonymous HTTPS spelling of `one`.
    const PUBLIC: &str = "https://host/one/org.vibevm.wal.git";
    let (_root, local_url) = local_registry();
    let pkg = PackageRef::parse("org.vibevm/wal@^1.0").unwrap();
    for strict in [false, true] {
        let cache = tempfile::tempdir().unwrap();
        let backend = Arc::new(Counting::default());
        backend.fake.seed_auth_failure(PUBLIC);
        let mut public = registry_section("one", "https://host/one");
        public.index_url = Some("none".into());
        let r = MultiRegistryResolver::from_manifest(
            &[
                public,
                registry_section("two", "git@host:two"),
                registry_section("local", &local_url),
            ],
            &[],
            &[],
            cache.path().to_path_buf(),
            backend.clone(),
            DEFAULT_FRESHNESS_SECS,
        )
        .unwrap()
        .with_progress(Progress::new(Arc::new(Recording::default())))
        .with_strict_auth(strict);
        for _ in 0..2 {
            let outcome = r.resolve(&pkg);
            if strict {
                assert!(matches!(
                    outcome,
                    Err(RegistryError::Git(GitError::AuthFailed { .. }))
                ));
            } else {
                assert_eq!(outcome.unwrap().registry_name.as_deref(), Some("local"));
            }
        }
        // Walked past on `auth = "none"`: asked once. Halting under strict
        // auth: asked again, so the operator sees the live answer each time.
        assert_eq!(backend.tag_calls(PUBLIC), if strict { 2 } else { 1 });
    }
}
