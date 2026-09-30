use std::collections::BTreeMap;
use std::path::Path;

use semver::Version;
use sha2::{Digest, Sha256};
use vibe_safefs::Project;
use vibe_wire::generated::publication_subject::{
    PublicationSmoke, PublicationSmokeProbe, PublicationSmokeProbeStatus,
};

use super::{
    CommittedInputBytes, PublicationCaptureError, capture_distribution, subject_from_captured,
};
use crate::release_manifest::{
    AggregateDistributionManifest, BundleDistributionManifest,
    DISTRIBUTION_SOURCE_ARCHIVE_FILENAME, DistributionAsset, DistributionComponent,
    DistributionComponentName, DistributionSourceArchive, PlatformDistributionFragment,
    SUPPORTED_DISTRIBUTION_TARGETS,
};

const COMMIT: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const OTHER_COMMIT: &str = "cccccccccccccccccccccccccccccccccccccccc";
const TREE: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

fn sha(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

fn smoke() -> PublicationSmoke {
    PublicationSmoke {
        policy_id: "distribution-smoke/e1".into(),
        policy_epoch: 1,
        probes: SUPPORTED_DISTRIBUTION_TARGETS
            .into_iter()
            .map(|target| PublicationSmokeProbe {
                probe_id: "launch".into(),
                target: target.into(),
                status: PublicationSmokeProbeStatus::Skipped,
                tested_sha256: None,
                runner_ref: None,
                evidence_ref: Some(
                    "reason=not-executed-in-capture-fixture;host=fixture/none".into(),
                ),
            })
            .collect(),
    }
}

fn committed<'a>(lock: &'a [u8], installer: &'a [u8]) -> CommittedInputBytes<'a> {
    CommittedInputBytes {
        commit: COMMIT,
        tree: TREE,
        cargo_lock: lock,
        bash_installer: installer,
        powershell_installer: b"powershell installer",
        cmd_installer: b"cmd installer",
        smoke: smoke(),
    }
}

fn fixture(root: &Path) -> Vec<PlatformDistributionFragment> {
    let mut fragments = Vec::new();
    for target in SUPPORTED_DISTRIBUTION_TARGETS {
        let suffix = if target == "x86_64-pc-windows-msvc" {
            ".exe"
        } else {
            ""
        };
        let vibe = format!("vibe bytes for {target}").into_bytes();
        let index = format!("index bytes for {target}").into_bytes();
        let bundle = format!("fixture bundle for {target}").into_bytes();
        let bootstrap_name = format!("vibe-bootstrap-{target}{suffix}");
        let bundle_name = format!("vibevm-1.1.0-{target}.zip");
        let fragment_name = format!("vibevm-1.1.0-{target}.fragment.json");
        let manifest = BundleDistributionManifest::new(
            Version::new(1, 1, 0),
            COMMIT,
            target,
            vec![
                DistributionComponent {
                    name: DistributionComponentName::Vibe,
                    path: format!("vibe{suffix}"),
                    size: vibe.len() as u64,
                    digest: sha(&vibe),
                },
                DistributionComponent {
                    name: DistributionComponentName::VibeIndex,
                    path: format!("vibe-index{suffix}"),
                    size: index.len() as u64,
                    digest: sha(&index),
                },
            ],
            DistributionSourceArchive {
                path: DISTRIBUTION_SOURCE_ARCHIVE_FILENAME.into(),
                size: b"source archive".len() as u64,
                digest: sha(b"source archive"),
                tree_oid: TREE.into(),
            },
        )
        .unwrap();
        let fragment = PlatformDistributionFragment::new(
            DistributionAsset {
                name: bundle_name.clone(),
                size: bundle.len() as u64,
                digest: sha(&bundle),
            },
            DistributionAsset {
                name: bootstrap_name.clone(),
                size: vibe.len() as u64,
                digest: sha(&vibe),
            },
            manifest,
        )
        .unwrap();
        std::fs::write(root.join(bundle_name), bundle).unwrap();
        std::fs::write(root.join(bootstrap_name), vibe).unwrap();
        std::fs::write(root.join(fragment_name), fragment.to_json_bytes().unwrap()).unwrap();
        fragments.push(fragment);
    }
    fragments
}

fn bytes_under(root: &Path) -> BTreeMap<String, Vec<u8>> {
    walkdir::WalkDir::new(root)
        .into_iter()
        .map(Result::unwrap)
        .filter(|entry| entry.file_type().is_file())
        .map(|entry| {
            (
                entry
                    .path()
                    .strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .into_owned(),
                std::fs::read(entry.path()).unwrap(),
            )
        })
        .collect()
}

#[test]
fn complete_five_target_capture_is_read_only_and_aggregate_byte_identical() {
    let root = tempfile::tempdir().unwrap();
    let fragments = fixture(root.path());
    let before = bytes_under(root.path());
    let project = Project::open(root.path()).unwrap();
    let captured = capture_distribution(&project, "1.1.0").unwrap();
    let first =
        subject_from_captured(&captured, committed(b"committed lock", b"bash installer")).unwrap();
    let second =
        subject_from_captured(&captured, committed(b"committed lock", b"bash installer")).unwrap();
    let expected = AggregateDistributionManifest::from_platforms(fragments)
        .unwrap()
        .to_json_bytes()
        .unwrap();
    assert_eq!(first.aggregate_bytes, expected);
    assert_eq!(first.subject, second.subject);
    assert_eq!(first.subject_digest, second.subject_digest);
    assert_eq!(first.subject.targets.len(), 5);
    assert_eq!(bytes_under(root.path()), before);
}

#[test]
fn changed_bundle_bootstrap_or_missing_target_refuses_capture() {
    for changed in ["bundle", "bootstrap", "missing"] {
        let root = tempfile::tempdir().unwrap();
        fixture(root.path());
        let target = SUPPORTED_DISTRIBUTION_TARGETS[0];
        let name = match changed {
            "bundle" => format!("vibevm-1.1.0-{target}.zip"),
            "bootstrap" => format!("vibe-bootstrap-{target}.exe"),
            "missing" => format!("vibevm-1.1.0-{target}.fragment.json"),
            _ => unreachable!(),
        };
        if changed == "missing" {
            std::fs::remove_file(root.path().join(&name)).unwrap();
        } else {
            std::fs::write(root.path().join(&name), b"changed bytes").unwrap();
        }
        let project = Project::open(root.path()).unwrap();
        let result = capture_distribution(&project, "1.1.0");
        assert!(
            matches!(
                result,
                Err(PublicationCaptureError::Mismatch { .. })
                    | Err(PublicationCaptureError::Missing { .. })
            ),
            "{changed}: {result:?}"
        );
    }
}

#[test]
fn changed_fragment_lock_or_installer_changes_subject_identity() {
    let root = tempfile::tempdir().unwrap();
    fixture(root.path());
    let project = Project::open(root.path()).unwrap();
    let captured = capture_distribution(&project, "1.1.0").unwrap();
    let baseline = subject_from_captured(&captured, committed(b"lock-a", b"installer-a"))
        .unwrap()
        .subject_digest;
    assert!(matches!(
        subject_from_captured(
            &captured,
            CommittedInputBytes {
                commit: OTHER_COMMIT,
                ..committed(b"lock-a", b"installer-a")
            },
        ),
        Err(PublicationCaptureError::Mismatch {
            role: "committed source",
            ..
        })
    ));
    let changed_lock = subject_from_captured(&captured, committed(b"lock-b", b"installer-a"))
        .unwrap()
        .subject_digest;
    let changed_installer = subject_from_captured(&captured, committed(b"lock-a", b"installer-b"))
        .unwrap()
        .subject_digest;
    assert_ne!(baseline, changed_lock);
    assert_ne!(baseline, changed_installer);

    let fragment = root.path().join(format!(
        "vibevm-1.1.0-{}.fragment.json",
        SUPPORTED_DISTRIBUTION_TARGETS[0]
    ));
    let mut bytes = std::fs::read(&fragment).unwrap();
    bytes.extend_from_slice(b"\n");
    std::fs::write(&fragment, bytes).unwrap();
    let changed = capture_distribution(&project, "1.1.0").unwrap();
    let changed_fragment = subject_from_captured(&changed, committed(b"lock-a", b"installer-a"))
        .unwrap()
        .subject_digest;
    assert_ne!(baseline, changed_fragment);
}

#[test]
fn hardlinked_bundle_and_symlinked_fragment_refuse_held_capture() {
    let root = tempfile::tempdir().unwrap();
    fixture(root.path());
    let bundle = root.path().join(format!(
        "vibevm-1.1.0-{}.zip",
        SUPPORTED_DISTRIBUTION_TARGETS[0]
    ));
    std::fs::hard_link(&bundle, root.path().join("bundle-alias.zip")).unwrap();
    let project = Project::open(root.path()).unwrap();
    assert!(matches!(
        capture_distribution(&project, "1.1.0"),
        Err(PublicationCaptureError::Observation { role: "bundle", .. })
    ));

    #[cfg(unix)]
    {
        let root = tempfile::tempdir().unwrap();
        fixture(root.path());
        let fragment = root.path().join(format!(
            "vibevm-1.1.0-{}.fragment.json",
            SUPPORTED_DISTRIBUTION_TARGETS[0]
        ));
        let real = root.path().join("fragment-real.json");
        std::fs::rename(&fragment, &real).unwrap();
        std::os::unix::fs::symlink(&real, &fragment).unwrap();
        let project = Project::open(root.path()).unwrap();
        assert!(matches!(
            capture_distribution(&project, "1.1.0"),
            Err(PublicationCaptureError::Observation {
                role: "fragment",
                ..
            })
        ));
    }
}
