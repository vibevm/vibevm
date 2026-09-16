use super::*;
use tempfile::tempdir;
use zip::write::SimpleFileOptions;

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
    let manifest_bytes = serde_json::to_vec(&manifest).unwrap();
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
