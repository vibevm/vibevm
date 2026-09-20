use super::*;
use std::sync::{Arc, Mutex};
use tempfile::tempdir;
use vibe_core::progress::{ProgressEvent, ProgressEventKind, ProgressObserver};
use zip::write::SimpleFileOptions;

#[derive(Default)]
struct Recorded(Mutex<Vec<ProgressEvent>>);

impl ProgressObserver for Recorded {
    fn observe(&self, event: ProgressEvent) {
        self.0.lock().unwrap().push(event);
    }
}

fn identity() -> ApplicationIdentity {
    ApplicationIdentity {
        id: "demo".into(),
        package: super::super::model::PackageIdentity {
            group: "org.example".into(),
            name: "demo".into(),
            version: "1.0.0".into(),
        },
        installer_package: super::super::model::PackageIdentity {
            group: "org.example".into(),
            name: "installer".into(),
            version: "1.0.0".into(),
        },
        commands: vec!["demo".into()],
    }
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn fixture(special_manifest: bool) -> (tempfile::TempDir, PathBuf, DistributionTarget) {
    let temp = tempdir().unwrap();
    let archive_path = temp.path().join("bundle.zip");
    let files = [
        ("management/launch.cmd", b"@echo off\r\n".as_slice()),
        ("launchers/demo.cmd", b"@echo off\r\n".as_slice()),
    ];
    let manifest = BundleManifest {
        protocol: BUNDLE_PROTOCOL.into(),
        application: identity(),
        os: "windows".into(),
        arch: "x86_64".into(),
        libc: None,
        source_commit: "a".repeat(40),
        source_tree: format!("sha256-tree/1:{}", "b".repeat(64)),
        management: BundleManagement {
            runtime: "builtin".into(),
            entry: "management/launch.cmd".into(),
        },
        launchers: vec![BundleLauncher {
            command: "demo".into(),
            path: "launchers/demo.cmd".into(),
            destination: "demo.cmd".into(),
        }],
        files: files
            .iter()
            .map(|(path, bytes)| BundleFile {
                path: (*path).into(),
                sha256: digest(bytes),
                size: bytes.len() as u64,
            })
            .collect(),
    };
    let manifest_bytes = serde_json::to_vec(&bundle_manifest_to_wire(&manifest)).unwrap();
    let file = File::create(&archive_path).unwrap();
    let mut zip = zip::ZipWriter::new(file);
    if special_manifest {
        zip.add_symlink(BUNDLE_MANIFEST, "elsewhere", SimpleFileOptions::default())
            .unwrap();
    } else {
        zip.start_file(BUNDLE_MANIFEST, SimpleFileOptions::default())
            .unwrap();
        zip.write_all(&manifest_bytes).unwrap();
    }
    for (path, bytes) in files {
        zip.start_file(path, SimpleFileOptions::default()).unwrap();
        zip.write_all(bytes).unwrap();
    }
    zip.finish().unwrap();
    let size = fs::metadata(&archive_path).unwrap().len();
    let target = DistributionTarget {
        os: "windows".into(),
        arch: "x86_64".into(),
        libc: None,
        format: "zip".into(),
        url: "https://example.invalid/demo.zip".into(),
        sha256: digest(&fs::read(&archive_path).unwrap()),
        size,
        source_commit: "a".repeat(40),
        source_tree: format!("sha256-tree/1:{}", "b".repeat(64)),
    };
    (temp, archive_path, target)
}

#[test]
fn strict_bundle_extracts_only_declared_hashed_files() {
    let (temp, archive, target) = fixture(false);
    let staging = temp.path().join("payload");
    let verified = verify_archive(&archive, &target, &identity(), &staging).unwrap();
    assert_eq!(verified.manifest.application.commands, ["demo"]);
    assert!(staging.join("management/launch.cmd").is_file());
}

#[test]
fn observed_extraction_reports_real_archive_entries_and_completion() {
    let (temp, archive, target) = fixture(false);
    let observer = Arc::new(Recorded::default());
    let progress = Progress::new(observer.clone());
    verify_archive_observed(
        &archive,
        &target,
        &identity(),
        &temp.path().join("payload"),
        &progress,
    )
    .unwrap();
    let events = observer.0.lock().unwrap();
    assert!(matches!(
        &events[0].kind,
        ProgressEventKind::Started { label }
            if label == "Verifying and extracting application distribution"
    ));
    assert!(events.iter().any(|event| matches!(
        event.kind,
        ProgressEventKind::Progress {
            total: Some(3),
            ref unit,
            ..
        } if unit == "entries"
    )));
    assert!(matches!(
        events.last().unwrap().kind,
        ProgressEventKind::Finished
    ));
}

#[test]
fn special_descriptor_is_refused_before_execution() {
    let (temp, archive, target) = fixture(true);
    let error = verify_archive(&archive, &target, &identity(), &temp.path().join("payload"))
        .unwrap_err()
        .to_string();
    assert!(error.contains("manifest entry is invalid"), "{error}");
}

#[test]
fn missing_current_platform_is_typed_absence() {
    let index = DistributionIndex {
        protocol: INDEX_PROTOCOL.into(),
        application: identity(),
        release_tag: "v1.0.0".into(),
        distributions: vec![DistributionTarget {
            os: "plan9".into(),
            arch: "mips".into(),
            libc: None,
            format: "zip".into(),
            url: "https://example.invalid/x.zip".into(),
            sha256: "a".repeat(64),
            size: 1,
            source_commit: "b".repeat(40),
            source_tree: format!("sha256-tree/1:{}", "c".repeat(64)),
        }],
    };
    assert!(matching_target(&index, &identity()).unwrap().is_none());
}

#[test]
fn linux_target_selection_distinguishes_gnu_and_musl() {
    let target = |libc: &str| DistributionTarget {
        os: "linux".into(),
        arch: "x86_64".into(),
        libc: Some(libc.into()),
        format: "zip".into(),
        url: format!("https://example.invalid/{libc}.zip"),
        sha256: "a".repeat(64),
        size: 1,
        source_commit: "b".repeat(40),
        source_tree: format!("sha256-tree/1:{}", "c".repeat(64)),
    };
    let index = DistributionIndex {
        protocol: INDEX_PROTOCOL.into(),
        application: identity(),
        release_tag: "v1.0.0".into(),
        distributions: vec![target("musl"), target("gnu")],
    };
    let selected = matching_target_for(&index, "linux", "x86_64", Some("gnu"))
        .unwrap()
        .unwrap();
    assert_eq!(selected.libc.as_deref(), Some("gnu"));
}
