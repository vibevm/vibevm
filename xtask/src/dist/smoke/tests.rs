use std::fs;
use std::io::{Cursor, Write};

use semver::Version;
use vibe_publish::{
    BundleDistributionManifest, DistributionAsset, DistributionComponent,
    DistributionComponentName, DistributionSourceArchive, PlatformDistributionFragment,
    sha256_digest,
};
use vibe_wire::generated::publication_subject::{
    PublicationAsset, PublicationComponent, PublicationComponentName, PublicationSourceArchive,
    PublicationTarget,
};

use super::super::archive::{ArchiveEntry, write_zip};
use super::stage_verified_bundle;

const TARGET: &str = "x86_64-unknown-linux-gnu";

fn fixture() -> (Vec<u8>, PlatformDistributionFragment, PublicationTarget) {
    let vibe = b"fixture-vibe".to_vec();
    let index = b"fixture-index".to_vec();
    let source = b"fixture-source".to_vec();
    let components = vec![
        DistributionComponent {
            name: DistributionComponentName::Vibe,
            path: "vibe".into(),
            size: vibe.len() as u64,
            digest: sha256_digest(&vibe),
        },
        DistributionComponent {
            name: DistributionComponentName::VibeIndex,
            path: "vibe-index".into(),
            size: index.len() as u64,
            digest: sha256_digest(&index),
        },
    ];
    let manifest = BundleDistributionManifest::new(
        Version::new(1, 1, 0),
        "a".repeat(40),
        TARGET,
        components.clone(),
        DistributionSourceArchive {
            path: "vibevm-source.zip".into(),
            size: source.len() as u64,
            digest: sha256_digest(&source),
            tree_oid: "b".repeat(40),
        },
    )
    .unwrap();
    let entries = vec![
        ArchiveEntry {
            name: "vibe".into(),
            bytes: vibe.clone(),
            mode: 0o755,
        },
        ArchiveEntry {
            name: "vibe-index".into(),
            bytes: index,
            mode: 0o755,
        },
        ArchiveEntry {
            name: "vibevm-source.zip".into(),
            bytes: source,
            mode: 0o644,
        },
        ArchiveEntry {
            name: "DISTRIBUTION.json".into(),
            bytes: manifest.to_json_bytes().unwrap(),
            mode: 0o644,
        },
        ArchiveEntry {
            name: "README.md".into(),
            bytes: b"readme".to_vec(),
            mode: 0o644,
        },
        ArchiveEntry {
            name: "LICENSE.md".into(),
            bytes: b"license".to_vec(),
            mode: 0o644,
        },
    ];
    let bytes = write_zip(&entries).unwrap();
    let fragment = PlatformDistributionFragment::new(
        DistributionAsset {
            name: format!("vibevm-1.1.0-{TARGET}.zip"),
            size: bytes.len() as u64,
            digest: sha256_digest(&bytes),
        },
        DistributionAsset {
            name: format!("vibe-bootstrap-{TARGET}"),
            size: vibe.len() as u64,
            digest: sha256_digest(&vibe),
        },
        manifest,
    )
    .unwrap();
    let subject = subject_target(&fragment);
    (bytes, fragment, subject)
}

fn subject_target(fragment: &PlatformDistributionFragment) -> PublicationTarget {
    fn asset(value: &DistributionAsset, media: &str) -> PublicationAsset {
        PublicationAsset {
            name: value.name.clone(),
            media_type: media.into(),
            size_bytes: value.size.to_string(),
            sha256: value.digest.clone(),
        }
    }
    let fragment_bytes = fragment.to_json_bytes().unwrap();
    PublicationTarget {
        target: fragment.target.clone(),
        bundle: asset(&fragment.asset, "application/zip"),
        bootstrap: asset(&fragment.bootstrap, "application/octet-stream"),
        fragment: PublicationAsset {
            name: format!("vibevm-1.1.0-{}.fragment.json", fragment.target),
            media_type: "application/json".into(),
            size_bytes: fragment_bytes.len().to_string(),
            sha256: sha256_digest(&fragment_bytes),
        },
        components: fragment
            .bundle
            .components
            .iter()
            .map(|component| PublicationComponent {
                name: match component.name {
                    DistributionComponentName::Vibe => PublicationComponentName::Vibe,
                    DistributionComponentName::VibeIndex => PublicationComponentName::VibeIndex,
                },
                path: component.path.clone(),
                size_bytes: component.size.to_string(),
                sha256: component.digest.clone(),
            })
            .collect(),
        source_archive: PublicationSourceArchive {
            path: fragment.bundle.source_archive.path.clone(),
            size_bytes: fragment.bundle.source_archive.size.to_string(),
            sha256: fragment.bundle.source_archive.digest.clone(),
            tree_oid: fragment.bundle.source_archive.tree_oid.clone(),
        },
    }
}

fn rebind_bundle(
    bytes: &[u8],
    fragment: &mut PlatformDistributionFragment,
    subject: &mut PublicationTarget,
) {
    fragment.asset.size = bytes.len() as u64;
    fragment.asset.digest = sha256_digest(bytes);
    subject.bundle.size_bytes = fragment.asset.size.to_string();
    subject.bundle.sha256 = fragment.asset.digest.clone();
    let fragment_bytes = fragment.to_json_bytes().unwrap();
    subject.fragment.size_bytes = fragment_bytes.len().to_string();
    subject.fragment.sha256 = sha256_digest(&fragment_bytes);
}

#[test]
fn stages_exact_verified_components_in_fresh_home_without_execution() {
    let (bytes, fragment, subject) = fixture();
    let real_home = tempfile::tempdir().unwrap();
    let sentinel = real_home.path().join("sentinel");
    fs::write(&sentinel, b"untouched").unwrap();
    let stage = stage_verified_bundle(&bytes, TARGET, &fragment, &subject).unwrap();
    assert_eq!(stage.target, TARGET);
    assert_eq!(stage.bundle_digest, sha256_digest(&bytes));
    assert_eq!(fs::read(&stage.executable).unwrap(), b"fixture-vibe");
    assert_eq!(
        fs::read(stage.home.join("bin/vibe-index")).unwrap(),
        b"fixture-index"
    );
    assert!(!stage.home.starts_with(real_home.path()));
    assert_eq!(fs::read(&sentinel).unwrap(), b"untouched");
    let held_path = stage.home.clone();
    drop(stage);
    assert!(!held_path.exists());
}

#[test]
fn changed_digest_or_wrong_target_refuses_before_staging() {
    let (bytes, fragment, mut subject) = fixture();
    subject.bundle.sha256 = format!("sha256:{}", "f".repeat(64));
    let refusal = stage_verified_bundle(&bytes, TARGET, &fragment, &subject)
        .err()
        .unwrap();
    assert!(format!("{refusal:#}").contains("SHA-256"));
    let subject = subject_target(&fragment);
    assert!(
        stage_verified_bundle(&bytes, "x86_64-unknown-linux-musl", &fragment, &subject).is_err()
    );
}

#[test]
fn changed_subject_fragment_or_bootstrap_identity_refuses() {
    let (bytes, fragment, mut subject) = fixture();
    subject.fragment.sha256 = format!("sha256:{}", "f".repeat(64));
    assert!(stage_verified_bundle(&bytes, TARGET, &fragment, &subject).is_err());
    subject = subject_target(&fragment);
    subject.bootstrap.sha256 = format!("sha256:{}", "f".repeat(64));
    assert!(stage_verified_bundle(&bytes, TARGET, &fragment, &subject).is_err());
}

#[test]
fn corrupt_zip_with_rebound_digest_refuses_before_staging() {
    let (mut bytes, mut fragment, mut subject) = fixture();
    bytes[0] = b'X';
    rebind_bundle(&bytes, &mut fragment, &mut subject);
    let refusal = stage_verified_bundle(&bytes, TARGET, &fragment, &subject)
        .err()
        .unwrap();
    assert!(format!("{refusal:#}").contains("ZIP"));
}

#[test]
fn traversal_zip_with_rebound_digest_refuses_before_staging() {
    let (bytes, mut fragment, mut subject) = fixture();
    let entries = super::super::archive::read_zip(&bytes).unwrap();
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default();
    for entry in entries {
        writer.start_file(entry.name, options).unwrap();
        writer.write_all(&entry.bytes).unwrap();
    }
    writer.start_file("../escape", options).unwrap();
    writer.write_all(b"escape").unwrap();
    let malicious = writer.finish().unwrap().into_inner();
    rebind_bundle(&malicious, &mut fragment, &mut subject);
    let real_home = tempfile::tempdir().unwrap();
    let outside = real_home.path().join("escape");
    let refusal = stage_verified_bundle(&malicious, TARGET, &fragment, &subject)
        .err()
        .unwrap();
    assert!(format!("{refusal:#}").contains("unsafe ZIP entry path"));
    assert!(!outside.exists());
}
