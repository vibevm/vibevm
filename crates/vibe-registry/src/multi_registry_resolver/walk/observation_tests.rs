//! Full-walk observations with isolated caches and a counting backend.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-060#PACKAGE-RESOLUTION-WORK");

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

#[derive(Default)]
struct CountingBackend {
    fake: FakeBackend,
    calls: Mutex<Vec<String>>,
}
impl GitBackend for CountingBackend {
    fn bootstrap(&self, url: &str, reference: &str, dest: &Path) -> Result<(), GitError> {
        self.calls
            .lock()
            .unwrap()
            .push(format!("bootstrap {url} {reference}"));
        self.fake.bootstrap(url, reference, dest)
    }
    fn update(&self, dest: &Path, reference: &str) -> Result<(), GitError> {
        self.calls
            .lock()
            .unwrap()
            .push(format!("update {reference}"));
        self.fake.update(dest, reference)
    }
    fn list_tags(&self, url: &str) -> Result<Vec<String>, GitError> {
        self.calls.lock().unwrap().push(format!("tags {url}"));
        self.fake.list_tags(url)
    }
    fn fetch_file_at_ref(
        &self,
        url: &str,
        reference: &str,
        path: &str,
    ) -> Result<Vec<u8>, GitError> {
        self.calls
            .lock()
            .unwrap()
            .push(format!("file {url} {reference} {path}"));
        self.fake.fetch_file_at_ref(url, reference, path)
    }
}

const URL: &str = "git@host:registry/org.vibevm.wal.git";
fn resolver(
    cache: &Path,
    backend: Arc<CountingBackend>,
    progress: Progress,
) -> MultiRegistryResolver {
    MultiRegistryResolver::from_manifest(
        &[registry_section("test", "git@host:registry")],
        &[],
        &[],
        cache.to_path_buf(),
        backend,
        DEFAULT_FRESHNESS_SECS,
    )
    .unwrap()
    .with_progress(progress)
}

fn started(events: &[ProgressEvent]) -> Vec<&ProgressEvent> {
    events
        .iter()
        .filter(|event| matches!(event.kind, ProgressEventKind::Started { .. }))
        .collect()
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-060#PACKAGE-RESOLUTION-WORK"
)]
fn purpose_specific_full_walk_preserves_calls_results_and_freshness() {
    let pkg = PackageRef::parse("org.vibevm/wal@^1.0").unwrap();
    let mut baseline = None;
    for purpose in [
        ResolutionPurpose::VersionSelection,
        ResolutionPurpose::DownloadSource,
        ResolutionPurpose::CheckoutSource,
    ] {
        let cache = tempfile::tempdir().unwrap();
        let backend = Arc::new(CountingBackend::default());
        backend.fake.seed_tags(URL, vec!["v1.2.0".into()]);
        let recording = Arc::new(Recording::default());
        let progress = Progress::new(recording.clone());
        let parent = progress.task("fetch packages");
        let r = resolver(cache.path(), backend.clone(), Progress::default());
        for _ in 0..2 {
            let result = r
                .resolve_for_with_progress(&pkg, purpose, &parent.progress())
                .unwrap();
            assert_eq!(result.resolved.version.to_string(), "1.2.0");
            assert_eq!(result.source_url, URL);
        }
        parent.finish();
        let calls = backend.calls.lock().unwrap().clone();
        assert_eq!(
            calls
                .iter()
                .filter(|call| call.starts_with("tags "))
                .count(),
            2,
            "each required lookup retains its existing tag probe"
        );
        if let Some(baseline) = &baseline {
            assert_eq!(&calls, baseline);
        } else {
            baseline = Some(calls);
        }
        let events = recording.0.lock().unwrap();
        let rows = started(&events);
        assert_eq!(rows.len(), 3);
        let prefix = match purpose {
            ResolutionPurpose::VersionSelection => "Selecting version for",
            ResolutionPurpose::DownloadSource => "Locating package download source for",
            ResolutionPurpose::CheckoutSource => "Locating in-place checkout source for",
        };
        for row in &rows[1..] {
            assert_eq!(row.parent_id, Some(parent.id()));
            assert_eq!(
                row.kind,
                ProgressEventKind::Started {
                    label: format!("{prefix} {pkg}")
                }
            );
        }
        assert!(!events.iter().any(|event| matches!(
            event.kind,
            ProgressEventKind::Failed { .. } | ProgressEventKind::Stopped
        )));
    }
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-060#PACKAGE-RESOLUTION-WORK"
)]
fn public_resolve_defaults_to_version_selection_and_inert_progress_preserves_work() {
    let pkg = PackageRef::parse("org.vibevm/wal@=1.0.0").unwrap();
    let mut calls = Vec::new();
    for enabled in [false, true] {
        let cache = tempfile::tempdir().unwrap();
        let backend = Arc::new(CountingBackend::default());
        backend.fake.seed_tags(URL, vec!["v1.0.0".into()]);
        let recording = Arc::new(Recording::default());
        let progress = if enabled {
            Progress::new(recording.clone())
        } else {
            Progress::default()
        };
        assert_eq!(
            resolver(cache.path(), backend.clone(), progress)
                .resolve(&pkg)
                .unwrap()
                .resolved
                .version
                .to_string(),
            "1.0.0"
        );
        calls.push(backend.calls.lock().unwrap().clone());
        let events = recording.0.lock().unwrap();
        if enabled {
            assert_eq!(
                started(&events)[0].kind,
                ProgressEventKind::Started {
                    label: format!("Selecting version for {pkg}")
                }
            );
        } else {
            assert!(events.is_empty());
        }
    }
    assert_eq!(calls[0], calls[1]);
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-060#PACKAGE-RESOLUTION-WORK"
)]
fn metadata_exact_and_pinned_redirect_fallback_have_one_responsible_task() {
    for fallback in [false, true] {
        let cache = tempfile::tempdir().unwrap();
        let backend = Arc::new(CountingBackend::default());
        let recording = Arc::new(Recording::default());
        let version = semver::Version::new(1, 0, 0);
        if fallback {
            backend.fake.seed_tags(URL, vec!["v9.9.9".into()]);
            backend.fake.seed_file(URL, "v9.9.9", "vibe-redirect.toml", b"[redirect]\ntarget_url = \"git@host:target/wal.git\"\nref_policy = \"pinned\"\npinned_ref = \"v1.0.0\"\n".to_vec());
            backend.fake.seed_file(
                "git@host:target/wal.git",
                "v1.0.0",
                "vibe.toml",
                manifest_text("wal", "flow", "1.0.0").into_bytes(),
            );
        } else {
            backend.fake.seed_tags(URL, vec!["v1.0.0".into()]);
            backend.fake.seed_file(
                URL,
                "v1.0.0",
                "vibe.toml",
                manifest_text("wal", "flow", "1.0.0").into_bytes(),
            );
        }
        let r = resolver(
            cache.path(),
            backend.clone(),
            Progress::new(recording.clone()),
        );
        assert_eq!(
            r.fetch_manifest(&org(), "wal", &version)
                .unwrap()
                .package
                .unwrap()
                .version,
            version
        );
        let events = recording.0.lock().unwrap();
        let rows = started(&events);
        assert_eq!(rows.len(), 1);
        assert_eq!(
            rows[0].kind,
            ProgressEventKind::Started {
                label: "Reading package metadata for org.vibevm/wal@1.0.0".into()
            }
        );
        assert!(!events.iter().any(|event| matches!(
            event.kind,
            ProgressEventKind::Failed { .. } | ProgressEventKind::Stopped
        )));
        assert_eq!(events.last().unwrap().kind, ProgressEventKind::Finished);
        let observed_calls = backend.calls.lock().unwrap().clone();
        backend.calls.lock().unwrap().clear();
        let inert_cache = tempfile::tempdir().unwrap();
        let inert = resolver(inert_cache.path(), backend.clone(), Progress::default());
        assert_eq!(
            inert
                .fetch_manifest(&org(), "wal", &version)
                .unwrap()
                .package
                .unwrap()
                .version,
            version
        );
        assert_eq!(
            *backend.calls.lock().unwrap(),
            observed_calls,
            "observation leaves exact/fallback I/O unchanged"
        );
        assert_eq!(events.iter().any(|event| matches!(&event.kind, ProgressEventKind::Detail { message } if message.contains("probing latest"))), fallback);
    }
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-060#PACKAGE-RESOLUTION-WORK"
)]
fn discovery_fallback_and_metadata_failure_keep_original_error_and_own_rows() {
    let cache = tempfile::tempdir().unwrap();
    let backend = Arc::new(CountingBackend::default());
    let recording = Arc::new(Recording::default());
    let r = resolver(
        cache.path(),
        backend.clone(),
        Progress::new(recording.clone()),
    );
    let discovery = r.list_versions(&org(), "wal").unwrap_err();
    assert!(matches!(
        discovery,
        RegistryError::PackageNotFoundEverywhere { .. }
    ));
    let metadata = r
        .fetch_manifest(&org(), "wal", &semver::Version::new(1, 0, 0))
        .unwrap_err();
    assert_eq!(discovery.to_string(), metadata.to_string());
    let events = recording.0.lock().unwrap();
    let rows = started(&events);
    assert_eq!(rows.len(), 2);
    for row in rows {
        let outcomes: Vec<_> = events
            .iter()
            .filter(|event| {
                event.task_id == row.task_id
                    && matches!(
                        event.kind,
                        ProgressEventKind::Failed { .. }
                            | ProgressEventKind::Finished
                            | ProgressEventKind::Stopped
                    )
            })
            .collect();
        assert_eq!(outcomes.len(), 1);
        assert!(matches!(outcomes[0].kind, ProgressEventKind::Failed { .. }));
    }
    // Missing-package discovery still enumerates and probes, metadata still tries exact then latest.
    assert_eq!(backend.calls.lock().unwrap().len(), 4);
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-060#PACKAGE-RESOLUTION-WORK"
)]
fn pinned_discovery_success_reuses_its_row_and_keeps_backend_probes() {
    let cache = tempfile::tempdir().unwrap();
    let backend = Arc::new(CountingBackend::default());
    let recording = Arc::new(Recording::default());
    backend.fake.seed_file(
        URL,
        "v1.0.0",
        "vibe.toml",
        manifest_text("wal", "flow", "1.0.0").into_bytes(),
    );
    let r = MultiRegistryResolver::from_manifest(
        &[],
        &[],
        &[OverrideSection {
            pkgref: "org.vibevm/wal".into(),
            source_url: URL.into(),
            r#ref: Some("v1.0.0".into()),
            reason: None,
        }],
        cache.path().to_path_buf(),
        backend.clone(),
        DEFAULT_FRESHNESS_SECS,
    )
    .unwrap()
    .with_progress(Progress::new(recording.clone()));
    assert_eq!(
        r.list_versions(&org(), "wal").unwrap(),
        vec![semver::Version::new(1, 0, 0)]
    );
    let events = recording.0.lock().unwrap();
    assert_eq!(started(&events).len(), 1);
    assert_eq!(events.last().unwrap().kind, ProgressEventKind::Finished);
    assert!(!events.iter().any(|event| matches!(
        event.kind,
        ProgressEventKind::Failed { .. } | ProgressEventKind::Stopped
    )));
    assert_eq!(
        *backend.calls.lock().unwrap(),
        vec![format!("file {URL} v1.0.0 vibe.toml")]
    );
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-060#PACKAGE-RESOLUTION-WORK"
)]
fn metadata_latest_fallback_refuses_a_different_exact_version_in_its_own_row() {
    let cache = tempfile::tempdir().unwrap();
    let backend = Arc::new(CountingBackend::default());
    let recording = Arc::new(Recording::default());
    backend.fake.seed_tags(URL, vec!["v2.0.0".into()]);
    let r = resolver(
        cache.path(),
        backend.clone(),
        Progress::new(recording.clone()),
    );
    let error = r
        .fetch_manifest(&org(), "wal", &semver::Version::new(1, 0, 0))
        .unwrap_err();
    assert!(matches!(error, RegistryError::NoMatchingVersion { ref req, .. } if req == "=1.0.0"));
    let events = recording.0.lock().unwrap();
    assert_eq!(started(&events).len(), 1);
    assert!(matches!(
        events.last().unwrap().kind,
        ProgressEventKind::Failed { .. }
    ));
    assert_eq!(
        *backend.calls.lock().unwrap(),
        vec![
            format!("tags {URL}"),
            format!("tags {URL}"),
            format!("file {URL} v2.0.0 vibe-redirect.toml"),
        ]
    );
}
