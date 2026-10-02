//! Observations across the public install source dispatch.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-060#PACKAGE-RESOLUTION-WORK");

use super::*;
use std::sync::{Arc, Mutex};
use vibe_core::manifest::{AuthKind, NamingConvention, RegistrySection};
use vibe_core::progress::{Progress, ProgressEvent, ProgressEventKind, ProgressObserver};
use vibe_registry::{GitBackend, GitError};

#[derive(Default)]
struct Recording(Mutex<Vec<ProgressEvent>>);
impl ProgressObserver for Recording {
    fn observe(&self, event: ProgressEvent) {
        self.0.lock().unwrap().push(event);
    }
}

const MANIFEST: &str =
    "[package]\ngroup = \"org.example\"\nname = \"tool\"\nkind = \"flow\"\nversion = \"1.0.0\"\n";

#[derive(Default)]
struct Backend(Mutex<Vec<String>>);
impl GitBackend for Backend {
    fn bootstrap(&self, url: &str, reference: &str, dest: &Path) -> Result<(), GitError> {
        self.0
            .lock()
            .unwrap()
            .push(format!("bootstrap {url} {reference}"));
        std::fs::create_dir_all(dest.join(".git")).unwrap();
        std::fs::write(dest.join("vibe.toml"), MANIFEST).unwrap();
        Ok(())
    }
    fn update(&self, _dest: &Path, reference: &str) -> Result<(), GitError> {
        self.0.lock().unwrap().push(format!("update {reference}"));
        Ok(())
    }
    fn list_tags(&self, url: &str) -> Result<Vec<String>, GitError> {
        self.0.lock().unwrap().push(format!("tags {url}"));
        Ok(vec!["v1.0.0".into()])
    }
    fn fetch_file_at_ref(
        &self,
        url: &str,
        reference: &str,
        path: &str,
    ) -> Result<Vec<u8>, GitError> {
        self.0
            .lock()
            .unwrap()
            .push(format!("file {url} {reference} {path}"));
        Err(GitError::FileNotFoundInRef {
            url: url.into(),
            refname: reference.into(),
            path: path.into(),
        })
    }
}

fn declared(
    cache: &Path,
    backend: Arc<Backend>,
    progress: Progress,
    empty: bool,
) -> MultiRegistryResolver {
    let sections = if empty {
        vec![]
    } else {
        vec![RegistrySection {
            name: "test".into(),
            url: "git@host:registry".into(),
            r#ref: "main".into(),
            naming: NamingConvention::Fqdn,
            auth: AuthKind::None,
            token_env: None,
            enabled: true,
            index_url: None,
        }]
    };
    MultiRegistryResolver::from_manifest(&sections, &[], &[], cache.to_path_buf(), backend, 3600)
        .unwrap()
        .with_progress(progress)
}

fn source(
    multi: MultiRegistryResolver,
    local: &Path,
    embedded: bool,
    first: bool,
) -> InstallResolver {
    if embedded {
        InstallResolver::Embedded {
            locals: vec![LocalRegistry::new(local).unwrap()],
            project_local_count: 0,
            declared: Some(Box::new(multi)),
            precedence: if first {
                vibe_resolver::EmbeddedPrecedence::EmbeddedFirst
            } else {
                vibe_resolver::EmbeddedPrecedence::EmbeddedLast
            },
            short_circuit: false,
            solver: None,
        }
    } else {
        InstallResolver::Multi(Box::new(multi), None)
    }
}

fn rows(events: &[ProgressEvent]) -> Vec<&ProgressEvent> {
    events
        .iter()
        .filter(|event| matches!(event.kind, ProgressEventKind::Started { .. }))
        .collect()
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-060#PACKAGE-RESOLUTION-WORK"
)]
fn download_lookup_follows_fetch_scope_and_preserves_dispatch_io() {
    let pkg = PackageRef::parse("org.example/tool@=1.0.0").unwrap();
    for embedded in [false, true] {
        for first in [false, true] {
            let mut baseline = None;
            for explicit in [false, true] {
                let temp = tempfile::tempdir().unwrap();
                let local = temp.path().join("local");
                std::fs::create_dir(&local).unwrap();
                let recording = Arc::new(Recording::default());
                let caller = Progress::new(recording.clone()).task("Fetching packages");
                let resolver_recording = Arc::new(Recording::default());
                let backend = Arc::new(Backend::default());
                let resolver = declared(
                    &temp.path().join("cache"),
                    backend.clone(),
                    Progress::new(resolver_recording.clone()),
                    false,
                );
                let s = source(resolver, &local, embedded, first);
                let result = if explicit {
                    s.resolve_and_fetch_with_progress(
                        &pkg,
                        &temp.path().join("store"),
                        None,
                        &caller.progress(),
                    )
                } else {
                    s.resolve_and_fetch(&pkg, &temp.path().join("store"), None)
                }
                .unwrap();
                assert_eq!(result.package_meta().version.to_string(), "1.0.0");
                assert_eq!(result.source_uri, "git@host:registry/org.example.tool.git");
                let signature = (result.content_hash, backend.0.lock().unwrap().clone());
                if let Some(baseline) = &baseline {
                    assert_eq!(&signature, baseline);
                } else {
                    baseline = Some(signature);
                }
                caller.finish();
                let caller_events = recording.0.lock().unwrap();
                let resolver_events = resolver_recording.0.lock().unwrap();
                let events = if explicit {
                    &caller_events
                } else {
                    &resolver_events
                };
                let all_rows = rows(events);
                let lookup = all_rows.iter().find(|event| matches!(&event.kind, ProgressEventKind::Started { label } if label == &format!("Locating package download source for {pkg}"))).unwrap();
                if explicit {
                    assert!(
                        resolver_events.is_empty(),
                        "explicit caller owns every child observation"
                    );
                    let parent = if embedded {
                        all_rows[1].task_id
                    } else {
                        caller.id()
                    };
                    assert_eq!(lookup.parent_id, Some(parent));
                    let fetch = all_rows.iter().find(|event| matches!(&event.kind, ProgressEventKind::Started { label } if label == "Fetching org.example/tool@1.0.0")).unwrap();
                    assert_eq!(fetch.parent_id, Some(parent));
                }
                assert!(!events.iter().any(|event| matches!(
                    event.kind,
                    ProgressEventKind::Failed { .. } | ProgressEventKind::Stopped
                )));
                let calls = backend.0.lock().unwrap();
                assert_eq!(
                    calls
                        .iter()
                        .filter(|call| call.starts_with("tags "))
                        .count(),
                    1
                );
                assert_eq!(
                    calls
                        .iter()
                        .filter(|call| call.starts_with("bootstrap "))
                        .count(),
                    1
                );
            }
        }
    }
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-060#PACKAGE-RESOLUTION-WORK"
)]
fn checkout_lookup_uses_checkout_purpose_on_multi_and_declared_embedded() {
    for embedded in [false, true] {
        let temp = tempfile::tempdir().unwrap();
        let recording = Arc::new(Recording::default());
        let backend = Arc::new(Backend::default());
        let s = source(
            declared(
                &temp.path().join("cache"),
                backend.clone(),
                Progress::new(recording.clone()),
                false,
            ),
            temp.path(),
            embedded,
            true,
        );
        let pkg = PackageRef::parse("org.example/tool@=1.0.0").unwrap();
        let result = s
            .materialise_in_place(&pkg, &temp.path().join("slot"))
            .unwrap();
        assert_eq!(result.source_ref, "v1.0.0");
        assert_eq!(
            result.manifest.package.unwrap().version.to_string(),
            "1.0.0"
        );
        let events = recording.0.lock().unwrap();
        assert_eq!(
            rows(&events)[0].kind,
            ProgressEventKind::Started {
                label: format!("Locating in-place checkout source for {pkg}")
            }
        );
        assert_eq!(
            backend
                .0
                .lock()
                .unwrap()
                .iter()
                .filter(|call| call.starts_with("bootstrap "))
                .count(),
            1
        );
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
fn source_lookup_failure_preserves_error_and_terminalizes_responsible_scope() {
    for embedded in [false, true] {
        let temp = tempfile::tempdir().unwrap();
        let local = temp.path().join("empty");
        std::fs::create_dir(&local).unwrap();
        let backend = Arc::new(Backend::default());
        let recording = Arc::new(Recording::default());
        let caller = Progress::new(recording.clone()).task("Fetching packages");
        let s = source(
            declared(
                &temp.path().join("cache"),
                backend.clone(),
                Progress::default(),
                true,
            ),
            &local,
            embedded,
            false,
        );
        let pkg = PackageRef::parse("org.example/tool@=1.0.0").unwrap();
        let observed = s
            .resolve_and_fetch_with_progress(
                &pkg,
                &temp.path().join("store"),
                None,
                &caller.progress(),
            )
            .unwrap_err();
        let unobserved = s
            .resolve_and_fetch(&pkg, &temp.path().join("store"), None)
            .unwrap_err();
        assert_eq!(observed.to_string(), unobserved.to_string());
        assert!(matches!(observed, RegistryError::UnknownPackage { .. }));
        assert!(backend.0.lock().unwrap().is_empty());
        caller.fail("install failed");
        let events = recording.0.lock().unwrap();
        for row in rows(&events) {
            assert!(events.iter().any(|event| event.task_id == row.task_id
                && matches!(event.kind, ProgressEventKind::Failed { .. })));
            assert!(!events.iter().any(|event| event.task_id == row.task_id
                && matches!(
                    event.kind,
                    ProgressEventKind::Finished | ProgressEventKind::Stopped
                )));
        }
    }
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-060#PACKAGE-RESOLUTION-WORK"
)]
fn explicit_inert_fetch_scope_does_not_leak_to_resolver_observer() {
    let temp = tempfile::tempdir().unwrap();
    let recording = Arc::new(Recording::default());
    let backend = Arc::new(Backend::default());
    let s = source(
        declared(
            &temp.path().join("cache"),
            backend.clone(),
            Progress::new(recording.clone()),
            false,
        ),
        temp.path(),
        true,
        true,
    );
    let pkg = PackageRef::parse("org.example/tool@=1.0.0").unwrap();
    s.resolve_and_fetch_with_progress(&pkg, &temp.path().join("store"), None, &Progress::default())
        .unwrap();
    assert!(recording.0.lock().unwrap().is_empty());
    assert_eq!(
        backend
            .0
            .lock()
            .unwrap()
            .iter()
            .filter(|call| call.starts_with("bootstrap "))
            .count(),
        1
    );
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-060#PACKAGE-RESOLUTION-WORK"
)]
fn embedded_precedence_retains_local_short_circuit_and_provenance() {
    for first in [false, true] {
        let temp = tempfile::tempdir().unwrap();
        let local = temp.path().join("local");
        let package = local.join("org.example/tool/v1.0.0");
        std::fs::create_dir_all(&package).unwrap();
        std::fs::write(package.join("vibe.toml"), MANIFEST).unwrap();
        let recording = Arc::new(Recording::default());
        let backend = Arc::new(Backend::default());
        let s = source(
            declared(
                &temp.path().join("cache"),
                backend.clone(),
                Progress::default(),
                false,
            ),
            &local,
            true,
            first,
        );
        let pkg = PackageRef::parse("org.example/tool@=1.0.0").unwrap();
        let result = s
            .resolve_and_fetch_with_progress(
                &pkg,
                &temp.path().join("store"),
                None,
                &Progress::new(recording.clone()),
            )
            .unwrap();
        assert_eq!(result.is_embedded, first);
        assert_eq!(backend.0.lock().unwrap().is_empty(), first);
        let events = recording.0.lock().unwrap();
        let lookup_count = rows(&events).iter().filter(|event| matches!(&event.kind, ProgressEventKind::Started { label } if label.starts_with("Locating package download source"))).count();
        assert_eq!(lookup_count, usize::from(!first));
        if first {
            assert!(
                events
                    .iter()
                    .any(|event| matches!(event.kind, ProgressEventKind::Skipped { .. }))
            );
        }
    }
}
